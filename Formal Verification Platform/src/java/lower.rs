#![allow(dead_code, unused_variables, unused_imports, private_interfaces)]

//! Verification analysis and lowering engine for Java source code.
//!
//! Verifies 5 critical defect classes:
//! 1. `java-npe`: Null pointer dereferences without null checks.
//! 2. `java-array-oob`: Array indexing without bounds check.
//! 3. `java-div-by-zero`: Integer division/modulo by zero without guard.
//! 4. `java-class-cast`: Downcast without preceding instanceof guard.
//! 5. `java-negative-array-size`: Array creation with potentially negative dimension.

use crate::java::ast::*;
use crate::langaudit::Finding;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum NullState {
    DefinitelyNull,
    DefinitelyNonNull,
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
    pub loop_bounds: HashMap<String, String>, // loop_var -> array_name
    pub guarded_bounds: HashSet<(String, String)>, // (index_var, array_name)
    pub instanceof_guards: HashSet<(String, String)>, // (var_name, TargetType)
    pub guarded_non_zero: HashSet<String>,
    pub guarded_non_null: HashSet<String>,
    pub guarded_non_neg: HashSet<String>,
    pub known_classes: HashSet<String>,
}

impl Env {
    pub fn new() -> Self {
        let mut known_classes = HashSet::new();
        // Common standard Java classes to recognize static method/field accesses
        for c in &[
            "Math",
            "Arrays",
            "Collections",
            "System",
            "Integer",
            "Long",
            "Double",
            "Float",
            "Boolean",
            "Character",
            "Byte",
            "Short",
            "String",
            "Objects",
            "UUID",
            "Pattern",
            "Optional",
            "Files",
            "Paths",
            "Stream",
            "List",
            "Map",
            "Set",
            "Enum",
            "Thread",
            "Runtime",
            "Class",
            "BigInteger",
            "BigDecimal",
        ] {
            known_classes.insert(c.to_string());
        }

        Self {
            null_states: HashMap::new(),
            intervals: HashMap::new(),
            array_sizes: HashMap::new(),
            loop_bounds: HashMap::new(),
            guarded_bounds: HashSet::new(),
            instanceof_guards: HashSet::new(),
            guarded_non_zero: HashSet::new(),
            guarded_non_null: HashSet::new(),
            guarded_non_neg: HashSet::new(),
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

        // Collect declared type names
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

        // Parameters
        for p in &method.params {
            if p.ty.is_reference() {
                env.null_states.insert(p.name.clone(), NullState::Unknown);
                if p.ty.array_dims > 0 {
                    env.array_sizes.insert(p.name.clone(), None);
                }
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
            if p.ty.is_reference() {
                env.null_states.insert(p.name.clone(), NullState::Unknown);
                if p.ty.array_dims > 0 {
                    env.array_sizes.insert(p.name.clone(), None);
                }
            } else {
                env.intervals.insert(p.name.clone(), Interval::unknown());
            }
        }

        self.verify_block(body, &mut env);
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

    /// Verifies a statement. Returns true if execution definitely terminates (return/throw).
    fn verify_stmt(&mut self, stmt: &Stmt, env: &mut Env) -> bool {
        match stmt {
            Stmt::Block(b) => self.verify_block(b, env),

            Stmt::Expr(e, _) => {
                self.check_expr(e, env);
                false
            }

            Stmt::VarDecl { ty, vars, .. } => {
                for (name, init) in vars {
                    if let Some(init_expr) = init {
                        self.check_expr(init_expr, env);
                        if ty.is_reference() {
                            match init_expr {
                                Expr::Null(_) => {
                                    env.null_states
                                        .insert(name.clone(), NullState::DefinitelyNull);
                                }
                                Expr::NewObject { .. }
                                | Expr::NewArray { .. }
                                | Expr::StringLit(_, _) => {
                                    env.null_states
                                        .insert(name.clone(), NullState::DefinitelyNonNull);
                                    env.guarded_non_null.insert(name.clone());
                                }
                                Expr::Ident(other, _) => {
                                    if let Some(st) = env.null_states.get(other).cloned() {
                                        env.null_states.insert(name.clone(), st);
                                    }
                                }
                                _ => {
                                    env.null_states
                                        .insert(name.clone(), NullState::DefinitelyNonNull);
                                    env.guarded_non_null.insert(name.clone());
                                }
                            }
                            if let Expr::NewArray { dims, .. } = init_expr {
                                if let Some(first_dim) = dims.first() {
                                    if let Some(sz) = get_int_constant(first_dim) {
                                        if sz >= 0 {
                                            env.array_sizes
                                                .insert(name.clone(), Some(sz as usize));
                                        }
                                    }
                                }
                            }
                        } else {
                            if let Expr::Int(val, _) = init_expr {
                                env.intervals
                                    .insert(name.clone(), Interval::point(*val));
                                if *val != 0 {
                                    env.guarded_non_zero.insert(name.clone());
                                }
                                if *val >= 0 {
                                    env.guarded_non_neg.insert(name.clone());
                                }
                            } else {
                                env.intervals
                                    .insert(name.clone(), Interval::unknown());
                            }
                        }
                    } else {
                        if ty.is_reference() {
                            env.null_states.insert(name.clone(), NullState::Unknown);
                        } else {
                            env.intervals
                                .insert(name.clone(), Interval::unknown());
                        }
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
                if let Some(else_stmt) = else_branch {
                    let mut else_env = env.clone();
                    extract_guards(cond, false, &mut else_env);
                    else_terminates = self.verify_stmt(else_stmt, &mut else_env);
                }

                // If then-branch definitely terminates (e.g. if (x == null) return;),
                // the inverse guards apply to the subsequent statements!
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
                if let Some(inits) = init {
                    for s in inits {
                        self.verify_stmt(s, &mut for_env);
                    }
                }

                // Detect for (int i = 0; i < arr.length; i++)
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
                ..
            } => {
                self.check_expr(iterable, env);
                let mut body_env = env.clone();
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
                resources,
                body,
                catches,
                finally_body,
                ..
            } => {
                let mut try_env = env.clone();
                for r in resources {
                    self.verify_stmt(r, &mut try_env);
                }
                self.verify_block(body, &mut try_env);

                for c in catches {
                    let mut catch_env = env.clone();
                    catch_env
                        .null_states
                        .insert(c.param_name.clone(), NullState::DefinitelyNonNull);
                    catch_env.guarded_non_null.insert(c.param_name.clone());
                    self.verify_block(&c.body, &mut catch_env);
                }

                if let Some(fin) = finally_body {
                    self.verify_block(fin, env);
                }
                false
            }

            Stmt::Assert { cond, msg, .. } => {
                self.check_expr(cond, env);
                if let Some(m) = msg {
                    self.check_expr(m, env);
                }
                extract_guards(cond, true, env);
                false
            }

            Stmt::Synchronized { lock, body, .. } => {
                self.check_npe_obligation(lock, block_or_expr_line(lock), env);
                self.check_expr(lock, env);
                self.verify_block(body, env);
                false
            }

            Stmt::Break(_, _) | Stmt::Continue(_, _) | Stmt::Empty(_) => false,
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
            | Expr::ClassLiteral(_, _)
            | Expr::This(_)
            | Expr::Super(_)
            | Expr::Ident(_, _) => {}

            Expr::FieldAccess {
                target,
                field,
                line,
            } => {
                if field == "length" {
                    // Array length access: arr.length
                    self.check_npe_obligation(target, *line, env);
                } else {
                    self.check_npe_obligation(target, *line, env);
                }
                self.check_expr(target, env);
            }

            Expr::MethodCall {
                target,
                method,
                args,
                line,
            } => {
                if let Some(t) = target {
                    self.check_npe_obligation(t, *line, env);
                    self.check_expr(t, env);

                    // Objects.requireNonNull(x) pattern
                    if method == "requireNonNull" {
                        if let Expr::Ident(ref target_id, _) = **t {
                            if target_id == "Objects" {
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

            Expr::ArrayAccess { array, index, line } => {
                // Obligation 1: NPE on array
                self.check_npe_obligation(array, *line, env);
                // Obligation 2: Array bounds on index
                self.check_array_oob_obligation(array, index, *line, env);

                self.check_expr(array, env);
                self.check_expr(index, env);
            }

            Expr::NewObject { args, .. } => {
                for arg in args {
                    self.check_expr(arg, env);
                }
            }

            Expr::NewArray {
                dims, initializer, line, ..
            } => {
                for dim in dims {
                    // Obligation: Negative array size
                    self.check_negative_array_size_obligation(dim, *line, env);
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
                line,
            } => {
                self.check_expr(inner, env);
                self.check_class_cast_obligation(inner, target_ty, *line, env);
            }

            Expr::InstanceOf { expr: inner, .. } => {
                self.check_expr(inner, env);
            }

            Expr::Unary { expr: inner, .. } => {
                self.check_expr(inner, env);
            }

            Expr::Binary {
                op,
                left,
                right,
                line,
            } => {
                if *op == BinaryOp::Div || *op == BinaryOp::Rem {
                    // Obligation: Division by zero
                    self.check_div_by_zero_obligation(right, *line, env);
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
                line,
            } => {
                if *op == AssignOp::DivAssign || *op == AssignOp::RemAssign {
                    self.check_div_by_zero_obligation(value, *line, env);
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
                        Expr::NewObject { .. }
                        | Expr::NewArray { .. }
                        | Expr::StringLit(_, _) => {
                            env.null_states
                                .insert(name.clone(), NullState::DefinitelyNonNull);
                            env.guarded_non_null.insert(name.clone());
                        }
                        Expr::Int(val, _) => {
                            env.intervals.insert(name.clone(), Interval::point(*val));
                            if *val != 0 {
                                env.guarded_non_zero.insert(name.clone());
                            }
                            if *val >= 0 {
                                env.guarded_non_neg.insert(name.clone());
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
                if let Some(e) = body_expr {
                    self.check_expr(e, env);
                }
                if let Some(b) = body_block {
                    self.verify_block(b, env);
                }
            }

            Expr::MethodRef { target, .. } => {
                self.check_expr(target, env);
            }
        }
    }

    // ─── SPECIFIC OBLIGATION CHECKS ──────────────────────────────────────────

    fn check_npe_obligation(&mut self, target: &Expr, line: usize, env: &Env) {
        self.obligations_total += 1;

        match target {
            Expr::This(_) | Expr::Super(_) => {
                self.obligations_proved_safe += 1;
            }
            Expr::StringLit(_, _)
            | Expr::Int(_, _)
            | Expr::Float(_, _)
            | Expr::Bool(_, _)
            | Expr::CharLit(_, _)
            | Expr::ClassLiteral(_, _) => {
                self.obligations_proved_safe += 1;
            }
            Expr::NewObject { .. } | Expr::NewArray { .. } => {
                self.obligations_proved_safe += 1;
            }
            Expr::Binary {
                op: BinaryOp::Add, ..
            } => {
                // String concatenation produces a non-null String
                self.obligations_proved_safe += 1;
            }
            Expr::Ident(name, _) => {
                // Static class reference
                let is_local =
                    env.null_states.contains_key(name) || env.intervals.contains_key(name);
                let starts_upper = name.chars().next().map_or(false, |c| c.is_uppercase());
                if !is_local && (starts_upper || env.known_classes.contains(name)) {
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
                            line,
                            class: "java-npe".to_string(),
                            verdict: "proved-defect".to_string(),
                            message: format!("Null pointer dereference: '{name}' is definitely null"),
                        });
                    }
                    _ => {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line,
                            class: "java-npe".to_string(),
                            verdict: "possible-defect".to_string(),
                            message: format!(
                                "Null pointer dereference: '{name}' dereferenced without preceding null check"
                            ),
                        });
                    }
                }
            }
            Expr::MethodCall { method, .. } => {
                if is_known_non_null_method(method) {
                    self.obligations_proved_safe += 1;
                } else {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line,
                        class: "java-npe".to_string(),
                        verdict: "possible-defect".to_string(),
                        message: format!(
                            "Null pointer dereference: chained method call on potentially null result of '{method}()'"
                        ),
                    });
                }
            }
            Expr::FieldAccess { field, .. } => {
                if field == "out" || field == "err" || field == "in" {
                    self.obligations_proved_safe += 1;
                } else {
                    self.obligations_proved_safe += 1;
                }
            }
            _ => {
                self.obligations_proved_safe += 1;
            }
        }
    }

    fn check_array_oob_obligation(
        &mut self,
        array: &Expr,
        index: &Expr,
        line: usize,
        env: &Env,
    ) {
        self.obligations_total += 1;

        let arr_name = match array {
            Expr::Ident(name, _) => Some(name.clone()),
            _ => None,
        };

        if let Some(val) = get_int_constant(index) {
            if val < 0 {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "java-array-oob".to_string(),
                    verdict: "proved-defect".to_string(),
                    message: format!("Array index out of bounds: negative index {val}"),
                });
                return;
            }

            if let Some(ref name) = arr_name {
                if let Some(Some(sz)) = env.array_sizes.get(name) {
                    if (val as usize) >= *sz {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line,
                            class: "java-array-oob".to_string(),
                            verdict: "proved-defect".to_string(),
                            message: format!(
                                "Array index out of bounds: index {val} exceeds array length {sz}"
                            ),
                        });
                        return;
                    } else {
                        self.obligations_proved_safe += 1;
                        return;
                    }
                }
            }

            if val == 0 {
                if let Some(ref name) = arr_name {
                    if env.guarded_bounds.iter().any(|(_, arr)| arr == name) {
                        self.obligations_proved_safe += 1;
                        return;
                    }
                }
            }

            if let Some(ref name) = arr_name {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "java-array-oob".to_string(),
                    verdict: "possible-defect".to_string(),
                    message: format!(
                        "Array index '{name}[{val}]' without check that {name}.length > {val}"
                    ),
                });
            } else {
                self.obligations_proved_safe += 1;
            }
            return;
        }

        match index {
            Expr::Ident(idx_name, _) => {
                if let Some(ref name) = arr_name {
                    if env
                        .loop_bounds
                        .get(idx_name)
                        .map_or(false, |bound_arr| bound_arr == name)
                    {
                        self.obligations_proved_safe += 1;
                        return;
                    }
                    if env.guarded_bounds.contains(&(idx_name.clone(), name.clone())) {
                        self.obligations_proved_safe += 1;
                        return;
                    }
                } else if env.loop_bounds.contains_key(idx_name) {
                    self.obligations_proved_safe += 1;
                    return;
                }

                let msg = if let Some(ref name) = arr_name {
                    format!(
                        "Array index '{name}[{idx_name}]' without bounds check '0 <= {idx_name} < {name}.length'"
                    )
                } else {
                    format!("Array index without bounds check '0 <= {idx_name} < length'")
                };

                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "java-array-oob".to_string(),
                    verdict: "possible-defect".to_string(),
                    message: msg,
                });
            }
            _ => {
                self.obligations_proved_safe += 1;
            }
        }
    }

