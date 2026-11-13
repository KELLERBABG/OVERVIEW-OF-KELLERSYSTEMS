//! Lowering and formal verification analysis for C# source code.
//!
//! Verifies 5 critical defect classes in C#:
//! 1. `cs-null-reference`: Dereferencing nullable `T?` or unverified null without null-check (`if (x != null)`) or null-coalescing (`??`)
//! 2. `cs-index-out-of-range`: Array indexing `arr[i]` where index is not provably within `0 <= i < arr.Length`
//! 3. `cs-span-oob`: `Span<T>` or `ReadOnlySpan<T>` slicing/indexing beyond length
//! 4. `cs-div-by-zero`: Division/modulo by integer variable that can be zero
//! 5. `cs-invalid-cast`: Explicit casting without preceding `is` or `as` check

use crate::csharp::ast::*;
use crate::langaudit::Finding;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum NullState {
    DefinitelyNull,
    DefinitelyNonNull,
    NullableAnnotated,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Interval {
    pub min: i64,
    pub max: i64,
}

impl Interval {
    pub fn point(v: i64) -> Self {
        Self { min: v, max: v }
    }
    pub fn non_negative() -> Self {
        Self {
            min: 0,
            max: 1_000_000_000,
        }
    }
    pub fn unknown() -> Self {
        Self {
            min: -1_000_000_000,
            max: 1_000_000_000,
        }
    }
    pub fn contains_zero(&self) -> bool {
        self.min <= 0 && self.max >= 0
    }
}

#[derive(Debug, Clone)]
pub struct Env {
    pub null_states: HashMap<String, NullState>,
    pub intervals: HashMap<String, Interval>,
    pub array_sizes: HashMap<String, Option<usize>>,
    pub span_sizes: HashMap<String, Option<usize>>,
    pub is_span: HashSet<String>,
    pub is_array: HashSet<String>,
    pub var_types: HashMap<String, CSharpType>,
    pub loop_bounds: HashMap<String, String>, // loop_var -> collection_name
    pub guarded_bounds: HashSet<(String, String)>, // (index_var, collection_name)
    pub is_guards: HashSet<(String, String)>, // (var_name, TargetType)
    pub as_guards: HashSet<(String, String)>, // (var_name, TargetType)
    pub guarded_non_zero: HashSet<String>,
    pub guarded_non_null: HashSet<String>,
    pub known_classes: HashSet<String>,
}

impl Env {
    pub fn new() -> Self {
        let mut known_classes = HashSet::new();
        for c in &[
            "Math",
            "Console",
            "Convert",
            "Array",
            "String",
            "Environment",
            "File",
            "Directory",
            "Path",
            "Task",
            "Thread",
            "GC",
            "DateTime",
            "TimeSpan",
            "Guid",
            "Regex",
            "BigInteger",
            "Assert",
            "Debug",
            "Trace",
            "BitConverter",
            "MemoryMarshal",
            "Unsafe",
            "MemoryExtensions",
            "Enumerable",
            "Buffer",
            "Int32",
            "Int64",
            "Double",
            "Single",
            "Boolean",
            "Char",
            "Byte",
            "ArgumentNullException",
            "ArgumentException",
            "InvalidOperationException",
            "IndexOutOfRangeException",
        ] {
            known_classes.insert(c.to_string());
        }

        Self {
            null_states: HashMap::new(),
            intervals: HashMap::new(),
            array_sizes: HashMap::new(),
            span_sizes: HashMap::new(),
            is_span: HashSet::new(),
            is_array: HashSet::new(),
            var_types: HashMap::new(),
            loop_bounds: HashMap::new(),
            guarded_bounds: HashSet::new(),
            is_guards: HashSet::new(),
            as_guards: HashSet::new(),
            guarded_non_zero: HashSet::new(),
            guarded_non_null: HashSet::new(),
            known_classes,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LowerResult {
    pub functions_lowered: usize,
    pub obligations_total: usize,
    pub obligations_proved_safe: usize,
    pub findings: Vec<Finding>,
}

pub struct Verifier {
    file_path: String,
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

    pub fn verify_unit(&mut self, unit: &CompilationUnit) -> LowerResult {
        let mut base_env = Env::new();

        // Collect declared type names into known_classes
        for t in &unit.types {
            self.collect_type_names(t, &mut base_env);
        }

        for t in &unit.types {
            self.verify_type(t, &base_env);
        }

        LowerResult {
            functions_lowered: self.functions_lowered,
            obligations_total: self.obligations_total,
            obligations_proved_safe: self.obligations_proved_safe,
            findings: self.findings.clone(),
        }
    }

    fn collect_type_names(&self, ty: &TypeDecl, env: &mut Env) {
        env.known_classes.insert(ty.name.clone());
        for nested in &ty.nested_types {
            self.collect_type_names(nested, env);
        }
    }

    fn verify_type(&mut self, ty: &TypeDecl, base_env: &Env) {
        for m in &ty.methods {
            self.verify_method(m, base_env);
        }
        for c in &ty.constructors {
            self.verify_constructor(c, base_env);
        }
        for p in &ty.properties {
            self.verify_property(p, base_env);
        }
        for nested in &ty.nested_types {
            self.verify_type(nested, base_env);
        }
    }

    fn verify_method(&mut self, method: &MethodDecl, base_env: &Env) {
        self.functions_lowered += 1;
        let Some(ref body) = method.body else {
            return;
        };

        let mut env = base_env.clone();

        for p in &method.params {
            env.var_types.insert(p.name.clone(), p.ty.clone());
            if p.ty.is_span || p.name.to_lowercase().contains("span") {
                env.is_span.insert(p.name.clone());
                env.span_sizes.insert(p.name.clone(), None);
            } else if p.ty.array_dims > 0 {
                env.is_array.insert(p.name.clone());
                env.array_sizes.insert(p.name.clone(), None);
            }

            if p.ty.is_nullable {
                env.null_states
                    .insert(p.name.clone(), NullState::NullableAnnotated);
            } else if p.ty.is_reference() {
                env.null_states.insert(p.name.clone(), NullState::Unknown);
            } else {
                env.intervals.insert(p.name.clone(), Interval::unknown());
            }
        }

        self.verify_block(body, &mut env);
    }

    fn verify_constructor(&mut self, ctor: &ConstructorDecl, base_env: &Env) {
        self.functions_lowered += 1;
        let Some(ref body) = ctor.body else {
            return;
        };

        let mut env = base_env.clone();

        for p in &ctor.params {
            env.var_types.insert(p.name.clone(), p.ty.clone());
            if p.ty.is_span || p.name.to_lowercase().contains("span") {
                env.is_span.insert(p.name.clone());
                env.span_sizes.insert(p.name.clone(), None);
            } else if p.ty.array_dims > 0 {
                env.is_array.insert(p.name.clone());
                env.array_sizes.insert(p.name.clone(), None);
            }

            if p.ty.is_nullable {
                env.null_states
                    .insert(p.name.clone(), NullState::NullableAnnotated);
            } else if p.ty.is_reference() {
                env.null_states.insert(p.name.clone(), NullState::Unknown);
            } else {
                env.intervals.insert(p.name.clone(), Interval::unknown());
            }
        }

        self.verify_block(body, &mut env);
    }

    fn verify_property(&mut self, prop: &PropertyDecl, base_env: &Env) {
        if let Some(ref getter) = prop.getter {
            self.functions_lowered += 1;
            let mut env = base_env.clone();
            self.verify_block(getter, &mut env);
        }
        if let Some(ref setter) = prop.setter {
            self.functions_lowered += 1;
            let mut env = base_env.clone();
            env.var_types.insert("value".into(), prop.ty.clone());
            if prop.ty.is_reference() {
                env.null_states
                    .insert("value".into(), NullState::Unknown);
            }
            self.verify_block(setter, &mut env);
        }
    }

    fn verify_block(&mut self, block: &Block, env: &mut Env) -> bool {
        for stmt in &block.stmts {
            let terminates = self.verify_stmt(stmt, env);
            if terminates {
                return true;
            }
        }
        false
    }

    fn verify_stmt(&mut self, stmt: &Stmt, env: &mut Env) -> bool {
        match stmt {
            Stmt::Block(b) => self.verify_block(b, env),

            Stmt::Expr(e, _) => {
                self.check_expr(e, env);
                false
            }

            Stmt::VarDecl { ty, vars, .. } => {
                for (name, init) in vars {
                    env.var_types.insert(name.clone(), ty.clone());
                    if ty.is_span || name.to_lowercase().contains("span") {
                        env.is_span.insert(name.clone());
                        env.span_sizes.insert(name.clone(), None);
                    } else if ty.array_dims > 0 {
                        env.is_array.insert(name.clone());
                        env.array_sizes.insert(name.clone(), None);
                    }

                    if let Some(init_expr) = init {
                        self.check_expr(init_expr, env);

                        match init_expr {
                            Expr::Null(_) => {
                                env.null_states
                                    .insert(name.clone(), NullState::DefinitelyNull);
                                env.guarded_non_null.remove(name);
                            }
                            Expr::NewObject { .. } => {
                                env.null_states
                                    .insert(name.clone(), NullState::DefinitelyNonNull);
                                env.guarded_non_null.insert(name.clone());
                            }
                            Expr::NewArray { dims, .. } => {
                                env.null_states
                                    .insert(name.clone(), NullState::DefinitelyNonNull);
                                env.guarded_non_null.insert(name.clone());
                                env.is_array.insert(name.clone());
                                if let Some(Expr::Int(sz, _)) = dims.first() {
                                    if *sz >= 0 {
                                        env.array_sizes.insert(name.clone(), Some(*sz as usize));
                                    }
                                }
                            }
                            Expr::StringLit(..) => {
                                env.null_states
                                    .insert(name.clone(), NullState::DefinitelyNonNull);
                                env.guarded_non_null.insert(name.clone());
                            }
                            Expr::Int(v, _) => {
                                env.intervals.insert(name.clone(), Interval::point(*v));
                                if *v != 0 {
                                    env.guarded_non_zero.insert(name.clone());
                                }
                            }
                            Expr::NullCoalescing { rhs, .. } => {
                                if !matches!(**rhs, Expr::Null(_)) {
                                    env.null_states
                                        .insert(name.clone(), NullState::DefinitelyNonNull);
                                    env.guarded_non_null.insert(name.clone());
                                }
                            }
                            Expr::Ident(other, _) => {
                                if env.guarded_non_null.contains(other) {
                                    env.guarded_non_null.insert(name.clone());
                                    env.null_states
                                        .insert(name.clone(), NullState::DefinitelyNonNull);
                                } else if let Some(st) = env.null_states.get(other).cloned() {
                                    env.null_states.insert(name.clone(), st);
                                }
                                if let Some(iv) = env.intervals.get(other).cloned() {
                                    env.intervals.insert(name.clone(), iv);
                                }
                            }
                            _ => {
                                if ty.is_reference() {
                                    if ty.is_nullable {
                                        env.null_states
                                            .insert(name.clone(), NullState::NullableAnnotated);
                                    } else {
                                        env.null_states.insert(name.clone(), NullState::Unknown);
                                    }
                                }
                            }
                        }
                    } else if ty.is_nullable {
                        env.null_states
                            .insert(name.clone(), NullState::NullableAnnotated);
                    } else if ty.is_reference() {
                        env.null_states.insert(name.clone(), NullState::Unknown);
                    }
                }
                false
            }

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
                if let Some(eb) = else_branch {
                    let mut else_env = env.clone();
                    extract_guards(cond, false, &mut else_env);
                    else_terminates = self.verify_stmt(eb, &mut else_env);
                }

                if then_terminates {
                    // Then branch definitely terminated: apply negative guard to main env
                    extract_guards(cond, false, env);
                } else if else_terminates {
                    // Else branch definitely terminated: apply positive guard to main env
                    extract_guards(cond, true, env);
                }

                then_terminates && else_terminates
            }

            Stmt::While { cond, body, .. } => {
                self.check_expr(cond, env);
                let mut loop_env = env.clone();
                extract_guards(cond, true, &mut loop_env);
                self.verify_stmt(body, &mut loop_env);
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
                if let Some(inits) = init {
                    for s in inits {
                        self.verify_stmt(s, &mut for_env);
                    }
                }

                if let Some(ref c) = cond {
                    self.check_expr(c, &mut for_env);
                    extract_loop_bounds(c, &mut for_env);
                }

                if let Some(updates) = update {
                    for u in updates {
                        self.check_expr(u, &mut for_env);
                    }
                }

                self.verify_stmt(body, &mut for_env);
                false
            }

            Stmt::ForEach {
                var_name,
                var_ty,
                iterable,
                body,
                loc,
            } => {
                self.check_null_obligation(iterable, *loc, env);
                self.check_expr(iterable, env);

                let mut body_env = env.clone();
                body_env.var_types.insert(var_name.clone(), var_ty.clone());
                if var_ty.is_reference() {
                    body_env
                        .null_states
                        .insert(var_name.clone(), NullState::DefinitelyNonNull);
                    body_env.guarded_non_null.insert(var_name.clone());
                }
                self.verify_stmt(body, &mut body_env);
                false
            }

            Stmt::Switch {
                selector, cases, ..
            } => {
                self.check_expr(selector, env);
                for c in cases {
                    for l in &c.labels {
                        self.check_expr(l, env);
                    }
                    let mut case_env = env.clone();
                    for s in &c.stmts {
                        self.verify_stmt(s, &mut case_env);
                    }
                }
                false
            }

            Stmt::Return(opt_expr, _) => {
                if let Some(e) = opt_expr {
                    self.check_expr(e, env);
                }
                true
            }

            Stmt::Throw(e, _) => {
                self.check_expr(e, env);
                true
            }

            Stmt::Try {
                body,
                catches,
                finally_body,
                ..
            } => {
                let mut try_env = env.clone();
                self.verify_block(body, &mut try_env);

                for c in catches {
                    let mut catch_env = env.clone();
                    if let Some(ref p) = c.param_name {
                        catch_env
                            .null_states
                            .insert(p.clone(), NullState::DefinitelyNonNull);
                        catch_env.guarded_non_null.insert(p.clone());
                    }
                    self.verify_block(&c.body, &mut catch_env);
                }

                if let Some(fin) = finally_body {
                    self.verify_block(fin, env);
                }
                false
            }

            Stmt::Using {
                init, expr, body, ..
            } => {
                let mut using_env = env.clone();
                if let Some(i) = init {
                    self.verify_stmt(i, &mut using_env);
                }
                if let Some(e) = expr {
                    self.check_expr(e, &mut using_env);
                }
                self.verify_stmt(body, &mut using_env);
                false
            }

            Stmt::Lock {
                lock_expr, body, ..
            } => {
                self.check_null_obligation(lock_expr, lock_expr.loc(), env);
                self.check_expr(lock_expr, env);
                self.verify_block(body, env);
                false
            }

            Stmt::LocalFunction(m) => {
                self.verify_method(m, env);
                false
            }

            Stmt::Break(_) | Stmt::Continue(_) | Stmt::Empty(_) => false,
        }
    }

    // ─── EXPRESSION CHECKING & OBLIGATION DISPATCH ───────────────────────────

    pub fn check_expr(&mut self, expr: &Expr, env: &mut Env) {
        match expr {
            Expr::Null(_)
            | Expr::Bool(_, _)
            | Expr::Int(_, _)
            | Expr::Float(_, _)
            | Expr::StringLit(_, _)
            | Expr::CharLit(_, _)
            | Expr::This(_)
            | Expr::Base(_)
            | Expr::Default(_, _)
            | Expr::Ident(_, _) => {}

            Expr::MemberAccess {
                target,
                member: _,
                is_null_conditional,
                loc,
            } => {
                if !*is_null_conditional {
                    self.check_null_obligation(target, *loc, env);
                } else {
                    self.obligations_total += 1;
                    self.obligations_proved_safe += 1;
                }
                self.check_expr(target, env);
            }

            Expr::MethodCall {
                target,
                method,
                args,
                is_null_conditional,
                loc,
            } => {
                if let Some(t) = target {
                    // Check if Target is Span and method is Slice
                    let is_span_target = match &**t {
                        Expr::Ident(id, _) => {
                            env.is_span.contains(id) || id.to_lowercase().contains("span")
                        }
                        _ => false,
                    };

                    if is_span_target && method == "Slice" {
                        self.check_span_slice_obligation(t, args, *loc, env);
                    } else if !*is_null_conditional {
                        self.check_null_obligation(t, *loc, env);
                    } else {
                        self.obligations_total += 1;
                        self.obligations_proved_safe += 1;
                    }

                    self.check_expr(t, env);

                    // ArgumentNullException.ThrowIfNull(x)
                    if method == "ThrowIfNull" {
                        if let Expr::Ident(ref target_id, _) = **t {
                            if target_id == "ArgumentNullException" {
                                if let Some(Expr::Ident(ref arg_id, _)) = args.first() {
                                    env.guarded_non_null.insert(arg_id.clone());
                                    env.null_states.insert(
                                        arg_id.clone(),
                                        NullState::DefinitelyNonNull,
                                    );
                                }
                            }
                        }
                    }
                }

                for arg in args {
                    self.check_expr(arg, env);
                }
            }

            Expr::ElementAccess {
                target,
                index,
                is_null_conditional,
                loc,
            } => {
                if !*is_null_conditional {
                    self.check_null_obligation(target, *loc, env);
                } else {
                    self.obligations_total += 1;
                    self.obligations_proved_safe += 1;
                }

                // Check if target is Span or Array
                let is_span = match &**target {
                    Expr::Ident(id, _) => {
                        env.is_span.contains(id) || id.to_lowercase().contains("span")
                    }
                    _ => false,
                };

                if is_span {
                    self.check_span_index_obligation(target, index, *loc, env);
                } else {
                    self.check_array_oob_obligation(target, index, *loc, env);
                }

                self.check_expr(target, env);
                self.check_expr(index, env);
            }

            Expr::NewObject { args, .. } => {
                for arg in args {
                    self.check_expr(arg, env);
                }
            }

            Expr::NewArray {
                dims, initializer, ..
            } => {
                for dim in dims {
                    self.check_expr(dim, env);
                }
                if let Some(inits) = initializer {
                    for init in inits {
                        self.check_expr(init, env);
                    }
                }
            }

            Expr::Cast {
                target_ty,
                expr: inner,
                loc,
            } => {
                self.check_expr(inner, env);
                self.check_cast_obligation(inner, target_ty, *loc, env);
            }

            Expr::IsPattern {
                expr: inner,
                pattern_ty,
                pattern_var,
                is_negated: _,
                ..
            } => {
                self.check_expr(inner, env);
                if let Expr::Ident(ref id, _) = **inner {
                    env.is_guards.insert((id.clone(), pattern_ty.name.clone()));
                }
                if let Some(ref pv) = pattern_var {
                    env.guarded_non_null.insert(pv.clone());
                    env.var_types.insert(pv.clone(), pattern_ty.clone());
                }
            }

            Expr::AsCast {
                expr: inner,
                target_ty,
                ..
            } => {
                self.check_expr(inner, env);
                if let Expr::Ident(ref id, _) = **inner {
                    env.as_guards.insert((id.clone(), target_ty.name.clone()));
                }
            }

            Expr::NullCoalescing { lhs, rhs, .. } => {
                self.check_expr(lhs, env);
                self.check_expr(rhs, env);
            }

            Expr::Unary { expr: inner, .. } => {
                self.check_expr(inner, env);
            }

            Expr::Binary {
                op,
                left,
                right,
                loc,
            } => {
                if *op == BinaryOp::Div || *op == BinaryOp::Rem {
                    self.check_div_by_zero_obligation(right, *loc, env);
                }

                if *op == BinaryOp::And {
                    self.check_expr(left, env);
                    let mut right_env = env.clone();
                    extract_guards(left, true, &mut right_env);
                    self.check_expr(right, &mut right_env);
                } else if *op == BinaryOp::Or {
                    self.check_expr(left, env);
                    let mut right_env = env.clone();
                    extract_guards(left, false, &mut right_env);
                    self.check_expr(right, &mut right_env);
                } else {
                    self.check_expr(left, env);
                    self.check_expr(right, env);
                }
            }

            Expr::Ternary {
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

            Expr::Assign {
                op,
                target,
                value,
                loc,
            } => {
                if *op == AssignOp::DivAssign || *op == AssignOp::RemAssign {
                    self.check_div_by_zero_obligation(value, *loc, env);
                }
                self.check_expr(target, env);
                self.check_expr(value, env);

                if let Expr::Ident(name, _) = &**target {
                    match &**value {
                        Expr::Null(_) => {
                            env.null_states
                                .insert(name.clone(), NullState::DefinitelyNull);
                            env.guarded_non_null.remove(name);
                        }
                        Expr::NewObject { .. } => {
                            env.null_states
                                .insert(name.clone(), NullState::DefinitelyNonNull);
                            env.guarded_non_null.insert(name.clone());
                        }
                        Expr::NewArray { dims, .. } => {
                            env.null_states
                                .insert(name.clone(), NullState::DefinitelyNonNull);
                            env.guarded_non_null.insert(name.clone());
                            env.is_array.insert(name.clone());
                            if let Some(Expr::Int(sz, _)) = dims.first() {
                                if *sz >= 0 {
                                    env.array_sizes.insert(name.clone(), Some(*sz as usize));
                                }
                            }
                        }
                        Expr::StringLit(..) => {
                            env.null_states
                                .insert(name.clone(), NullState::DefinitelyNonNull);
                            env.guarded_non_null.insert(name.clone());
                        }
                        Expr::Int(v, _) => {
                            env.intervals.insert(name.clone(), Interval::point(*v));
                            if *v != 0 {
                                env.guarded_non_zero.insert(name.clone());
                            }
                        }
                        Expr::Ident(other, _) => {
                            if env.guarded_non_null.contains(other) {
                                env.guarded_non_null.insert(name.clone());
                                env.null_states
                                    .insert(name.clone(), NullState::DefinitelyNonNull);
                            } else if let Some(st) = env.null_states.get(other).cloned() {
                                env.null_states.insert(name.clone(), st);
                            }
                            if let Some(iv) = env.intervals.get(other).cloned() {
                                env.intervals.insert(name.clone(), iv);
                            }
                        }
                        _ => {}
                    }
                }
            }

            Expr::Lambda {
                body_expr,
                body_block,
                ..
            } => {
                if let Some(be) = body_expr {
                    self.check_expr(be, env);
                }
                if let Some(bb) = body_block {
                    self.verify_block(bb, env);
                }
            }

            Expr::Tuple(parts, _) => {
                for p in parts {
                    self.check_expr(p, env);
                }
            }

            Expr::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.check_expr(s, env);
                }
                if let Some(e) = end {
                    self.check_expr(e, env);
                }
            }

            Expr::ThrowExpr(e, _) => {
                self.check_expr(e, env);
            }
        }
    }

    // ─── OBLIGATION 1: cs-null-reference ─────────────────────────────────────

    fn check_null_obligation(&mut self, target: &Expr, loc: SourceLoc, env: &Env) {
        match target {
            Expr::This(_) | Expr::Base(_) => {
                self.obligations_total += 1;
                self.obligations_proved_safe += 1;
            }
            Expr::StringLit(..) => {
                self.obligations_total += 1;
                self.obligations_proved_safe += 1;
            }
            Expr::NewObject { .. } | Expr::NewArray { .. } => {
                self.obligations_total += 1;
                self.obligations_proved_safe += 1;
            }
            Expr::NullCoalescing { rhs, .. } => {
                if !matches!(**rhs, Expr::Null(_)) {
                    self.obligations_total += 1;
                    self.obligations_proved_safe += 1;
                } else {
                    self.obligations_total += 1;
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: loc.line,
                        class: "cs-null-reference".into(),
                        verdict: "possible-defect".into(),
                        message: "Potential null reference: null-coalescing fallback is null"
                            .into(),
                    });
                }
            }
            Expr::Unary {
                op: UnaryOp::NullForgiving,
                ..
            } => {
                self.obligations_total += 1;
                self.obligations_proved_safe += 1;
            }
            Expr::Ident(name, _) => {
                if env.known_classes.contains(name) {
                    // Static access
                    return;
                }

                self.obligations_total += 1;

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
                            class: "cs-null-reference".into(),
                            verdict: "proved-defect".into(),
                            message: format!(
                                "Null reference dereference: '{name}' is definitely null"
                            ),
                        });
                    }
                    Some(NullState::NullableAnnotated) => {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: loc.line,
                            class: "cs-null-reference".into(),
                            verdict: "possible-defect".into(),
                            message: format!(
                                "Nullable reference dereference: '{name}' dereferenced without null check or null-coalescing"
                            ),
                        });
                    }
                    Some(NullState::Unknown) | None => {
                        let is_ref = env
                            .var_types
                            .get(name)
                            .map_or(true, |t| t.is_reference());
                        if is_ref {
                            self.findings.push(Finding {
                                file: self.file_path.clone(),
                                line: loc.line,
                                class: "cs-null-reference".into(),
                                verdict: "possible-defect".into(),
                                message: format!(
                                    "Null reference dereference: '{name}' dereferenced without preceding null check"
                                ),
                            });
                        } else {
                            self.obligations_proved_safe += 1;
                        }
                    }
                }
            }
            _ => {
                self.obligations_total += 1;
                self.obligations_proved_safe += 1;
            }
        }
    }

    // ─── OBLIGATION 2: cs-index-out-of-range ─────────────────────────────────

    fn check_array_oob_obligation(
        &mut self,
        target: &Expr,
        index: &Expr,
        loc: SourceLoc,
        env: &Env,
    ) {
        let arr_name = match target {
            Expr::Ident(name, _) => name.clone(),
            _ => "array".to_string(),
        };

        self.obligations_total += 1;

        if let Some(c) = eval_const_int(index) {
            if c < 0 {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "cs-index-out-of-range".into(),
                    verdict: "proved-defect".into(),
                    message: format!("Array index '{c}' is negative"),
                });
                return;
            }

            if let Some(Some(sz)) = env.array_sizes.get(&arr_name) {
                if (c as usize) < *sz {
                    self.obligations_proved_safe += 1;
                } else {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: loc.line,
                        class: "cs-index-out-of-range".into(),
                        verdict: "proved-defect".into(),
                        message: format!(
                            "Array index '{c}' exceeds known array size {sz}"
                        ),
                    });
                }
            } else if env.guarded_bounds.contains(&("0".into(), arr_name.clone()))
                || env.guarded_bounds.contains(&(c.to_string(), arr_name.clone()))
            {
                self.obligations_proved_safe += 1;
            } else {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "cs-index-out-of-range".into(),
                    verdict: "possible-defect".into(),
                    message: format!(
                        "Array index '{arr_name}[{c}]' without check that {arr_name}.Length > {c}"
                    ),
                });
            }
            return;
        }

        match index {
            Expr::Ident(idx_name, _) => {
                if env.loop_bounds.get(idx_name) == Some(&arr_name)
                    || env
                        .guarded_bounds
                        .contains(&(idx_name.clone(), arr_name.clone()))
                {
                    self.obligations_proved_safe += 1;
                } else {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: loc.line,
                        class: "cs-index-out-of-range".into(),
                        verdict: "possible-defect".into(),
                        message: format!(
                            "Array index '{arr_name}[{idx_name}]' without check that 0 <= index < {arr_name}.Length"
                        ),
                    });
                }
            }
            _ => {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "cs-index-out-of-range".into(),
                    verdict: "possible-defect".into(),
                    message: format!(
                        "Array index access into '{arr_name}' without provable bounds check"
                    ),
                });
            }
        }
    }

    // ─── OBLIGATION 3: cs-span-oob ───────────────────────────────────────────

    fn check_span_index_obligation(
        &mut self,
        target: &Expr,
        index: &Expr,
        loc: SourceLoc,
        env: &Env,
    ) {
        let span_name = match target {
            Expr::Ident(name, _) => name.clone(),
            _ => "span".to_string(),
        };

        self.obligations_total += 1;

        if let Some(c) = eval_const_int(index) {
            if c < 0 {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "cs-span-oob".into(),
                    verdict: "proved-defect".into(),
                    message: format!("Span index '{c}' is negative"),
                });
            } else if env.guarded_bounds.contains(&(c.to_string(), span_name.clone()))
                || env.guarded_bounds.contains(&("0".into(), span_name.clone()))
            {
                self.obligations_proved_safe += 1;
            } else {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "cs-span-oob".into(),
                    verdict: "possible-defect".into(),
                    message: format!(
                        "Span index '{span_name}[{c}]' without check against {span_name}.Length"
                    ),
                });
            }
            return;
        }

        match index {
            Expr::Ident(idx_name, _) => {
                if env.loop_bounds.get(idx_name) == Some(&span_name)
                    || env
                        .guarded_bounds
                        .contains(&(idx_name.clone(), span_name.clone()))
                {
                    self.obligations_proved_safe += 1;
                } else {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: loc.line,
                        class: "cs-span-oob".into(),
                        verdict: "possible-defect".into(),
                        message: format!(
                            "Span index '{span_name}[{idx_name}]' without check that 0 <= index < {span_name}.Length"
                        ),
                    });
                }
            }
            _ => {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "cs-span-oob".into(),
                    verdict: "possible-defect".into(),
                    message: format!(
                        "Span index access into '{span_name}' without provable bounds check"
                    ),
                });
            }
        }
    }

    fn check_span_slice_obligation(
        &mut self,
        target: &Expr,
        args: &[Expr],
        loc: SourceLoc,
        env: &Env,
    ) {
        let span_name = match target {
            Expr::Ident(name, _) => name.clone(),
            _ => "span".to_string(),
        };

        self.obligations_total += 1;

        if let Some(first_arg) = args.first() {
            if let Expr::Ident(id, _) = first_arg {
                if env.guarded_bounds.contains(&(id.clone(), span_name.clone()))
                    || env.loop_bounds.get(id) == Some(&span_name)
                {
                    self.obligations_proved_safe += 1;
                    return;
                }
            }
        }

        self.findings.push(Finding {
            file: self.file_path.clone(),
            line: loc.line,
            class: "cs-span-oob".into(),
            verdict: "possible-defect".into(),
            message: format!(
                "Span slice '{span_name}.Slice(...)' without check against {span_name}.Length"
            ),
        });
    }

    // ─── OBLIGATION 4: cs-div-by-zero ────────────────────────────────────────

    fn check_div_by_zero_obligation(&mut self, divisor: &Expr, loc: SourceLoc, env: &Env) {
        match divisor {
            Expr::Float(..) => {
                self.obligations_total += 1;
                self.obligations_proved_safe += 1;
            }
            Expr::Int(val, _) => {
                self.obligations_total += 1;
                if *val == 0 {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: loc.line,
                        class: "cs-div-by-zero".into(),
                        verdict: "proved-defect".into(),
                        message: "Division by constant zero".into(),
                    });
                } else {
                    self.obligations_proved_safe += 1;
                }
            }
            Expr::Ident(name, _) => {
                self.obligations_total += 1;
                if env.guarded_non_zero.contains(name) {
                    self.obligations_proved_safe += 1;
                } else if env
                    .intervals
                    .get(name)
                    .map_or(false, |iv| !iv.contains_zero())
                {
                    self.obligations_proved_safe += 1;
                } else {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: loc.line,
                        class: "cs-div-by-zero".into(),
                        verdict: "possible-defect".into(),
                        message: format!(
                            "Division by potentially zero variable '{name}' without zero check"
                        ),
                    });
                }
            }
            _ => {
                self.obligations_total += 1;
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "cs-div-by-zero".into(),
                    verdict: "possible-defect".into(),
                    message: "Division by unverified divisor".into(),
                });
            }
        }
    }

    // ─── OBLIGATION 5: cs-invalid-cast ───────────────────────────────────────

    fn check_cast_obligation(
        &mut self,
        expr: &Expr,
        target_ty: &CSharpType,
        loc: SourceLoc,
        env: &Env,
    ) {
        // Safe casts: primitive numeric/bool/char conversions, object upcast, dynamic
        if target_ty.is_numeric_primitive()
            || target_ty.name == "bool"
            || target_ty.name == "object"
            || target_ty.name == "dynamic"
            || target_ty.name == "nint"
            || target_ty.name == "nuint"
        {
            self.obligations_total += 1;
            self.obligations_proved_safe += 1;
            return;
        }

        self.obligations_total += 1;

        match expr {
            Expr::Ident(name, _) => {
                if env.is_guards.contains(&(name.clone(), target_ty.name.clone()))
                    || env.as_guards.contains(&(name.clone(), target_ty.name.clone()))
                {
                    self.obligations_proved_safe += 1;
                } else {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: loc.line,
                        class: "cs-invalid-cast".into(),
                        verdict: "possible-defect".into(),
                        message: format!(
                            "Explicit cast to '{target_ty}' without preceding 'is' or 'as' check"
                        ),
                    });
                }
            }
            Expr::NewObject { ty, .. } if ty.name == target_ty.name => {
                self.obligations_proved_safe += 1;
            }
            _ => {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: loc.line,
                    class: "cs-invalid-cast".into(),
                    verdict: "possible-defect".into(),
                    message: format!(
                        "Explicit cast to '{target_ty}' without preceding 'is' or 'as' check"
                    ),
                });
            }
        }
    }
}

