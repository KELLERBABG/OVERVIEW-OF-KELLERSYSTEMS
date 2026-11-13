use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_int_or_str<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct IntOrStr;
    impl<'de> serde::de::Visitor<'de> for IntOrStr {
        type Value = String;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or string")
        }
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
            Ok(v.to_string())
        }
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
            Ok(v.to_string())
        }
        fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E> {
            Ok(v.to_string())
        }
        fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E> {
            Ok(v.to_string())
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
            Ok(v.to_string())
        }
    }
    deserializer.deserialize_any(IntOrStr)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PyExpr {
    #[serde(rename = "int")]
    Int {
        #[serde(deserialize_with = "deserialize_int_or_str")]
        val: String,
    },
    #[serde(rename = "float")]
    Float { val: f64 },
    #[serde(rename = "bool")]
    Bool { val: bool },
    #[serde(rename = "str")]
    Str { val: String },
    #[serde(rename = "none")]
    NoneVal,
    #[serde(rename = "var")]
    Var { name: String },
    #[serde(rename = "unary")]
    Unary { op: String, arg: Box<PyExpr> },
    #[serde(rename = "binop")]
    BinOp {
        op: String,
        left: Box<PyExpr>,
        right: Box<PyExpr>,
    },
    #[serde(rename = "compare")]
    Compare {
        op: String,
        left: Box<PyExpr>,
        right: Box<PyExpr>,
    },
    #[serde(rename = "and")]
    And { items: Vec<PyExpr> },
    #[serde(rename = "or")]
    Or { items: Vec<PyExpr> },
    #[serde(rename = "call")]
    Call {
        #[serde(rename = "fn", default)]
        fn_: Option<String>,
        #[serde(default)]
        args: Vec<PyExpr>,
    },
    #[serde(rename = "call_opaque")]
    CallOpaque {
        #[serde(rename = "fn", default)]
        fn_: Option<String>,
    },
    #[serde(rename = "attr")]
    Attr { value: Box<PyExpr>, attr: String },
    #[serde(rename = "subscript")]
    Subscript {
        value: Box<PyExpr>,
        slice: Box<PyExpr>,
    },
    #[serde(rename = "list")]
    List {
        #[serde(default)]
        elts: Vec<PyExpr>,
    },
    #[serde(rename = "tuple")]
    Tuple {
        #[serde(default)]
        elts: Vec<PyExpr>,
    },
    #[serde(other)]
    Unsupported,
}

impl PyExpr {
    #[allow(dead_code)]
    pub fn collect_vars(&self, vars: &mut HashSet<String>) {
        match self {
            PyExpr::Var { name } => {
                vars.insert(name.clone());
            }
            PyExpr::Unary { arg, .. } => arg.collect_vars(vars),
            PyExpr::BinOp { left, right, .. } | PyExpr::Compare { left, right, .. } => {
                left.collect_vars(vars);
                right.collect_vars(vars);
            }
            PyExpr::And { items } | PyExpr::Or { items } => {
                for item in items {
                    item.collect_vars(vars);
                }
            }
            PyExpr::Call { args, fn_, .. } => {
                let name = fn_.as_deref().unwrap_or("");
                if name == "len" && !args.is_empty() {
                    let mut seq_vars = HashSet::new();
                    args[0].collect_vars(&mut seq_vars);
                    for sv in seq_vars {
                        vars.insert(format!("len_{sv}"));
                    }
                } else {
                    for arg in args {
                        arg.collect_vars(vars);
                    }
                }
            }
            PyExpr::Attr { value, attr } => {
                let mut base_vars = HashSet::new();
                value.collect_vars(&mut base_vars);
                for bv in base_vars {
                    vars.insert(format!("{bv}_{attr}"));
                }
            }
            PyExpr::Subscript { value, slice } => {
                value.collect_vars(vars);
                slice.collect_vars(vars);
            }
            PyExpr::List { elts } | PyExpr::Tuple { elts } => {
                for e in elts {
                    e.collect_vars(vars);
                }
            }
            _ => {}
        }
    }

