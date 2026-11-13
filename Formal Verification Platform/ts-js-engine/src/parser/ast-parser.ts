/**
 * AST Parser: Converts TypeScript/JavaScript source into our internal AST.
 * Uses the TypeScript Compiler API for robust parsing, then maps nodes
 * to our simplified internal representation.
 */

import * as ts from "typescript";
import {
  SourceFile,
  Statement,
  Expression,
  ExprKind,
  StatementKind,
  IdentifierNode,
  NumericLiteralNode,
  StringLiteralNode,
  BooleanLiteralNode,
  NullLiteralNode,
  UndefinedLiteralNode,
  ObjectLiteralNode,
  ArrayLiteralNode,
  PropertyAccessNode,
  ElementAccessNode,
  CallExpressionNode,
  BinaryExpressionNode,
  UnaryExpressionNode,
  ConditionalExpressionNode,
  AssignmentExpressionNode,
  NewExpressionNode,
  VariableDeclarationNode,
  FunctionDeclarationNode,
  ClassDeclarationNode,
  PropertyAssignment,
  ParamDecl,
  ClassMember,
} from "./ast-types.js";

export class AstParser {
  private sourceFile: ts.SourceFile;
  private fileName: string;

  constructor(source: string, fileName: string) {
    this.fileName = fileName;
    this.sourceFile = ts.createSourceFile(
      fileName,
      source,
      ts.ScriptTarget.ES2022,
      true,
      fileName.endsWith(".ts") || fileName.endsWith(".tsx")
        ? ts.ScriptKind.TS
        : ts.ScriptKind.JS
    );
  }

  parse(): SourceFile {
    const statements: Statement[] = [];
    for (const stmt of this.sourceFile.statements) {
      const parsed = this.parseStatement(stmt);
      if (parsed) statements.push(parsed);
    }
    return { kind: "SourceFile", fileName: this.fileName, statements };
  }

  private parseStatement(node: ts.Statement): Statement | null {
    const loc = this.getLocation(node);

    switch (node.kind) {
      case ts.SyntaxKind.VariableStatement: {
        const decl = node as ts.VariableStatement;
        const vars: VariableDeclarationNode[] = [];
        for (const d of decl.declarationList.declarations) {
          vars.push(this.parseVariableDeclaration(d));
        }
        return {
          kind: StatementKind.VariableDeclaration,
          loc,
          declarations: vars,
        };
      }
      case ts.SyntaxKind.FunctionDeclaration: {
        const fn = node as ts.FunctionDeclaration;
        return {
          kind: StatementKind.FunctionDeclaration,
          loc,
          ...this.parseFunctionDeclaration(fn),
        };
      }
      case ts.SyntaxKind.ClassDeclaration: {
        const cls = node as ts.ClassDeclaration;
        return {
          kind: StatementKind.ClassDeclaration,
          loc,
          ...this.parseClassDeclaration(cls),
        };
      }
      case ts.SyntaxKind.ImportDeclaration: {
        return {
          kind: StatementKind.ImportDeclaration,
          loc,
          moduleSpecifier: this.getText(node),
        };
      }
      case ts.SyntaxKind.ExportDeclaration: {
        return {
          kind: StatementKind.ExportDeclaration,
          loc,
        };
      }
      case ts.SyntaxKind.ExpressionStatement: {
        const exprStmt = node as ts.ExpressionStatement;
        const expr = this.parseExpression(exprStmt.expression);
        if (!expr) return null;
        return {
          kind: StatementKind.ExpressionStatement,
          loc,
          expression: expr,
        };
      }
      case ts.SyntaxKind.IfStatement: {
        const ifStmt = node as ts.IfStatement;
        return {
          kind: StatementKind.IfStatement,
          loc,
          condition: this.parseExpression(ifStmt.expression),
          consequent: this.parseBlock(ifStmt.thenStatement),
          alternate: ifStmt.elseStatement
            ? this.parseBlock(ifStmt.elseStatement)
            : null,
        };
      }
      case ts.SyntaxKind.WhileStatement: {
        const whileStmt = node as ts.WhileStatement;
        return {
          kind: StatementKind.WhileStatement,
          loc,
          condition: this.parseExpression(whileStmt.expression),
          body: this.parseBlock(whileStmt.statement),
        };
      }
      case ts.SyntaxKind.ForStatement: {
        const forStmt = node as ts.ForStatement;
        return {
          kind: StatementKind.ForStatement,
          loc,
          init: forStmt.initializer
            ? this.parseExpression(forStmt.initializer)
            : null,
          condition: forStmt.condition
            ? this.parseExpression(forStmt.condition)
            : null,
          increment: forStmt.increment
            ? this.parseExpression(forStmt.increment)
            : null,
          body: this.parseBlock(forStmt.statement),
        };
      }
      case ts.SyntaxKind.ForOfStatement: {
        const forOf = node as ts.ForOfStatement;
        return {
          kind: StatementKind.ForOfStatement,
          loc,
          variable: this.getText(forOf.initializer),
          iterable: this.parseExpression(forOf.expression),
          body: this.parseBlock(forOf.statement),
        };
      }
      case ts.SyntaxKind.ReturnStatement: {
        const ret = node as ts.ReturnStatement;
        return {
          kind: StatementKind.ReturnStatement,
          loc,
          expression: ret.expression ? this.parseExpression(ret.expression) : null,
        };
      }
      case ts.SyntaxKind.TryStatement: {
        const tryStmt = node as ts.TryStatement;
        return {
          kind: StatementKind.TryStatement,
          loc,
          block: this.parseBlock(tryStmt.tryBlock),
          catchClause: tryStmt.catchClause
            ? {
                param: this.getText(tryStmt.catchClause.variableDeclaration!),
                block: this.parseBlock(tryStmt.catchClause.block),
              }
            : null,
          finallyBlock: tryStmt.finallyBlock
            ? this.parseBlock(tryStmt.finallyBlock)
            : null,
        };
      }
      default:
        return null;
    }
  }

