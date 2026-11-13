//! C# Syntax Tree / CIL Ingestion Layer.
//!
//! Parses C# source files and (optionally) compiled CIL bytecode into an
//! internal representation suitable for formal verification passes.
//!
//! The ingestion layer produces a `CSharpProject` containing:
//! - Parsed syntax trees (simplified Roslyn-compatible AST)
//! - Type information (resolved types, nullability annotations)
//! - Method bodies with control-flow structure

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::csharp_engine::EngineError;

/// A fully ingested C# project.
#[derive(Debug)]
pub struct CSharpProject {
    /// Project name (from .csproj or directory).
    pub project_name: String,
    /// All parsed compilation units.
    pub compilation_units: Vec<CompilationUnit>,
    /// Resolved type table: type name → TypeDescriptor.
    pub type_table: HashMap<String, TypeDescriptor>,
    /// Assembly references (for CIL cross-assembly analysis).
    pub assembly_refs: Vec<String>,
}

/// A single .cs file parsed into a compilation unit.
#[derive(Debug)]
pub struct CompilationUnit {
    /// File path relative to project root.
    pub file_path: PathBuf,
    /// Top-level namespace declarations.
    pub namespaces: Vec<NamespaceDecl>,
    /// Using directives.
    pub usings: Vec<String>,
}

/// A namespace declaration containing types.
#[derive(Debug)]
pub struct NamespaceDecl {
    /// Fully qualified namespace (e.g., "MyCompany.MyProduct.Core").
    pub name: String,
    /// Types declared in this namespace.
    pub types: Vec<TypeDecl>,
}

/// A type declaration (class, struct, interface, enum, delegate).
#[derive(Debug)]
pub struct TypeDecl {
    /// Kind of type.
    pub kind: TypeKind,
    /// Simple name.
    pub name: String,
    /// Fully qualified name.
    pub full_name: String,
    /// Base types / interfaces implemented.
    pub base_types: Vec<String>,
    /// Generic type parameters.
    pub type_parameters: Vec<TypeParameter>,
    /// Members (fields, properties, methods).
    pub members: Vec<MemberDecl>,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Class,
    Struct,
    Interface,
    Enum,
    Delegate,
    Record,
}

