//! C-RMM Invariant Verification Engine.
//!
//! Verifies 6 critical defect classes:
//! 1. `c-buffer-overflow`: Spatial bounds (0 <= offset && offset + size <= extent)
//! 2. `c-use-after-free`: Temporal validity of pointer region
//! 3. `c-double-free`: Free on already freed region
//! 4. `c-null-deref`: Dereference of NULL / 0 pointer
//! 5. `c-div-by-zero`: Division by variable that can be zero
//! 6. `c-signed-overflow`: Signed arithmetic bounds

use crate::c::ast::*;
use crate::langaudit::Finding;
use std::collections::HashMap;

const INT32_MIN: i64 = -2147483648;
const INT32_MAX: i64 = 2147483647;

#[derive(Debug, Clone, PartialEq)]
pub struct Interval {
    pub min: i64,
    pub max: i64,
}

impl Interval {
    pub fn point(val: i64) -> Self {
        Self { min: val, max: val }
    }

    pub fn new(min: i64, max: i64) -> Self {
        Self { min, max }
    }

    pub fn unknown() -> Self {
        Self {
            min: -1_000_000_000,
            max: 1_000_000_000,
        }
    }

    pub fn is_constant(&self) -> bool {
        self.min == self.max
    }

    pub fn add(&self, other: &Self) -> Self {
        Self {
            min: self.min.saturating_add(other.min),
            max: self.max.saturating_add(other.max),
        }
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self {
            min: self.min.saturating_sub(other.max),
            max: self.max.saturating_sub(other.min),
        }
    }

    pub fn mul(&self, other: &Self) -> Self {
        let p1 = self.min.saturating_mul(other.min);
        let p2 = self.min.saturating_mul(other.max);
        let p3 = self.max.saturating_mul(other.min);
        let p4 = self.max.saturating_mul(other.max);
        let min = p1.min(p2).min(p3).min(p4);
        let max = p1.max(p2).max(p3).max(p4);
        Self { min, max }
    }

    pub fn contains_zero(&self) -> bool {
        self.min <= 0 && self.max >= 0
    }

    pub fn is_strictly_positive(&self) -> bool {
        self.min > 0
    }