fn eval_const_int(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Int(v, _) => Some(*v),
        Expr::Unary {
            op: UnaryOp::Minus,
            expr,
            ..
        } => eval_const_int(expr).map(|v| -v),
        Expr::Unary {
            op: UnaryOp::Plus,
            expr,
            ..
        } => eval_const_int(expr),
        _ => None,
    }
}

// ─── GUARD EXTRACTION ────────────────────────────────────────────────────────

fn extract_guards(cond: &Expr, positive: bool, env: &mut Env) {
    match cond {
        Expr::Binary {
            op, left, right, ..
        } => {
            if *op == BinaryOp::Ne && positive || *op == BinaryOp::Eq && !positive {
                // x != null
                if matches!(**right, Expr::Null(_)) {
                    if let Expr::Ident(ref id, _) = **left {
                        env.guarded_non_null.insert(id.clone());
                        env.null_states
                            .insert(id.clone(), NullState::DefinitelyNonNull);
                    }
                } else if matches!(**left, Expr::Null(_)) {
                    if let Expr::Ident(ref id, _) = **right {
                        env.guarded_non_null.insert(id.clone());
                        env.null_states
                            .insert(id.clone(), NullState::DefinitelyNonNull);
                    }
                }
                // x != 0
                if let Expr::Int(0, _) = **right {
                    if let Expr::Ident(ref id, _) = **left {
                        env.guarded_non_zero.insert(id.clone());
                    }
                } else if let Expr::Int(0, _) = **left {
                    if let Expr::Ident(ref id, _) = **right {
                        env.guarded_non_zero.insert(id.clone());
                    }
                }
            } else if *op == BinaryOp::Eq && positive || *op == BinaryOp::Ne && !positive {
                // x == null
                if matches!(**right, Expr::Null(_)) {
                    if let Expr::Ident(ref id, _) = **left {
                        env.null_states
                            .insert(id.clone(), NullState::DefinitelyNull);
                    }
                }
                // x == 0
                if let Expr::Int(0, _) = **right {
                    if let Expr::Ident(ref id, _) = **left {
                        env.intervals.insert(id.clone(), Interval::point(0));
                    }
                }
            } else if (*op == BinaryOp::Lt && positive) || (*op == BinaryOp::Ge && !positive) {
                // i < arr.Length
                if let Expr::Ident(ref idx_id, _) = **left {
                    if let Some(arr_id) = get_length_target(right) {
                        env.guarded_bounds.insert((idx_id.clone(), arr_id));
                    }
                }
                // 0 < x => x > 0
                if let Expr::Int(0, _) = **left {
                    if let Expr::Ident(ref id, _) = **right {
                        env.guarded_non_zero.insert(id.clone());
                    }
                }
            } else if (*op == BinaryOp::Gt && positive) || (*op == BinaryOp::Le && !positive) {
                // arr.Length > i
                if let Expr::Ident(ref idx_id, _) = **right {
                    if let Some(arr_id) = get_length_target(left) {
                        env.guarded_bounds.insert((idx_id.clone(), arr_id));
                    }
                }
                // x > 0
                if let Expr::Int(0, _) = **right {
                    if let Expr::Ident(ref id, _) = **left {
                        env.guarded_non_zero.insert(id.clone());
                    }
                }
            } else if *op == BinaryOp::And && positive {
                extract_guards(left, true, env);
                extract_guards(right, true, env);
            } else if *op == BinaryOp::Or && !positive {
                extract_guards(left, false, env);
                extract_guards(right, false, env);
            }
        }

        Expr::Unary {
            op: UnaryOp::Not,
            expr: inner,
            ..
        } => {
            extract_guards(inner, !positive, env);
        }

        Expr::IsPattern {
            expr: inner,
            pattern_ty,
            pattern_var,
            is_negated,
            ..
        } => {
            let eff_pos = if *is_negated { !positive } else { positive };
            if eff_pos {
                if let Expr::Ident(ref id, _) = **inner {
                    if pattern_ty.name != "null" {
                        env.is_guards.insert((id.clone(), pattern_ty.name.clone()));
                        env.guarded_non_null.insert(id.clone());
                        env.null_states
                            .insert(id.clone(), NullState::DefinitelyNonNull);
                    }
                }
                if let Some(ref pv) = pattern_var {
                    env.guarded_non_null.insert(pv.clone());
                    env.var_types.insert(pv.clone(), pattern_ty.clone());
                }
            }
        }

        _ => {}
    }
}

fn get_length_target(expr: &Expr) -> Option<String> {
    match expr {
        Expr::MemberAccess { target, member, .. }
            if member == "Length" || member == "Count" =>
        {
            if let Expr::Ident(ref id, _) = **target {
                Some(id.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_loop_bounds(cond: &Expr, env: &mut Env) {
    if let Expr::Binary {
        op: BinaryOp::Lt,
        left,
        right,
        ..
    } = cond
    {
        if let Expr::Ident(ref idx_id, _) = **left {
            if let Some(arr_id) = get_length_target(right) {
                env.loop_bounds.insert(idx_id.clone(), arr_id.clone());
                env.guarded_bounds.insert((idx_id.clone(), arr_id));
            }
        }
    }
}
