#![allow(dead_code, unused_variables, unused_imports, private_interfaces)]

use crate::go::ast::*;
use crate::go::{Diagnostic, DiagCategory, Severity};
use std::collections::{HashMap, HashSet};

/// Result of lowering a Go AST into verification conditions.
#[derive(Debug, Clone)]
pub struct LowerResult {
    pub diagnostics: Vec<Diagnostic>,
    pub goroutine_count: usize,
    pub channel_count: usize,
    pub functions_lowered: usize,
    pub obligations_total: usize,
    pub obligations_proved_safe: usize,
}

/// Lower a Go AST and produce concurrency/memory-safety diagnostics.
pub fn lower(pkg: &Package) -> LowerResult {
    let mut analyzer = Analyzer::new();
    for func in &pkg.functions {
        analyzer.analyze_function(func);
    }
    LowerResult {
        diagnostics: analyzer.diagnostics,
        goroutine_count: analyzer.goroutine_count,
        channel_count: analyzer.channel_count,
        functions_lowered: pkg.functions.len(),
        obligations_total: analyzer.obligations_total,
        obligations_proved_safe: analyzer.obligations_proved_safe,
    }
}

#[derive(Debug, Clone)]
struct VarInfo {
    ty: Option<Type>,
    is_non_nil: bool,
    is_nil: bool,
    is_pointer_or_interface: bool,
    is_slice: bool,
    slice_len: Option<usize>,
    is_chan: bool,
    chan_cap: Option<usize>,
    is_closed: bool,
}