  private parseVariableDeclaration(node: ts.VariableDeclaration): VariableDeclarationNode {
    const name = (node.name as ts.Identifier).text;
    const typeAnnotation = node.type
      ? this.getText(node.type)
      : undefined;
    const initializer = node.initializer
      ? this.parseExpression(node.initializer)
      : undefined;

    // Determine const/let by walking up to the declaration list
    let isConst = false;
    let isLet = false;
    if (ts.isVariableDeclarationList(node.parent)) {
      isConst = node.parent.flags === ts.NodeFlags.Const;
      isLet = node.parent.flags === ts.NodeFlags.Let;
    }

    return { name, typeAnnotation, initializer, isConst, isLet };
  }

  private parseFunctionDeclaration(node: ts.FunctionDeclaration): FunctionDeclarationNode {
    const name = (node.name as ts.Identifier).text;
    const params: ParamDecl[] = node.parameters.map((p) => ({
      name: (p.name as ts.Identifier).text,
      typeAnnotation: p.type ? this.getText(p.type) : undefined,
      defaultValue: p.initializer ? this.parseExpression(p.initializer) : undefined,
    }));
    const body = node.body ? this.parseBlock(node.body) : [];
    const returnTypeAnnotation = node.type ? this.getText(node.type) : undefined;
    return { name, params, body, returnTypeAnnotation };
  }

  private parseClassDeclaration(node: ts.ClassDeclaration): ClassDeclarationNode {
    const name = (node.name as ts.Identifier).text;
    const extendsExpr = node.heritageClauses
      ?.find((c) => c.token === ts.SyntaxKind.ExtendsKeyword)
      ? this.parseExpression(node.heritageClauses[0].types[0]!)
      : undefined;

    const members: ClassMember[] = [];
    for (const member of node.members) {
      if (ts.isMethodDeclaration(member)) {
        members.push({
          kind: "method",
          name: (member.name as ts.Identifier).text,
          params: member.parameters.map((p) => ({
            name: (p.name as ts.Identifier).text,
            typeAnnotation: p.type ? this.getText(p.type) : undefined,
          })),
          body: member.body ? this.parseBlock(member.body) : [],
        });
      } else if (ts.isPropertyDeclaration(member)) {
        members.push({
          kind: "property",
          name: (member.name as ts.Identifier).text,
          initializer: member.initializer
            ? this.parseExpression(member.initializer)
            : undefined,
        });
      } else if (ts.isGetAccessor(member)) {
        members.push({
          kind: "getter",
          name: (member.name as ts.Identifier).text,
          body: member.body ? this.parseBlock(member.body) : [],
        });
      } else if (ts.isSetAccessor(member)) {
        members.push({
          kind: "setter",
          name: (member.name as ts.Identifier).text,
          params: member.parameters.map((p) => ({
            name: (p.name as ts.Identifier).text,
          })),
          body: member.body ? this.parseBlock(member.body) : [],
        });
      }
    }

    return { name, extendsExpr, members };
  }