    fn check_div_by_zero_obligation(&mut self, right: &Expr, line: usize, env: &Env) {
        self.obligations_total += 1;

        if let Some(val) = get_int_constant(right) {
            if val == 0 {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "java-div-by-zero".to_string(),
                    verdict: "proved-defect".to_string(),
                    message: "Division or modulo by constant zero".to_string(),
                });
            } else {
                self.obligations_proved_safe += 1;
            }
            return;
        }

        match right {
            Expr::Float(_, _) => {
                self.obligations_proved_safe += 1;
            }
            Expr::Ident(name, _) => {
                if env.guarded_non_zero.contains(name) {
                    self.obligations_proved_safe += 1;
                    return;
                }
                if let Some(iv) = env.intervals.get(name) {
                    if !iv.contains_zero() {
                        self.obligations_proved_safe += 1;
                        return;
                    }
                }
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "java-div-by-zero".to_string(),
                    verdict: "possible-defect".to_string(),
                    message: format!(
                        "Division or modulo by variable '{name}' that can be zero without '!= 0' check"
                    ),
                });
            }
            _ => {
                self.obligations_proved_safe += 1;
            }
        }
    }

    fn check_class_cast_obligation(
        &mut self,
        expr: &Expr,
        target_ty: &JavaType,
        line: usize,
        env: &Env,
    ) {
        if !target_ty.is_reference() {
            return;
        }

        self.obligations_total += 1;

        if target_ty.name == "Object" {
            self.obligations_proved_safe += 1;
            return;
        }

        match expr {
            Expr::Ident(name, _) => {
                if env
                    .instanceof_guards
                    .contains(&(name.clone(), target_ty.name.clone()))
                {
                    self.obligations_proved_safe += 1;
                    return;
                }
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "java-class-cast".to_string(),
                    verdict: "possible-defect".to_string(),
                    message: format!(
                        "Downcast to '{}' without preceding 'instanceof' guard",
                        target_ty.name
                    ),
                });
            }
            Expr::NewObject { ty, .. } => {
                if ty.name == target_ty.name {
                    self.obligations_proved_safe += 1;
                } else {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line,
                        class: "java-class-cast".to_string(),
                        verdict: "possible-defect".to_string(),
                        message: format!(
                            "Downcast to '{}' without preceding 'instanceof' guard",
                            target_ty.name
                        ),
                    });
                }
            }
            _ => {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "java-class-cast".to_string(),
                    verdict: "possible-defect".to_string(),
                    message: format!(
                        "Downcast to '{}' without preceding 'instanceof' guard",
                        target_ty.name
                    ),
                });
            }
        }
    }

    fn check_negative_array_size_obligation(&mut self, dim: &Expr, line: usize, env: &Env) {
        self.obligations_total += 1;

        if let Some(val) = get_int_constant(dim) {
            if val < 0 {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "java-negative-array-size".to_string(),
                    verdict: "proved-defect".to_string(),
                    message: format!("Negative array size: new Type[{val}]"),
                });
            } else {
                self.obligations_proved_safe += 1;
            }
            return;
        }

        match dim {
            Expr::FieldAccess { field, .. } if field == "length" => {
                self.obligations_proved_safe += 1;
            }
            Expr::MethodCall { method, .. } if method == "size" || method == "length" => {
                self.obligations_proved_safe += 1;
            }
            Expr::Ident(name, _) => {
                if env.guarded_non_neg.contains(name) {
                    self.obligations_proved_safe += 1;
                    return;
                }
                if let Some(iv) = env.intervals.get(name) {
                    if iv.min >= 0 {
                        self.obligations_proved_safe += 1;
                        return;
                    }
                }
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "java-negative-array-size".to_string(),
                    verdict: "possible-defect".to_string(),
                    message: format!(
                        "Array allocation 'new Type[{name}]' where {name} < 0 is possible without non-negative check"
                    ),
                });
            }
            _ => {
                self.obligations_proved_safe += 1;
            }
        }
    }
}

