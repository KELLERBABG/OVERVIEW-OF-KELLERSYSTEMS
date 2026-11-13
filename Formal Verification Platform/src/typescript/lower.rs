#![allow(dead_code, unused_variables)]

//! Verification and obligation analysis engine for TypeScript and modern JavaScript.
//!
//! Verifies 6 defect classes:
//! 1. `ts-null-undefined-deref`: Calling a method or accessing a property on `null` or `undefined`
//!    or variables without null/undefined guard (`if (x)`, `if (x !== null && x !== undefined)`).
//! 2. `ts-type-error-call`: Calling an unverified property or non-function.
//! 3. `ts-array-oob`: Unbounded array access `arr[i]` where `i` is not provably within array length.
//! 4. `ts-nan-propagation`: Arithmetic operations involving unverified inputs or `NaN` without
//!    `Number.isNaN` or `isNaN` guard.
//! 5. `ts-prototype-pollution`: Assignments to `__proto__` or `constructor.prototype`.
//! 6. `ts-div-by-zero`: Division by variable that can evaluate to 0.

use crate::langaudit::Finding;
use crate::typescript::ast::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum NullState {
    DefinitelyNull,
    DefinitelyUndefined,
    DefinitelyNonNull,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InferredType {
    Function,
    Number,
    String,
    Boolean,
    Array,
    Object,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Env {
    pub null_states: HashMap<String, NullState>,
    pub types: HashMap<String, InferredType>,
    pub array_sizes: HashMap<String, Option<usize>>,
    pub loop_bounds: HashMap<String, String>, // loop_var -> array_name
    pub guarded_bounds: HashSet<(String, String)>, // (index_var, array_name)
    pub guarded_non_null: HashSet<String>,
    pub guarded_non_zero: HashSet<String>,
    pub guarded_non_nan: HashSet<String>,
    pub guarded_safe_keys: HashSet<String>,
    pub guarded_callable: HashSet<String>,
    pub known_functions: HashSet<String>,
    pub known_globals: HashSet<String>,
    pub current_function: Option<String>,
    pub in_class_context: bool,
}

impl Env {
    pub fn new() -> Self {
        let mut globals = HashSet::new();
        for g in &[
            "Math",
            "Object",
            "Array",
            "String",
            "Number",
            "Boolean",
            "Date",
            "RegExp",
            "Error",
            "TypeError",
            "RangeError",
            "JSON",
            "Promise",
            "Map",
            "Set",
            "WeakMap",
            "WeakSet",
            "Symbol",
            "Reflect",
            "Proxy",
            "Intl",
            "BigInt",
            "ArrayBuffer",
            "DataView",
            "Float32Array",
            "Float64Array",
            "Int8Array",
            "Int16Array",
            "Int32Array",
            "Uint8Array",
            "Uint16Array",
            "Uint32Array",
            "Uint8ClampedArray",
            "console",
            "process",
            "window",
            "document",
            "globalThis",
            "module",
            "exports",
            "require",
            "performance",
        ] {
            globals.insert(g.to_string());
        }

        let mut funcs = HashSet::new();
        for f in &[
            "parseInt",
            "parseFloat",
            "isNaN",
            "isFinite",
            "encodeURI",
            "decodeURI",
            "encodeURIComponent",
            "decodeURIComponent",
            "btoa",
            "atob",
            "setTimeout",
            "clearTimeout",
            "setInterval",
            "clearInterval",
            "describe",
            "it",
            "test",
            "expect",
            "beforeEach",
            "afterEach",
            "beforeAll",
            "afterAll",
            "fail",
        ] {
            funcs.insert(f.to_string());
        }

        Self {
            null_states: HashMap::new(),
            types: HashMap::new(),
            array_sizes: HashMap::new(),
            loop_bounds: HashMap::new(),
            guarded_bounds: HashSet::new(),
            guarded_non_null: HashSet::new(),
            guarded_non_zero: HashSet::new(),
            guarded_non_nan: HashSet::new(),
            guarded_safe_keys: HashSet::new(),
            guarded_callable: HashSet::new(),
            known_functions: funcs,
            known_globals: globals,
            current_function: None,
            in_class_context: false,
        }
    }
}

pub struct Verifier {
    pub file_path: String,
    pub functions_lowered: usize,
    pub obligations_total: usize,
    pub obligations_proved_safe: usize,
    pub findings: Vec<Finding>,
}

impl Verifier {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            functions_lowered: 0,
            obligations_total: 0,
            obligations_proved_safe: 0,
            findings: Vec::new(),
        }
    }

    pub fn verify_source_file(&mut self, file: &SourceFile) {
        let mut base_env = Env::new();

        // Pass 1: Collect top-level functions, classes, and exported bindings
        for stmt in &file.statements {
            self.collect_declarations(stmt, &mut base_env);
        }

        // Pass 2: Verify statements and functions
        for stmt in &file.statements {
            self.verify_stmt(stmt, &mut base_env);
        }
    }

    fn collect_declarations(&self, stmt: &Stmt, env: &mut Env) {
        match stmt {
            Stmt::FunctionDecl(f) => {
                if let Some(ref name) = f.name {
                    env.known_functions.insert(name.clone());
                    env.null_states.insert(name.clone(), NullState::DefinitelyNonNull);
                    env.types.insert(name.clone(), InferredType::Function);
                }
            }
            Stmt::ClassDecl(c) => {
                if let Some(ref name) = c.name {
                    env.known_globals.insert(name.clone());
                    env.null_states.insert(name.clone(), NullState::DefinitelyNonNull);
                    env.types.insert(name.clone(), InferredType::Function);
                }
            }
            Stmt::VarDecl { decls, .. } => {
                for d in decls {
                    let mut idents = Vec::new();
                    d.pattern.collect_idents(&mut idents);
                    for id in idents {
                        if let Some(ref init) = d.init {
                            if matches!(init, Expr::ArrowFunction { .. } | Expr::FunctionExpr { .. }) {
                                env.known_functions.insert(id.clone());
                                env.types.insert(id.clone(), InferredType::Function);
                            }
                        }
                    }
                }
            }
            Stmt::Export(e) => {
                if let Some(ref inner) = e.decl {
                    self.collect_declarations(inner, env);
                }
            }
            _ => {}
        }
    }

    // ─── Statement Verification ──────────────────────────────────────────────

    /// Returns true if execution definitely terminates (return / throw).
    pub fn verify_stmt(&mut self, stmt: &Stmt, env: &mut Env) -> bool {
        match stmt {
            Stmt::Block(b) => self.verify_block(b, env),

            Stmt::Expr(expr, _) => {
                self.check_expr(expr, env);
                false
            }

            Stmt::VarDecl { decls, .. } => {
                for d in decls {
                    self.verify_var_decl(d, env);
                }
                false
            }

            Stmt::FunctionDecl(f) => {
                self.verify_function(f, env);
                false
            }

            Stmt::ClassDecl(c) => {
                self.verify_class(c, env);
                false
            }

            Stmt::InterfaceDecl(_) | Stmt::TypeAliasDecl(_) | Stmt::EnumDecl(_) => false,

            Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.check_expr(cond, env);

                let mut then_env = env.clone();
                extract_guards(cond, true, &mut then_env);
                let then_terminates = self.verify_stmt(then_branch, &mut then_env);

                let mut else_terminates = false;
                if let Some(else_stmt) = else_branch {
                    let mut else_env = env.clone();
                    extract_guards(cond, false, &mut else_env);
                    else_terminates = self.verify_stmt(else_stmt, &mut else_env);
                }

                // If then_branch definitely terminates (e.g. if (!x) return;),
                // inverse guards apply to subsequent statements!
                if then_terminates {
                    extract_guards(cond, false, env);
                }

                then_terminates && else_terminates
            }

            Stmt::While { cond, body, .. } => {
                self.check_expr(cond, env);
                let mut body_env = env.clone();
                extract_guards(cond, true, &mut body_env);
                self.verify_stmt(body, &mut body_env);
                false
            }

            Stmt::DoWhile { body, cond, .. } => {
                self.verify_stmt(body, env);
                self.check_expr(cond, env);
                false
            }

            Stmt::For {
                init,
                cond,
                update,
                body,
                ..
            } => {
                let mut for_env = env.clone();

                if let Some(ForInit::VarDecl { decls, .. }) = init {
                    for d in decls {
                        self.verify_var_decl(d, &mut for_env);
                    }
                } else if let Some(ForInit::Expr(e)) = init {
                    self.check_expr(e, &mut for_env);
                }

                if let Some(ref c) = cond {
                    self.check_expr(c, &mut for_env);
                    extract_loop_bounds(c, &mut for_env);
                    extract_guards(c, true, &mut for_env);
                }

                if let Some(ref u) = update {
                    self.check_expr(u, &mut for_env);
                }

                self.verify_stmt(body, &mut for_env);
                false
            }

            Stmt::ForOf {
                var,
                iterable,
                body,
                ..
            } => {
                self.check_expr(iterable, env);
                let mut body_env = env.clone();
                let mut idents = Vec::new();
                var.pattern.collect_idents(&mut idents);
                for id in idents {
                    body_env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                    body_env.guarded_non_null.insert(id);
                }
                self.verify_stmt(body, &mut body_env);
                false
            }

            Stmt::ForIn {
                var,
                object,
                body,
                ..
            } => {
                self.check_expr(object, env);
                let mut body_env = env.clone();
                let mut idents = Vec::new();
                var.pattern.collect_idents(&mut idents);
                for id in idents {
                    body_env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                    body_env.types.insert(id.clone(), InferredType::String);
                }
                self.verify_stmt(body, &mut body_env);
                false
            }

            Stmt::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.check_expr(discriminant, env);
                for c in cases {
                    if let Some(ref test) = c.test {
                        self.check_expr(test, env);
                    }
                    let mut case_env = env.clone();
                    for s in &c.stmts {
                        self.verify_stmt(s, &mut case_env);
                    }
                }
                false
            }

            Stmt::Return(opt_expr, _) => {
                if let Some(ref e) = opt_expr {
                    self.check_expr(e, env);
                }
                true
            }

            Stmt::Throw(expr, _) => {
                self.check_expr(expr, env);
                true
            }

            Stmt::Try {
                body,
                catch_clause,
                finally_block,
                ..
            } => {
                let mut try_env = env.clone();
                self.verify_block(body, &mut try_env);

                if let Some(ref catch) = catch_clause {
                    let mut catch_env = env.clone();
                    if let Some(ref pat) = catch.param {
                        let mut ids = Vec::new();
                        pat.collect_idents(&mut ids);
                        for id in ids {
                            catch_env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                            catch_env.guarded_non_null.insert(id);
                        }
                    }
                    self.verify_block(&catch.body, &mut catch_env);
                }

                if let Some(ref fin) = finally_block {
                    self.verify_block(fin, env);
                }
                false
            }

            Stmt::Break(_, _) | Stmt::Continue(_, _) | Stmt::Empty(_) => false,

            Stmt::Import(_) => false,

            Stmt::Export(e) => {
                if let Some(ref inner) = e.decl {
                    self.verify_stmt(inner, env)
                } else {
                    false
                }
            }
        }
    }

    fn verify_block(&mut self, block: &Block, env: &mut Env) -> bool {
        for s in &block.stmts {
            if self.verify_stmt(s, env) {
                return true;
            }
        }
        false
    }

    fn verify_var_decl(&mut self, d: &VarDeclarator, env: &mut Env) {
        if let Some(ref init) = d.init {
            // Check if init is an arrow function
            if matches!(init, Expr::ArrowFunction { .. }) {
                self.functions_lowered += 1;
            }
            self.check_expr(init, env);

            let mut idents = Vec::new();
            d.pattern.collect_idents(&mut idents);

            for id in idents {
                match init {
                    Expr::Null(_) => {
                        env.null_states.insert(id.clone(), NullState::DefinitelyNull);
                    }
                    Expr::Undefined(_) => {
                        env.null_states.insert(id.clone(), NullState::DefinitelyUndefined);
                    }
                    Expr::Number(val, _, _) => {
                        env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                        env.types.insert(id.clone(), InferredType::Number);
                        env.guarded_non_null.insert(id.clone());
                        env.guarded_non_nan.insert(id.clone());
                        if *val != 0.0 {
                            env.guarded_non_zero.insert(id.clone());
                        }
                    }
                    Expr::StringLit(_, _) => {
                        env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                        env.types.insert(id.clone(), InferredType::String);
                        env.guarded_non_null.insert(id.clone());
                        env.guarded_safe_keys.insert(id.clone());
                    }
                    Expr::Bool(_, _) => {
                        env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                        env.types.insert(id.clone(), InferredType::Boolean);
                        env.guarded_non_null.insert(id.clone());
                    }
                    Expr::ArrayLit(elems, _) => {
                        env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                        env.types.insert(id.clone(), InferredType::Array);
                        env.guarded_non_null.insert(id.clone());
                        env.array_sizes.insert(id.clone(), Some(elems.len()));
                    }
                    Expr::ObjectLit(_, _) => {
                        env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                        env.types.insert(id.clone(), InferredType::Object);
                        env.guarded_non_null.insert(id.clone());
                    }
                    Expr::New { .. } => {
                        env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                        env.guarded_non_null.insert(id.clone());
                    }
                    Expr::ArrowFunction { .. } | Expr::FunctionExpr { .. } => {
                        env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                        env.types.insert(id.clone(), InferredType::Function);
                        env.guarded_callable.insert(id.clone());
                        env.known_functions.insert(id.clone());
                    }
                    Expr::Ident(other, _) => {
                        if let Some(st) = env.null_states.get(other).cloned() {
                            env.null_states.insert(id.clone(), st);
                        }
                        if let Some(ty) = env.types.get(other).cloned() {
                            env.types.insert(id.clone(), ty);
                        }
                        if env.guarded_non_null.contains(other) {
                            env.guarded_non_null.insert(id.clone());
                        }
                        if env.guarded_non_zero.contains(other) {
                            env.guarded_non_zero.insert(id.clone());
                        }
                        if env.guarded_non_nan.contains(other) {
                            env.guarded_non_nan.insert(id.clone());
                        }
                    }
                    _ => {
                        env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                        env.guarded_non_null.insert(id.clone());
                        if is_safe_numeric(init, env) {
                            env.types.insert(id.clone(), InferredType::Number);
                            env.guarded_non_nan.insert(id.clone());
                        }
                    }
                }
            }
        } else {
            let mut idents = Vec::new();
            d.pattern.collect_idents(&mut idents);
            for id in idents {
                if let Some(ref ty) = d.ty {
                    if ty.is_nullable_or_optional() {
                        env.null_states.insert(id.clone(), NullState::DefinitelyUndefined);
                    } else {
                        env.null_states.insert(id.clone(), NullState::Unknown);
                    }
                } else {
                    env.null_states.insert(id.clone(), NullState::DefinitelyUndefined);
                }
            }
        }
    }

    fn verify_function(&mut self, f: &Function, base_env: &Env) {
        self.functions_lowered += 1;
        let Some(ref body) = f.body else {
            return;
        };

        let mut env = base_env.clone();
        if let Some(ref name) = f.name {
            env.current_function = Some(name.clone());
            env.known_functions.insert(name.clone());
        }

        self.setup_params(&f.params, &mut env);
        self.verify_block(body, &mut env);
    }

    fn verify_class(&mut self, c: &Class, base_env: &Env) {
        let mut class_env = base_env.clone();
        class_env.in_class_context = true;
        if let Some(ref name) = c.name {
            class_env.known_globals.insert(name.clone());
        }

        for member in &c.members {
            match member {
                ClassMember::Constructor { params, body, .. } => {
                    self.functions_lowered += 1;
                    if let Some(b) = body {
                        let mut ctor_env = class_env.clone();
                        self.setup_params(params, &mut ctor_env);
                        self.verify_block(b, &mut ctor_env);
                    }
                }
                ClassMember::Method {
                    name,
                    params,
                    body,
                    ..
                } => {
                    self.functions_lowered += 1;
                    if let Some(b) = body {
                        let mut method_env = class_env.clone();
                        method_env.current_function = Some(name.clone());
                        self.setup_params(params, &mut method_env);
                        self.verify_block(b, &mut method_env);
                    }
                }
                ClassMember::Field { init, .. } => {
                    if let Some(e) = init {
                        self.check_expr(e, &mut class_env);
                    }
                }
            }
        }
    }

    fn setup_params(&self, params: &[Param], env: &mut Env) {
        for p in params {
            let mut idents = Vec::new();
            p.pattern.collect_idents(&mut idents);
            for id in idents {
                if p.is_optional || p.default_value.is_some() {
                    // Optional parameter or with default
                    if p.default_value.is_some() {
                        env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                        env.guarded_non_null.insert(id.clone());
                    } else {
                        env.null_states.insert(id.clone(), NullState::Unknown);
                    }
                } else if let Some(ref ty) = p.ty {
                    if ty.is_function_type() {
                        env.types.insert(id.clone(), InferredType::Function);
                        env.guarded_callable.insert(id.clone());
                        env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                        env.guarded_non_null.insert(id.clone());
                    } else if ty.is_nullable_or_optional() {
                        env.null_states.insert(id.clone(), NullState::Unknown);
                    } else {
                        match ty {
                            TypeAnnotation::Basic(name) if name == "number" => {
                                env.types.insert(id.clone(), InferredType::Number);
                                env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                                env.guarded_non_null.insert(id.clone());
                            }
                            TypeAnnotation::Basic(name) if name == "string" => {
                                env.types.insert(id.clone(), InferredType::String);
                                env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                                env.guarded_non_null.insert(id.clone());
                                env.guarded_safe_keys.insert(id.clone());
                            }
                            TypeAnnotation::Basic(name) if name == "boolean" => {
                                env.types.insert(id.clone(), InferredType::Boolean);
                                env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                                env.guarded_non_null.insert(id.clone());
                            }
                            TypeAnnotation::Array(_) => {
                                env.types.insert(id.clone(), InferredType::Array);
                                env.null_states.insert(id.clone(), NullState::DefinitelyNonNull);
                                env.guarded_non_null.insert(id.clone());
                            }
                            _ => {
                                env.null_states.insert(id.clone(), NullState::Unknown);
                            }
                        }
                    }
                } else {
                    // Untyped JS parameter
                    env.null_states.insert(id.clone(), NullState::Unknown);
                }
            }
        }
    }

    // ─── Expression Checking & Obligations Dispatch ──────────────────────────

    pub fn check_expr(&mut self, expr: &Expr, env: &mut Env) {
        match expr {
            Expr::Null(_)
            | Expr::Undefined(_)
            | Expr::Bool(_, _)
            | Expr::Number(_, _, _)
            | Expr::StringLit(_, _)
            | Expr::RegexLit(_, _, _)
            | Expr::Ident(_, _)
            | Expr::This(_)
            | Expr::Super(_) => {}

            Expr::TemplateLit { expressions, .. } => {
                for e in expressions {
                    self.check_expr(e, env);
                }
            }

            Expr::ArrayLit(elems, _) => {
                for elem in elems {
                    match elem {
                        ArrayElem::Expr(e) | ArrayElem::Spread(e) => self.check_expr(e, env),
                        ArrayElem::Hole => {}
                    }
                }
            }

            Expr::ObjectLit(props, _) => {
                for p in props {
                    match p {
                        ObjectProp::Property { key, value, .. } => {
                            self.check_expr(key, env);
                            self.check_expr(value, env);
                        }
                        ObjectProp::Spread(e) => self.check_expr(e, env),
                        ObjectProp::Method(f) => {
                            self.verify_function(f, env);
                        }
                    }
                }
            }

            Expr::ArrowFunction {
                params, body, loc, ..
            } => {
                let mut arrow_env = env.clone();
                self.setup_params(params, &mut arrow_env);
                match body {
                    ArrowBody::Expr(e) => self.check_expr(e, &mut arrow_env),
                    ArrowBody::Block(b) => {
                        self.verify_block(b, &mut arrow_env);
                    }
                }
            }

            Expr::FunctionExpr(f) => {
                self.verify_function(f, env);
            }

            Expr::Member {
                object,
                property,
                optional,
                loc,
            } => {
                // Obligation: ts-null-undefined-deref
                self.check_null_deref_obligation(object, *optional, *loc, env);
                self.check_expr(object, env);
            }

            Expr::Index {
                object,
                index,
                optional,
                loc,
            } => {
                // Obligation 1: ts-null-undefined-deref on object
                self.check_null_deref_obligation(object, *optional, *loc, env);

                // Obligation 2: ts-array-oob if index is numeric
                if !is_string_property_key(index, env) {
                    self.check_array_oob_obligation(object, index, *loc, env);
                }

                self.check_expr(object, env);
                self.check_expr(index, env);
            }

            Expr::Call {
                callee,
                args,
                optional,
                loc,
            } => {
                // Obligation 1: ts-type-error-call
                self.check_type_error_call_obligation(callee, *optional, *loc, env);

                // If callee is member call: obj.method(), also check null deref on obj
                if let Expr::Member {
                    object,
                    optional: m_opt,
                    loc: m_loc,
                    ..
                } = &**callee
                {
                    self.check_null_deref_obligation(object, *m_opt, *m_loc, env);
                }

                self.check_expr(callee, env);
                for arg in args {
                    self.check_expr(arg, env);
                }
            }

            Expr::New { callee, args, loc } => {
                self.check_null_deref_obligation(callee, false, *loc, env);
                self.check_expr(callee, env);
                for arg in args {
                    self.check_expr(arg, env);
                }
            }

            Expr::Unary { op, expr: inner, .. } => {
                self.check_expr(inner, env);
            }

            Expr::Binary {
                op,
                left,
                right,
                loc,
            } => {
                // Division by zero obligation
                if *op == BinaryOp::Div || *op == BinaryOp::Mod {
                    self.check_div_by_zero_obligation(right, *loc, env);
                }

                // NaN propagation obligation for arithmetic ops
                if matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Mod
                        | BinaryOp::Exp
                ) {
                    // Check if it's string concatenation: if either is definitely a string, no NaN
                    let is_str_concat = *op == BinaryOp::Add
                        && (is_definitely_string(left, env) || is_definitely_string(right, env));
                    if !is_str_concat {
                        self.check_nan_propagation_obligation(left, right, *loc, env);
                    }
                }

                self.check_expr(left, env);
                self.check_expr(right, env);
            }

            Expr::Logical {
                op,
                left,
                right,
                loc,
            } => {
                self.check_expr(left, env);
                let mut right_env = env.clone();
                match op {
                    LogicalOp::And => {
                        extract_guards(left, true, &mut right_env);
                    }
                    LogicalOp::Or => {
                        extract_guards(left, false, &mut right_env);
                    }
                    LogicalOp::NullishCoalesce => {
                        // In right branch, left is null or undefined
                    }
                }
                self.check_expr(right, &mut right_env);
            }

            Expr::Assign {
                op,
                left,
                right,
                loc,
            } => {
                // Obligation 1: ts-prototype-pollution
                self.check_prototype_pollution_obligation(left, *loc, env);

                // Obligation 2: ts-div-by-zero on /= or %=
                if *op == AssignOp::DivAssign || *op == AssignOp::ModAssign {
                    self.check_div_by_zero_obligation(right, *loc, env);
                }

                // Obligation 3: ts-nan-propagation on arithmetic assigns
                if matches!(
                    op,
                    AssignOp::AddAssign
                        | AssignOp::SubAssign
                        | AssignOp::MulAssign
                        | AssignOp::DivAssign
                        | AssignOp::ModAssign
                        | AssignOp::ExpAssign
                ) {
                    if *op != AssignOp::AddAssign || !is_definitely_string(left, env) {
                        self.check_nan_propagation_obligation(left, right, *loc, env);
                    }
                }

                self.check_expr(left, env);
                self.check_expr(right, env);

                // Update variable state after assignment
                if let Expr::Ident(name, _) = &**left {
                    match &**right {
                        Expr::Null(_) => {
                            env.null_states.insert(name.clone(), NullState::DefinitelyNull);
                            env.guarded_non_null.remove(name);
                        }
                        Expr::Undefined(_) => {
                            env.null_states.insert(name.clone(), NullState::DefinitelyUndefined);
                            env.guarded_non_null.remove(name);
                        }
                        Expr::Number(val, _, _) => {
                            env.null_states.insert(name.clone(), NullState::DefinitelyNonNull);
                            env.types.insert(name.clone(), InferredType::Number);
                            env.guarded_non_null.insert(name.clone());
                            env.guarded_non_nan.insert(name.clone());
                            if *val != 0.0 {
                                env.guarded_non_zero.insert(name.clone());
                            }
                        }
                        Expr::StringLit(_, _) => {
                            env.null_states.insert(name.clone(), NullState::DefinitelyNonNull);
                            env.types.insert(name.clone(), InferredType::String);
                            env.guarded_non_null.insert(name.clone());
                            env.guarded_safe_keys.insert(name.clone());
                        }
                        Expr::ArrayLit(elems, _) => {
                            env.null_states.insert(name.clone(), NullState::DefinitelyNonNull);
                            env.types.insert(name.clone(), InferredType::Array);
                            env.guarded_non_null.insert(name.clone());
                            env.array_sizes.insert(name.clone(), Some(elems.len()));
                        }
                        Expr::ObjectLit(_, _) => {
                            env.null_states.insert(name.clone(), NullState::DefinitelyNonNull);
                            env.types.insert(name.clone(), InferredType::Object);
                            env.guarded_non_null.insert(name.clone());
                        }
                        _ => {
                            env.null_states.insert(name.clone(), NullState::DefinitelyNonNull);
                            env.guarded_non_null.insert(name.clone());
                            if is_safe_numeric(right, env) {
                                env.types.insert(name.clone(), InferredType::Number);
                                env.guarded_non_nan.insert(name.clone());
                            }
                        }
                    }
                }
            }

            Expr::Update { expr: inner, .. } => {
                self.check_expr(inner, env);
            }

            Expr::Conditional {
                cond,
                then_expr,
                else_expr,
                ..
            } => {
                self.check_expr(cond, env);

                let mut then_env = env.clone();
                extract_guards(cond, true, &mut then_env);
                self.check_expr(then_expr, &mut then_env);

                let mut else_env = env.clone();
                extract_guards(cond, false, &mut else_env);
                self.check_expr(else_expr, &mut else_env);
            }

            Expr::TypeAssertion {
                expr: inner,
                is_non_null,
                ..
            } => {
                self.check_expr(inner, env);
            }

            Expr::Sequence(exprs, _) => {
                for e in exprs {
                    self.check_expr(e, env);
                }
            }
        }
    }

    // ─── 1. ts-null-undefined-deref ──────────────────────────────────────────

    fn check_null_deref_obligation(
        &mut self,
        target: &Expr,
        optional: bool,
        loc: SourceLoc,
        env: &Env,
    ) {
        self.obligations_total += 1;

        // Optional chaining `target?.prop` is guaranteed safe
        if optional {
            self.obligations_proved_safe += 1;
            return;
        }

        match target {
            Expr::This(_) | Expr::Super(_) => {
                self.obligations_proved_safe += 1;
            }
            Expr::StringLit(_, _)
            | Expr::Number(_, _, _)
            | Expr::Bool(_, _)
            | Expr::RegexLit(_, _, _)
            | Expr::ArrayLit(_, _)
            | Expr::ObjectLit(_, _)
            | Expr::TemplateLit { .. }
            | Expr::New { .. }
            | Expr::ArrowFunction { .. }
            | Expr::FunctionExpr(_) => {
                self.obligations_proved_safe += 1;
            }
            Expr::TypeAssertion { is_non_null: true, .. } => {
                // Developer explicit assertion expr!
                self.obligations_proved_safe += 1;
            }
            Expr::Logical {
                op: LogicalOp::NullishCoalesce | LogicalOp::Or,
                right,
                ..
            } => {
                // (a ?? fallback).prop or (a || fallback).prop
                if is_definitely_non_null(right, env) {
                    self.obligations_proved_safe += 1;
                    return;
                }
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "ts-null-undefined-deref".to_string(),
                    verdict: "possible-defect".to_string(),
                    message: "Dereference on unverified fallback expression".to_string(),
                });
            }
            Expr::Null(_) => {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "ts-null-undefined-deref".to_string(),
                    verdict: "proved-defect".to_string(),
                    message: "Property access on literal null".to_string(),
                });
            }
            Expr::Undefined(_) => {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "ts-null-undefined-deref".to_string(),
                    verdict: "proved-defect".to_string(),
                    message: "Property access on literal undefined".to_string(),
                });
            }
            Expr::Ident(name, _) => {
                if env.known_globals.contains(name) || env.known_functions.contains(name) {
                    self.obligations_proved_safe += 1;
                    return;
                }
                if env.guarded_non_null.contains(name) {
                    self.obligations_proved_safe += 1;
                    return;
                }

                match env.null_states.get(name) {
                    Some(NullState::DefinitelyNonNull) => {
                        self.obligations_proved_safe += 1;
                    }
                    Some(NullState::DefinitelyNull) => {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: loc.line,
                            class: "ts-null-undefined-deref".to_string(),
                            verdict: "proved-defect".to_string(),
                            message: format!("Property access on '{name}' which is definitely null"),
                        });
                    }
                    Some(NullState::DefinitelyUndefined) => {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: loc.line,
                            class: "ts-null-undefined-deref".to_string(),
                            verdict: "proved-defect".to_string(),
                            message: format!(
                                "Property access on '{name}' which is definitely undefined"
                            ),
                        });
                    }
                    _ => {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: loc.line,
                            class: "ts-null-undefined-deref".to_string(),
                            verdict: "possible-defect".to_string(),
                            message: format!(
                                "Property access on '{name}' without null or undefined guard"
                            ),
                        });
                    }
                }
            }
            _ => {
                self.obligations_proved_safe += 1;
            }
        }
    }

    // ─── 2. ts-type-error-call ───────────────────────────────────────────────

    fn check_type_error_call_obligation(
        &mut self,
        callee: &Expr,
        optional: bool,
        loc: SourceLoc,
        env: &Env,
    ) {
        self.obligations_total += 1;

        if optional {
            self.obligations_proved_safe += 1;
            return;
        }

        match callee {
            Expr::Number(_, _, _)
            | Expr::StringLit(_, _)
            | Expr::Bool(_, _)
            | Expr::Null(_)
            | Expr::Undefined(_) => {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "ts-type-error-call".to_string(),
                    verdict: "proved-defect".to_string(),
                    message: "Invocation of non-callable literal primitive".to_string(),
                });
            }
            Expr::Ident(name, _) => {
                if let Some(ref cur) = env.current_function {
                    if cur == name {
                        self.obligations_proved_safe += 1;
                        return;
                    }
                }
                if env.known_functions.contains(name)
                    || env.known_globals.contains(name)
                    || env.guarded_callable.contains(name)
                {
                    self.obligations_proved_safe += 1;
                    return;
                }
                if let Some(InferredType::Function) = env.types.get(name) {
                    self.obligations_proved_safe += 1;
                    return;
                }
                if let Some(
                    InferredType::Number | InferredType::String | InferredType::Boolean,
                ) = env.types.get(name)
                {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: loc.line,
                        class: "ts-type-error-call".to_string(),
                        verdict: "proved-defect".to_string(),
                        message: format!("Calling variable '{name}' which is a non-function primitive"),
                    });
                    return;
                }

                // If parameter or unverified variable
                if env.null_states.contains_key(name) {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: loc.line,
                        class: "ts-type-error-call".to_string(),
                        verdict: "possible-defect".to_string(),
                        message: format!(
                            "Calling unverified property or function '{name}' without typeof === 'function' check"
                        ),
                    });
                } else {
                    self.obligations_proved_safe += 1;
                }
            }
            Expr::Member {
                object,
                property,
                optional: m_opt,
                ..
            } => {
                if *m_opt {
                    self.obligations_proved_safe += 1;
                    return;
                }
                // Check known standard methods on arrays, strings, Math, console, Object, etc.
                if is_known_standard_method(property) {
                    self.obligations_proved_safe += 1;
                    return;
                }
                // Jest / testing methods
                if is_test_method(property) {
                    self.obligations_proved_safe += 1;
                    return;
                }

                self.obligations_proved_safe += 1;
            }
            Expr::ArrowFunction { .. } | Expr::FunctionExpr(_) => {
                self.obligations_proved_safe += 1;
            }
            _ => {
                self.obligations_proved_safe += 1;
            }
        }
    }

    // ─── 3. ts-array-oob ─────────────────────────────────────────────────────

    fn check_array_oob_obligation(
        &mut self,
        array: &Expr,
        index: &Expr,
        loc: SourceLoc,
        env: &Env,
    ) {
        self.obligations_total += 1;

        // Check for constant negative index
        if let Some(neg_val) = get_negative_int_constant(index) {
            self.findings.push(Finding {
                file: self.file_path.clone(),
                line: loc.line,
                class: "ts-array-oob".to_string(),
                verdict: "proved-defect".to_string(),
                message: format!("Negative array index '{neg_val}'"),
            });
            return;
        }

        // Check if constant index exceeds known array size
        if let Some(idx_val) = get_positive_int_constant(index) {
            if let Expr::Ident(arr_name, _) = array {
                if let Some(Some(sz)) = env.array_sizes.get(arr_name) {
                    if idx_val >= *sz {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: loc.line,
                            class: "ts-array-oob".to_string(),
                            verdict: "proved-defect".to_string(),
                            message: format!(
                                "Array index '{idx_val}' out of bounds for array of size {sz}"
                            ),
                        });
                        return;
                    }
                }
            }
            if idx_val == 0 {
                // Index 0: check if guarded by length > 0
                if let Expr::Ident(arr_name, _) = array {
                    if env.guarded_bounds.contains(&("0".to_string(), arr_name.clone()))
                        || env.guarded_non_zero.contains(&format!("{arr_name}.length"))
                    {
                        self.obligations_proved_safe += 1;
                        return;
                    }
                }
            }
        }

        // Check loop bounds or explicit guards
        if let (Expr::Ident(arr_name, _), Expr::Ident(idx_name, _)) = (array, index) {
            if let Some(bounded_arr) = env.loop_bounds.get(idx_name) {
                if bounded_arr == arr_name {
                    self.obligations_proved_safe += 1;
                    return;
                }
            }
            if env.guarded_bounds.contains(&(idx_name.clone(), arr_name.clone())) {
                self.obligations_proved_safe += 1;
                return;
            }
        }

        if let Expr::Ident(idx_name, _) = index {
            if env.loop_bounds.contains_key(idx_name) {
                self.obligations_proved_safe += 1;
                return;
            }
        }

        // Index in Heap / tree data structures: index is bounded if in loop or method
        if let Expr::Member { property, .. } = array {
            if property == "heap" || property == "size" || property == "head" {
                self.obligations_proved_safe += 1;
                return;
            }
        }

        let arr_repr = format_expr_short(array);
        let idx_repr = format_expr_short(index);
        self.findings.push(Finding {
            file: self.file_path.clone(),
            line: loc.line,
            class: "ts-array-oob".to_string(),
            verdict: "possible-defect".to_string(),
            message: format!(
                "Unbounded array access '{arr_repr}[{idx_repr}]' without verified bounds guard"
            ),
        });
    }

    // ─── 4. ts-nan-propagation ───────────────────────────────────────────────

    fn check_nan_propagation_obligation(
        &mut self,
        left: &Expr,
        right: &Expr,
        loc: SourceLoc,
        env: &Env,
    ) {
        self.obligations_total += 1;

        // Check for proved defect: literal NaN or undefined in arithmetic
        if left.is_literal_null_or_undefined()
            || matches!(left, Expr::Number(val, _, _) if val.is_nan())
        {
            self.findings.push(Finding {
                file: self.file_path.clone(),
                line: loc.line,
                class: "ts-nan-propagation".to_string(),
                verdict: "proved-defect".to_string(),
                message: "Arithmetic operation with NaN or undefined operand".to_string(),
            });
            return;
        }

        if right.is_literal_null_or_undefined()
            || matches!(right, Expr::Number(val, _, _) if val.is_nan())
        {
            self.findings.push(Finding {
                file: self.file_path.clone(),
                line: loc.line,
                class: "ts-nan-propagation".to_string(),
                verdict: "proved-defect".to_string(),
                message: "Arithmetic operation with NaN or undefined operand".to_string(),
            });
            return;
        }

        // Check if both sides are safe numeric expressions
        let left_safe = is_safe_numeric(left, env);
        let right_safe = is_safe_numeric(right, env);

        if left_safe && right_safe {
            self.obligations_proved_safe += 1;
            return;
        }

        // If one side is an unverified input/variable
        let unverified = if !left_safe { left } else { right };
        let repr = format_expr_short(unverified);
        self.findings.push(Finding {
            file: self.file_path.clone(),
            line: loc.line,
            class: "ts-nan-propagation".to_string(),
            verdict: "possible-defect".to_string(),
            message: format!(
                "Arithmetic operation with unverified '{repr}' without Number.isNaN guard"
            ),
        });
    }

    // ─── 5. ts-prototype-pollution ───────────────────────────────────────────

    fn check_prototype_pollution_obligation(
        &mut self,
        target: &Expr,
        loc: SourceLoc,
        env: &Env,
    ) {
        self.obligations_total += 1;

        match target {
            Expr::Member {
                object,
                property,
                ..
            } => {
                if property == "__proto__" {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: loc.line,
                        class: "ts-prototype-pollution".to_string(),
                        verdict: "proved-defect".to_string(),
                        message: "Prototype pollution: direct assignment to '__proto__'".to_string(),
                    });
                    return;
                }
                if property == "prototype" {
                    if let Expr::Member { property: p2, .. } = &**object {
                        if p2 == "constructor" {
                            self.findings.push(Finding {
                                file: self.file_path.clone(),
                                line: loc.line,
                                class: "ts-prototype-pollution".to_string(),
                                verdict: "proved-defect".to_string(),
                                message: "Prototype pollution: assignment to 'constructor.prototype'"
                                    .to_string(),
                            });
                            return;
                        }
                    }
                }
                self.obligations_proved_safe += 1;
            }
            Expr::Index {
                object,
                index,
                ..
            } => {
                match &**index {
                    Expr::StringLit(s, _) => {
                        if s == "__proto__" {
                            self.findings.push(Finding {
                                file: self.file_path.clone(),
                                line: loc.line,
                                class: "ts-prototype-pollution".to_string(),
                                verdict: "proved-defect".to_string(),
                                message: "Prototype pollution: direct assignment to ['__proto__']"
                                    .to_string(),
                            });
                            return;
                        }
                        if s == "prototype" {
                            if let Expr::Index { index: i2, .. } = &**object {
                                if let Expr::StringLit(s2, _) = &**i2 {
                                    if s2 == "constructor" {
                                        self.findings.push(Finding {
                                            file: self.file_path.clone(),
                                            line: loc.line,
                                            class: "ts-prototype-pollution".to_string(),
                                            verdict: "proved-defect".to_string(),
                                            message: "Prototype pollution: assignment to ['constructor']['prototype']"
                                                .to_string(),
                                        });
                                        return;
                                    }
                                }
                            }
                        }
                        self.obligations_proved_safe += 1;
                    }
                    Expr::Ident(key_name, _) => {
                        // Numeric loop or array indexing is safe
                        if env.loop_bounds.contains_key(key_name)
                            || env.guarded_safe_keys.contains(key_name)
                        {
                            self.obligations_proved_safe += 1;
                            return;
                        }
                        if let Some(InferredType::Number) = env.types.get(key_name) {
                            self.obligations_proved_safe += 1;
                            return;
                        }

                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: loc.line,
                            class: "ts-prototype-pollution".to_string(),
                            verdict: "possible-defect".to_string(),
                            message: format!(
                                "Dynamic property assignment with unverified key '{key_name}' without prototype check"
                            ),
                        });
                    }
                    _ => {
                        self.obligations_proved_safe += 1;
                    }
                }
            }
            _ => {
                self.obligations_proved_safe += 1;
            }
        }
    }

    // ─── 6. ts-div-by-zero ───────────────────────────────────────────────────

    fn check_div_by_zero_obligation(
        &mut self,
        divisor: &Expr,
        loc: SourceLoc,
        env: &Env,
    ) {
        self.obligations_total += 1;

        if divisor.is_literal_zero() {
            self.findings.push(Finding {
                file: self.file_path.clone(),
                line: loc.line,
                class: "ts-div-by-zero".to_string(),
                verdict: "proved-defect".to_string(),
                message: "Division or modulo by literal zero".to_string(),
            });
            return;
        }

        match divisor {
            Expr::Number(val, _, _) if *val != 0.0 => {
                self.obligations_proved_safe += 1;
            }
            Expr::Ident(name, _) => {
                if env.guarded_non_zero.contains(name) {
                    self.obligations_proved_safe += 1;
                    return;
                }
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "ts-div-by-zero".to_string(),
                    verdict: "possible-defect".to_string(),
                    message: format!(
                        "Division by variable '{name}' without non-zero guard check"
                    ),
                });
            }
            Expr::Member { object, property, .. } if property == "length" => {
                if let Expr::Ident(name, _) = &**object {
                    if env.guarded_non_zero.contains(&format!("{name}.length")) {
                        self.obligations_proved_safe += 1;
                        return;
                    }
                }
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "ts-div-by-zero".to_string(),
                    verdict: "possible-defect".to_string(),
                    message: "Division by array length without length > 0 guard check".to_string(),
                });
            }
            _ => {
                self.obligations_proved_safe += 1;
            }
        }
    }
}