/// A generic type parameter with variance and constraints.
#[derive(Debug, Clone)]
pub struct TypeParameter {
    pub name: String,
    /// Variance: Covariant, Contravariant, or Invariant.
    pub variance: Variance,
    /// Constraints (e.g., "class", "struct", "new()", base types).
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Variance {
    Covariant,
    Contravariant,
    Invariant,
}

/// A member declaration within a type.
#[derive(Debug)]
pub struct MemberDecl {
    /// Kind of member.
    pub kind: MemberKind,
    /// Simple name.
    pub name: String,
    /// Type signature (simplified).
    pub type_sig: TypeSignature,
    /// Nullability annotation on the member itself.
    pub nullability: NullabilityAnnotation,
    /// Body (for methods/properties with accessors).
    pub body: Option<MethodBody>,
    /// Attributes applied.
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum MemberKind {
    Field,
    Property,
    Method,
    Constructor,
    Operator,
    Event,
}

/// Nullability annotation as found in source.
#[derive(Debug, Clone, PartialEq)]
pub enum NullabilityAnnotation {
    /// Explicitly nullable: `string?`
    Nullable,
    /// Explicitly non-null: `string` (in nullable context) or `string!`
    NonNull,
    /// Unannotated (depends on #nullable context).
    Unannotated,
}

/// A simplified type signature.
#[derive(Debug, Clone)]
pub struct TypeSignature {
    /// The type name (e.g., "string", "List<int>", "Span<byte>").
    pub type_name: String,
    /// Whether the type is a reference type.
    pub is_reference: bool,
    /// Generic arguments (if any).
    pub generic_args: Vec<String>,
}

/// A method body with statements for analysis.
#[derive(Debug)]
pub struct MethodBody {
    /// Parameters.
    pub parameters: Vec<Parameter>,
    /// Local variables.
    pub locals: Vec<LocalVariable>,
    /// Statements in order.
    pub statements: Vec<Statement>,
    /// Return type.
    pub return_type: TypeSignature,
}

/// A method parameter.
#[derive(Debug)]
pub struct Parameter {
    pub name: String,
    pub type_sig: TypeSignature,
    pub nullability: NullabilityAnnotation,
    /// Default value (for optional params).
    pub default_value: Option<String>,
}

/// A local variable declaration.
#[derive(Debug)]
pub struct LocalVariable {
    pub name: String,
    pub type_sig: TypeSignature,
    pub nullability: NullabilityAnnotation,
    /// Whether initialized at declaration.
    pub has_initializer: bool,
}

/// A statement in a method body (simplified for analysis).
#[derive(Debug)]
pub enum Statement {
    /// Variable assignment: `target = expr;`
    Assignment {
        target: String,
        expr: Expression,
    },
    /// Method invocation.
    Invocation {
        receiver: Option<String>,
        method_name: String,
        args: Vec<Expression>,
    },
    /// Array/Span index access: `arr[idx]` or `span[idx]`.
    IndexAccess {
        target: String,
        index: Expression,
        is_span: bool,
    },
    /// If-statement.
    If {
        condition: Box<Expression>,
        then_branch: Vec<Statement>,
        else_branch: Vec<Statement>,
    },
    /// For loop.
    ForLoop {
        init: Option<Box<Statement>>,
        condition: Option<Box<Expression>>,
        increment: Option<Box<Statement>>,
        body: Vec<Statement>,
    },
    /// While loop.
    WhileLoop {
        condition: Box<Expression>,
        body: Vec<Statement>,
    },
    /// Return statement.
    Return(Option<Box<Expression>>),
    /// Throw statement.
    Throw(Box<Expression>),
    /// Try-catch-finally.
    TryCatch {
        try_block: Vec<Statement>,
        catches: Vec<(String, Vec<Statement>)>,
        finally_block: Vec<Statement>,
    },
    /// Expression statement (e.g., a method call as a statement).
    ExprStmt(Expression),
    /// Block of statements.
    Block(Vec<Statement>),
}

/// A simplified expression node.
#[derive(Debug)]
pub enum Expression {
    /// Literal value.
    Literal(String),
    /// Variable reference.
    Variable(String),
    /// Binary operation.
    BinaryOp {
        op: String,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// Unary operation.
    UnaryOp {
        op: String,
        operand: Box<Expression>,
    },
    /// Method call expression.
    Call {
        receiver: Option<String>,
        method: String,
        args: Vec<Expression>,
    },
    /// Index access expression: `arr[i]`.
    Index {
        target: Box<Expression>,
        index: Box<Expression>,
    },
    /// Cast expression.
    Cast {
        type_name: String,
        operand: Box<Expression>,
    },
    /// Null literal.
    Null,
    /// `default(T)` expression.
    Default(TypeSignature),
    /// Object creation: `new T(args)`.
    NewObject {
        type_name: String,
        args: Vec<Expression>,
    },
    /// Span creation: `span.Slice(start, length)` or `stackalloc`.
    SpanCreation {
        source: Option<Box<Expression>>,
        start: Option<Box<Expression>>,
        length: Option<Box<Expression>>,
    },
    /// Conditional expression: `cond ? a : b`.
    Conditional {
        condition: Box<Expression>,
        true_expr: Box<Expression>,
        false_expr: Box<Expression>,
    },
}

/// Type descriptor for the resolved type table.
#[derive(Debug)]
pub struct TypeDescriptor {
    /// Fully qualified name.
    pub full_name: String,
    /// Kind.
    pub kind: TypeKind,
    /// Whether it's a reference type.
    pub is_reference: bool,
    /// Whether it's a Span<T> or Memory<T>.
    pub is_span_like: bool,
    /// Generic arity.
    pub generic_arity: usize,
}

/// Ingest a C# project from the given path.
///
/// This performs:
/// 1. Discovery of .cs files and .csproj
/// 2. Parsing of syntax trees (simplified Roslyn-compatible)
/// 3. Type resolution (basic name-based resolution)
pub fn ingest_project(path: &Path) -> Result<CSharpProject, EngineError> {
    if !path.exists() {
        return Err(EngineError::Config(format!(
            "Target path does not exist: {}",
            path.display()
        )));
    }

    let project_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "UnknownProject".to_string());

    // Discover .cs files
    let cs_files = discover_cs_files(path);
    if cs_files.is_empty() {
        return Err(EngineError::Parse(format!(
            "No .cs files found in {}",
            path.display()
        )));
    }

    // Parse each file into a compilation unit
    let mut compilation_units = Vec::new();
    let mut type_table: HashMap<String, TypeDescriptor> = HashMap::new();

    for file_path in &cs_files {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| EngineError::Parse(format!("Failed to read {}: {e}", file_path.display())))?;

        let unit = parse_compilation_unit(&content, file_path)?;

