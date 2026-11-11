#!/usr/bin/env python3
"""Keller Formal Verification - Python AST Extractor (Sidecar)

Extracts functions, path conditions (guards), proof obligations, loop induction
bounds, exception handler scopes, and interprocedural call sites from Python source code
into a JSON structure consumed by Keller's Z3 engine.
"""

from __future__ import annotations

import ast
import json
import os
import sys
from dataclasses import dataclass, field, asdict
from typing import Any, List, Optional, Dict, Tuple, Set


def sanitize_ast_ident(node: ast.AST) -> str:
    """Derive a safe identifier token for SMT variables representing expressions."""
    if isinstance(node, ast.Constant):
        s = str(node.value)
        cleaned = "".join(c if c.isalnum() or c == "_" else "_" for c in s).strip("_")
        return cleaned or "val"
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        return f"{sanitize_ast_ident(node.value)}_{node.attr}"
    return "item"


def expr_to_smt_dict(node: ast.AST) -> Dict[str, Any]:
    """Serialize an AST expression into an SMT-translatable JSON structure."""
    if isinstance(node, ast.Constant):
        if isinstance(node.value, bool):
            return {"type": "bool", "val": node.value}
        if isinstance(node.value, int):
            return {"type": "int", "val": str(node.value)}
        if isinstance(node.value, float):
            return {"type": "float", "val": node.value}
        if node.value is None:
            return {"type": "none"}
        return {"type": "str", "val": str(node.value)}

    if isinstance(node, ast.Name):
        return {"type": "var", "name": node.id}

    if isinstance(node, ast.UnaryOp):
        op_name = type(node.op).__name__
        if op_name == "Not":
            return {
                "type": "unary",
                "op": "not",
                "arg": ensure_bool_expr(node.operand),
            }
        op_map = {"USub": "-", "UAdd": "+", "Invert": "~"}
        return {
            "type": "unary",
            "op": op_map.get(op_name, op_name.lower()),
            "arg": expr_to_smt_dict(node.operand),
        }

    if isinstance(node, ast.BinOp):
        op_map = {
            "Add": "+",
            "Sub": "-",
            "Mult": "*",
            "Div": "/",
            "FloorDiv": "div",
            "Mod": "mod",
            "Pow": "^",
            "BitAnd": "&",
            "BitOr": "|",
            "BitXor": "^",
        }
        op_name = type(node.op).__name__
        return {
            "type": "binop",
            "op": op_map.get(op_name, op_name.lower()),
            "left": expr_to_smt_dict(node.left),
            "right": expr_to_smt_dict(node.right),
        }

    if isinstance(node, ast.Compare):
        if len(node.ops) == 1:
            op = node.ops[0]
            comp = node.comparators[0]
            # Pattern: x is None / x == None
            if isinstance(op, (ast.Is, ast.Eq)):
                if isinstance(comp, ast.Constant) and comp.value is None and isinstance(node.left, ast.Name):
                    return {"type": "compare", "op": "==", "left": {"type": "var", "name": f"{node.left.id}_is_none"}, "right": {"type": "int", "val": "1"}}
                if isinstance(node.left, ast.Constant) and node.left.value is None and isinstance(comp, ast.Name):
                    return {"type": "compare", "op": "==", "left": {"type": "var", "name": f"{comp.id}_is_none"}, "right": {"type": "int", "val": "1"}}
            # Pattern: x is not None / x != None
            elif isinstance(op, (ast.IsNot, ast.NotEq)):
                if isinstance(comp, ast.Constant) and comp.value is None and isinstance(node.left, ast.Name):
                    return {"type": "compare", "op": "==", "left": {"type": "var", "name": f"{node.left.id}_is_none"}, "right": {"type": "int", "val": "0"}}
                if isinstance(node.left, ast.Constant) and node.left.value is None and isinstance(comp, ast.Name):
                    return {"type": "compare", "op": "==", "left": {"type": "var", "name": f"{comp.id}_is_none"}, "right": {"type": "int", "val": "0"}}
            # Pattern: key in mapping
            elif isinstance(op, ast.In):
                k_id = sanitize_ast_ident(node.left)
                d_id = sanitize_ast_ident(comp)
                return {"type": "compare", "op": "==", "left": {"type": "var", "name": f"{k_id}_in_{d_id}"}, "right": {"type": "int", "val": "1"}}
            # Pattern: key not in mapping
            elif isinstance(op, ast.NotIn):
                k_id = sanitize_ast_ident(node.left)
                d_id = sanitize_ast_ident(comp)
                return {"type": "compare", "op": "==", "left": {"type": "var", "name": f"{k_id}_in_{d_id}"}, "right": {"type": "int", "val": "0"}}

        op_map = {
            "Eq": "==",
            "NotEq": "!=",
            "Lt": "<",
            "LtE": "<=",
            "Gt": ">",
            "GtE": ">=",
            "Is": "is",
            "IsNot": "is not",
            "In": "in",
            "NotIn": "not in",
        }
        left = expr_to_smt_dict(node.left)
        comparisons = []
        curr = left
        for op, comp in zip(node.ops, node.comparators):
            comp_dict = expr_to_smt_dict(comp)
            comparisons.append({
                "type": "compare",
                "op": op_map.get(type(op).__name__, "=="),
                "left": curr,
                "right": comp_dict,
            })
            curr = comp_dict
        if len(comparisons) == 1:
            return comparisons[0]
        return {"type": "and", "items": comparisons}

    if isinstance(node, ast.BoolOp):
        op_name = "and" if isinstance(node.op, ast.And) else "or"
        return {
            "type": op_name,
            "items": [ensure_bool_expr(val) for val in node.values],
        }

    if isinstance(node, ast.Call):
        func_name = ""
        if isinstance(node.func, ast.Name):
            func_name = node.func.id
        elif isinstance(node.func, ast.Attribute):
            func_name = node.func.attr

        # Special casing for len(x), abs(x), int(x), min(a,b), max(a,b)
        if func_name in ("len", "abs", "int", "min", "max"):
            return {
                "type": "call",
                "fn": func_name,
                "args": [expr_to_smt_dict(a) for a in node.args],
            }
        return {"type": "call_opaque", "fn": func_name}

    if isinstance(node, ast.Attribute):
        return {
            "type": "attr",
            "value": expr_to_smt_dict(node.value),
            "attr": node.attr,
        }

    if isinstance(node, ast.Subscript):
        return {
            "type": "subscript",
            "value": expr_to_smt_dict(node.value),
            "slice": expr_to_smt_dict(node.slice),
        }

    return {"type": "unsupported", "ast": type(node).__name__}