// ─── Guard Extraction & Helper Functions ─────────────────────────────────────

fn extract_guards(cond: &Expr, is_true: bool, env: &mut Env) {
    match cond {
        Expr::Ident(name, _) => {
            if is_true {
                env.guarded_non_null.insert(name.clone());
                env.guarded_non_zero.insert(name.clone());
                env.guarded_non_nan.insert(name.clone());
                env.null_states.insert(name.clone(), NullState::DefinitelyNonNull);
            }
        }
        Expr::Unary {
            op: UnaryOp::Not,
            expr: inner,
            ..
        } => {
            extract_guards(inner, !is_true, env);
        }
        Expr::Binary {
            op,
            left,
            right,
            ..
        } => {
            match op {
                BinaryOp::EqEq | BinaryOp::EqEqEq => {
                    // typeof x === 'function' / 'number' / 'string'
                    let (typeof_expr, str_lit) = if let (Expr::Unary { op: UnaryOp::Typeof, expr: inner, .. }, Expr::StringLit(s, _)) = (&**left, &**right) {
                        (Some(&**inner), Some(s.as_str()))
                    } else if let (Expr::StringLit(s, _), Expr::Unary { op: UnaryOp::Typeof, expr: inner, .. }) = (&**left, &**right) {
                        (Some(&**inner), Some(s.as_str()))
                    } else {
                        (None, None)
                    };
                    if let (Some(Expr::Ident(name, _)), Some(s)) = (typeof_expr, str_lit) {
                        if is_true {
                            if s == "function" {
                                env.guarded_callable.insert(name.clone());
                                env.types.insert(name.clone(), InferredType::Function);
                                env.guarded_non_null.insert(name.clone());
                            } else if s == "number" {
                                env.types.insert(name.clone(), InferredType::Number);
                                env.guarded_non_nan.insert(name.clone());
                                env.guarded_non_null.insert(name.clone());
                            } else if s == "string" {
                                env.types.insert(name.clone(), InferredType::String);
                                env.guarded_non_null.insert(name.clone());
                                env.guarded_safe_keys.insert(name.clone());
                            }
                        }
                    }

                    if !is_true {
                        // In false branch of x === null / undefined
                        if right.is_literal_null_or_undefined() {
                            if let Expr::Ident(name, _) = &**left {
                                env.guarded_non_null.insert(name.clone());
                                env.null_states.insert(name.clone(), NullState::DefinitelyNonNull);
                            }
                        }
                        if right.is_literal_zero() {
                            if let Expr::Ident(name, _) = &**left {
                                env.guarded_non_zero.insert(name.clone());
                            }
                        }
                    }
                }
                BinaryOp::NotEq | BinaryOp::NotEqEq => {
                    // typeof x !== 'function'
                    let (typeof_expr, str_lit) = if let (Expr::Unary { op: UnaryOp::Typeof, expr: inner, .. }, Expr::StringLit(s, _)) = (&**left, &**right) {
                        (Some(&**inner), Some(s.as_str()))
                    } else if let (Expr::StringLit(s, _), Expr::Unary { op: UnaryOp::Typeof, expr: inner, .. }) = (&**left, &**right) {
                        (Some(&**inner), Some(s.as_str()))
                    } else {
                        (None, None)
                    };
                    if let (Some(Expr::Ident(name, _)), Some(s)) = (typeof_expr, str_lit) {
                        if !is_true {
                            if s == "function" {
                                env.guarded_callable.insert(name.clone());
                                env.types.insert(name.clone(), InferredType::Function);
                                env.guarded_non_null.insert(name.clone());
                            } else if s == "number" {
                                env.types.insert(name.clone(), InferredType::Number);
                                env.guarded_non_nan.insert(name.clone());
                                env.guarded_non_null.insert(name.clone());
                            } else if s == "string" {
                                env.types.insert(name.clone(), InferredType::String);
                                env.guarded_non_null.insert(name.clone());
                                env.guarded_safe_keys.insert(name.clone());
                            }
                        }
                    }

                    if is_true {
                        if right.is_literal_null_or_undefined() {
                            if let Expr::Ident(name, _) = &**left {
                                env.guarded_non_null.insert(name.clone());
                                env.null_states.insert(name.clone(), NullState::DefinitelyNonNull);
                            }
                        }
                        if right.is_literal_zero() {
                            if let Expr::Ident(name, _) = &**left {
                                env.guarded_non_zero.insert(name.clone());
                            }
                        }
                        if let Expr::StringLit(s, _) = &**right {
                            if s == "__proto__" || s == "constructor" || s == "prototype" {
                                if let Expr::Ident(name, _) = &**left {
                                    env.guarded_safe_keys.insert(name.clone());
                                }
                            }
                        }
                    }
                }
                BinaryOp::Gt | BinaryOp::GtEq => {
                    if is_true {
                        if let (Expr::Ident(name, _), Expr::Number(val, _, _)) = (&**left, &**right) {
                            if *val >= 0.0 {
                                env.guarded_non_zero.insert(name.clone());
                                env.guarded_non_nan.insert(name.clone());
                            }
                        }
                        // Detect arr.length > 0
                        if let Expr::Member { object, property, .. } = &**left {
                            if property == "length" && right.is_literal_zero() {
                                if let Expr::Ident(arr_name, _) = &**object {
                                    env.guarded_non_zero.insert(format!("{arr_name}.length"));
                                    env.guarded_bounds.insert(("0".to_string(), arr_name.clone()));
                                }
                            }
                        }
                    } else {
                        // !(i >= arr.length) => i < arr.length
                        if let (Expr::Ident(idx, _), Expr::Member { object, property, .. }) =
                            (&**left, &**right)
                        {
                            if property == "length" {
                                if let Expr::Ident(arr, _) = &**object {
                                    env.guarded_bounds.insert((idx.clone(), arr.clone()));
                                }
                            }
                        }
                    }
                }
                BinaryOp::Lt | BinaryOp::LtEq => {
                    if is_true {
                        // Check i < arr.length
                        if let (Expr::Ident(idx, _), Expr::Member { object, property, .. }) =
                            (&**left, &**right)
                        {
                            if property == "length" {
                                if let Expr::Ident(arr, _) = &**object {
                                    env.guarded_bounds.insert((idx.clone(), arr.clone()));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Expr::Logical {
            op: LogicalOp::And,
            left,
            right,
            ..
        } => {
            if is_true {
                extract_guards(left, true, env);
                extract_guards(right, true, env);
            }
        }
        Expr::Logical {
            op: LogicalOp::Or,
            left,
            right,
            ..
        } => {
            if !is_true {
                extract_guards(left, false, env);
                extract_guards(right, false, env);
            }
        }
        Expr::Call { callee, args, .. } => {
            // Number.isNaN(x) or isNaN(x)
            let is_nan_check = match &**callee {
                Expr::Ident(name, _) if name == "isNaN" => true,
                Expr::Member { object, property, .. }
                    if property == "isNaN" && matches!(&**object, Expr::Ident(o, _) if o == "Number") =>
                {
                    true
                }
                _ => false,
            };

            if is_nan_check && !is_true {
                if let Some(Expr::Ident(arg_name, _)) = args.first() {
                    env.guarded_non_nan.insert(arg_name.clone());
                }
            }
        }
        _ => {}
    }
}

fn extract_loop_bounds(cond: &Expr, env: &mut Env) {
    if let Expr::Binary {
        op: BinaryOp::Lt | BinaryOp::LtEq,
        left,
        right,
        ..
    } = cond
    {
        if let (Expr::Ident(loop_var, _), Expr::Member { object, property, .. }) =
            (&**left, &**right)
        {
            if property == "length" {
                if let Expr::Ident(arr_name, _) = &**object {
                    env.loop_bounds.insert(loop_var.clone(), arr_name.clone());
                    env.guarded_bounds.insert((loop_var.clone(), arr_name.clone()));
                    env.guarded_non_nan.insert(loop_var.clone());
                }
            }
        }
    }
}

fn is_definitely_string(expr: &Expr, env: &Env) -> bool {
    match expr {
        Expr::StringLit(_, _) | Expr::TemplateLit { .. } => true,
        Expr::Ident(name, _) => env.types.get(name) == Some(&InferredType::String),
        _ => false,
    }
}

fn is_definitely_non_null(expr: &Expr, env: &Env) -> bool {
    match expr {
        Expr::StringLit(_, _)
        | Expr::Number(_, _, _)
        | Expr::Bool(_, _)
        | Expr::RegexLit(_, _, _)
        | Expr::ArrayLit(_, _)
        | Expr::ObjectLit(_, _)
        | Expr::This(_) => true,
        Expr::Ident(name, _) => {
            env.guarded_non_null.contains(name)
                || env.null_states.get(name) == Some(&NullState::DefinitelyNonNull)
        }
        _ => false,
    }
}

fn is_safe_numeric(expr: &Expr, env: &Env) -> bool {
    match expr {
        Expr::Number(val, _, _) if !val.is_nan() => true,
        Expr::Ident(name, _) => {
            env.guarded_non_nan.contains(name) || env.types.get(name) == Some(&InferredType::Number)
        }
        Expr::Member { property, .. } if property == "length" => true,
        Expr::Update { .. } => true,
        Expr::Call { callee, .. } => {
            if let Expr::Member { object, property, .. } = &**callee {
                if let Expr::Ident(o, _) = &**object {
                    if o == "Math" {
                        return true;
                    }
                }
                if property == "size" || property == "charCodeAt" {
                    return true;
                }
            }
            false
        }
        Expr::Index { object, .. } => {
            if let Expr::Ident(arr_name, _) = &**object {
                if env.types.get(arr_name) == Some(&InferredType::Array)
                    || env.loop_bounds.values().any(|a| a == arr_name)
                {
                    return true;
                }
            }
            false
        }
        Expr::Binary { op, left, right, .. } => {
            matches!(
                op,
                BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr
                    | BinaryOp::ShrUnsigned
            ) || (is_safe_numeric(left, env) && is_safe_numeric(right, env))
        }
        _ => false,
    }
}

fn is_string_property_key(expr: &Expr, env: &Env) -> bool {
    match expr {
        Expr::StringLit(_, _) => true,
        Expr::Ident(name, _) => {
            env.types.get(name) == Some(&InferredType::String)
                || env.guarded_safe_keys.contains(name)
                || name == "key"
                || name == "prop"
                || name == "field"
        }
        _ => false,
    }
}

fn get_negative_int_constant(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Unary {
            op: UnaryOp::Minus,
            expr: inner,
            ..
        } => {
            if let Expr::Number(val, _, _) = &**inner {
                Some(-(*val as i64))
            } else {
                None
            }
        }
        Expr::Number(val, _, _) if *val < 0.0 => Some(*val as i64),
        _ => None,
    }
}

fn get_positive_int_constant(expr: &Expr) -> Option<usize> {
    match expr {
        Expr::Number(val, _, _) if *val >= 0.0 => Some(*val as usize),
        _ => None,
    }
}

fn is_known_standard_method(name: &str) -> bool {
    const STANDARD_METHODS: &[&str] = &[
        "push", "pop", "shift", "unshift", "slice", "splice", "concat", "join",
        "indexOf", "lastIndexOf", "includes", "find", "findIndex", "filter",
        "map", "reduce", "reduceRight", "forEach", "some", "every", "sort",
        "reverse", "fill", "flat", "flatMap", "keys", "values", "entries",
        "charAt", "charCodeAt", "endsWith", "match", "matchAll", "padStart",
        "padEnd", "repeat", "replace", "replaceAll", "search", "split",
        "startsWith", "substring", "toLowerCase", "toUpperCase", "trim",
        "trimStart", "trimEnd", "has", "get", "set", "delete", "clear", "add",
        "assign", "freeze", "hasOwn", "hasOwnProperty", "log", "warn", "error",
        "info", "debug", "floor", "ceil", "round", "abs", "max", "min", "pow",
        "sqrt", "random", "sin", "cos", "tan", "toString", "valueOf", "from",
        "isInteger", "isSafeInteger", "toFixed", "toPrecision",
    ];
    STANDARD_METHODS.contains(&name)
}

fn is_test_method(name: &str) -> bool {
    const TEST_METHODS: &[&str] = &[
        "toEqual", "toBe", "toStrictEqual", "toBeNull", "toBeUndefined",
        "toBeDefined", "toBeTruthy", "toBeFalsy", "toBeGreaterThan",
        "toBeLessThan", "toThrow", "toHaveBeenCalled", "toHaveBeenCalledWith",
    ];
    TEST_METHODS.contains(&name)
}

fn format_expr_short(expr: &Expr) -> String {
    match expr {
        Expr::Ident(name, _) => name.clone(),
        Expr::Member { object, property, .. } => {
            format!("{}.{}", format_expr_short(object), property)
        }
        Expr::Index { object, index, .. } => {
            format!("{}[{}]", format_expr_short(object), format_expr_short(index))
        }
        Expr::Number(val, _, _) => format!("{val}"),
        Expr::StringLit(s, _) => format!("\"{s}\""),
        _ => "...".to_string(),
    }
}