        // Register types in the type table
        for ns in &unit.namespaces {
            for ty in &ns.types {
                type_table.insert(
                    ty.full_name.clone(),
                    TypeDescriptor {
                        full_name: ty.full_name.clone(),
                        kind: ty.kind.clone(),
                        is_reference: matches!(ty.kind, TypeKind::Class | TypeKind::Interface),
                        is_span_like: ty.name == "Span" || ty.name == "Memory",
                        generic_arity: ty.type_parameters.len(),
                    },
                );
            }
        }

        compilation_units.push(unit);
    }

    // Discover assembly references from .csproj
    let assembly_refs = discover_assembly_refs(path);

    Ok(CSharpProject {
        project_name,
        compilation_units,
        type_table,
        assembly_refs,
    })
}

/// Recursively discover all .cs files under a directory.
fn discover_cs_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if root.is_file() && root.extension().map(|e| e == "cs").unwrap_or(false) {
        files.push(root.to_path_buf());
        return files;
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip common non-source directories
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if name == "obj" || name == "bin" || name == ".git" || name == "node_modules" {
                    continue;
                }
                files.extend(discover_cs_files(&path));
            } else if path.extension().map(|e| e == "cs").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    files
}

/// Discover assembly references from .csproj file.
fn discover_assembly_refs(project_dir: &Path) -> Vec<String> {
    let mut refs = Vec::new();
    // Look for .csproj in the project directory or parent
    let csproj_path = find_csproj(project_dir);
    if let Some(csproj) = csproj_path {
        if let Ok(content) = std::fs::read_to_string(&csproj) {
            // Simple XML-ish parsing for PackageReference and ProjectReference
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.contains("PackageReference") || trimmed.contains("ProjectReference") {
                    // Extract Include="..." attribute
                    if let Some(include_start) = trimmed.find("Include=\"") {
                        let rest = &trimmed[include_start + 9..];
                        if let Some(end) = rest.find('"') {
                            refs.push(rest[..end].to_string());
                        }
                    }
                }
            }
        }
    }
    refs
}

fn find_csproj(dir: &Path) -> Option<PathBuf> {
    // Check current dir, then walk up
    let mut current = Some(dir.to_path_buf());
    while let Some(d) = current {
        for entry in std::fs::read_dir(&d).ok()?.flatten() {
            if entry.path().extension().map(|e| e == "csproj").unwrap_or(false) {
                return Some(entry.path());
            }
        }
        current = d.parent().map(|p| p.to_path_buf());
    }
    None
}

/// Parse a C# compilation unit from source text.
///
/// This is a simplified parser that extracts the structural information
/// needed for verification (types, members, method bodies with statements).
/// It does NOT produce a full Roslyn-compatible AST but captures the
/// semantic structure required by the verification passes.
fn parse_compilation_unit(source: &str, file_path: &Path) -> Result<CompilationUnit, EngineError> {
    let mut usings = Vec::new();
    let mut namespaces = Vec::new();

    // Extract using directives
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("using ") && trimmed.ends_with(';') {
            let ns = trimmed["using ".len()..trimmed.len() - 1].trim().to_string();
            usings.push(ns);
        }
    }

    // Extract namespace blocks (simplified)
    let ns_pattern = "namespace ";
    if let Some(ns_start) = source.find(ns_pattern) {
        let after_ns = &source[ns_start + ns_pattern.len()..];
        // Find the namespace name (up to '{')
        if let Some(brace_pos) = after_ns.find('{') {
            let ns_name = after_ns[..brace_pos].trim().to_string();
            // Extract types within this namespace
            let body_start = ns_start + ns_pattern.len() + brace_pos + 1;
            let body_end = find_matching_brace(source, body_start);
            let body = &source[body_start..body_end.unwrap_or(source.len())];

            let types = parse_types_from_body(body, &ns_name);
            namespaces.push(NamespaceDecl {
                name: ns_name,
                types,
            });
        }
    } else {
        // No namespace — types are in global scope
        let types = parse_types_from_body(source, "");
        if !types.is_empty() {
            namespaces.push(NamespaceDecl {
                name: String::new(),
                types,
            });
        }
    }

    Ok(CompilationUnit {
        file_path: file_path.to_path_buf(),
        namespaces,
        usings,
    })
}