def ensure_bool_expr(node: ast.AST, optional_vars: Optional[Set[str]] = None) -> Dict[str, Any]:
    """Ensure an expression in a boolean context (if/while/assert) evaluates to a boolean."""
    if optional_vars:
        if isinstance(node, ast.Name) and node.id in optional_vars:
            return {"type": "compare", "op": "==", "left": {"type": "var", "name": f"{node.id}_is_none"}, "right": {"type": "int", "val": "0"}}
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.Not) and isinstance(node.operand, ast.Name) and node.operand.id in optional_vars:
            return {"type": "compare", "op": "==", "left": {"type": "var", "name": f"{node.operand.id}_is_none"}, "right": {"type": "int", "val": "1"}}

    expr = expr_to_smt_dict(node)
    if expr["type"] in ("compare", "and", "or", "bool"):
        return expr
    if expr["type"] == "unary" and expr.get("op") == "not":
        return expr
    return {"type": "compare", "op": "!=", "left": expr, "right": {"type": "int", "val": "0"}}


def matches_exception(kind: str, caught_type: str) -> bool:
    """Check whether a caught exception type covers a formal defect kind."""
    if caught_type in ("BaseException", "Exception", "*"):
        return True
    if kind == "div_zero":
        return caught_type in ("ZeroDivisionError", "ArithmeticError")
    if kind == "index_bounds":
        return caught_type in ("IndexError", "LookupError")
    if kind == "assert":
        return caught_type in ("AssertionError",)
    if kind == "none_deref":
        return caught_type in ("AttributeError", "TypeError")
    if kind == "key_error":
        return caught_type in ("KeyError", "LookupError")
    if kind == "type_error":
        return caught_type in ("TypeError",)
    if kind == "stop_iteration":
        return caught_type in ("StopIteration",)
    if kind == "eval_exec":
        return caught_type in ("SyntaxError", "ValueError")
    if kind == "unpacking_error":
        return caught_type in ("ValueError",)
    if kind == "format_error":
        return caught_type in ("TypeError", "ValueError")
    if kind == "concurrency_race":
        return caught_type in ("RuntimeError", "Exception", "BaseException")
    return False


def get_caught_types(handler: ast.ExceptHandler) -> List[str]:
    """Extract list of exception names caught by an except clause."""
    if handler.type is None:
        return ["BaseException"]
    if isinstance(handler.type, ast.Name):
        return [handler.type.id]
    if isinstance(handler.type, ast.Attribute):
        return [handler.type.attr]
    if isinstance(handler.type, ast.Tuple):
        types: List[str] = []
        for elt in handler.type.elts:
            if isinstance(elt, ast.Name):
                types.append(elt.id)
            elif isinstance(elt, ast.Attribute):
                types.append(elt.attr)
        return types if types else ["BaseException"]
    return ["BaseException"]


@dataclass
class ObligationItem:
    id: str
    file: str
    module: str
    function: str
    line: int
    col: int
    kind: str  # div_zero, index_bounds, assert, none_deref
    guards: List[Dict[str, Any]]
    target: Dict[str, Any]
    params: Dict[str, str]  # param_name -> type_annotation
    source_line: str = ""
    handled: bool = False
    handler: Optional[str] = None
    loop_bounds: List[Dict[str, Any]] = field(default_factory=list)


@dataclass
class LoopBoundItem:
    module: str
    function: str
    line: int
    variable: str
    kind: str  # "for_range", "for_enumerate", "while"
    lower: Optional[Dict[str, Any]]
    upper: Optional[Dict[str, Any]]
    step: Optional[Dict[str, Any]] = None


@dataclass
class ExceptionHandlerScope:
    module: str
    function: str
    start_line: int
    end_line: int
    caught_exceptions: List[str]
    handlers_lines: List[int] = field(default_factory=list)


@dataclass
class CallSiteInfo:
    caller_module: str
    caller_function: str
    callee: str
    line: int
    col: int
    args: List[Dict[str, Any]]
    guards: List[Dict[str, Any]]
    kwargs: Dict[str, Dict[str, Any]] = field(default_factory=dict)