struct Analyzer {
    pub diagnostics: Vec<Diagnostic>,
    pub goroutine_count: usize,
    pub channel_count: usize,
    pub obligations_total: usize,
    pub obligations_proved_safe: usize,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            goroutine_count: 0,
            channel_count: 0,
            obligations_total: 0,
            obligations_proved_safe: 0,
        }
    }

    fn diag(&mut self, cat: DiagCategory, sev: Severity, msg: &str, line: usize) {
        self.diagnostics.push(Diagnostic {
            severity: sev,
            category: cat,
            message: msg.to_string(),
            line,
            col: 0,
        });
    }

    pub fn analyze_function(&mut self, func: &Function) {
        let mut vars: HashMap<String, VarInfo> = HashMap::new();

        // Register receiver
        if let Some(ref r) = func.receiver {
            if let Some(ref name) = r.name {
                let is_ptr = matches!(r.ty, Type::Pointer(_) | Type::Interface(_));
                vars.insert(name.clone(), VarInfo {
                    ty: Some(r.ty.clone()),
                    is_non_nil: false, // receiver might be nil in Go!
                    is_nil: false,
                    is_pointer_or_interface: is_ptr,
                    is_slice: matches!(r.ty, Type::Slice(_)),
                    slice_len: None,
                    is_chan: matches!(r.ty, Type::Chan(_, _)),
                    chan_cap: None,
                    is_closed: false,
                });
            }
        }

        // Register parameters
        for p in &func.params {
            if let Some(ref name) = p.name {
                let is_ptr = matches!(p.ty, Type::Pointer(_) | Type::Interface(_));
                vars.insert(name.clone(), VarInfo {
                    ty: Some(p.ty.clone()),
                    is_non_nil: false,
                    is_nil: false,
                    is_pointer_or_interface: is_ptr,
                    is_slice: matches!(p.ty, Type::Slice(_)),
                    slice_len: None,
                    is_chan: matches!(p.ty, Type::Chan(_, _)),
                    chan_cap: None,
                    is_closed: false,
                });
            }
        }

        let mut active_guards = ActiveGuards::new();
        let mut func_goroutines = 0;
        let mut sends: Vec<(String, usize)> = Vec::new();
        let mut recvs: Vec<(String, usize)> = Vec::new();
        let mut closed_chans: HashSet<String> = HashSet::new();

        self.analyze_block(
            &func.body,
            &mut vars,
            &mut active_guards,
            &mut func_goroutines,
            &mut sends,
            &mut recvs,
            &mut closed_chans,
            None,
        );

        // Check channel deadlock: unbuffered channel in single goroutine
        if func_goroutines == 0 {
            for (ch, line) in &sends {
                let is_unbuf = vars.get(ch).map_or(true, |v| v.chan_cap.unwrap_or(0) == 0);
                if is_unbuf {
                    self.diag(
                        DiagCategory::ChannelDeadlock,
                        Severity::Error,
                        &format!("channel send on unbuffered channel `{ch}` with no concurrent receiver will deadlock"),
                        *line,
                    );
                }
            }
            for (ch, line) in &recvs {
                let is_unbuf = vars.get(ch).map_or(true, |v| v.chan_cap.unwrap_or(0) == 0);
                if is_unbuf && !closed_chans.contains(ch) {
                    self.diag(
                        DiagCategory::ChannelDeadlock,
                        Severity::Error,
                        &format!("channel receive on unbuffered channel `{ch}` with no concurrent sender will deadlock"),
                        *line,
                    );
                }
            }
        } else {
            // Concurrent context: check for sends without any recv or recv without any send
            let send_names: HashSet<&str> = sends.iter().map(|(s, _)| s.as_str()).collect();
            let recv_names: HashSet<&str> = recvs.iter().map(|(r, _)| r.as_str()).collect();

            for (ch, line) in &sends {
                if !recv_names.contains(ch.as_str()) && vars.get(ch).map_or(true, |v| v.is_chan && v.chan_cap.unwrap_or(0) == 0) {
                    self.diag(
                        DiagCategory::ChannelDeadlock,
                        Severity::Warning,
                        &format!("channel `{ch}` has send without matching receive in scope, potential deadlock"),
                        *line,
                    );
                    break;
                }
            }
            for (ch, line) in &recvs {
                if !send_names.contains(ch.as_str()) && !closed_chans.contains(ch) && vars.get(ch).map_or(true, |v| v.is_chan && v.chan_cap.unwrap_or(0) == 0) {
                    self.diag(
                        DiagCategory::ChannelDeadlock,
                        Severity::Warning,
                        &format!("channel `{ch}` has receive without matching send in scope, potential deadlock"),
                        *line,
                    );
                    break;
                }
            }
        }
    }

    fn analyze_block(
        &mut self,
        block: &Block,
        vars: &mut HashMap<String, VarInfo>,
        guards: &mut ActiveGuards,
        goroutines: &mut usize,
        sends: &mut Vec<(String, usize)>,
        recvs: &mut Vec<(String, usize)>,
        closed_chans: &mut HashSet<String>,
        loop_context: Option<&LoopContext>,
    ) {
        for stmt in &block.stmts {
            self.analyze_stmt(stmt, vars, guards, goroutines, sends, recvs, closed_chans, loop_context);
        }
    }

    fn analyze_stmt(
        &mut self,
        stmt: &Stmt,
        vars: &mut HashMap<String, VarInfo>,
        guards: &mut ActiveGuards,
        goroutines: &mut usize,
        sends: &mut Vec<(String, usize)>,
        recvs: &mut Vec<(String, usize)>,
        closed_chans: &mut HashSet<String>,
        loop_context: Option<&LoopContext>,
    ) {
        match stmt {
            Stmt::Decl(d) => {
                for name in &d.names {
                    let is_ptr = d.ty.as_ref().map_or(false, |t| matches!(t, Type::Pointer(_) | Type::Interface(_)));
                    let is_slice = d.ty.as_ref().map_or(false, |t| matches!(t, Type::Slice(_)));
                    let is_chan = d.ty.as_ref().map_or(false, |t| matches!(t, Type::Chan(_, _)));
                    if is_chan { self.channel_count += 1; }

                    let (is_non_nil, is_nil) = if let Some(ref init) = d.init {
                        self.check_expr(init, vars, guards, loop_context);
                        (is_known_non_nil(init), is_known_nil(init))
                    } else {
                        // Uninitialized pointer in Go is nil!
                        (false, is_ptr)
                    };

                    vars.insert(name.clone(), VarInfo {
                        ty: d.ty.clone(),
                        is_non_nil,
                        is_nil,
                        is_pointer_or_interface: is_ptr,
                        is_slice,
                        slice_len: None,
                        is_chan,
                        chan_cap: None,
                        is_closed: false,
                    });
                }
            }
            Stmt::Assign(a) => {
                // Analyze RHS expressions
                for rhs in &a.rhs {
                    self.check_expr(rhs, vars, guards, loop_context);
                }

                // Update variable tracking
                for (i, lhs) in a.lhs.iter().enumerate() {
                    let rhs = a.rhs.get(i);
                    if let Expr::Ident(id) = lhs {
                        let is_non_nil = rhs.map_or(false, is_known_non_nil);
                        let is_nil = rhs.map_or(false, is_known_nil);
                        let is_slice = rhs.map_or(false, |e| matches!(e, Expr::Slice(_) | Expr::CompositeLit(_)));
                        let (is_chan, cap) = if let Some(Expr::ChanMake(cm)) = rhs {
                            self.channel_count += 1;
                            let c = cm.cap.as_ref().and_then(|e| eval_const_int(e)).map(|v| v as usize);
                            (true, c)
                        } else {
                            (false, None)
                        };

                        let slice_len = if let Some(Expr::CompositeLit(cl)) = rhs {
                            Some(cl.elems.len())
                        } else {
                            None
                        };

                        if let Some(existing) = vars.get_mut(&id.name) {
                            existing.is_non_nil = is_non_nil;
                            existing.is_nil = is_nil;
                            if is_chan { existing.is_chan = true; existing.chan_cap = cap; }
                            if slice_len.is_some() { existing.slice_len = slice_len; }
                        } else {
                            vars.insert(id.name.clone(), VarInfo {
                                ty: None,
                                is_non_nil,
                                is_nil,
                                is_pointer_or_interface: is_non_nil || is_nil,
                                is_slice,
                                slice_len,
                                is_chan,
                                chan_cap: cap,
                                is_closed: false,
                            });
                        }
                    } else {
                        self.check_expr(lhs, vars, guards, loop_context);
                    }
                }
            }
            Stmt::IncDec(e, _, _) => {
                self.check_expr(e, vars, guards, loop_context);
            }
            Stmt::If(i) => {
                if let Some(ref init) = i.init {
                    self.analyze_stmt(init, vars, guards, goroutines, sends, recvs, closed_chans, loop_context);
                }
                self.check_expr(&i.cond, vars, guards, loop_context);

                // Extract guards from condition
                let branch_guards = extract_condition_guards(&i.cond);

                // Then branch with guards active
                let mut then_guards = guards.clone();
                for g in &branch_guards.positive {
                    then_guards.add(g.clone());
                }
                self.analyze_block(&i.then, vars, &mut then_guards, goroutines, sends, recvs, closed_chans, loop_context);

                // Check if then-branch unconditionally exits (return, panic)
                let then_exits = block_always_terminates(&i.then);
                if then_exits {
                    // After the if, the inverted guards are permanently true!
                    for g in &branch_guards.negative {
                        guards.add(g.clone());
                    }
                }

                // Else branch
                if let Some(ref els) = i.els {
                    let mut else_guards = guards.clone();
                    for g in &branch_guards.negative {
                        else_guards.add(g.clone());
                    }
                    self.analyze_block(els, vars, &mut else_guards, goroutines, sends, recvs, closed_chans, loop_context);
                }
            }
            Stmt::For(f) => {
                if let Some(ref init) = f.init {
                    self.analyze_stmt(init, vars, guards, goroutines, sends, recvs, closed_chans, loop_context);
                }
                if let Some(ref cond) = f.cond {
                    self.check_expr(cond, vars, guards, loop_context);
                }

                // Detect for i := 0; i < len(s); i++
                let mut loop_ctx = LoopContext::default();
                if let Some(ref cond) = f.cond {
                    if let Expr::Binary(b) = cond {
                        if matches!(b.op, BinOp::Lt | BinOp::Le) {
                            if let Expr::Ident(ref idx) = *b.lhs {
                                loop_ctx.loop_vars.insert(idx.name.clone());
                                // If rhs is len(s)
                                if let Expr::Call(c) = &*b.rhs {
                                    if let Expr::Ident(fn_name) = &*c.func {
                                        if fn_name.name == "len" || fn_name.name == "builtin.len" {
                                            if let Some(Expr::Ident(s_id)) = c.args.first() {
                                                loop_ctx.bounded_indices.insert(idx.name.clone(), s_id.name.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let mut loop_guards = guards.clone();
                for (idx, slice) in &loop_ctx.bounded_indices {
                    loop_guards.add(Guard::IndexBounded { index: idx.clone(), slice: slice.clone() });
                }

                self.analyze_block(&f.body, vars, &mut loop_guards, goroutines, sends, recvs, closed_chans, Some(&loop_ctx));

                if let Some(ref post) = f.post {
                    self.analyze_stmt(post, vars, guards, goroutines, sends, recvs, closed_chans, loop_context);
                }
            }
            Stmt::Range(r) => {
                self.check_expr(&r.iter, vars, guards, loop_context);
                let mut loop_ctx = LoopContext::default();
                let iter_name = match &r.iter {
                    Expr::Ident(id) => Some(id.name.clone()),
                    _ => None,
                };

                if let Some(ref k) = r.key {
                    loop_ctx.loop_vars.insert(k.clone());
                    if let Some(ref s) = iter_name {
                        loop_ctx.bounded_indices.insert(k.clone(), s.clone());
                    }
                }
                if let Some(ref v) = r.val {
                    loop_ctx.loop_vars.insert(v.clone());
                }

                let mut loop_guards = guards.clone();
                for (idx, slice) in &loop_ctx.bounded_indices {
                    loop_guards.add(Guard::IndexBounded { index: idx.clone(), slice: slice.clone() });
                }

                self.analyze_block(&r.body, vars, &mut loop_guards, goroutines, sends, recvs, closed_chans, Some(&loop_ctx));
            }
            Stmt::Switch(s) => {
                if let Some(ref init) = s.init {
                    self.analyze_stmt(init, vars, guards, goroutines, sends, recvs, closed_chans, loop_context);
                }
                if let Some(ref tag) = s.tag {
                    self.check_expr(tag, vars, guards, loop_context);
                }
                for case in &s.cases {
                    for expr in &case.exprs {
                        self.check_expr(expr, vars, guards, loop_context);
                    }
                    let mut case_guards = guards.clone();
                    self.analyze_block(&case.body, vars, &mut case_guards, goroutines, sends, recvs, closed_chans, loop_context);
                }
            }
            Stmt::Select(s) => {
                self.obligations_total += 1;
                let has_default = s.default.is_some();
                if has_default {
                    // Select with default never blocks
                    self.obligations_proved_safe += 1;
                }

                let mut case_sends = Vec::new();
                let mut case_recvs = Vec::new();
                for case in &s.cases {
                    if let Some(ref comm) = case.comm {
                        self.analyze_stmt(comm, vars, guards, goroutines, &mut case_sends, &mut case_recvs, closed_chans, loop_context);
                    }
                    let mut case_guards = guards.clone();
                    self.analyze_block(&case.body, vars, &mut case_guards, goroutines, sends, recvs, closed_chans, loop_context);
                }
                if !has_default {
                    sends.extend(case_sends);
                    recvs.extend(case_recvs);
                }
                if let Some(ref def) = s.default {
                    let mut def_guards = guards.clone();
                    self.analyze_block(def, vars, &mut def_guards, goroutines, sends, recvs, closed_chans, loop_context);
                }
            }
            Stmt::Return(r) => {
                for v in &r.values {
                    self.check_expr(v, vars, guards, loop_context);
                }
            }
            Stmt::Go(g) => {
                self.goroutine_count += 1;
                *goroutines += 1;
                self.obligations_total += 1;

                // Check for data race on captured variables
                let mut captured_vars = HashSet::new();
                let mut target = &g.call;
                while let Expr::Paren(inner) = target {
                    target = inner;
                }
                if let Expr::Call(c) = target {
                    let mut callee = &*c.func;
                    while let Expr::Paren(inner) = callee {
                        callee = inner;
                    }
                    if let Expr::FuncLit(fl) = callee {
                        for s in &fl.body.stmts {
                            collect_idents_in_stmt(s, &mut captured_vars);
                        }
                        for p in &fl.params {
                            if let Some(ref pname) = p.name {
                                captured_vars.remove(pname);
                            }
                        }
                    } else {
                        collect_idents_in_expr(callee, &mut captured_vars);
                    }
                } else {
                    collect_idents_in_expr(&g.call, &mut captured_vars);
                }

                // If inside a loop, check if loop variable is captured directly
                if let Some(ctx) = loop_context {
                    for lv in &ctx.loop_vars {
                        if captured_vars.contains(lv) {
                            self.diag(
                                DiagCategory::DataRace,
                                Severity::Error,
                                &format!("goroutine captures loop variable `{lv}` by reference, causing a data race"),
                                g.line,
                            );
                            break;
                        }
                    }
                }

                // Check for unsynchronized shared variable access
                let mut has_race = false;
                for vname in &captured_vars {
                    if let Some(vinfo) = vars.get(vname) {
                        if !vinfo.is_chan && !vinfo.is_non_nil && vinfo.ty.is_none() {
                            self.diag(
                                DiagCategory::DataRace,
                                Severity::Warning,
                                &format!("variable `{vname}` accessed concurrently by goroutine without synchronization"),
                                g.line,
                            );
                            has_race = true;
                            break;
                        }
                    }
                }
                if !has_race {
                    self.obligations_proved_safe += 1;
                }

                self.check_expr(&g.call, vars, guards, loop_context);
            }
            Stmt::Defer(d) => {
                self.check_expr(&d.call, vars, guards, loop_context);
            }
            Stmt::Send(s) => {
                self.obligations_total += 1;
                self.check_expr(&s.ch, vars, guards, loop_context);
                self.check_expr(&s.val, vars, guards, loop_context);

                if let Expr::Ident(id) = &*s.ch {
                    sends.push((id.name.clone(), s.line));
                    if closed_chans.contains(&id.name) {
                        self.diag(
                            DiagCategory::ChannelDeadlock,
                            Severity::Error,
                            &format!("send on closed channel `{}`", id.name),
                            s.line,
                        );
                    } else if vars.get(&id.name).map_or(false, |v| v.chan_cap.unwrap_or(0) > 0) {
                        // Buffered channel send
                        self.obligations_proved_safe += 1;
                    }
                }
            }
            Stmt::Recv(r) => {
                self.obligations_total += 1;
                self.check_expr(&r.ch, vars, guards, loop_context);
                if let Expr::Ident(id) = &*r.ch {
                    recvs.push((id.name.clone(), r.line));
                    if vars.get(&id.name).map_or(false, |v| v.chan_cap.unwrap_or(0) > 0) {
                        self.obligations_proved_safe += 1;
                    }
                }
            }
            Stmt::Close(c) => {
                self.check_expr(&c.ch, vars, guards, loop_context);
                if let Expr::Ident(id) = &*c.ch {
                    closed_chans.insert(id.name.clone());
                    if let Some(v) = vars.get_mut(&id.name) {
                        v.is_closed = true;
                    }
                }
            }
            Stmt::Block(b) => {
                self.analyze_block(b, vars, guards, goroutines, sends, recvs, closed_chans, loop_context);
            }
            Stmt::Expr(e) => {
                self.check_expr(&e.expr, vars, guards, loop_context);
            }
            _ => {}
        }
    }

    fn check_expr(
        &mut self,
        expr: &Expr,
        vars: &HashMap<String, VarInfo>,
        guards: &ActiveGuards,
        loop_ctx: Option<&LoopContext>,
    ) {
        match expr {
            Expr::Binary(b) => {
                // Div-by-zero check on Div (/) and Mod (%)
                if matches!(b.op, BinOp::Div | BinOp::Mod) {
                    self.obligations_total += 1;
                    let op_name = if b.op == BinOp::Div { "division" } else { "modulo" };

                    if is_constant_zero(&b.rhs) {
                        self.diag(
                            DiagCategory::DivByZero,
                            Severity::Error,
                            &format!("{op_name} by constant zero"),
                            b.line,
                        );
                    } else if is_constant_non_zero(&b.rhs) {
                        self.obligations_proved_safe += 1;
                    } else if let Expr::Ident(id) = &*b.rhs {
                        if guards.is_non_zero(&id.name) {
                            self.obligations_proved_safe += 1;
                        } else {
                            self.diag(
                                DiagCategory::DivByZero,
                                Severity::Warning,
                                &format!("{op_name} by `{}` may divide by zero without non-zero guard", id.name),
                                b.line,
                            );
                        }
                    } else {
                        // General RHS expression
                        self.obligations_proved_safe += 1;
                    }
                }

                self.check_expr(&b.lhs, vars, guards, loop_ctx);
                self.check_expr(&b.rhs, vars, guards, loop_ctx);
            }
            Expr::Unary(u) => {
                // Pointer dereference check (*p)
                if u.op == UnOp::Deref {
                    self.obligations_total += 1;
                    if let Expr::Ident(id) = &*u.operand {
                        if guards.is_non_nil(&id.name) || vars.get(&id.name).map_or(false, |v| v.is_non_nil) {
                            self.obligations_proved_safe += 1;
                        } else if vars.get(&id.name).map_or(false, |v| v.is_nil) {
                            self.diag(
                                DiagCategory::NilDeref,
                                Severity::Error,
                                &format!("nil pointer dereference on `{}` (assigned or defaulted to nil)", id.name),
                                u.line,
                            );
                        } else {
                            self.diag(
                                DiagCategory::NilDeref,
                                Severity::Warning,
                                &format!("potential nil pointer dereference on `{}` without prior nil check", id.name),
                                u.line,
                            );
                        }
                    } else {
                        self.obligations_proved_safe += 1;
                    }
                }
                self.check_expr(&u.operand, vars, guards, loop_ctx);
            }
            Expr::Field(f) => {
                // Selector dereference: p.field when p is a pointer/interface
                if let Expr::Ident(id) = &*f.base {
                    let is_ptr = vars.get(&id.name).map_or(false, |v| v.is_pointer_or_interface);
                    if is_ptr {
                        self.obligations_total += 1;
                        if guards.is_non_nil(&id.name) || vars.get(&id.name).map_or(false, |v| v.is_non_nil) {
                            self.obligations_proved_safe += 1;
                        } else if vars.get(&id.name).map_or(false, |v| v.is_nil) {
                            self.diag(
                                DiagCategory::NilDeref,
                                Severity::Error,
                                &format!("nil pointer dereference accessing `{}.{}`", id.name, f.name),
                                f.line,
                            );
                        } else {
                            self.diag(
                                DiagCategory::NilDeref,
                                Severity::Warning,
                                &format!("potential nil pointer dereference on `{}` accessing field `{}`", id.name, f.name),
                                f.line,
                            );
                        }
                    }
                }
                self.check_expr(&f.base, vars, guards, loop_ctx);
            }
            Expr::Index(idx) => {
                self.obligations_total += 1;
                let slice_name = match &*idx.base {
                    Expr::Ident(id) => Some(id.name.clone()),
                    _ => None,
                };

                // Check negative index
                if let Some(val) = eval_const_int(&idx.index) {
                    if val < 0 {
                        self.diag(
                            DiagCategory::SlicePanic,
                            Severity::Error,
                            &format!("negative slice index {val} will panic"),
                            idx.line,
                        );
                    } else if let Some(ref s) = slice_name {
                        if let Some(known_len) = vars.get(s).and_then(|v| v.slice_len) {
                            if (val as usize) >= known_len {
                                self.diag(
                                    DiagCategory::SlicePanic,
                                    Severity::Error,
                                    &format!("slice index out of bounds: index {val} >= known length {known_len}"),
                                    idx.line,
                                );
                            } else {
                                self.obligations_proved_safe += 1;
                            }
                        } else {
                            self.obligations_proved_safe += 1;
                        }
                    } else {
                        self.obligations_proved_safe += 1;
                    }
                } else if let Expr::Ident(i_id) = &*idx.index {
                    let is_guarded = if let Some(ref s) = slice_name {
                        guards.is_bounded(&i_id.name, s)
                            || loop_ctx.map_or(false, |c| c.bounded_indices.get(&i_id.name) == Some(s))
                    } else {
                        false
                    };

                    if is_guarded {
                        self.obligations_proved_safe += 1;
                    } else {
                        self.diag(
                            DiagCategory::SlicePanic,
                            Severity::Warning,
                            &format!("slice index `{}[{}]` without bounds check against len({})", slice_name.as_deref().unwrap_or("s"), i_id.name, slice_name.as_deref().unwrap_or("s")),
                            idx.line,
                        );
                    }
                } else {
                    self.obligations_proved_safe += 1;
                }

                self.check_expr(&idx.base, vars, guards, loop_ctx);
                self.check_expr(&idx.index, vars, guards, loop_ctx);
            }
            Expr::Slice(s) => {
                self.check_expr(&s.base, vars, guards, loop_ctx);
                if let Some(ref l) = s.low { self.check_expr(l, vars, guards, loop_ctx); }
                if let Some(ref h) = s.high { self.check_expr(h, vars, guards, loop_ctx); }
                if let Some(ref m) = s.max { self.check_expr(m, vars, guards, loop_ctx); }
            }
            Expr::Call(c) => {
                self.check_expr(&c.func, vars, guards, loop_ctx);
                for a in &c.args { self.check_expr(a, vars, guards, loop_ctx); }
            }
            Expr::TypeAssert(t) => {
                self.check_expr(&t.base, vars, guards, loop_ctx);
            }
            Expr::CompositeLit(cl) => {
                for e in &cl.elems { self.check_expr(e, vars, guards, loop_ctx); }
            }
            Expr::KeyValue(k, v, _) => {
                self.check_expr(k, vars, guards, loop_ctx);
                self.check_expr(v, vars, guards, loop_ctx);
            }
            Expr::Paren(p) => {
                self.check_expr(p, vars, guards, loop_ctx);
            }
            _ => {}
        }
    }
}

// ─── Guard & Context Helpers ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Guard {
    NonNil(String),
    NonZero(String),
    IndexBounded { index: String, slice: String },
}

#[derive(Debug, Clone, Default)]
struct ActiveGuards {
    guards: HashSet<Guard>,
}

impl ActiveGuards {
    fn new() -> Self { Self::default() }

    fn add(&mut self, g: Guard) {
        self.guards.insert(g);
    }

    fn is_non_nil(&self, name: &str) -> bool {
        self.guards.contains(&Guard::NonNil(name.to_string()))
    }

    fn is_non_zero(&self, name: &str) -> bool {
        self.guards.contains(&Guard::NonZero(name.to_string()))
    }

    fn is_bounded(&self, index: &str, slice: &str) -> bool {
        self.guards.contains(&Guard::IndexBounded {
            index: index.to_string(),
            slice: slice.to_string(),
        })
    }
}

#[derive(Debug, Clone, Default)]
struct LoopContext {
    loop_vars: HashSet<String>,
    bounded_indices: HashMap<String, String>, // index_var -> slice_var
}

struct ConditionGuards {
    positive: Vec<Guard>,
    negative: Vec<Guard>,
}

fn extract_condition_guards(expr: &Expr) -> ConditionGuards {
    let mut positive = Vec::new();
    let mut negative = Vec::new();

    match expr {
        Expr::Binary(b) => match b.op {
            BinOp::Ne => {
                // x != nil
                if is_nil_expr(&b.rhs) {
                    if let Expr::Ident(id) = &*b.lhs {
                        positive.push(Guard::NonNil(id.name.clone()));
                        negative.push(Guard::NonNil(id.name.clone())); // inside else, x == nil
                    }
                } else if is_nil_expr(&b.lhs) {
                    if let Expr::Ident(id) = &*b.rhs {
                        positive.push(Guard::NonNil(id.name.clone()));
                    }
                }
                // x != 0
                if is_constant_zero(&b.rhs) {
                    if let Expr::Ident(id) = &*b.lhs {
                        positive.push(Guard::NonZero(id.name.clone()));
                    }
                } else if is_constant_zero(&b.lhs) {
                    if let Expr::Ident(id) = &*b.rhs {
                        positive.push(Guard::NonZero(id.name.clone()));
                    }
                }
            }
            BinOp::Eq => {
                // x == nil
                if is_nil_expr(&b.rhs) {
                    if let Expr::Ident(id) = &*b.lhs {
                        negative.push(Guard::NonNil(id.name.clone())); // inside else, x != nil!
                    }
                } else if is_nil_expr(&b.lhs) {
                    if let Expr::Ident(id) = &*b.rhs {
                        negative.push(Guard::NonNil(id.name.clone()));
                    }
                }
                // x == 0
                if is_constant_zero(&b.rhs) {
                    if let Expr::Ident(id) = &*b.lhs {
                        negative.push(Guard::NonZero(id.name.clone()));
                    }
                }
            }
            BinOp::Gt => {
                // x > 0
                if is_constant_zero(&b.rhs) {
                    if let Expr::Ident(id) = &*b.lhs {
                        positive.push(Guard::NonZero(id.name.clone()));
                    }
                }
            }
            BinOp::Lt => {
                // i < len(s)
                if let Expr::Ident(i_id) = &*b.lhs {
                    if let Expr::Call(c) = &*b.rhs {
                        if let Expr::Ident(f_id) = &*c.func {
                            if f_id.name == "len" || f_id.name == "builtin.len" {
                                if let Some(Expr::Ident(s_id)) = c.args.first() {
                                    positive.push(Guard::IndexBounded {
                                        index: i_id.name.clone(),
                                        slice: s_id.name.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            BinOp::And => {
                let left = extract_condition_guards(&b.lhs);
                let right = extract_condition_guards(&b.rhs);
                positive.extend(left.positive);
                positive.extend(right.positive);
            }
            _ => {}
        },
        Expr::Paren(p) => return extract_condition_guards(p),
        _ => {}
    }

    ConditionGuards { positive, negative }
}

fn block_always_terminates(block: &Block) -> bool {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Return(_) => return true,
            Stmt::Branch(BranchKind::Break | BranchKind::Continue | BranchKind::Goto, _, _) => return true,
            Stmt::Expr(ExprStmt { expr: Expr::Call(c), .. }) => {
                if let Expr::Ident(id) = &*c.func {
                    if id.name == "panic" || id.name == "builtin.panic" || id.name == "log.Fatal" || id.name == "os.Exit" {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

fn is_nil_expr(e: &Expr) -> bool {
    matches!(e, Expr::Literal(Literal::Nil(_)))
}

fn is_known_non_nil(e: &Expr) -> bool {
    match e {
        Expr::Unary(u) => u.op == UnOp::Addr, // &x is non-nil
        Expr::ChanMake(_) => true,            // make(...) returns non-nil
        Expr::CompositeLit(_) => true,        // T{...} is value / non-nil
        Expr::Call(c) => {
            if let Expr::Ident(id) = &*c.func {
                id.name == "new" || id.name == "builtin.new" || id.name == "make" || id.name == "builtin.make"
            } else {
                false
            }
        }
        _ => false,
    }
}

fn is_known_nil(e: &Expr) -> bool {
    is_nil_expr(e)
}

fn eval_const_int(e: &Expr) -> Option<i64> {
    match e {
        Expr::Literal(Literal::Int(v, _)) => Some(*v),
        Expr::Unary(u) if u.op == UnOp::Neg => eval_const_int(&u.operand).map(|v| -v),
        Expr::Paren(p) => eval_const_int(p),
        _ => None,
    }
}

fn is_constant_zero(e: &Expr) -> bool {
    eval_const_int(e) == Some(0)
}

fn is_constant_non_zero(e: &Expr) -> bool {
    match eval_const_int(e) {
        Some(v) => v != 0,
        None => match e {
            Expr::Literal(Literal::Float(f, _)) => *f != 0.0,
            _ => false,
        },
    }
}

fn collect_idents_in_expr(e: &Expr, idents: &mut HashSet<String>) {
    match e {
        Expr::Ident(id) => { idents.insert(id.name.clone()); }
        Expr::Binary(b) => {
            collect_idents_in_expr(&b.lhs, idents);
            collect_idents_in_expr(&b.rhs, idents);
        }
        Expr::Unary(u) => collect_idents_in_expr(&u.operand, idents),
        Expr::Call(c) => {
            collect_idents_in_expr(&c.func, idents);
            for a in &c.args { collect_idents_in_expr(a, idents); }
        }
        Expr::Index(i) => {
            collect_idents_in_expr(&i.base, idents);
            collect_idents_in_expr(&i.index, idents);
        }
        Expr::Slice(s) => {
            collect_idents_in_expr(&s.base, idents);
            if let Some(ref l) = s.low { collect_idents_in_expr(l, idents); }
            if let Some(ref h) = s.high { collect_idents_in_expr(h, idents); }
        }
        Expr::Field(f) => collect_idents_in_expr(&f.base, idents),
        Expr::CompositeLit(cl) => {
            for elem in &cl.elems { collect_idents_in_expr(elem, idents); }
        }
        Expr::FuncLit(fl) => {
            // inside func literal, collect variables used in body
            for stmt in &fl.body.stmts {
                collect_idents_in_stmt(stmt, idents);
            }
        }
        Expr::Paren(p) => collect_idents_in_expr(p, idents),
        _ => {}
    }
}

fn collect_idents_in_stmt(stmt: &Stmt, idents: &mut HashSet<String>) {
    match stmt {
        Stmt::Assign(a) => {
            for l in &a.lhs { collect_idents_in_expr(l, idents); }
            for r in &a.rhs { collect_idents_in_expr(r, idents); }
        }
        Stmt::Expr(e) => collect_idents_in_expr(&e.expr, idents),
        Stmt::If(i) => {
            collect_idents_in_expr(&i.cond, idents);
            for s in &i.then.stmts { collect_idents_in_stmt(s, idents); }
        }
        Stmt::For(f) => {
            for s in &f.body.stmts { collect_idents_in_stmt(s, idents); }
        }
        Stmt::Return(r) => {
            for v in &r.values { collect_idents_in_expr(v, idents); }
        }
        Stmt::Send(s) => {
            collect_idents_in_expr(&s.ch, idents);
            collect_idents_in_expr(&s.val, idents);
        }
        _ => {}
    }
}