/// Parse type declarations from a namespace body.
fn parse_types_from_body(body: &str, namespace: &str) -> Vec<TypeDecl> {
    let mut types = Vec::new();
    let keywords = ["class ", "struct ", "interface ", "enum ", "record ", "delegate "];

    for keyword in &keywords {
        let kw_len = keyword.len();
        let mut search_from = 0;
        while let Some(pos) = body[search_from..].find(keyword) {
            let abs_pos = search_from + pos;
            // Skip if preceded by a non-word character (avoid matching inside strings/comments)
            if abs_pos > 0 {
                let prev_char = body.as_bytes()[abs_pos - 1];
                if prev_char.is_ascii_alphabetic() || prev_char == b'.' {
                    search_from = abs_pos + kw_len;
                    continue;
                }
            }

            let after_kw = &body[abs_pos + kw_len..];
            // Extract type name
            let name_end = after_kw
                .find(|c: char| c == '{' || c == ' ' || c == '<' || c == ':')
                .unwrap_or(after_kw.len());
            let raw_name = &after_kw[..name_end];
            let name = raw_name.trim().to_string();

            if name.is_empty() {
                search_from = abs_pos + kw_len;
                continue;
            }

            // Determine kind
            let kind = match keyword {
                "class " => TypeKind::Class,
                "struct " => TypeKind::Struct,
                "interface " => TypeKind::Interface,
                "enum " => TypeKind::Enum,
                "record " => TypeKind::Record,
                "delegate " => TypeKind::Delegate,
                _ => TypeKind::Class,
            };

            let full_name = if namespace.is_empty() {
                name.clone()
            } else {
                format!("{namespace}.{name}")
            };

            // Find the body of this type (matching braces)
            let brace_start = after_kw.find('{').map(|p| abs_pos + kw_len + p).unwrap_or(abs_pos);
            let body_end = find_matching_brace(body, brace_start + 1).unwrap_or(body.len());
            let type_body = &body[brace_start + 1..body_end];

            // Parse members from the type body
            let members = parse_members(type_body);

            types.push(TypeDecl {
                kind,
                name: name.clone(),
                full_name,
                base_types: Vec::new(),
                type_parameters: Vec::new(),
                members,
            });

            search_from = body_end;
        }
    }

    types
}

/// Parse members from a type body.
fn parse_members(type_body: &str) -> Vec<MemberDecl> {
    let mut members = Vec::new();

    // Detect methods (simplified: look for access modifiers + return type + name + parens)
    let method_pattern_starts = ["public ", "private ", "protected ", "internal ", "static ", "void ", "int ", "string ", "bool "];

    for line in type_body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('//') || trimmed.starts_with('/*') {
            continue;
        }

        // Try to detect a method signature
        if contains_method_signature(trimmed) {
            let member = parse_method_signature(trimmed);
            if let Some(m) = member {
                members.push(m);
            }
        }
    }

    members
}

fn contains_method_signature(line: &str) -> bool {
    // Heuristic: has a return type, a name, and parentheses
    let has_parens = line.contains('(') && line.contains(')');
    if !has_parens {
        return false;
    }
    // Has at least one access modifier or return type keyword
    let keywords = ["public", "private", "protected", "internal", "static", "void", "int", "string", "bool", "var", "Task", "Span"];
    keywords.iter().any(|kw| line.contains(kw))
}

fn parse_method_signature(line: &str) -> Option<MemberDecl> {
    // Extract method name (word before '(')
    let paren_pos = line.find('(')?;
    let before_paren = &line[..paren_pos];
    let name_start = before_paren.rfind(|c: char| c.is_ascii_alphabetic() || c == '_')?;
    // Walk back to find the start of the identifier
    let mut start = name_start;
    while start > 0 && (before_paren.as_bytes()[start - 1].is_ascii_alphanumeric() || before_paren.as_bytes()[start - 1] == b'_') {
        start -= 1;
    }
    let name = before_paren[start..name_start + 1].to_string();

    // Determine if it's a constructor (no return type before name)
    let is_ctor = !line.contains(" ") || line.starts_with("public") && !line.contains("void");

    Some(MemberDecl {
        kind: if is_ctor { MemberKind::Constructor } else { MemberKind::Method },
        name,
        type_sig: TypeSignature {
            type_name: "void".to_string(),
            is_reference: false,
            generic_args: Vec::new(),
        },
        nullability: NullabilityAnnotation::Unannotated,
        body: None, // Full body parsing would require a proper parser
        attributes: Vec::new(),
    })
}

/// Find the position of the matching closing brace for an opening brace at `start`.
fn find_matching_brace(source: &str, start: usize) -> Option<usize> {
    if start >= source.len() || source.as_bytes()[start] != b'{' {
        return None;
    }
    let mut depth = 0;
    for (i, ch) in source[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + i);
                }
            }
            _ => {}
        }
    }
    None
}