// ─── HELPER FUNCTIONS ────────────────────────────────────────────────────────

fn get_int_constant(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Int(val, _) => Some(*val),
        Expr::Unary {
            op: UnaryOp::Minus,
            expr: inner,
            ..
        } => {
            if let Expr::Int(val, _) = **inner {
                Some(-val)
            } else {
                None
            }
        }
        Expr::Unary {
            op: UnaryOp::Plus,
            expr: inner,
            ..
        } => {
            if let Expr::Int(val, _) = **inner {
                Some(val)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_known_non_null_method(method: &str) -> bool {
    matches!(
        method,
        "append"
            | "toString"
            | "valueOf"
            | "substring"
            | "trim"
            | "toLowerCase"
            | "toUpperCase"
            | "replace"
            | "split"
            | "format"
            | "concat"
            | "charAt"
            | "strip"
            | "stripLeading"
            | "stripTrailing"
            | "repeat"
            | "formatted"
            | "indent"
            | "of"
            | "copyOf"
            | "asList"
            | "emptyList"
            | "emptySet"
            | "emptyMap"
            | "size"
            | "length"
            | "hashCode"
            | "equals"
            | "isEmpty"
            | "contains"
            | "iterator"
            | "stream"
    )
}

fn block_or_expr_line(expr: &Expr) -> usize {
    expr.line()
}

fn extract_guards(cond: &Expr, is_true: bool, env: &mut Env) {
    match cond {
        Expr::Binary {
            op: BinaryOp::Ne,
            left,
            right,
            ..
        } => {
            if is_true {
                if let (Expr::Ident(name, _), Expr::Null(_)) = (&**left, &**right) {
                    env.guarded_non_null.insert(name.clone());
                    env.null_states
                        .insert(name.clone(), NullState::DefinitelyNonNull);
                } else if let (Expr::Null(_), Expr::Ident(name, _)) = (&**left, &**right) {
                    env.guarded_non_null.insert(name.clone());
                    env.null_states
                        .insert(name.clone(), NullState::DefinitelyNonNull);
                } else if let (Expr::Ident(name, _), Expr::Int(0, _)) = (&**left, &**right) {
                    env.guarded_non_zero.insert(name.clone());
                } else if let (Expr::Int(0, _), Expr::Ident(name, _)) = (&**left, &**right) {
                    env.guarded_non_zero.insert(name.clone());
                }
            } else {
                if let (Expr::Ident(name, _), Expr::Null(_)) = (&**left, &**right) {
                    env.null_states
                        .insert(name.clone(), NullState::DefinitelyNull);
                    env.guarded_non_null.remove(name);
                }
            }
        }

        Expr::Binary {
            op: BinaryOp::Eq,
            left,
            right,
            ..
        } => {
            if is_true {
                if let (Expr::Ident(name, _), Expr::Null(_)) = (&**left, &**right) {
                    env.null_states
                        .insert(name.clone(), NullState::DefinitelyNull);
                    env.guarded_non_null.remove(name);
                } else if let (Expr::Null(_), Expr::Ident(name, _)) = (&**left, &**right) {
                    env.null_states
                        .insert(name.clone(), NullState::DefinitelyNull);
                    env.guarded_non_null.remove(name);
                }
            } else {
                if let (Expr::Ident(name, _), Expr::Null(_)) = (&**left, &**right) {
                    env.guarded_non_null.insert(name.clone());
                    env.null_states
                        .insert(name.clone(), NullState::DefinitelyNonNull);
                } else if let (Expr::Null(_), Expr::Ident(name, _)) = (&**left, &**right) {
                    env.guarded_non_null.insert(name.clone());
                    env.null_states
                        .insert(name.clone(), NullState::DefinitelyNonNull);
                } else if let (Expr::Ident(name, _), Expr::Int(0, _)) = (&**left, &**right) {
                    env.guarded_non_zero.insert(name.clone());
                } else if let (Expr::Int(0, _), Expr::Ident(name, _)) = (&**left, &**right) {
                    env.guarded_non_zero.insert(name.clone());
                }
            }
        }

        Expr::Binary {
            op: BinaryOp::Gt,
            left,
            right,
            ..
        } => {
            if is_true {
                if let (Expr::Ident(name, _), Expr::Int(0, _)) = (&**left, &**right) {
                    env.guarded_non_zero.insert(name.clone());
                    env.guarded_non_neg.insert(name.clone());
                } else if let (
                    Expr::FieldAccess { target, field, .. },
                    Expr::Int(0, _),
                ) = (&**left, &**right)
                {
                    if field == "length" {
                        if let Expr::Ident(ref arr_name, _) = **target {
                            env.guarded_bounds
                                .insert(("0".to_string(), arr_name.clone()));
                        }
                    }
                }
            } else {
                if let (Expr::Ident(name, _), Expr::Int(0, _)) = (&**left, &**right) {
                    // <= 0
                }
            }
        }

        Expr::Binary {
            op: BinaryOp::Ge,
            left,
            right,
            ..
        } => {
            if is_true {
                if let (Expr::Ident(name, _), Expr::Int(0, _)) = (&**left, &**right) {
                    env.guarded_non_neg.insert(name.clone());
                }
            } else {
                if let (Expr::Ident(name, _), Expr::Int(0, _)) = (&**left, &**right) {
                    // < 0
                }
            }
        }

        Expr::Binary {
            op: BinaryOp::Lt,
            left,
            right,
            ..
        } => {
            if is_true {
                // i < arr.length
                if let (Expr::Ident(idx, _), Expr::FieldAccess { target, field, .. }) =
                    (&**left, &**right)
                {
                    if field == "length" {
                        if let Expr::Ident(ref arr_name, _) = **target {
                            env.guarded_bounds
                                .insert((idx.clone(), arr_name.clone()));
                        }
                    }
                }
            } else {
                if let (Expr::Ident(name, _), Expr::Int(0, _)) = (&**left, &**right) {
                    // not (i < 0) => i >= 0
                    env.guarded_non_neg.insert(name.clone());
                }
            }
        }

        Expr::Binary {
            op: BinaryOp::Le,
            left,
            right,
            ..
        } => {
            if !is_true {
                if let (Expr::Ident(name, _), Expr::Int(0, _)) = (&**left, &**right) {
                    // not (i <= 0) => i > 0
                    env.guarded_non_neg.insert(name.clone());
                    env.guarded_non_zero.insert(name.clone());
                }
            }
        }

        Expr::InstanceOf {
            expr,
            check_ty,
            pattern_var,
            ..
        } => {
            if is_true {
                if let Expr::Ident(name, _) = &**expr {
                    env.instanceof_guards
                        .insert((name.clone(), check_ty.name.clone()));
                    env.guarded_non_null.insert(name.clone());
                    env.null_states
                        .insert(name.clone(), NullState::DefinitelyNonNull);
                }
                if let Some(pvar) = pattern_var {
                    env.instanceof_guards
                        .insert((pvar.clone(), check_ty.name.clone()));
                    env.guarded_non_null.insert(pvar.clone());
                    env.null_states
                        .insert(pvar.clone(), NullState::DefinitelyNonNull);
                }
            }
        }

        Expr::Unary {
            op: UnaryOp::Not,
            expr,
            ..
        } => {
            extract_guards(expr, !is_true, env);
        }

        Expr::Binary {
            op: BinaryOp::And,
            left,
            right,
            ..
        } => {
            if is_true {
                extract_guards(left, true, env);
                extract_guards(right, true, env);
            }
        }

        Expr::Binary {
            op: BinaryOp::Or,
            left,
            right,
            ..
        } => {
            if !is_true {
                extract_guards(left, false, env);
                extract_guards(right, false, env);
            }
        }

        _ => {}
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
        if let Expr::Ident(idx_name, _) = &**left {
            if let Expr::FieldAccess { target, field, .. } = &**right {
                if field == "length" {
                    if let Expr::Ident(arr_name, _) = &**target {
                        env.loop_bounds.insert(idx_name.clone(), arr_name.clone());
                        env.guarded_non_neg.insert(idx_name.clone());
                        env.guarded_non_null.insert(arr_name.clone());
                    }
                }
            }
        }
    }
}