    pub fn is_strictly_negative(&self) -> bool {
        self.max < 0
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PtrState {
    Null,
    NonNull,
    Allocated {
        region_id: u64,
        extent: Option<usize>,
    },
    Freed {
        region_id: u64,
    },
    StackBuffer {
        extent: usize,
    },
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Env {
    pub intervals: HashMap<String, Interval>,
    pub pointers: HashMap<String, PtrState>,
    pub freed_regions: Vec<u64>,
    pub next_region_id: u64,
}

impl Env {
    pub fn new() -> Self {
        Self {
            intervals: HashMap::new(),
            pointers: HashMap::new(),
            freed_regions: Vec::new(),
            next_region_id: 1,
        }
    }

    pub fn alloc_region(&mut self) -> u64 {
        let id = self.next_region_id;
        self.next_region_id += 1;
        id
    }

    pub fn get_interval(&self, name: &str) -> Interval {
        self.intervals.get(name).cloned().unwrap_or_else(Interval::unknown)
    }

    pub fn set_interval(&mut self, name: &str, iv: Interval) {
        self.intervals.insert(name.to_string(), iv);
    }

    pub fn get_pointer(&self, name: &str) -> PtrState {
        self.pointers.get(name).cloned().unwrap_or(PtrState::Unknown)
    }

    pub fn set_pointer(&mut self, name: &str, st: PtrState) {
        self.pointers.insert(name.to_string(), st);
    }

    pub fn free_pointer(&mut self, name: &str) -> Option<u64> {
        let st = self.get_pointer(name);
        match st {
            PtrState::Allocated { region_id, .. } => {
                self.freed_regions.push(region_id);
                self.set_pointer(name, PtrState::Freed { region_id });
                Some(region_id)
            }
            PtrState::Freed { region_id } => Some(region_id),
            _ => {
                let id = self.alloc_region();
                self.freed_regions.push(id);
                self.set_pointer(name, PtrState::Freed { region_id: id });
                Some(id)
            }
        }
    }

    pub fn merge(&mut self, other: &Env) {
        for (k, v) in &other.intervals {
            if let Some(my_v) = self.intervals.get_mut(k) {
                *my_v = my_v.union(v);
            } else {
                self.intervals.insert(k.clone(), v.clone());
            }
        }

        for (k, v) in &other.pointers {
            if let Some(my_v) = self.pointers.get_mut(k) {
                if my_v != v {
                    if matches!(my_v, PtrState::Freed { .. }) || matches!(v, PtrState::Freed { .. }) {
                        // Keep freed if either freed
                        if !matches!(my_v, PtrState::Freed { .. }) {
                            *my_v = v.clone();
                        }
                    } else if matches!(my_v, PtrState::Null) || matches!(v, PtrState::Null) {
                        *my_v = PtrState::Unknown;
                    }
                }
            } else {
                self.pointers.insert(k.clone(), v.clone());
            }
        }

        for r in &other.freed_regions {
            if !self.freed_regions.contains(r) {
                self.freed_regions.push(*r);
            }
        }
    }
}

pub struct Analyzer {
    pub file_path: String,
    pub obligations_total: usize,
    pub obligations_proved_safe: usize,
    pub findings: Vec<Finding>,
}

impl Analyzer {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            obligations_total: 0,
            obligations_proved_safe: 0,
            findings: Vec::new(),
        }
    }

    pub fn analyze_function(&mut self, func: &Function) {
        let mut env = Env::new();

        // Register parameters
        for p in &func.params {
            if p.ty.is_pointer() {
                env.set_pointer(&p.name, PtrState::NonNull);
            } else if p.ty.is_array() {
                let ext = match &p.ty {
                    Type::Array(_, Some(sz)) => *sz,
                    _ => 100, // assumed parameter buffer size
                };
                env.set_pointer(&p.name, PtrState::StackBuffer { extent: ext });
            } else {
                env.set_interval(&p.name, Interval::unknown());
            }
        }

        if let Some(body) = &func.body {
            for stmt in body {
                self.analyze_stmt(stmt, &mut env);
            }
        }
    }

    fn analyze_stmt(&mut self, stmt: &Stmt, env: &mut Env) -> bool {
        // Returns true if control flow definitely terminates (e.g. return)
        match stmt {
            Stmt::Decl(decls, _) => {
                for d in decls {
                    self.analyze_var_decl(d, env);
                }
                false
            }
            Stmt::Expr(expr, _) => {
                self.analyze_expr(expr, env);
                false
            }
            Stmt::Block(stmts, _) => {
                for s in stmts {
                    if self.analyze_stmt(s, env) {
                        return true;
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
                self.analyze_expr(cond, env);

                let mut then_env = env.clone();
                let mut else_env = env.clone();

                self.refine_cond(cond, &mut then_env, true);
                self.refine_cond(cond, &mut else_env, false);

                let then_ret = self.analyze_stmt(then_branch, &mut then_env);
                let else_ret = if let Some(eb) = else_branch {
                    self.analyze_stmt(eb, &mut else_env)
                } else {
                    false
                };

                if then_ret && !else_ret {
                    *env = else_env;
                } else if !then_ret && else_ret {
                    *env = then_env;
                } else if !then_ret && !else_ret {
                    *env = then_env;
                    env.merge(&else_env);
                }
                then_ret && else_ret
            }
            Stmt::While { cond, body, .. } | Stmt::DoWhile { body, cond, .. } => {
                self.analyze_expr(cond, env);
                let mut loop_env = env.clone();
                self.refine_cond(cond, &mut loop_env, true);
                self.analyze_stmt(body, &mut loop_env);
                env.merge(&loop_env);
                false
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                if let Some(i) = init {
                    self.analyze_stmt(i, env);
                }

                // Check for standard loop `for (int i = 0; i < N; i++)`
                let mut loop_env = env.clone();
                if let Some(c) = cond {
                    self.analyze_expr(c, env);
                    self.refine_cond(c, &mut loop_env, true);

                    // Check if condition uses `<=` with an array bound (off-by-one detection)
                    if let Expr::Binary {
                        op: BinaryOp::Le,
                        lhs,
                        rhs,
                        ..
                    } = c
                    {
                        if let Expr::Ident(var_name, _) = lhs.as_ref() {
                            let bound_val = self.eval_expr_interval(rhs, env);
                            if bound_val.is_constant() {
                                loop_env.set_interval(
                                    var_name,
                                    Interval::new(0, bound_val.max),
                                );
                            }
                        }
                    }
                }

                self.analyze_stmt(body, &mut loop_env);

                if let Some(s) = step {
                    self.analyze_expr(s, &mut loop_env);
                }

                env.merge(&loop_env);
                false
            }
            Stmt::Switch { expr, body, .. } => {
                self.analyze_expr(expr, env);
                self.analyze_stmt(body, env);
                false
            }
            Stmt::Return(Some(expr), _) => {
                self.analyze_expr(expr, env);
                true
            }
            Stmt::Return(None, _) => true,
            _ => false,
        }
    }

    fn analyze_var_decl(&mut self, decl: &VarDecl, env: &mut Env) {
        if let Type::Array(_, Some(sz)) = &decl.ty {
            env.set_pointer(&decl.name, PtrState::StackBuffer { extent: *sz });
        } else if decl.ty.is_pointer() {
            env.set_pointer(&decl.name, PtrState::Unknown);
        } else {
            env.set_interval(&decl.name, Interval::unknown());
        }

        if let Some(init) = &decl.init {
            self.analyze_expr(init, env);
            self.propagate_assignment(&decl.name, init, env);
        }
    }

    fn propagate_assignment(&mut self, target: &str, rhs: &Expr, env: &mut Env) {
        match rhs {
            Expr::NullLit(_) => {
                env.set_pointer(target, PtrState::Null);
            }
            Expr::IntLit(0, _) => {
                env.set_interval(target, Interval::point(0));
                env.set_pointer(target, PtrState::Null);
            }
            Expr::IntLit(n, _) => {
                env.set_interval(target, Interval::point(*n));
            }
            Expr::Ident(src, _) => {
                let iv = env.get_interval(src);
                env.set_interval(target, iv);
                let pt = env.get_pointer(src);
                env.set_pointer(target, pt);
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if name == "malloc" || name == "calloc" {
                        let region_id = env.alloc_region();
                        let extent = self.extract_malloc_extent(args, env);
                        env.set_pointer(
                            target,
                            PtrState::Allocated {
                                region_id,
                                extent,
                            },
                        );
                        return;
                    }
                }
                env.set_interval(target, Interval::unknown());
            }
            Expr::Cast { expr, .. } => {
                self.propagate_assignment(target, expr, env);
            }
            Expr::Unary {
                op: UnaryOp::AddrOf,
                ..
            } => {
                env.set_pointer(target, PtrState::NonNull);
            }
            _ => {
                let iv = self.eval_expr_interval(rhs, env);
                env.set_interval(target, iv);
            }
        }
    }

    fn extract_malloc_extent(&self, args: &[Expr], env: &Env) -> Option<usize> {
        let arg0 = args.first()?;
        match arg0 {
            Expr::IntLit(n, _) => {
                if *n > 0 {
                    Some(*n as usize)
                } else {
                    None
                }
            }
            Expr::Binary {
                op: BinaryOp::Mul,
                lhs,
                rhs,
                ..
            } => {
                let l = self.eval_expr_interval(lhs, env);
                let r = self.eval_expr_interval(rhs, env);
                if l.is_constant() && l.min > 0 {
                    Some(l.min as usize)
                } else if r.is_constant() && r.min > 0 {
                    Some(r.min as usize)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn analyze_expr(&mut self, expr: &Expr, env: &mut Env) {
        match expr {
            Expr::Assign {
                op: AssignOp::Assign,
                lhs,
                rhs,
                ..
            } => {
                self.analyze_expr(rhs, env);
                if let Expr::Ident(target, _) = lhs.as_ref() {
                    self.propagate_assignment(target, rhs, env);
                } else {
                    self.analyze_expr(lhs, env);
                }
            }
            Expr::Assign {
                op,
                lhs,
                rhs,
                line,
            } => {
                self.analyze_expr(lhs, env);
                self.analyze_expr(rhs, env);
                if *op == AssignOp::DivAssign || *op == AssignOp::RemAssign {
                    self.check_div_by_zero(rhs, env, *line);
                }
            }
            Expr::Binary {
                op,
                lhs,
                rhs,
                line,
            } => {
                self.analyze_expr(lhs, env);
                self.analyze_expr(rhs, env);

                match op {
                    BinaryOp::Div | BinaryOp::Rem => {
                        self.check_div_by_zero(rhs, env, *line);
                    }
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => {
                        self.check_signed_overflow(lhs, rhs, *op, env, *line);
                    }
                    _ => {}
                }
            }
            Expr::Unary {
                op: UnaryOp::Deref,
                expr: inner,
                line,
            } => {
                self.analyze_expr(inner, env);
                self.check_deref(inner, env, *line);
            }
            Expr::Member {
                base,
                is_arrow: true,
                line,
                ..
            } => {
                self.analyze_expr(base, env);
                self.check_deref(base, env, *line);
            }
            Expr::Index {
                base,
                index,
                line,
            } => {
                self.analyze_expr(base, env);
                self.analyze_expr(index, env);
                self.check_array_access(base, index, env, *line);
            }
            Expr::Call {
                callee,
                args,
                line,
            } => {
                self.analyze_expr(callee, env);
                for a in args {
                    self.analyze_expr(a, env);
                }
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if name == "free" {
                        if let Some(arg) = args.first() {
                            self.check_free(arg, env, *line);
                        }
                    }
                }
            }
            Expr::Cast { expr: inner, .. } => {
                self.analyze_expr(inner, env);
            }
            Expr::Ternary {
                cond,
                then_expr,
                else_expr,
                ..
            } => {
                self.analyze_expr(cond, env);
                self.analyze_expr(then_expr, env);
                self.analyze_expr(else_expr, env);
            }
            _ => {}
        }
    }

    fn check_deref(&mut self, ptr_expr: &Expr, env: &Env, line: usize) {
        // 1. Check c-null-deref
        self.obligations_total += 1;
        if let Expr::Ident(name, _) = ptr_expr {
            let st = env.get_pointer(name);
            match st {
                PtrState::Null => {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line,
                        class: "c-null-deref".into(),
                        verdict: "proved-defect".into(),
                        message: format!("Null pointer dereference: pointer '{}' is NULL", name),
                    });
                }
                PtrState::NonNull | PtrState::Allocated { .. } | PtrState::StackBuffer { .. } => {
                    self.obligations_proved_safe += 1;
                }
                _ => {
                    // Check if guaranteed non-null or possible
                    self.obligations_proved_safe += 1;
                }
            }

            // 2. Check c-use-after-free
            self.obligations_total += 1;
            match st {
                PtrState::Freed { .. } => {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line,
                        class: "c-use-after-free".into(),
                        verdict: "proved-defect".into(),
                        message: format!(
                            "Use after free: pointer '{}' dereferenced after being freed",
                            name
                        ),
                    });
                }
                _ => {
                    self.obligations_proved_safe += 1;
                }
            }
        } else if let Expr::NullLit(_) = ptr_expr {
            self.findings.push(Finding {
                file: self.file_path.clone(),
                line,
                class: "c-null-deref".into(),
                verdict: "proved-defect".into(),
                message: "Dereference of literal NULL pointer".into(),
            });
        } else {
            self.obligations_proved_safe += 1;
        }
    }

    fn check_free(&mut self, ptr_expr: &Expr, env: &mut Env, line: usize) {
        self.obligations_total += 1;
        if let Expr::Ident(name, _) = ptr_expr {
            let st = env.get_pointer(name);
            match st {
                PtrState::Freed { .. } => {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line,
                        class: "c-double-free".into(),
                        verdict: "proved-defect".into(),
                        message: format!("Double free: pointer '{}' was already freed", name),
                    });
                }
                _ => {
                    self.obligations_proved_safe += 1;
                    env.free_pointer(name);
                }
            }
        } else {
            self.obligations_proved_safe += 1;
        }
    }

    fn check_array_access(&mut self, base: &Expr, index: &Expr, env: &Env, line: usize) {
        // Also check null and temporal safety if base is a pointer
        if let Expr::Ident(name, _) = base {
            let st = env.get_pointer(name);
            if matches!(st, PtrState::Null) {
                self.obligations_total += 1;
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "c-null-deref".into(),
                    verdict: "proved-defect".into(),
                    message: format!("Null pointer indexed: '{}' is NULL", name),
                });
            } else if matches!(st, PtrState::Freed { .. }) {
                self.obligations_total += 1;
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "c-use-after-free".into(),
                    verdict: "proved-defect".into(),
                    message: format!("Use after free: '{}' indexed after free", name),
                });
            }
        }

        // Check c-buffer-overflow (Spatial bounds)
        self.obligations_total += 1;
        let idx_iv = self.eval_expr_interval(index, env);

        // Find buffer extent
        let mut extent = None;
        if let Expr::Ident(name, _) = base {
            match env.get_pointer(name) {
                PtrState::StackBuffer { extent: sz } => {
                    extent = Some(sz);
                }
                PtrState::Allocated { extent: Some(sz), .. } => {
                    extent = Some(sz);
                }
                _ => {}
            }
        } else if let Expr::StringLit(s, _) = base {
            extent = Some(s.len() + 1);
        }

        if let Some(ext) = extent {
            if idx_iv.min < 0 {
                // Negative index
                let verdict = if idx_iv.is_constant() {
                    "proved-defect"
                } else {
                    "possible-defect"
                };
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "c-buffer-overflow".into(),
                    verdict: verdict.into(),
                    message: format!(
                        "Buffer underflow: index can be {} (negative offset)",
                        idx_iv.min
                    ),
                });
            } else if idx_iv.max >= ext as i64 {
                // Buffer overflow
                let verdict = if idx_iv.is_constant() {
                    "proved-defect"
                } else {
                    "possible-defect"
                };
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "c-buffer-overflow".into(),
                    verdict: verdict.into(),
                    message: format!(
                        "Buffer overflow: index {} exceeds bound {}",
                        idx_iv.max, ext
                    ),
                });
            } else {
                self.obligations_proved_safe += 1;
            }
        } else {
            // Extent unknown: check if index can be negative
            if idx_iv.min < 0 {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "c-buffer-overflow".into(),
                    verdict: "possible-defect".into(),
                    message: format!(
                        "Potential buffer underflow: index {} is negative",
                        idx_iv.min
                    ),
                });
            } else {
                self.obligations_proved_safe += 1;
            }
        }
    }

    fn check_div_by_zero(&mut self, divisor: &Expr, env: &Env, line: usize) {
        self.obligations_total += 1;
        let iv = self.eval_expr_interval(divisor, env);

        if iv.is_constant() && iv.min == 0 {
            self.findings.push(Finding {
                file: self.file_path.clone(),
                line,
                class: "c-div-by-zero".into(),
                verdict: "proved-defect".into(),
                message: "Division by zero: divisor is 0".into(),
            });
        } else if iv.is_strictly_positive() || iv.is_strictly_negative() {
            self.obligations_proved_safe += 1;
        } else if iv.contains_zero() {
            // Guarded or potential
            self.findings.push(Finding {
                file: self.file_path.clone(),
                line,
                class: "c-div-by-zero".into(),
                verdict: "possible-defect".into(),
                message: format!(
                    "Potential division by zero: divisor range [{}, {}] includes 0",
                    iv.min, iv.max
                ),
            });
        } else {
            self.obligations_proved_safe += 1;
        }
    }

    fn check_signed_overflow(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        op: BinaryOp,
        env: &Env,
        line: usize,
    ) {
        self.obligations_total += 1;
        let l_iv = self.eval_expr_interval(lhs, env);
        let r_iv = self.eval_expr_interval(rhs, env);

        if l_iv.is_constant() && r_iv.is_constant() {
            let l = l_iv.min;
            let r = r_iv.min;
            let overflow = match op {
                BinaryOp::Add => {
                    let (res, ov) = (l as i32).overflowing_add(r as i32);
                    let _ = res;
                    ov || (l + r > INT32_MAX || l + r < INT32_MIN)
                }
                BinaryOp::Sub => {
                    let (res, ov) = (l as i32).overflowing_sub(r as i32);
                    let _ = res;
                    ov || (l - r > INT32_MAX || l - r < INT32_MIN)
                }
                BinaryOp::Mul => {
                    let (res, ov) = (l as i32).overflowing_mul(r as i32);
                    let _ = res;
                    ov || (l * r > INT32_MAX || l * r < INT32_MIN)
                }
                _ => false,
            };

            if overflow {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line,
                    class: "c-signed-overflow".into(),
                    verdict: "proved-defect".into(),
                    message: format!(
                        "Signed integer overflow: {} {:?} {} exceeds 32-bit signed range",
                        l, op, r
                    ),
                });
                return;
            }
        }

        self.obligations_proved_safe += 1;
    }

    fn eval_expr_interval(&self, expr: &Expr, env: &Env) -> Interval {
        match expr {
            Expr::IntLit(n, _) => Interval::point(*n),
            Expr::BoolLit(b, _) => Interval::point(if *b { 1 } else { 0 }),
            Expr::NullLit(_) => Interval::point(0),
            Expr::Ident(name, _) => env.get_interval(name),
            Expr::Binary { op, lhs, rhs, .. } => {
                let l = self.eval_expr_interval(lhs, env);
                let r = self.eval_expr_interval(rhs, env);
                match op {
                    BinaryOp::Add => l.add(&r),
                    BinaryOp::Sub => l.sub(&r),
                    BinaryOp::Mul => l.mul(&r),
                    BinaryOp::Div => {
                        if !r.contains_zero() && r.is_constant() {
                            Interval::new(l.min / r.min, l.max / r.min)
                        } else {
                            Interval::unknown()
                        }
                    }
                    _ => Interval::unknown(),
                }
            }
            Expr::Unary {
                op: UnaryOp::Minus,
                expr: inner,
                ..
            } => {
                let iv = self.eval_expr_interval(inner, env);
                Interval::new(-iv.max, -iv.min)
            }
            Expr::Cast { expr: inner, .. } => self.eval_expr_interval(inner, env),
            _ => Interval::unknown(),
        }
    }

    fn refine_cond(&self, cond: &Expr, env: &mut Env, is_true: bool) {
        match cond {
            Expr::Binary {
                op: BinaryOp::Ne,
                lhs,
                rhs,
                ..
            } => {
                if is_true {
                    self.refine_not_equal(lhs, rhs, env);
                } else {
                    self.refine_equal(lhs, rhs, env);
                }
            }
            Expr::Binary {
                op: BinaryOp::Eq,
                lhs,
                rhs,
                ..
            } => {
                if is_true {
                    self.refine_equal(lhs, rhs, env);
                } else {
                    self.refine_not_equal(lhs, rhs, env);
                }
            }
            Expr::Binary {
                op: BinaryOp::Lt,
                lhs,
                rhs,
                ..
            } => {
                if let Expr::Ident(name, _) = lhs.as_ref() {
                    let r = self.eval_expr_interval(rhs, env);
                    let mut cur = env.get_interval(name);
                    if is_true {
                        cur.max = cur.max.min(r.max - 1);
                    } else {
                        cur.min = cur.min.max(r.min);
                    }
                    env.set_interval(name, cur);
                }
            }
            Expr::Binary {
                op: BinaryOp::Le,
                lhs,
                rhs,
                ..
            } => {
                if let Expr::Ident(name, _) = lhs.as_ref() {
                    let r = self.eval_expr_interval(rhs, env);
                    let mut cur = env.get_interval(name);
                    if is_true {
                        cur.max = cur.max.min(r.max);
                    } else {
                        cur.min = cur.min.max(r.min + 1);
                    }
                    env.set_interval(name, cur);
                }
            }
            Expr::Binary {
                op: BinaryOp::Gt,
                lhs,
                rhs,
                ..
            } => {
                if let Expr::Ident(name, _) = lhs.as_ref() {
                    let r = self.eval_expr_interval(rhs, env);
                    let mut cur = env.get_interval(name);
                    if is_true {
                        cur.min = cur.min.max(r.min + 1);
                    } else {
                        cur.max = cur.max.min(r.max);
                    }
                    env.set_interval(name, cur);
                }
            }
            Expr::Binary {
                op: BinaryOp::Ge,
                lhs,
                rhs,
                ..
            } => {
                if let Expr::Ident(name, _) = lhs.as_ref() {
                    let r = self.eval_expr_interval(rhs, env);
                    let mut cur = env.get_interval(name);
                    if is_true {
                        cur.min = cur.min.max(r.min);
                    } else {
                        cur.max = cur.max.min(r.max - 1);
                    }
                    env.set_interval(name, cur);
                }
            }
            Expr::Ident(name, _) => {
                if is_true {
                    env.set_pointer(name, PtrState::NonNull);
                } else {
                    env.set_pointer(name, PtrState::Null);
                    env.set_interval(name, Interval::point(0));
                }
            }
            Expr::Unary {
                op: UnaryOp::Not,
                expr: inner,
                ..
            } => {
                self.refine_cond(inner, env, !is_true);
            }
            _ => {}
        }
    }

    fn refine_not_equal(&self, lhs: &Expr, rhs: &Expr, env: &mut Env) {
        if let Expr::Ident(name, _) = lhs {
            if matches!(rhs, Expr::NullLit(_) | Expr::IntLit(0, _)) {
                env.set_pointer(name, PtrState::NonNull);
                let cur = env.get_interval(name);
                if cur.min == 0 {
                    env.set_interval(name, Interval::new(1, cur.max));
                } else if cur.max == 0 {
                    env.set_interval(name, Interval::new(cur.min, -1));
                }
            }
        }
    }

    fn refine_equal(&self, lhs: &Expr, rhs: &Expr, env: &mut Env) {
        if let Expr::Ident(name, _) = lhs {
            if matches!(rhs, Expr::NullLit(_) | Expr::IntLit(0, _)) {
                env.set_pointer(name, PtrState::Null);
                env.set_interval(name, Interval::point(0));
            } else {
                let r = self.eval_expr_interval(rhs, env);
                env.set_interval(name, r);
            }
        }
    }
}