class FunctionPathVisitor:
    def __init__(
        self,
        rel_path: str,
        module_name: str,
        fn_name: str,
        params: Dict[str, str],
        source_lines: List[str],
    ):
        self.rel_path = rel_path
        self.module_name = module_name
        self.fn_name = fn_name
        self.params = params
        self.source_lines = source_lines
        self.obligations: List[ObligationItem] = []
        self.call_sites: List[CallSiteInfo] = []
        self.loop_bounds: List[LoopBoundItem] = []
        self.exception_scopes: List[ExceptionHandlerScope] = []
        self._active_handlers: List[List[str]] = []
        self._active_loop_bounds: List[Dict[str, Any]] = []
        self._check_counter = 0

        self.known_dicts: Set[str] = set()
        self.known_sequences: Set[str] = set()
        self.known_scalars: Set[str] = set()
        self.optional_vars: Set[str] = set()
        self.recent_assigns: Dict[str, Dict[str, Any]] = {}
        self._active_locks: List[str] = []

        for p_name, p_ann in self.params.items():
            ann_lower = p_ann.lower()
            if any(opt_kw in p_ann for opt_kw in ("Optional", "None", "| None", "None |")):
                self.optional_vars.add(p_name)
            if any(d_kw in ann_lower for d_kw in ("dict", "mapping")):
                self.known_dicts.add(p_name)
            elif any(s_kw in ann_lower for s_kw in ("list", "sequence", "tuple", "set")):
                self.known_sequences.add(p_name)
            elif any(num_kw in ann_lower for num_kw in ("int", "float", "bool")):
                self.known_scalars.add(p_name)

    def _get_source_line(self, lineno: int) -> str:
        if 1 <= lineno <= len(self.source_lines):
            return self.source_lines[lineno - 1].strip()
        return ""

    def _next_id(self, kind: str, lineno: int) -> str:
        self._check_counter += 1
        safe_fn = self.fn_name.replace(".", "_")
        return f"{self.module_name}_{safe_fn}_{kind}_L{lineno}_{self._check_counter}"

    def add_obligation(
        self,
        kind: str,
        lineno: int,
        col: int,
        guards: List[Dict[str, Any]],
        target: Dict[str, Any],
    ) -> ObligationItem:
        obl_id = self._next_id(kind, lineno)

        # Check if enclosed in a matching exception handler
        is_handled = False
        matching_handler: Optional[str] = None
        for scope_handlers in reversed(self._active_handlers):
            for exc in scope_handlers:
                if matches_exception(kind, exc):
                    is_handled = True
                    matching_handler = exc
                    break
            if is_handled:
                break

        obl = ObligationItem(
            id=obl_id,
            file=self.rel_path,
            module=self.module_name,
            function=self.fn_name,
            line=lineno,
            col=col,
            kind=kind,
            guards=list(guards),
            target=target,
            params=dict(self.params),
            source_line=self._get_source_line(lineno),
            handled=is_handled,
            handler=matching_handler,
            loop_bounds=list(self._active_loop_bounds),
        )
        self.obligations.append(obl)
        return obl

    def collect_call_sites(self, node: ast.AST, guards: List[Dict[str, Any]]):
        for n in ast.walk(node):
            if isinstance(n, ast.Call):
                callee = ""
                if isinstance(n.func, ast.Name):
                    callee = n.func.id
                elif isinstance(n.func, ast.Attribute):
                    callee = n.func.attr
                if not callee or callee in ("range", "enumerate"):
                    continue
                args_smt = [expr_to_smt_dict(a) for a in n.args]
                kwargs_smt = {kw.arg: expr_to_smt_dict(kw.value) for kw in n.keywords if kw.arg}
                self.call_sites.append(
                    CallSiteInfo(
                        caller_module=self.module_name,
                        caller_function=self.fn_name,
                        callee=callee,
                        line=getattr(n, "lineno", 0),
                        col=getattr(n, "col_offset", 0),
                        args=args_smt,
                        guards=list(guards),
                        kwargs=kwargs_smt,
                    )
                )

    def _extract_while_bounds(self, stmt: ast.While) -> List[LoopBoundItem]:
        bounds: List[LoopBoundItem] = []
        if isinstance(stmt.test, ast.Compare):
            node = stmt.test

            def find_step(target_var: str) -> Dict[str, Any]:
                for body_stmt in stmt.body:
                    if isinstance(body_stmt, ast.AugAssign) and isinstance(body_stmt.target, ast.Name) and body_stmt.target.id == target_var:
                        if isinstance(body_stmt.op, ast.Add):
                            return expr_to_smt_dict(body_stmt.value)
                        elif isinstance(body_stmt.op, ast.Sub):
                            val = expr_to_smt_dict(body_stmt.value)
                            return {"type": "unary", "op": "-", "arg": val}
                return {"type": "int", "val": "1"}

            if len(node.ops) == 1 and len(node.comparators) == 1:
                op = node.ops[0]
                comp = node.comparators[0]
                # Standard comparison: i < N or i <= N
                if isinstance(node.left, ast.Name):
                    var_name = node.left.id
                    step = find_step(var_name)
                    if isinstance(op, (ast.Lt, ast.LtE)):
                        lower = self.recent_assigns.get(var_name)
                        if lower is None:
                            lower = {"type": "int", "val": "0"}
                        bounds.append(
                            LoopBoundItem(
                                module=self.module_name,
                                function=self.fn_name,
                                line=stmt.lineno,
                                variable=var_name,
                                kind="while",
                                lower=lower,
                                upper=expr_to_smt_dict(comp),
                                step=step,
                            )
                        )
                    elif isinstance(op, (ast.Gt, ast.GtE)):
                        upper = self.recent_assigns.get(var_name)
                        bounds.append(
                            LoopBoundItem(
                                module=self.module_name,
                                function=self.fn_name,
                                line=stmt.lineno,
                                variable=var_name,
                                kind="while",
                                lower=expr_to_smt_dict(comp),
                                upper=upper,
                                step=step,
                            )
                        )
                # Symmetric comparison: N > i or len(items) > idx
                elif isinstance(comp, ast.Name):
                    var_name = comp.id
                    step = find_step(var_name)
                    if isinstance(op, (ast.Gt, ast.GtE)):
                        lower = self.recent_assigns.get(var_name)
                        if lower is None:
                            lower = {"type": "int", "val": "0"}
                        bounds.append(
                            LoopBoundItem(
                                module=self.module_name,
                                function=self.fn_name,
                                line=stmt.lineno,
                                variable=var_name,
                                kind="while",
                                lower=lower,
                                upper=expr_to_smt_dict(node.left),
                                step=step,
                            )
                        )
                    elif isinstance(op, (ast.Lt, ast.LtE)):
                        upper = self.recent_assigns.get(var_name)
                        bounds.append(
                            LoopBoundItem(
                                module=self.module_name,
                                function=self.fn_name,
                                line=stmt.lineno,
                                variable=var_name,
                                kind="while",
                                lower=expr_to_smt_dict(node.left),
                                upper=upper,
                                step=step,
                            )
                        )
            elif len(node.ops) == 2 and len(node.comparators) == 2:
                # e.g. 0 <= i < stop
                if (
                    isinstance(node.comparators[0], ast.Name)
                    and isinstance(node.ops[0], (ast.Lt, ast.LtE))
                    and isinstance(node.ops[1], (ast.Lt, ast.LtE))
                ):
                    var_name = node.comparators[0].id
                    step = find_step(var_name)
                    bounds.append(
                        LoopBoundItem(
                            module=self.module_name,
                            function=self.fn_name,
                            line=stmt.lineno,
                            variable=var_name,
                            kind="while",
                            lower=expr_to_smt_dict(node.left),
                            upper=expr_to_smt_dict(node.comparators[1]),
                            step=step,
                        )
                    )
        return bounds

    def visit_stmts(self, stmts: List[ast.stmt], guards: List[Dict[str, Any]]):
        current_guards = list(guards)
        for stmt in stmts:
            if isinstance(stmt, ast.If):
                self.collect_call_sites(stmt.test, current_guards)
                test_expr = ensure_bool_expr(stmt.test, self.optional_vars)
                # True branch: current_guards + [test_expr]
                self.visit_stmts(stmt.body, current_guards + [test_expr])
                # False branch: current_guards + [not(test_expr)]
                neg_test = {"type": "unary", "op": "not", "arg": test_expr}
                self.visit_stmts(stmt.orelse, current_guards + [neg_test])

            elif isinstance(stmt, ast.Assert):
                self.collect_call_sites(stmt.test, current_guards)
                test_expr = ensure_bool_expr(stmt.test, self.optional_vars)
                fail_cond = {"type": "unary", "op": "not", "arg": test_expr}
                self.add_obligation("assert", stmt.lineno, stmt.col_offset, current_guards, fail_cond)
                # After assert, condition holds in this path
                current_guards.append(test_expr)
                self.check_expr(stmt.test, stmt.lineno, stmt.col_offset, current_guards)

            elif isinstance(stmt, ast.Assign):
                self.collect_call_sites(stmt.value, current_guards)

                # Check for dict initialization: d = {} or d = dict()
                if isinstance(stmt.value, ast.Dict) or (
                    isinstance(stmt.value, ast.Call)
                    and isinstance(stmt.value.func, ast.Name)
                    and stmt.value.func.id == "dict"
                ):
                    for target in stmt.targets:
                        if isinstance(target, ast.Name):
                            self.known_dicts.add(target.id)

                for target in stmt.targets:
                    if isinstance(target, ast.Name):
                        val_expr = expr_to_smt_dict(stmt.value)
                        self.recent_assigns[target.id] = val_expr
                        current_guards.append({
                            "type": "compare",
                            "op": "==",
                            "left": {"type": "var", "name": target.id},
                            "right": val_expr,
                        })
                        if isinstance(stmt.value, ast.Constant) and stmt.value.value is None:
                            self.optional_vars.add(target.id)
                            current_guards.append({
                                "type": "compare",
                                "op": "==",
                                "left": {"type": "var", "name": f"{target.id}_is_none"},
                                "right": {"type": "int", "val": "1"},
                            })
                        elif isinstance(stmt.value, (ast.Constant, ast.Dict, ast.List, ast.Tuple, ast.Set)):
                            current_guards.append({
                                "type": "compare",
                                "op": "==",
                                "left": {"type": "var", "name": f"{target.id}_is_none"},
                                "right": {"type": "int", "val": "0"},
                            })
                    elif isinstance(target, (ast.Tuple, ast.List)):
                        # P1: Unpacking mismatch (ValueError)
                        has_starred = any(isinstance(elt, ast.Starred) for elt in target.elts)
                        expected_len = len(target.elts)
                        if isinstance(stmt.value, (ast.Tuple, ast.List)):
                            actual_len = len(stmt.value.elts)
                            is_mismatch = (actual_len < expected_len - 1) if has_starred else (actual_len != expected_len)
                            if is_mismatch:
                                target_cond = {"type": "compare", "op": "==", "left": {"type": "var", "name": "unpacking_mismatch"}, "right": {"type": "int", "val": 1}}
                                self.add_obligation("unpacking_error", stmt.lineno, stmt.col_offset, current_guards, target_cond)
                        else:
                            # Dynamic sequence: SMT target condition len(seq) != expected_len
                            seq_expr = expr_to_smt_dict(stmt.value)
                            len_expr = {"type": "call", "fn": "len", "args": [seq_expr]}
                            if has_starred:
                                target_cond = {"type": "compare", "op": "<", "left": len_expr, "right": {"type": "int", "val": str(expected_len - 1)}}
                            else:
                                target_cond = {"type": "compare", "op": "!=", "left": len_expr, "right": {"type": "int", "val": str(expected_len)}}
                            self.add_obligation("unpacking_error", stmt.lineno, stmt.col_offset, current_guards, target_cond)
                    elif isinstance(target, ast.Subscript):
                        d_name = sanitize_ast_ident(target.value)
                        k_name = sanitize_ast_ident(target.slice)
                        key_var = f"{k_name}_in_{d_name}"
                        current_guards.append({
                            "type": "compare",
                            "op": "==",
                            "left": {"type": "var", "name": key_var},
                            "right": {"type": "int", "val": "1"},
                        })
                self.check_stmt_expressions(stmt, current_guards)

            elif isinstance(stmt, (ast.Return, ast.AnnAssign, ast.AugAssign, ast.Expr)):
                self.collect_call_sites(stmt, current_guards)
                self.check_stmt_expressions(stmt, current_guards)

            elif isinstance(stmt, ast.Try):
                # Collect all caught types across all except handlers
                all_caught: List[str] = []
                handler_lines: List[int] = []
                for h in stmt.handlers:
                    all_caught.extend(get_caught_types(h))
                    handler_lines.append(h.lineno)

                scope = ExceptionHandlerScope(
                    module=self.module_name,
                    function=self.fn_name,
                    start_line=stmt.lineno,
                    end_line=getattr(stmt, "end_lineno", stmt.lineno),
                    caught_exceptions=list(all_caught),
                    handlers_lines=handler_lines,
                )
                self.exception_scopes.append(scope)

                # Push caught types onto active handlers stack for body
                self._active_handlers.append(all_caught)
                self.visit_stmts(stmt.body, current_guards)
                self._active_handlers.pop()

                # Process handlers
                for h in stmt.handlers:
                    self.visit_stmts(h.body, current_guards)

                if stmt.orelse:
                    self.visit_stmts(stmt.orelse, current_guards)

                if stmt.finalbody:
                    self.visit_stmts(stmt.finalbody, current_guards)

            elif isinstance(stmt, (ast.With, ast.AsyncWith)):
                lock_names = []
                for item in stmt.items:
                    expr = item.context_expr
                    l_name = ""
                    if isinstance(expr, ast.Name):
                        l_name = expr.id
                    elif isinstance(expr, ast.Attribute):
                        l_name = expr.attr
                    elif isinstance(expr, ast.Call):
                        if isinstance(expr.func, ast.Name):
                            l_name = expr.func.id
                        elif isinstance(expr.func, ast.Attribute):
                            l_name = expr.func.attr
                    if l_name and "lock" in l_name.lower():
                        lock_names.append(l_name)

                for ln in lock_names:
                    self._active_locks.append(ln)

                self.check_stmt_expressions(stmt, current_guards)
                self.visit_stmts(stmt.body, current_guards)

                for _ in lock_names:
                    self._active_locks.pop()

            elif isinstance(stmt, ast.For):
                self.collect_call_sites(stmt.iter, current_guards)
                bound_guards = []
                extracted_bound: Optional[LoopBoundItem] = None

                # Pattern 1: for i in range(...)
                if (
                    isinstance(stmt.iter, ast.Call)
                    and isinstance(stmt.iter.func, ast.Name)
                    and stmt.iter.func.id == "range"
                    and isinstance(stmt.target, ast.Name)
                ):
                    var_name = stmt.target.id
                    r_args = stmt.iter.args
                    if len(r_args) == 1:
                        lower = {"type": "int", "val": "0"}
                        upper = expr_to_smt_dict(r_args[0])
                        step = {"type": "int", "val": "1"}
                    elif len(r_args) == 2:
                        lower = expr_to_smt_dict(r_args[0])
                        upper = expr_to_smt_dict(r_args[1])
                        step = {"type": "int", "val": "1"}
                    elif len(r_args) >= 3:
                        lower = expr_to_smt_dict(r_args[0])
                        upper = expr_to_smt_dict(r_args[1])
                        step = expr_to_smt_dict(r_args[2])
                    else:
                        lower, upper, step = None, None, None

                    if lower is not None and upper is not None:
                        bound_guards = [
                            {"type": "compare", "op": ">=", "left": {"type": "var", "name": var_name}, "right": lower},
                            {"type": "compare", "op": "<", "left": {"type": "var", "name": var_name}, "right": upper},
                        ]
                        extracted_bound = LoopBoundItem(
                            module=self.module_name,
                            function=self.fn_name,
                            line=stmt.lineno,
                            variable=var_name,
                            kind="for_range",
                            lower=lower,
                            upper=upper,
                            step=step,
                        )

                # Pattern 2: for i, item in enumerate(seq)
                elif (
                    isinstance(stmt.iter, ast.Call)
                    and isinstance(stmt.iter.func, ast.Name)
                    and stmt.iter.func.id == "enumerate"
                    and stmt.iter.args
                    and isinstance(stmt.target, (ast.Tuple, ast.List))
                    and len(stmt.target.elts) >= 1
                    and isinstance(stmt.target.elts[0], ast.Name)
                ):
                    var_name = stmt.target.elts[0].id
                    seq_expr = expr_to_smt_dict(stmt.iter.args[0])
                    lower = {"type": "int", "val": "0"}
                    upper = {"type": "call", "fn": "len", "args": [seq_expr]}
                    step = {"type": "int", "val": "1"}
                    bound_guards = [
                        {"type": "compare", "op": ">=", "left": {"type": "var", "name": var_name}, "right": lower},
                        {"type": "compare", "op": "<", "left": {"type": "var", "name": var_name}, "right": upper},
                    ]
                    extracted_bound = LoopBoundItem(
                        module=self.module_name,
                        function=self.fn_name,
                        line=stmt.lineno,
                        variable=var_name,
                        kind="for_enumerate",
                        lower=lower,
                        upper=upper,
                        step=step,
                    )

                if extracted_bound is not None:
                    self.loop_bounds.append(extracted_bound)
                    self._active_loop_bounds.append(asdict(extracted_bound))

                loop_body_guards = current_guards + bound_guards
                self.visit_stmts(stmt.body, loop_body_guards)

                if extracted_bound is not None:
                    self._active_loop_bounds.pop()

                if stmt.orelse:
                    self.visit_stmts(stmt.orelse, current_guards)

            elif isinstance(stmt, ast.While):
                self.collect_call_sites(stmt.test, current_guards)
                self.check_expr(stmt.test, stmt.lineno, stmt.col_offset, current_guards)
                test_expr = ensure_bool_expr(stmt.test)

                # Extract induction bounds if test is a comparison
                extracted_bounds = self._extract_while_bounds(stmt)
                bound_guards = []
                for eb in extracted_bounds:
                    self.loop_bounds.append(eb)
                    self._active_loop_bounds.append(asdict(eb))
                    var_name = eb.variable
                    if eb.lower is not None:
                        bound_guards.append({
                            "type": "compare",
                            "op": ">=",
                            "left": {"type": "var", "name": var_name},
                            "right": eb.lower,
                        })
                    if eb.upper is not None:
                        bound_guards.append({
                            "type": "compare",
                            "op": "<",
                            "left": {"type": "var", "name": var_name},
                            "right": eb.upper,
                        })

                self.visit_stmts(stmt.body, current_guards + [test_expr] + bound_guards)

                for _ in extracted_bounds:
                    self._active_loop_bounds.pop()

                if stmt.orelse:
                    neg_test = {"type": "unary", "op": "not", "arg": test_expr}
                    self.visit_stmts(stmt.orelse, current_guards + [neg_test])

    def check_single_node(self, node: ast.AST, default_lineno: int, default_col: int, guards: List[Dict[str, Any]]):
        lineno = getattr(node, "lineno", default_lineno)
        col = getattr(node, "col_offset", default_col)

        # 1. Division / Modulo by Zero & String Formatting (% operator)
        if isinstance(node, ast.BinOp) and isinstance(node.op, (ast.Div, ast.FloorDiv, ast.Mod)):
            # Distinguish string formatting (% operator) from numeric modulo
            is_str_fmt = False
            if isinstance(node.op, ast.Mod):
                fmt_str = ""
                if isinstance(node.left, ast.Constant) and isinstance(node.left.value, str):
                    fmt_str = node.left.value
                elif isinstance(node.left, ast.Name):
                    recent = self.recent_assigns.get(node.left.id)
                    if recent and recent.get("type") == "str" and isinstance(recent.get("val"), str):
                        fmt_str = recent["val"]
                    elif any(s_kw in node.left.id.lower() or s_kw in self.params.get(node.left.id, "").lower() for s_kw in ("str", "fmt", "template", "msg", "text")):
                        is_str_fmt = True

                if fmt_str:
                    is_str_fmt = True
                    import re
                    specifiers = re.findall(r"%([%diouxXeEfFgGcrsap])", fmt_str)
                    pos_specs = [s for s in specifiers if s != "%"]
                    if pos_specs:
                        expected_count = len(pos_specs)
                        if isinstance(node.right, ast.Tuple):
                            actual_count = len(node.right.elts)
                            if actual_count != expected_count:
                                target = {"type": "compare", "op": "==", "left": {"type": "var", "name": "format_mismatch"}, "right": {"type": "int", "val": 1}}
                                self.add_obligation("format_error", lineno, col, guards, target)
                        elif expected_count > 1:
                            target = {"type": "compare", "op": "==", "left": {"type": "var", "name": "format_mismatch"}, "right": {"type": "int", "val": 1}}
                            self.add_obligation("format_error", lineno, col, guards, target)

            # Distinguish mathematical division from pathlib.Path '/' join
            is_path_join = False
            for side in (node.left, node.right):
                if isinstance(side, ast.Call) and isinstance(side.func, ast.Name) and side.func.id == "Path":
                    is_path_join = True
                elif isinstance(side, ast.Name) and ("path" in side.id.lower() or side.id in ("root", "dir", "base")):
                    is_path_join = True
                elif isinstance(side, ast.Attribute) and side.attr in ("parent", "resolve", "name", "stem"):
                    is_path_join = True
            if not is_path_join and not is_str_fmt:
                denom = expr_to_smt_dict(node.right)
                target = {"type": "compare", "op": "==", "left": denom, "right": {"type": "int", "val": 0}}
                self.add_obligation("div_zero", lineno, col, guards, target)
        elif isinstance(node, ast.AugAssign) and isinstance(node.op, (ast.Div, ast.FloorDiv, ast.Mod)):
            denom = expr_to_smt_dict(node.value)
            target = {"type": "compare", "op": "==", "left": denom, "right": {"type": "int", "val": 0}}
            self.add_obligation("div_zero", lineno, col, guards, target)

        # 2. KeyError vs IndexError on Subscript
        elif isinstance(node, ast.Subscript):
            # In Python, slices like seq[1:5] or seq[:] NEVER raise IndexError
            if isinstance(node.slice, ast.Slice):
                return

            var_name = getattr(node.value, "id", "") if isinstance(node.value, ast.Name) else ""
            var_ann = self.params.get(var_name, "").lower() if var_name else ""

            # Check if subscripting a non-subscriptable scalar (A5 - TypeError)
            if (var_name and var_name in self.known_scalars) or (var_ann in ("int", "float", "bool")):
                target = {"type": "compare", "op": "==", "left": {"type": "var", "name": "type_mismatch"}, "right": {"type": "int", "val": 1}}
                self.add_obligation("type_error", lineno, col, guards, target)
                return

            # Defaultdict never raises KeyError on missing keys
            if "defaultdict" in var_ann:
                return

            is_dict = False
            if var_name and (var_name in self.known_dicts or any(d_kw in var_ann for d_kw in ("dict", "mapping"))):
                is_dict = True
            elif isinstance(node.slice, ast.Constant) and isinstance(node.slice.value, (str, bytes)):
                is_dict = True
            elif isinstance(node.value, ast.Dict):
                is_dict = True

            if is_dict:
                d_name = sanitize_ast_ident(node.value)
                k_name = sanitize_ast_ident(node.slice)
                key_var = f"{k_name}_in_{d_name}"
                target = {"type": "compare", "op": "==", "left": {"type": "var", "name": key_var}, "right": {"type": "int", "val": 0}}
                self.add_obligation("key_error", lineno, col, guards, target)
            else:
                seq_expr = expr_to_smt_dict(node.value)
                idx_expr = expr_to_smt_dict(node.slice)
                # Python sequence indexing allows negative wrap-around: valid is -len(seq) <= idx < len(seq)
                neg_len = {"type": "unary", "op": "-", "arg": {"type": "call", "fn": "len", "args": [seq_expr]}}
                oob_cond = {
                    "type": "or",
                    "items": [
                        {"type": "compare", "op": "<", "left": idx_expr, "right": neg_len},
                        {
                            "type": "compare",
                            "op": ">=",
                            "left": idx_expr,
                            "right": {"type": "call", "fn": "len", "args": [seq_expr]},
                        },
                    ],
                }
                self.add_obligation("index_bounds", lineno, col, guards, oob_cond)

        # 3. AttributeError on None dereference
        elif isinstance(node, ast.Attribute):
            if isinstance(node.value, ast.Name):
                v_name = node.value.id
                if v_name not in ("self", "cls") and not v_name.startswith("__"):
                    if v_name in self.optional_vars:
                        target = {"type": "compare", "op": "==", "left": {"type": "var", "name": f"{v_name}_is_none"}, "right": {"type": "int", "val": 1}}
                        self.add_obligation("none_deref", lineno, col, guards, target)

        # 4. TypeError on invalid operands / calls
        elif isinstance(node, ast.Call):
            if isinstance(node.func, ast.Name):
                fn_id = node.func.id
                if fn_id == "len" and node.args:
                    arg = node.args[0]
                    is_scalar = False
                    if isinstance(arg, ast.Constant) and isinstance(arg.value, (int, float, bool)):
                        is_scalar = True
                    elif isinstance(arg, ast.Name) and (arg.id in self.known_scalars or self.params.get(arg.id, "").lower() in ("int", "float", "bool")):
                        is_scalar = True
                    if is_scalar:
                        target = {"type": "compare", "op": "==", "left": {"type": "var", "name": "type_mismatch"}, "right": {"type": "int", "val": 1}}
                        self.add_obligation("type_error", lineno, col, guards, target)

                elif fn_id in ("getattr", "setattr") and node.args:
                    # C1: Dynamic reflection literal resolution
                    obj_arg = node.args[0]
                    obj_name = getattr(obj_arg, "id", "") if isinstance(obj_arg, ast.Name) else ""
                    is_none_candidate = False
                    if isinstance(obj_arg, ast.Constant) and obj_arg.value is None:
                        is_none_candidate = True
                    elif obj_name and obj_name in self.optional_vars:
                        is_none_candidate = True

                    if is_none_candidate:
                        if fn_id == "setattr" or len(node.args) == 2:
                            target = {"type": "compare", "op": "==", "left": {"type": "var", "name": f"{obj_name or 'target'}_is_none"}, "right": {"type": "int", "val": 1}}
                            self.add_obligation("none_deref", lineno, col, guards, target)

                elif fn_id == "next" and len(node.args) == 1:
                    # C3: Iterator exhaustion StopIteration
                    it_name = sanitize_ast_ident(node.args[0])
                    exhaust_var = f"{it_name}_exhausted"
                    target = {"type": "compare", "op": "==", "left": {"type": "var", "name": exhaust_var}, "right": {"type": "int", "val": 1}}
                    self.add_obligation("stop_iteration", lineno, col, guards, target)

                elif fn_id in ("eval", "exec") and node.args:
                    # C2: Dynamic code execution
                    target = {"type": "compare", "op": "==", "left": {"type": "var", "name": "eval_injection"}, "right": {"type": "int", "val": 1}}
                    self.add_obligation("eval_exec", lineno, col, guards, target)

                elif fn_id in ("Thread", "start_new_thread") and not self._active_locks:
                    # C4: Thread spawning without synchronization lock
                    target = {"type": "compare", "op": "==", "left": {"type": "var", "name": "concurrency_race"}, "right": {"type": "int", "val": 1}}
                    self.add_obligation("concurrency_race", lineno, col, guards, target)

            elif isinstance(node.func, ast.Attribute):
                if node.func.attr in ("Thread", "start_new_thread") and not self._active_locks:
                    # C4: Thread spawning without synchronization lock
                    target = {"type": "compare", "op": "==", "left": {"type": "var", "name": "concurrency_race"}, "right": {"type": "int", "val": 1}}
                    self.add_obligation("concurrency_race", lineno, col, guards, target)
                elif node.func.attr == "pop" and len(node.args) == 1:
                    # dict.pop(key) without default raises KeyError if key is missing!
                    d_node = node.func.value
                    var_name = getattr(d_node, "id", "") if isinstance(d_node, ast.Name) else ""
                    var_ann = self.params.get(var_name, "").lower() if var_name else ""
                    if var_name in self.known_dicts or any(d_kw in var_ann for d_kw in ("dict", "mapping")) or isinstance(d_node, ast.Dict):
                        d_name = sanitize_ast_ident(d_node)
                        k_name = sanitize_ast_ident(node.args[0])
                        key_var = f"{k_name}_in_{d_name}"
                        target = {"type": "compare", "op": "==", "left": {"type": "var", "name": key_var}, "right": {"type": "int", "val": 0}}
                        self.add_obligation("key_error", lineno, col, guards, target)
        elif isinstance(node, ast.BinOp) and isinstance(node.op, ast.Sub):
            left_is_str = (isinstance(node.left, ast.Constant) and isinstance(node.left.value, str)) or (isinstance(node.left, ast.Name) and "str" in self.params.get(node.left.id, "").lower())
            right_is_str = (isinstance(node.right, ast.Constant) and isinstance(node.right.value, str)) or (isinstance(node.right, ast.Name) and "str" in self.params.get(node.right.id, "").lower())
            if left_is_str or right_is_str:
                target = {"type": "compare", "op": "==", "left": {"type": "var", "name": "type_mismatch"}, "right": {"type": "int", "val": 1}}
                self.add_obligation("type_error", lineno, col, guards, target)

        elif isinstance(node, ast.FormattedValue):
            if node.format_spec and isinstance(node.format_spec, ast.JoinedStr):
                for part in node.format_spec.values:
                    if isinstance(part, ast.Constant) and isinstance(part.value, str):
                        spec = part.value.strip()
                        val_node = node.value
                        is_str_val = (isinstance(val_node, ast.Constant) and isinstance(val_node.value, str)) or (isinstance(val_node, ast.Name) and "str" in self.params.get(val_node.id, "").lower())
                        if is_str_val and any(spec.endswith(c) for c in ("d", "i", "x", "X", "o", "f", "e", "E", "g", "G")):
                            target = {"type": "compare", "op": "==", "left": {"type": "var", "name": "format_mismatch"}, "right": {"type": "int", "val": 1}}
                            self.add_obligation("format_error", lineno, col, guards, target)

    def check_expr_with_guards(self, expr: ast.AST, default_lineno: int, default_col: int, guards: List[Dict[str, Any]]):
        if expr is None:
            return

        # Contextual propagation for ternary IfExp (x if cond else y)
        if isinstance(expr, ast.IfExp):
            self.check_single_node(expr.test, default_lineno, default_col, guards)
            test_guard = ensure_bool_expr(expr.test, self.optional_vars)
            neg_guard = {"type": "unary", "op": "not", "arg": test_guard}
            self.check_expr_with_guards(expr.body, default_lineno, default_col, guards + [test_guard])
            self.check_expr_with_guards(expr.orelse, default_lineno, default_col, guards + [neg_guard])
            return

        self.check_single_node(expr, default_lineno, default_col, guards)
        for child in ast.iter_child_nodes(expr):
            self.check_expr_with_guards(child, default_lineno, default_col, guards)

    def check_stmt_expressions(self, stmt: ast.stmt, guards: List[Dict[str, Any]]):
        for child in ast.iter_child_nodes(stmt):
            self.check_expr_with_guards(child, getattr(stmt, "lineno", 0), getattr(stmt, "col_offset", 0), guards)

    def check_expr(self, expr: ast.AST, lineno: int, col: int, guards: List[Dict[str, Any]]):
        self.check_expr_with_guards(expr, lineno, col, guards)