    pub fn substitute(&self, mapping: &HashMap<String, PyExpr>) -> PyExpr {
        match self {
            PyExpr::Var { name } => {
                if let Some(replacement) = mapping.get(name) {
                    replacement.clone()
                } else {
                    self.clone()
                }
            }
            PyExpr::Unary { op, arg } => PyExpr::Unary {
                op: op.clone(),
                arg: Box::new(arg.substitute(mapping)),
            },
            PyExpr::BinOp { op, left, right } => PyExpr::BinOp {
                op: op.clone(),
                left: Box::new(left.substitute(mapping)),
                right: Box::new(right.substitute(mapping)),
            },
            PyExpr::Compare { op, left, right } => PyExpr::Compare {
                op: op.clone(),
                left: Box::new(left.substitute(mapping)),
                right: Box::new(right.substitute(mapping)),
            },
            PyExpr::And { items } => PyExpr::And {
                items: items.iter().map(|it| it.substitute(mapping)).collect(),
            },
            PyExpr::Or { items } => PyExpr::Or {
                items: items.iter().map(|it| it.substitute(mapping)).collect(),
            },
            PyExpr::Call { fn_, args } => PyExpr::Call {
                fn_: fn_.clone(),
                args: args.iter().map(|a| a.substitute(mapping)).collect(),
            },
            PyExpr::CallOpaque { fn_ } => PyExpr::CallOpaque { fn_: fn_.clone() },
            PyExpr::Attr { value, attr } => PyExpr::Attr {
                value: Box::new(value.substitute(mapping)),
                attr: attr.clone(),
            },
            PyExpr::Subscript { value, slice } => PyExpr::Subscript {
                value: Box::new(value.substitute(mapping)),
                slice: Box::new(slice.substitute(mapping)),
            },
            PyExpr::List { elts } => PyExpr::List {
                elts: elts.iter().map(|e| e.substitute(mapping)).collect(),
            },
            PyExpr::Tuple { elts } => PyExpr::Tuple {
                elts: elts.iter().map(|e| e.substitute(mapping)).collect(),
            },
            other => other.clone(),
        }
    }