  private parseExpression(node: ts.Expression): Expression | null {
    switch (node.kind) {
      case ts.SyntaxKind.Identifier:
        return {
          kind: ExprKind.Identifier,
          loc: this.getLocation(node),
          name: (node as ts.Identifier).text,
        } as IdentifierNode;

      case ts.SyntaxKind.NumericLiteral:
        return {
          kind: ExprKind.NumericLiteral,
          loc: this.getLocation(node),
          value: Number((node as ts.NumericLiteral).text),
        } as NumericLiteralNode;

      case ts.SyntaxKind.StringLiteral:
        return {
          kind: ExprKind.StringLiteral,
          loc: this.getLocation(node),
          value: (node as ts.StringLiteral).text,
        } as StringLiteralNode;

      case ts.SyntaxKind.TrueKeyword:
      case ts.SyntaxKind.FalseKeyword:
        return {
          kind: ExprKind.BooleanLiteral,
          loc: this.getLocation(node),
          value: node.kind === ts.SyntaxKind.TrueKeyword,
        } as BooleanLiteralNode;

      case ts.SyntaxKind.NullKeyword:
        return {
          kind: ExprKind.NullLiteral,
          loc: this.getLocation(node),
        } as NullLiteralNode;

      case ts.SyntaxKind.ObjectLiteralExpression: {
        const obj = node as ts.ObjectLiteralExpression;
        const properties: PropertyAssignment[] = [];
        for (const prop of obj.properties) {
          if (ts.isPropertyAssignment(prop)) {
            const propName = (prop.name as ts.Identifier).text;
            properties.push({
              name: propName,
              value: this.parseExpression(prop.initializer)!,
              isShorthand: false,
            });
          } else if (ts.isShorthandPropertyAssignment(prop)) {
            properties.push({
              name: (prop.name as ts.Identifier).text,
              value: {
                kind: ExprKind.Identifier,
                loc: this.getLocation(prop),
                name: (prop.name as ts.Identifier).text,
              } as IdentifierNode,
              isShorthand: true,
            });
          }
        }
        return {
          kind: ExprKind.ObjectLiteral,
          loc: this.getLocation(node),
          properties,
        } as ObjectLiteralNode;
      }

      case ts.SyntaxKind.ArrayLiteralExpression: {
        const arr = node as ts.ArrayLiteralExpression;
        const elements: Expression[] = [];
        for (const el of arr.elements) {
          const parsed = this.parseExpression(el);
          if (parsed) elements.push(parsed);
        }
        return {
          kind: ExprKind.ArrayLiteral,
          loc: this.getLocation(node),
          elements,
        } as ArrayLiteralNode;
      }

      case ts.SyntaxKind.PropertyAccessExpression: {
        const pa = node as ts.PropertyAccessExpression;
        return {
          kind: ExprKind.PropertyAccess,
          loc: this.getLocation(node),
          object: this.parseExpression(pa.expression)!,
          propertyName: (pa.name as ts.Identifier).text,
        } as PropertyAccessNode;
      }

      case ts.SyntaxKind.ElementAccessExpression: {
        const ea = node as ts.ElementAccessExpression;
        return {
          kind: ExprKind.ElementAccess,
          loc: this.getLocation(node),
          object: this.parseExpression(ea.expression)!,
          index: this.parseExpression(ea.argumentExpression)!,
        } as ElementAccessNode;
      }

      case ts.SyntaxKind.CallExpression: {
        const ce = node as ts.CallExpression;
        return {
          kind: ExprKind.CallExpression,
          loc: this.getLocation(node),
          callee: this.parseExpression(ce.expression)!,
          arguments: ce.arguments.map((a) => this.parseExpression(a)!).filter(Boolean),
        } as CallExpressionNode;
      }

      case ts.SyntaxKind.BinaryExpression: {
        const be = node as ts.BinaryExpression;
        return {
          kind: ExprKind.BinaryExpression,
          loc: this.getLocation(node),
          left: this.parseExpression(be.left)!,
          operator: this.getOperatorText(be.operatorToken),
          right: this.parseExpression(be.right)!,
        } as BinaryExpressionNode;
      }

      case ts.SyntaxKind.PrefixUnaryExpression:
      case ts.SyntaxKind.PostfixUnaryExpression: {
        const ue = node as ts.PrefixUnaryExpression | ts.PostfixUnaryExpression;
        return {
          kind: ExprKind.UnaryExpression,
          loc: this.getLocation(node),
          operator: this.getText(ue.operator),
          operand: this.parseExpression(ue.operand)!,
        } as UnaryExpressionNode;
      }

      case ts.SyntaxKind.ConditionalExpression: {
        const ce = node as ts.ConditionalExpression;
        return {
          kind: ExprKind.ConditionalExpression,
          loc: this.getLocation(node),
          condition: this.parseExpression(ce.condition)!,
          consequent: this.parseExpression(ce.whenTrue)!,
          alternate: this.parseExpression(ce.whenFalse)!,
        } as ConditionalExpressionNode;
      }

      case ts.SyntaxKind.BinaryExpression: {
        const be = node as ts.BinaryExpression;
        if (be.operatorToken.kind === ts.SyntaxKind.EqualsToken) {
          return {
            kind: ExprKind.AssignmentExpression,
            loc: this.getLocation(node),
            left: this.parseExpression(be.left)!,
            operator: "=",
            right: this.parseExpression(be.right)!,
          } as AssignmentExpressionNode;
        }
        break;
      }

      case ts.SyntaxKind.NewExpression: {
        const ne = node as ts.NewExpression;
        return {
          kind: ExprKind.NewExpression,
          loc: this.getLocation(node),
          expression: this.parseExpression(ne.expression)!,
          arguments: ne.arguments?.map((a) => this.parseExpression(a)!).filter(Boolean) ?? [],
        } as NewExpressionNode;

      case ts.SyntaxKind.TemplateExpression:
      case ts.SyntaxKind.NoSubstitutionTemplateLiteral:
        return {
          kind: ExprKind.StringLiteral,
          loc: this.getLocation(node),
          value: this.getText(node),
        } as StringLiteralNode;

      default:
        // Fallback: treat as identifier with raw text
        return {
          kind: ExprKind.Identifier,
          loc: this.getLocation(node),
          name: this.getText(node),
        } as IdentifierNode;
    }
  }

  private parseBlock(block: ts.Block | ts.Statement): Statement[] {
    if (ts.isBlock(block)) {
      return block.statements
        .map((s) => this.parseStatement(s))
        .filter((s): s is Statement => s !== null);
    }
    const single = this.parseStatement(block);
    return single ? [single] : [];
  }

  private getLocation(node: ts.Node): { line: number; column: number; endLine: number; endColumn: number } {
    const pos = this.sourceFile.getLineAndCharacterOfPosition(node.getStart());
    const endPos = this.sourceFile.getLineAndCharacterOfPosition(node.getEnd());
    return {
      line: pos.line + 1,
      column: pos.character,
      endLine: endPos.line + 1,
      endColumn: endPos.character,
    };
  }

  private getText(node: ts.Node): string {
    return this.sourceFile.text.slice(node.getStart(), node.getEnd());
  }

  private getOperatorToken(token: ts.SyntaxKind): string {
    const map: Record<number, string> = {
      [ts.SyntaxKind.PlusToken]: "+",
      [ts.SyntaxKind.MinusToken]: "-",
      [ts.SyntaxKind.AsteriskToken]: "*",
      [ts.SyntaxKind.AsteriskAsteriskToken]: "**",
      [ts.SyntaxKind.SlashToken]: "/",
      [ts.SyntaxKind.PercentToken]: "%",
      [ts.SyntaxKind.EqualsEqualsToken]: "==",
      [ts.SyntaxKind.EqualsEqualsEqualsToken]: "===",
      [ts.SyntaxKind.ExclamationEqualsToken]: "!=",
      [ts.SyntaxKind.ExclamationEqualsEqualsToken]: "!=",
      [ts.SyntaxKind.LessThanToken]: "<",
      [ts.SyntaxKind.GreaterThanToken]: ">",
      [ts.SyntaxKind.LessThanEqualsToken]: "<=",
      [ts.SyntaxKind.GreaterThanEqualsToken]: ">=",
      [ts.SyntaxKind.AmpersandToken]: "&",
      [ts.SyntaxKind.BarToken]: "|",
      [ts.SyntaxKind.CaretToken]: "^",
      [ts.SyntaxKind.LessThanLessThanToken]: "<<",
      [ts.SyntaxKind.GreaterThanGreaterThanToken]: ">>",
      [ts.SyntaxKind.GreaterThanGreaterThanGreaterThanToken]: ">>>",
    };
    return map[token] ?? "?";
  }

  private getOperatorText(token: ts.Token<ts.TokenFlags>): string {
    return this.getOperatorToken(token.kind);
  }
}