def parse_file(
    full_path: str, rel_path: str, module_name: str
) -> Tuple[List[ObligationItem], List[CallSiteInfo], List[LoopBoundItem], List[ExceptionHandlerScope]]:
    try:
        with open(full_path, "r", encoding="utf-8", errors="replace") as f:
            src = f.read()
        source_lines = src.splitlines()
        tree = ast.parse(src, filename=full_path)
    except Exception:
        return [], [], [], []

    obligations: List[ObligationItem] = []
    call_sites: List[CallSiteInfo] = []
    loop_bounds: List[LoopBoundItem] = []
    exception_scopes: List[ExceptionHandlerScope] = []

    def process_function(fn_node: ast.FunctionDef | ast.AsyncFunctionDef, prefix: str = ""):
        fn_name = f"{prefix}{fn_node.name}"
        params: Dict[str, str] = {}
        for arg in fn_node.args.args:
            ann_str = "Any"
            if arg.annotation:
                try:
                    ann_str = ast.unparse(arg.annotation)
                except Exception:
                    ann_str = "Any"
            params[arg.arg] = ann_str

        visitor = FunctionPathVisitor(rel_path, module_name, fn_name, params, source_lines)
        defaults = fn_node.args.defaults
        num_defaults = len(defaults)
        pos_args = fn_node.args.args
        for i, default_node in enumerate(defaults):
            arg_idx = len(pos_args) - num_defaults + i
            if 0 <= arg_idx < len(pos_args):
                arg_name = pos_args[arg_idx].arg
                if isinstance(default_node, ast.Constant) and default_node.value is None:
                    visitor.optional_vars.add(arg_name)

        visitor.visit_stmts(fn_node.body, [])
        obligations.extend(visitor.obligations)
        call_sites.extend(visitor.call_sites)
        loop_bounds.extend(visitor.loop_bounds)
        exception_scopes.extend(visitor.exception_scopes)

    for item in tree.body:
        if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
            process_function(item)
        elif isinstance(item, ast.ClassDef):
            for sub in item.body:
                if isinstance(sub, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    process_function(sub, prefix=f"{item.name}.")

    return obligations, call_sites, loop_bounds, exception_scopes


def scan_project(root_path: str) -> Dict[str, Any]:
    root_path = os.path.abspath(root_path)
    ignore_dirs = {
        ".git", ".svn", "__pycache__", ".pytest_cache", ".mypy_cache",
        ".venv", "venv", "env", "node_modules", "build", "dist", ".tox", ".eggs"
    }

    all_obligations: List[ObligationItem] = []
    all_call_sites: List[CallSiteInfo] = []
    all_loop_bounds: List[LoopBoundItem] = []
    all_exception_scopes: List[ExceptionHandlerScope] = []
    py_files_count = 0

    if os.path.isfile(root_path) and root_path.endswith(".py"):
        rel = os.path.basename(root_path)
        mod = os.path.splitext(rel)[0]
        obls, calls, loops, excs = parse_file(root_path, rel, mod)
        all_obligations.extend(obls)
        all_call_sites.extend(calls)
        all_loop_bounds.extend(loops)
        all_exception_scopes.extend(excs)
        py_files_count = 1
    else:
        for dirpath, dirnames, filenames in os.walk(root_path):
            dirnames[:] = [d for d in dirnames if d not in ignore_dirs and not d.startswith(".")]
            for filename in filenames:
                if filename.endswith(".py"):
                    full = os.path.join(dirpath, filename)
                    rel = os.path.relpath(full, root_path).replace("\\", "/")
                    parts = rel[:-3].split("/")
                    module_name = ".".join(parts)
                    obls, calls, loops, excs = parse_file(full, rel, module_name)
                    all_obligations.extend(obls)
                    all_call_sites.extend(calls)
                    all_loop_bounds.extend(loops)
                    all_exception_scopes.extend(excs)
                    py_files_count += 1

    call_graph: Dict[str, List[str]] = {}
    for cs in all_call_sites:
        caller_key = f"{cs.caller_module}.{cs.caller_function}"
        if caller_key not in call_graph:
            call_graph[caller_key] = []
        if cs.callee not in call_graph[caller_key]:
            call_graph[caller_key].append(cs.callee)

    return {
        "project_root": root_path,
        "files_scanned": py_files_count,
        "total_obligations": len(all_obligations),
        "obligations": [asdict(o) for o in all_obligations],
        "call_sites": [asdict(cs) for cs in all_call_sites],
        "loop_bounds": [asdict(lb) for lb in all_loop_bounds],
        "exception_handlers": [asdict(eh) for eh in all_exception_scopes],
        "call_graph": call_graph,
    }


def main():
    target = sys.argv[1] if len(sys.argv) > 1 else "."
    result = scan_project(target)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