    pub fn to_smt2(&self) -> String {
        match self {
            PyExpr::Int { val } => {
                if val.starts_with('-') {
                    format!("(- {})", &val[1..])
                } else {
                    val.clone()
                }
            }
            PyExpr::Float { val } => {
                if *val < 0.0 {
                    format!("(- {})", val.abs())
                } else {
                    val.to_string()
                }
            }
            PyExpr::Bool { val } => {
                if *val {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            PyExpr::Var { name } => sanitize_ident(name),
            PyExpr::Unary { op, arg } => {
                let inner = arg.to_smt2();
                match op.as_str() {
                    "not" => format!("(not {inner})"),
                    "-" => format!("(- {inner})"),
                    "+" => inner,
                    _ => inner,
                }
            }
            PyExpr::BinOp { op, left, right } => {
                let l = left.to_smt2();
                let r = right.to_smt2();
                let smt_op = match op.as_str() {
                    "+" => "+",
                    "-" => "-",
                    "*" => "*",
                    "/" | "div" | "//" => "div",
                    "mod" | "%" => "mod",
                    _ => "+",
                };
                format!("({smt_op} {l} {r})")
            }
            PyExpr::Compare { op, left, right } => {
                let l = left.to_smt2();
                let r = right.to_smt2();
                let smt_op = match op.as_str() {
                    "==" | "is" => "=",
                    "!=" | "is not" => "distinct",
                    "<" => "<",
                    "<=" => "<=",
                    ">" => ">",
                    ">=" => ">=",
                    _ => "=",
                };
                format!("({smt_op} {l} {r})")
            }
            PyExpr::And { items } => {
                if items.is_empty() {
                    "true".to_string()
                } else if items.len() == 1 {
                    items[0].to_smt2()
                } else {
                    let parts: Vec<String> = items.iter().map(|it| it.to_smt2()).collect();
                    format!("(and {})", parts.join(" "))
                }
            }
            PyExpr::Or { items } => {
                if items.is_empty() {
                    "false".to_string()
                } else if items.len() == 1 {
                    items[0].to_smt2()
                } else {
                    let parts: Vec<String> = items.iter().map(|it| it.to_smt2()).collect();
                    format!("(or {})", parts.join(" "))
                }
            }
            PyExpr::Call { fn_, args } => {
                let name = fn_.as_deref().unwrap_or("");
                match name {
                    "len" if !args.is_empty() => {
                        let inner = args[0].to_smt2();
                        let clean = inner.replace('(', "").replace(')', "").replace(' ', "_");
                        format!("len_{clean}")
                    }
                    "abs" if !args.is_empty() => {
                        let inner = args[0].to_smt2();
                        format!("(ite (>= {inner} 0) {inner} (- {inner}))")
                    }
                    "int" if !args.is_empty() => args[0].to_smt2(),
                    "min" if args.len() >= 2 => {
                        let a = args[0].to_smt2();
                        let b = args[1].to_smt2();
                        format!("(ite (<= {a} {b}) {a} {b})")
                    }
                    "max" if args.len() >= 2 => {
                        let a = args[0].to_smt2();
                        let b = args[1].to_smt2();
                        format!("(ite (>= {a} {b}) {a} {b})")
                    }
                    _ => "0".to_string(),
                }
            }
            PyExpr::Attr { value, attr } => {
                let inner = sanitize_ident(&value.to_smt2());
                format!("{inner}_{attr}")
            }
            PyExpr::Subscript { value, slice } => {
                let v = sanitize_ident(&value.to_smt2());
                let clean_s = sanitize_ident(&slice.to_smt2());
                format!("{v}_{clean_s}")
            }
            PyExpr::NoneVal => "0".to_string(),
            _ => "0".to_string(),
        }
    }
}

pub fn sanitize_ident(name: &str) -> String {
    let s = name.replace('.', "_").replace('-', "_");
    if s.is_empty() {
        "x".to_string()
    } else if s.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        if s.chars().all(|c| c.is_ascii_digit()) {
            s
        } else {
            format!("v_{s}")
        }
    } else {
        s
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopBoundInfo {
    #[serde(default)]
    pub module: String,
    #[serde(default)]
    pub function: String,
    #[serde(default)]
    pub line: u32,
    #[serde(default)]
    pub variable: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub lower: Option<PyExpr>,
    #[serde(default)]
    pub upper: Option<PyExpr>,
    #[serde(default)]
    pub step: Option<PyExpr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RawObligation {
    pub id: String,
    pub file: String,
    pub module: String,
    pub function: String,
    pub line: u32,
    pub col: u32,
    pub kind: String,
    pub guards: Vec<PyExpr>,
    pub induction_bounds: Vec<PyExpr>,
    pub loop_bounds: Vec<LoopBoundInfo>,
    pub handled: bool,
    pub handler: Option<String>,
    pub target: PyExpr,
    pub params: HashMap<String, String>,
    pub param_order: Vec<String>,
    pub source_line: String,
}

impl<'de> Deserialize<'de> for RawObligation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawHelper {
            id: String,
            file: String,
            module: String,
            function: String,
            line: u32,
            col: u32,
            kind: String,
            #[serde(default)]
            guards: Vec<PyExpr>,
            #[serde(default)]
            induction_bounds: Vec<PyExpr>,
            #[serde(default)]
            loop_bounds: Vec<LoopBoundInfo>,
            #[serde(default)]
            handled: bool,
            #[serde(default)]
            handler: Option<String>,
            target: PyExpr,
            #[serde(default)]
            params: serde_json::Value,
            #[serde(default)]
            param_order: Vec<String>,
            #[serde(default)]
            source_line: String,
        }

        let helper = RawHelper::deserialize(deserializer)?;
        let mut params_map = HashMap::new();
        let mut order = helper.param_order.clone();

        if let serde_json::Value::Object(map) = helper.params {
            for (k, v) in map {
                let v_str = match v {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                if helper.param_order.is_empty() {
                    order.push(k.clone());
                }
                params_map.insert(k, v_str);
            }
        }

        Ok(RawObligation {
            id: helper.id,
            file: helper.file,
            module: helper.module,
            function: helper.function,
            line: helper.line,
            col: helper.col,
            kind: helper.kind,
            guards: helper.guards,
            induction_bounds: helper.induction_bounds,
            loop_bounds: helper.loop_bounds,
            handled: helper.handled,
            handler: helper.handler,
            target: helper.target,
            params: params_map,
            param_order: order,
            source_line: helper.source_line,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSiteInfo {
    pub caller_module: String,
    pub caller_function: String,
    pub callee: String,
    pub line: u32,
    #[serde(default)]
    pub col: u32,
    #[serde(default)]
    pub args: Vec<PyExpr>,
    #[serde(default)]
    pub guards: Vec<PyExpr>,
    #[serde(default)]
    pub kwargs: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonScanResult {
    pub project_root: String,
    pub files_scanned: usize,
    pub total_obligations: usize,
    pub obligations: Vec<RawObligation>,
    #[serde(default)]
    pub call_sites: Vec<CallSiteInfo>,
    #[serde(default)]
    pub loop_bounds: Vec<LoopBoundInfo>,
    #[serde(default)]
    pub exception_handlers: serde_json::Value,
    #[serde(default)]
    pub call_graph: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverVerdict {
    Pass,
    Fail { model: HashMap<String, String> },
    Warn { reason: String },
}

impl SolverVerdict {
    #[allow(dead_code)]
    pub fn model(&self) -> Option<&HashMap<String, String>> {
        match self {
            SolverVerdict::Fail { model } => Some(model),
            _ => None,
        }
    }
}

pub struct PythonObligation {
    #[allow(dead_code)]
    pub raw: RawObligation,
    pub smt_content: String,
}

pub fn extract_smt_identifiers(text: &str) -> HashSet<String> {
    let smt_builtins: HashSet<&str> = [
        "assert", "check-sat", "get-model", "set-logic", "declare-const",
        "and", "or", "not", "=", "distinct", "<", "<=", ">", ">=",
        "+", "-", "*", "div", "mod", "ite", "true", "false", "Int", "Real", "Bool", "ALL",
    ]
    .into_iter()
    .collect();

    let mut idents = HashSet::new();
    for line in text.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.starts_with(';') {
            continue;
        }
        let cleaned = trimmed_line.replace('(', " ").replace(')', " ");
        for token in cleaned.split_whitespace() {
            let t = token.trim();
            if t.is_empty()
                || t.parse::<i64>().is_ok()
                || t.parse::<f64>().is_ok()
                || (t.starts_with('"') && t.ends_with('"'))
                || smt_builtins.contains(t)
            {
                continue;
            }
            if t.chars().all(|c| c.is_alphanumeric() || c == '_') {
                idents.insert(t.to_string());
            }
        }
    }
    idents
}

impl PythonObligation {
    #[allow(dead_code)]
    pub fn from_raw(raw: RawObligation) -> Self {
        Self::from_raw_with_extra_guards(raw, &[])
    }

    pub fn from_raw_with_extra_guards(raw: RawObligation, extra_guards: &[PyExpr]) -> Self {
        let mut body_lines = Vec::new();

        // Add path guards
        for (i, guard) in raw.guards.iter().enumerate() {
            let smt_guard = guard.to_smt2();
            if !smt_guard.is_empty() && smt_guard != "true" {
                body_lines.push(format!("; Guard {i}"));
                body_lines.push(format!("(assert {smt_guard})"));
            }
        }

        // Add loop induction bounds from induction_bounds
        for (i, bound) in raw.induction_bounds.iter().enumerate() {
            let smt_b = bound.to_smt2();
            if !smt_b.is_empty() && smt_b != "true" {
                body_lines.push(format!("; Loop induction bound {i}"));
                body_lines.push(format!("(assert {smt_b})"));
            }
        }

        // Add loop induction bounds from loop_bounds info
        for lb in &raw.loop_bounds {
            let var = sanitize_ident(&lb.variable);
            if let Some(ref lower) = lb.lower {
                body_lines.push(format!("; Loop bound lower for {var}"));
                body_lines.push(format!("(assert (>= {var} {}))", lower.to_smt2()));
            }
            if let Some(ref upper) = lb.upper {
                body_lines.push(format!("; Loop bound upper for {var}"));
                body_lines.push(format!("(assert (< {var} {}))", upper.to_smt2()));
            }
        }

        // Add proved helper precondition guards
        for (i, extra) in extra_guards.iter().enumerate() {
            let smt_extra = extra.to_smt2();
            if !smt_extra.is_empty() && smt_extra != "true" {
                body_lines.push(format!("; Proved helper precondition guard {i}"));
                body_lines.push(format!("(assert {smt_extra})"));
            }
        }

        // Add target defect condition
        body_lines.push("; Target defect condition (SAT = defect exists, UNSAT = safe proof)".to_string());
        let target_smt = raw.target.to_smt2();
        body_lines.push(format!("(assert {target_smt})"));
        body_lines.push("(check-sat)".to_string());

        let body_text = body_lines.join("\n");
        let all_idents = extract_smt_identifiers(&body_text);

        let mut lines = Vec::new();
        lines.push("; Keller-Verify Python Proof Obligation (SMT-LIB 2.6)".to_string());
        lines.push(format!("; Module: {}", raw.module));
        lines.push(format!("; Function: {}", raw.function));
        lines.push(format!("; File: {}:{}", raw.file, raw.line));
        lines.push(format!("; Kind: {}", raw.kind));
        lines.push(format!("; Source: {}", raw.source_line));
        lines.push("(set-logic ALL)".to_string());

        let mut sorted_vars: Vec<String> = all_idents.into_iter().collect();
        sorted_vars.sort();

        for var in &sorted_vars {
            let var_type = if let Some(ann) = raw.params.get(var) {
                match ann.to_lowercase().as_str() {
                    "float" => "Real",
                    "bool" => "Bool",
                    _ => "Int",
                }
            } else if var.contains("float") || var.contains("eps") || var.contains("dist") || var.contains("prob") {
                "Real"
            } else if var.starts_with("len_") {
                "Int"
            } else {
                "Int"
            };

            lines.push(format!("(declare-const {var} {var_type})"));
            if var.starts_with("len_") {
                lines.push(format!("(assert (>= {var} 0))"));
            }
            if var.ends_with("_is_none") || var.contains("_in_") || var == "type_mismatch" || var.ends_with("_exhausted") || var == "eval_injection" || var == "unpacking_mismatch" || var == "format_mismatch" || var == "concurrency_race" {
                lines.push(format!("(assert (or (= {var} 0) (= {var} 1)))"));
            }
        }

        lines.push(body_text);
        let smt_content = lines.join("\n");
        Self { raw, smt_content }
    }

    pub fn to_vir(&self, extra_guards: &[PyExpr]) -> (crate::vir::VirFunction, crate::vir::SmtEmitter) {
        crate::vir::lower_obligation_to_vir(&self.raw, extra_guards)
    }

    pub fn solve(&self, z3_bin: &Path, timeout_ms: u64) -> SolverVerdict {
        solve_smt_content(&self.smt_content, z3_bin, timeout_ms)
    }

    pub fn solve_with_options(
        &self,
        _options: &crate::report::VerifyOptions,
        z3_bin: &Path,
        timeout_ms: u64,
    ) -> SolverVerdict {
        self.solve(z3_bin, timeout_ms)
    }

    pub fn generate_pinned_witness_smt2(&self, model: &HashMap<String, String>) -> String {
        let mut lines = Vec::new();
        lines.push(self.smt_content.clone());
        lines.push("; Pinned counterexample witness values".to_string());
        for (var, val) in model {
            let clean_var = sanitize_ident(var);
            let val_smt = if let Ok(n) = val.parse::<i64>() {
                if n < 0 {
                    format!("(- {})", n.abs())
                } else {
                    n.to_string()
                }
            } else {
                val.clone()
            };
            lines.push(format!("(assert (= {clean_var} {val_smt}))"));
        }
        lines.join("\n")
    }
}

pub fn solve_smt_content(smt_content: &str, z3_bin: &Path, timeout_ms: u64) -> SolverVerdict {
    let mut cmd = Command::new(z3_bin);
    cmd.arg("-in").arg("-smt2");
    let timeout_sec = (timeout_ms / 1000).max(1);
    cmd.arg(format!("-T:{timeout_sec}"));
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return SolverVerdict::Warn {
                reason: format!("Failed to spawn Z3 solver: {e}"),
            };
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let input = format!("{}\n(get-model)\n", smt_content);
        let _ = stdin.write_all(input.as_bytes());
    }

    let output = match child.wait_with_output() {
        Ok(out) => out,
        Err(e) => {
            return SolverVerdict::Warn {
                reason: format!("Z3 process failed: {e}"),
            };
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();

    if trimmed.starts_with("unsat") {
        SolverVerdict::Pass
    } else if trimmed.starts_with("sat") {
        let model = parse_z3_model(&stdout);
        SolverVerdict::Fail { model }
    } else {
        SolverVerdict::Warn {
            reason: if trimmed.is_empty() {
                "Solver returned no output (timeout)".to_string()
            } else {
                trimmed.lines().next().unwrap_or("unknown").to_string()
            },
        }
    }
}

pub fn check_call_site_precondition(
    site: &CallSiteInfo,
    param_names: &[String],
    precondition: &PyExpr,
    z3_bin: &Path,
    timeout_ms: u64,
) -> bool {
    let mut mapping = HashMap::new();
    for (i, param) in param_names.iter().enumerate() {
        if i < site.args.len() {
            mapping.insert(param.clone(), site.args[i].clone());
        }
    }

    let substituted = precondition.substitute(&mapping);
    let neg_precond = PyExpr::Unary {
        op: "not".to_string(),
        arg: Box::new(substituted),
    };

    let mut body_lines = Vec::new();
    for (i, guard) in site.guards.iter().enumerate() {
        let smt_g = guard.to_smt2();
        if !smt_g.is_empty() && smt_g != "true" {
            body_lines.push(format!("; Caller guard {i}"));
            body_lines.push(format!("(assert {smt_g})"));
        }
    }
    body_lines.push("; Negated callee precondition to check violation".to_string());
    body_lines.push(format!("(assert {})", neg_precond.to_smt2()));
    body_lines.push("(check-sat)".to_string());

    let body_text = body_lines.join("\n");
    let all_idents = extract_smt_identifiers(&body_text);

    let mut lines = Vec::new();
    lines.push("(set-logic ALL)".to_string());
    let mut sorted_vars: Vec<String> = all_idents.into_iter().collect();
    sorted_vars.sort();

    for var in &sorted_vars {
        lines.push(format!("(declare-const {var} Int)"));
        if var.starts_with("len_") {
            lines.push(format!("(assert (>= {var} 0))"));
        }
    }
    lines.push(body_text);
    let smt_content = lines.join("\n");

    let verdict = solve_smt_content(&smt_content, z3_bin, timeout_ms);
    verdict == SolverVerdict::Pass
}

pub fn parse_z3_model(output: &str) -> HashMap<String, String> {
    let mut model = HashMap::new();
    let mut lines = output.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.starts_with("(define-fun ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 4 {
                let var_name = parts[1].to_string();
                if parts.len() >= 5 && parts[parts.len() - 1] != ")" {
                    let val = parts[4].trim_end_matches(')').to_string();
                    model.insert(var_name, val);
                } else if let Some(next_line) = lines.next() {
                    let next_trimmed = next_line.trim().trim_end_matches(')');
                    let val = if next_trimmed.starts_with("(- ") {
                        let neg_num = next_trimmed.trim_start_matches("(- ").trim_end_matches(')').trim();
                        format!("-{neg_num}")
                    } else {
                        next_trimmed.to_string()
                    };
                    model.insert(var_name, val);
                }
            }
        }
    }

    model
}

pub fn resolve_z3_binary(run_root: &Path) -> Result<PathBuf, String> {
    if let Ok(val) = std::env::var("Z3_BIN") {
        let p = PathBuf::from(val);
        if p.is_file() {
            return Ok(p);
        }
    }

    if let Some(bin) = crate::host_env::discover(run_root).z3_bin {
        return Ok(bin);
    }

    if let Ok(cwd) = std::env::current_dir() {
        if let Some(bin) = crate::host_env::discover(&cwd).z3_bin {
            return Ok(bin);
        }
    }

    if let Ok(output) = Command::new("where.exe").arg("z3.exe").output() {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = out_str.lines().next() {
                let p = PathBuf::from(first_line.trim());
                if p.is_file() {
                    return Ok(p);
                }
            }
        }
    }

    Err("Could not find z3 executable. Ensure Z3 is installed and in PATH or under .z3/.".to_string())
}
