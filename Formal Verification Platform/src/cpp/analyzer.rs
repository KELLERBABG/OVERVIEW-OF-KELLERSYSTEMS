//! C++ Invariant Verification Engine.
//!
//! Verifies 7 critical C++ defect classes:
//! 1. `cpp-iterator-invalidation`: Vector modification while iterating or using invalid iterators
//! 2. `cpp-pure-virtual-call`: Calling pure virtual method from constructor/destructor or deleted object
//! 3. `cpp-use-after-delete`: Accessing pointer after delete or delete[]
//! 4. `cpp-double-delete`: Deleting pointer twice without nulling
//! 5. `cpp-null-deref`: Pointer dereference without null check
//! 6. `cpp-vector-oob`: Unbounded vector index vec[i] or .at(i)
//! 7. `cpp-div-by-zero`: Division by variable that can evaluate to 0

use crate::cpp::ast::*;
use crate::langaudit::Finding;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtrState {
    Null,
    NonNull,
    Allocated,
    Deleted,
    Unknown,
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

    pub fn analyze_tu(&mut self, tu: &TranslationUnit) {
        // Collect class pure virtual method maps: class_name -> set of pure virtual method names
        let mut class_pure_virtuals: HashMap<String, HashSet<String>> = HashMap::new();
        for class in &tu.classes {
            let pvs: HashSet<String> = class
                .pure_virtual_methods()
                .iter()
                .map(|m| m.name.clone())
                .collect();
            class_pure_virtuals.insert(class.name.clone(), pvs);
        }

        // Propagate base class pure virtuals if not overridden
        for class in &tu.classes {
            for base in &class.base_classes {
                if let Some(base_pvs) = class_pure_virtuals.get(base).cloned() {
                    let cur_methods: HashSet<String> =
                        class.methods.iter().map(|m| m.name.clone()).collect();
                    let cur_pvs = class_pure_virtuals.entry(class.name.clone()).or_default();
                    for bpv in base_pvs {
                        if !cur_methods.contains(&bpv) {
                            cur_pvs.insert(bpv);
                        }
                    }
                }
            }
        }

        // Collect container initial sizes from TU
        let mut container_sizes: HashMap<String, usize> = HashMap::new();
        for c in &tu.containers {
            if let Some(sz) = c.initial_size {
                container_sizes.insert(c.var_name.clone(), sz);
            }
        }

        // Analyze each function
        for func in &tu.functions {
            self.analyze_function(func, &class_pure_virtuals, &container_sizes);
        }

        // Analyze class methods declared inside class bodies that were not in functions
        for class in &tu.classes {
            let pvs = class_pure_virtuals.get(&class.name).cloned().unwrap_or_default();
            for method in &class.methods {
                if !method.body_lines.is_empty() {
                    let is_ctor = method.name == class.name;
                    let is_dtor = method.name == format!("~{}", class.name);
                    let f = FunctionDecl {
                        name: format!("{}::{}", class.name, method.name),
                        class_name: Some(class.name.clone()),
                        return_type: method.return_type.clone(),
                        params: Vec::new(),
                        line: method.line,
                        body_lines: method.body_lines.clone(),
                        is_constructor: is_ctor,
                        is_destructor: is_dtor,
                    };
                    self.analyze_function(&f, &class_pure_virtuals, &container_sizes);
                }
            }

            // Check if class itself has pure virtual call sites
            for call in &tu.virtual_calls {
                if call.receiver_is_abstract && pvs.contains(&call.method_name) {
                    self.obligations_total += 1;
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: call.line,
                        class: "cpp-pure-virtual-call".into(),
                        verdict: "possible-defect".into(),
                        message: format!(
                            "Virtual call to pure virtual method '{}' through potentially abstract receiver '{}'",
                            call.method_name, call.receiver
                        ),
                    });
                }
            }
        }
    }

    fn analyze_function(
        &mut self,
        func: &FunctionDecl,
        class_pure_virtuals: &HashMap<String, HashSet<String>>,
        global_container_sizes: &HashMap<String, usize>,
    ) {
        let mut pointers: HashMap<String, PtrState> = HashMap::new();
        let mut int_vars: HashMap<String, i64> = HashMap::new();
        let mut active_iterators: HashMap<String, (String, usize, bool)> = HashMap::new();
        let mut container_sizes: HashMap<String, usize> = global_container_sizes.clone();

        let mut guarded_ptrs: HashSet<String> = HashSet::new();
        let mut guarded_vars: HashSet<String> = HashSet::new();
        let mut guarded_indices: HashSet<String> = HashSet::new();

        // 1. Pure virtual call in constructor/destructor check
        if func.is_constructor || func.is_destructor {
            if let Some(cname) = &func.class_name {
                if let Some(pvs) = class_pure_virtuals.get(cname) {
                    for (line_num, line) in &func.body_lines {
                        let trimmed = line.trim();
                        for pv in pvs {
                            // Check for call: pv(), this->pv(), Cname::pv()
                            let call1 = format!("{}(", pv);
                            let call2 = format!("->{}(", pv);
                            let call3 = format!(".{}(", pv);
                            let call4 = format!("::{}(", pv);
                            if (trimmed.contains(&call1)
                                || trimmed.contains(&call2)
                                || trimmed.contains(&call3)
                                || trimmed.contains(&call4))
                                && !trimmed.starts_with("virtual")
                            {
                                self.obligations_total += 1;
                                self.findings.push(Finding {
                                    file: self.file_path.clone(),
                                    line: *line_num,
                                    class: "cpp-pure-virtual-call".into(),
                                    verdict: "proved-defect".into(),
                                    message: format!(
                                        "Pure virtual method '{}' called from {} of class '{}'",
                                        pv,
                                        if func.is_constructor {
                                            "constructor"
                                        } else {
                                            "destructor"
                                        },
                                        cname
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }

        // 2. Scan loop constructs for iterator invalidation and loop bounds
        self.check_loop_invariants(&func.body_lines);

        // 3. Line-by-line sequential analysis
        for (line_num, line) in &func.body_lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }

            // --- Guard updates ---
            self.update_guards(
                trimmed,
                &mut guarded_ptrs,
                &mut guarded_vars,
                &mut guarded_indices,
            );

            // --- Vector declaration with size: std::vector<int> v(10); or v = {1, 2, 3}; ---
            self.track_container_decls(trimmed, &mut container_sizes);

            // --- Pointer state tracking ---
            self.track_pointer_decls_and_assignments(trimmed, &mut pointers);

            // --- Iterator tracking & sequential invalidation ---
            self.track_iterators(
                trimmed,
                *line_num,
                &mut active_iterators,
            );

            // --- Check double-delete & update delete state ---
            self.check_delete_operations(trimmed, *line_num, &mut pointers);

            // --- Check use-after-delete & null-deref ---
            self.check_pointer_dereferences(
                trimmed,
                *line_num,
                &pointers,
                &guarded_ptrs,
            );

            // --- Check virtual call on deleted pointer ---
            self.check_virtual_calls_on_deleted(trimmed, *line_num, &pointers);

            // --- Track integer constant values ---
            self.track_integer_assignments(trimmed, &mut int_vars);

            // --- Check vector out-of-bounds ---
            self.check_vector_bounds(
                trimmed,
                *line_num,
                &container_sizes,
                &guarded_indices,
            );

            // --- Check division by zero ---
            self.check_division_by_zero(
                trimmed,
                *line_num,
                &int_vars,
                &guarded_vars,
            );
        }
    }

    /// Check loop-specific invariants: range-for mutations, iterator-for mutations, off-by-one loops.
    fn check_loop_invariants(&mut self, body_lines: &[(usize, String)]) {
        let mut i = 0;
        while i < body_lines.len() {
            let (_line_num, line) = &body_lines[i];
            let trimmed = line.trim();

            // Range-based for loop: for (auto &x : vec_name)
            if trimmed.starts_with("for ") && trimmed.contains(':') && trimmed.contains('(') {
                if let Some(colon_pos) = trimmed.find(':') {
                    if let Some(close_paren) = trimmed[colon_pos..].find(')') {
                        let vec_part = trimmed[colon_pos + 1..colon_pos + close_paren].trim();
                        let vec_name: String = vec_part
                            .chars()
                            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                            .collect();

                        if !vec_name.is_empty() {
                            // Collect loop body
                            let (loop_body, consumed) = self.collect_loop_body(body_lines, i);
                            self.obligations_total += 1;
                            let mut mutated = false;
                            for (b_line, b_str) in &loop_body {
                                if self.is_container_mutating_call(b_str, &vec_name) {
                                    mutated = true;
                                    self.findings.push(Finding {
                                        file: self.file_path.clone(),
                                        line: *b_line,
                                        class: "cpp-iterator-invalidation".into(),
                                        verdict: "proved-defect".into(),
                                        message: format!(
                                            "Iterator invalidation: modifying container '{}' during range-based for loop",
                                            vec_name
                                        ),
                                    });
                                    break;
                                }
                            }
                            if !mutated {
                                self.obligations_proved_safe += 1;
                            }
                            i += consumed.max(1);
                            continue;
                        }
                    }
                }
            }

            // Iterator-based for loop: for (auto it = v.begin(); it != v.end(); ++it)
            if trimmed.starts_with("for ") && trimmed.contains(".begin()") && trimmed.contains(';') {
                if let Some(it_name) = self.extract_iterator_var_from_for(trimmed) {
                    if let Some(v_name) = self.extract_container_from_begin(trimmed) {
                        let (loop_body, consumed) = self.collect_loop_body(body_lines, i);
                        self.obligations_total += 1;
                        let mut mutated = false;
                        for (b_line, b_str) in &loop_body {
                            if self.is_container_mutating_call(b_str, &v_name) {
                                // Safe if iterator is reassigned: it = v.erase(it)
                                let reassign = format!("{} = ", it_name);
                                if !b_str.contains(&reassign) {
                                    mutated = true;
                                    self.findings.push(Finding {
                                        file: self.file_path.clone(),
                                        line: *b_line,
                                        class: "cpp-iterator-invalidation".into(),
                                        verdict: "proved-defect".into(),
                                        message: format!(
                                            "Iterator invalidation: mutating container '{}' during iterator loop on '{}'",
                                            v_name, it_name
                                        ),
                                    });
                                    break;
                                }
                            }
                        }
                        if !mutated {
                            self.obligations_proved_safe += 1;
                        }
                        i += consumed.max(1);
                        continue;
                    }
                }
            }

            // Loop off-by-one check: for (size_t i = 0; i <= v.size(); i++)
            if trimmed.starts_with("for ") && trimmed.contains(".size()") && trimmed.contains("<=") {
                let idx_var = self.extract_loop_index_var(trimmed);
                let vec_name = self.extract_container_from_size(trimmed);
                if let (Some(iv), Some(vn)) = (idx_var, vec_name) {
                    let (loop_body, consumed) = self.collect_loop_body(body_lines, i);
                    for (b_line, b_str) in &loop_body {
                        let index_pat = format!("{}[{}]", vn, iv);
                        if b_str.contains(&index_pat) {
                            self.obligations_total += 1;
                            self.findings.push(Finding {
                                file: self.file_path.clone(),
                                line: *b_line,
                                class: "cpp-vector-oob".into(),
                                verdict: "proved-defect".into(),
                                message: format!(
                                    "Vector out of bounds: loop condition '<=' causes index '{}' to reach size of '{}'",
                                    iv, vn
                                ),
                            });
                        }
                    }
                    i += consumed.max(1);
                    continue;
                }
            }

            i += 1;
        }
    }

    fn collect_loop_body(
        &self,
        body_lines: &[(usize, String)],
        start_idx: usize,
    ) -> (Vec<(usize, String)>, usize) {
        let mut loop_lines = Vec::new();
        let mut depth = 0;
        let mut found_open = false;
        let mut idx = start_idx;

        while idx < body_lines.len() {
            let line = &body_lines[idx].1;
            for ch in line.chars() {
                if ch == '{' {
                    depth += 1;
                    found_open = true;
                } else if ch == '}' {
                    depth -= 1;
                }
            }
            if idx > start_idx {
                loop_lines.push(body_lines[idx].clone());
            }
            if found_open && depth == 0 {
                break;
            }
            idx += 1;
        }

        let consumed = idx - start_idx;
        (loop_lines, consumed)
    }

    fn is_container_mutating_call(&self, line: &str, container: &str) -> bool {
        let mut_methods = [
            "push_back",
            "pop_back",
            "insert",
            "erase",
            "resize",
            "clear",
            "emplace_back",
            "assign",
        ];
        for m in &mut_methods {
            let pat1 = format!("{}.{}(", container, m);
            let pat2 = format!("{}->{}(", container, m);
            if line.contains(&pat1) || line.contains(&pat2) {
                return true;
            }
        }
        false
    }

    fn extract_iterator_var_from_for(&self, line: &str) -> Option<String> {
        // for (auto it = ...
        if let Some(eq) = line.find('=') {
            let before = line[..eq].trim();
            let var = before.split_whitespace().next_back()?.trim();
            Some(var.to_string())
        } else {
            None
        }
    }

    fn extract_container_from_begin(&self, line: &str) -> Option<String> {
        // v.begin()
        if let Some(pos) = line.find(".begin()") {
            let before = &line[..pos];
            let v = before
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .filter(|s| !s.is_empty())
                .next_back()?;
            Some(v.to_string())
        } else {
            None
        }
    }

    fn extract_loop_index_var(&self, line: &str) -> Option<String> {
        // for (int i = 0; ...
        let start = line.find('(')?;
        let semi1 = line.find(';')?;
        let init_part = &line[start + 1..semi1];
        let eq = init_part.find('=')?;
        let before_eq = init_part[..eq].trim();
        let var = before_eq.split_whitespace().next_back()?;
        Some(var.to_string())
    }

    fn extract_container_from_size(&self, line: &str) -> Option<String> {
        // v.size()
        let pos = line.find(".size()")?;
        let before = &line[..pos];
        let v = before
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .filter(|s| !s.is_empty())
            .next_back()?;
        Some(v.to_string())
    }

    fn update_guards(
        &self,
        line: &str,
        guarded_ptrs: &mut HashSet<String>,
        guarded_vars: &mut HashSet<String>,
        guarded_indices: &mut HashSet<String>,
    ) {
        if !line.starts_with("if") && !line.contains("if (") && !line.contains("if(") {
            return;
        }

        // if (p != nullptr) / if (p != NULL) / if (p != 0) / if (p)
        if line.contains("!= nullptr")
            || line.contains("!= NULL")
            || line.contains("!= 0")
        {
            if let Some(neq) = line.find("!=") {
                let before = &line[..neq];
                let p = before
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .filter(|s| !s.is_empty())
                    .next_back();
                if let Some(var) = p {
                    guarded_ptrs.insert(var.to_string());
                    guarded_vars.insert(var.to_string());
                }
            }
        } else if line.contains("== nullptr") || line.contains("== NULL") {
            // Negative guard
        } else if let Some(open) = line.find('(') {
            if let Some(close) = line[open..].find(')') {
                let cond = line[open + 1..open + close].trim();
                let ident: String = cond
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !ident.is_empty() && ident == cond {
                    guarded_ptrs.insert(ident.clone());
                    guarded_vars.insert(ident);
                }
            }
        }

        // if (b > 0) or if (b < 0)
        if line.contains("> 0") || line.contains("< 0") {
            if let Some(gt) = line.find('>').or_else(|| line.find('<')) {
                let before = &line[..gt];
                let var = before
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .filter(|s| !s.is_empty())
                    .next_back();
                if let Some(v) = var {
                    guarded_vars.insert(v.to_string());
                }
            }
        }

        // if (i < v.size()) or if (i < n)
        if line.contains('<') {
            if let Some(lt) = line.find('<') {
                let before = &line[..lt];
                let var = before
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .filter(|s| !s.is_empty())
                    .next_back();
                if let Some(v) = var {
                    guarded_indices.insert(v.to_string());
                }
            }
        }
    }

    fn track_container_decls(
        &self,
        line: &str,
        container_sizes: &mut HashMap<String, usize>,
    ) {
        if line.contains("vector<") || line.contains("std::vector<") {
            if let Some(gt) = line.find('>') {
                let after = line[gt + 1..].trim();
                let name: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    let rest = after[name.len()..].trim();
                    if rest.starts_with('(') {
                        if let Some(close) = rest.find(')') {
                            if let Ok(sz) = rest[1..close].trim().parse::<usize>() {
                                container_sizes.insert(name.clone(), sz);
                            }
                        }
                    } else if rest.starts_with('{') {
                        if let Some(close) = rest.find('}') {
                            let count = rest[1..close]
                                .split(',')
                                .filter(|s| !s.trim().is_empty())
                                .count();
                            container_sizes.insert(name.clone(), count);
                        }
                    } else if rest.starts_with('=') && rest.contains('{') {
                        if let Some(open) = rest.find('{') {
                            if let Some(close) = rest.find('}') {
                                let count = rest[open + 1..close]
                                    .split(',')
                                    .filter(|s| !s.trim().is_empty())
                                    .count();
                                container_sizes.insert(name.clone(), count);
                            }
                        }
                    } else if rest.starts_with(';') {
                        // Empty vector
                        container_sizes.insert(name, 0);
                    }
                }
            }
        }
    }

    fn track_pointer_decls_and_assignments(
        &self,
        line: &str,
        pointers: &mut HashMap<String, PtrState>,
    ) {
        // nullptr or NULL assignments
        if line.contains("= nullptr") || line.contains("= NULL") {
            if let Some(eq) = line.find('=') {
                let before = line[..eq].trim();
                let var = before
                    .split_whitespace()
                    .next_back()
                    .unwrap_or("")
                    .trim_start_matches('*');
                if !var.is_empty() {
                    pointers.insert(var.to_string(), PtrState::Null);
                }
            }
        } else if line.contains("= new ") || line.contains("= new\t") {
            if let Some(eq) = line.find('=') {
                let before = line[..eq].trim();
                let var = before
                    .split_whitespace()
                    .next_back()
                    .unwrap_or("")
                    .trim_start_matches('*');
                if !var.is_empty() {
                    pointers.insert(var.to_string(), PtrState::Allocated);
                }
            }
        } else if line.contains('*') && line.contains('=') && !line.contains("==") {
            // Potential pointer declaration: int *p = ...
            if let Some(star) = line.find('*') {
                let after_star = line[star + 1..].trim();
                let var: String = after_star
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !var.is_empty() && !pointers.contains_key(&var) {
                    if line.contains("new ") {
                        pointers.insert(var, PtrState::Allocated);
                    } else if line.contains("&") {
                        pointers.insert(var, PtrState::NonNull);
                    } else {
                        pointers.insert(var, PtrState::Unknown);
                    }
                }
            }
        }
    }

    fn track_iterators(
        &mut self,
        line: &str,
        line_num: usize,
        active_iterators: &mut HashMap<String, (String, usize, bool)>,
    ) {
        // Iterator declaration: auto it = v.begin();
        if (line.contains("auto ") || line.contains("iterator "))
            && line.contains(".begin()")
            && line.contains('=')
        {
            if let Some(eq) = line.find('=') {
                let before = line[..eq].trim();
                let it_name = before.split_whitespace().next_back().unwrap_or("");
                let after = line[eq + 1..].trim();
                if let Some(dot) = after.find(".begin()") {
                    let v_name = after[..dot].trim();
                    if !it_name.is_empty() && !v_name.is_empty() {
                        active_iterators.insert(
                            it_name.to_string(),
                            (v_name.to_string(), line_num, false),
                        );
                    }
                }
            }
        }

        // Container mutation invalidates iterators
        for (it, (v, _, is_inv)) in active_iterators.iter_mut() {
            if self.is_container_mutating_call(line, v) {
                // If line reassigns this iterator (e.g. it = v.erase(it)), don't invalidate
                let reassign = format!("{} = ", it);
                if !line.contains(&reassign) {
                    *is_inv = true;
                }
            }
        }

        // Sequential iterator use: *it, it->, ++it
        for (it, (v, _, is_inv)) in active_iterators.iter() {
            let use1 = format!("*{}", it);
            let use2 = format!("{}->", it);
            let use3 = format!("++{}", it);
            let use4 = format!("{}++", it);

            if line.contains(&use1)
                || line.contains(&use2)
                || line.contains(&use3)
                || line.contains(&use4)
            {
                self.obligations_total += 1;
                if *is_inv {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: line_num,
                        class: "cpp-iterator-invalidation".into(),
                        verdict: "proved-defect".into(),
                        message: format!(
                            "Iterator invalidation: iterator '{}' on '{}' used after container mutation",
                            it, v
                        ),
                    });
                } else {
                    self.obligations_proved_safe += 1;
                }
            }
        }
    }

    fn check_delete_operations(
        &mut self,
        line: &str,
        line_num: usize,
        pointers: &mut HashMap<String, PtrState>,
    ) {
        if line.starts_with("delete ")
            || line.starts_with("delete[] ")
            || line.contains(" delete ")
            || line.contains(" delete[] ")
        {
            let mut rest = if let Some(pos) = line.find("delete[]") {
                &line[pos + 8..]
            } else if let Some(pos) = line.find("delete") {
                &line[pos + 6..]
            } else {
                return;
            };
            rest = rest.trim();
            let var: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();

            if !var.is_empty() {
                self.obligations_total += 1;
                let current_state = pointers.get(&var).cloned().unwrap_or(PtrState::Unknown);

                if current_state == PtrState::Deleted {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: line_num,
                        class: "cpp-double-delete".into(),
                        verdict: "proved-defect".into(),
                        message: format!(
                            "Double delete: pointer '{}' deleted twice without intervening nulling",
                            var
                        ),
                    });
                } else {
                    // Safe delete (first delete, or deleting nullptr)
                    self.obligations_proved_safe += 1;
                    pointers.insert(var, PtrState::Deleted);
                }
            }
        }
    }

    fn check_pointer_dereferences(
        &mut self,
        line: &str,
        line_num: usize,
        pointers: &HashMap<String, PtrState>,
        guarded_ptrs: &HashSet<String>,
    ) {
        // Skip lines that are delete or declarations
        if line.starts_with("delete") || line.contains(" = new ") {
            return;
        }

        for (var, state) in pointers {
            if guarded_ptrs.contains(var) {
                continue;
            }

            let deref1 = format!("*{}", var);
            let deref2 = format!("{}->", var);
            let deref3 = format!("{}[", var);

            let has_deref = line.contains(&deref1)
                || line.contains(&deref2)
                || (line.contains(&deref3) && !line.starts_with("int ") && !line.starts_with("char "));

            if has_deref {
                self.obligations_total += 1;
                match state {
                    PtrState::Deleted => {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: line_num,
                            class: "cpp-use-after-delete".into(),
                            verdict: "proved-defect".into(),
                            message: format!(
                                "Use after delete: pointer '{}' dereferenced after delete",
                                var
                            ),
                        });
                    }
                    PtrState::Null => {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: line_num,
                            class: "cpp-null-deref".into(),
                            verdict: "proved-defect".into(),
                            message: format!(
                                "Null pointer dereference: pointer '{}' is null",
                                var
                            ),
                        });
                    }
                    PtrState::Allocated | PtrState::NonNull => {
                        self.obligations_proved_safe += 1;
                    }
                    PtrState::Unknown => {
                        self.obligations_proved_safe += 1;
                    }
                }
            }
        }
    }

    fn check_virtual_calls_on_deleted(
        &mut self,
        line: &str,
        line_num: usize,
        pointers: &HashMap<String, PtrState>,
    ) {
        if line.contains("->") {
            if let Some(pos) = line.find("->") {
                let before = &line[..pos];
                let var = before
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .filter(|s| !s.is_empty())
                    .next_back();
                if let Some(v) = var {
                    if let Some(PtrState::Deleted) = pointers.get(v) {
                        self.obligations_total += 1;
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: line_num,
                            class: "cpp-pure-virtual-call".into(),
                            verdict: "proved-defect".into(),
                            message: format!(
                                "Virtual call through deleted object pointer '{}'",
                                v
                            ),
                        });
                    }
                }
            }
        }
    }

    fn track_integer_assignments(&self, line: &str, int_vars: &mut HashMap<String, i64>) {
        if line.contains('=') && !line.contains("==") && !line.contains("!=") && !line.contains("<=") && !line.contains(">=") {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let lhs = parts[0].trim();
                let rhs = parts[1].trim().trim_end_matches(';').trim();
                let var_name = lhs.split_whitespace().next_back().unwrap_or("");
                if !var_name.is_empty() {
                    if let Ok(val) = rhs.parse::<i64>() {
                        int_vars.insert(var_name.to_string(), val);
                    }
                }
            }
        }
    }

    fn check_vector_bounds(
        &mut self,
        line: &str,
        line_num: usize,
        container_sizes: &HashMap<String, usize>,
        guarded_indices: &HashSet<String>,
    ) {
        for (vname, &size) in container_sizes {
            let pat_bracket = format!("{}[", vname);
            let pat_at = format!("{}.at(", vname);

            if let Some(pos) = line.find(&pat_bracket) {
                let after = &line[pos + pat_bracket.len()..];
                if let Some(close) = after.find(']') {
                    let idx_str = after[..close].trim();
                    self.eval_vector_index(
                        vname,
                        idx_str,
                        size,
                        line_num,
                        guarded_indices,
                    );
                }
            } else if let Some(pos) = line.find(&pat_at) {
                let after = &line[pos + pat_at.len()..];
                if let Some(close) = after.find(')') {
                    let idx_str = after[..close].trim();
                    self.eval_vector_index(
                        vname,
                        idx_str,
                        size,
                        line_num,
                        guarded_indices,
                    );
                }
            }
        }
    }

    fn eval_vector_index(
        &mut self,
        vname: &str,
        idx_str: &str,
        size: usize,
        line_num: usize,
        guarded_indices: &HashSet<String>,
    ) {
        self.obligations_total += 1;

        // Constant negative index
        if let Ok(idx_val) = idx_str.parse::<i64>() {
            if idx_val < 0 {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: line_num,
                    class: "cpp-vector-oob".into(),
                    verdict: "proved-defect".into(),
                    message: format!(
                        "Vector out of bounds: negative index '{}' on vector '{}'",
                        idx_val, vname
                    ),
                });
                return;
            } else if idx_val as usize >= size {
                self.findings.push(Finding {
                    file: self.file_path.clone(),
                    line: line_num,
                    class: "cpp-vector-oob".into(),
                    verdict: "proved-defect".into(),
                    message: format!(
                        "Vector out of bounds: constant index {} >= size {} on vector '{}'",
                        idx_val, size, vname
                    ),
                });
                return;
            } else {
                self.obligations_proved_safe += 1;
                return;
            }
        }

        // Variable index
        if guarded_indices.contains(idx_str) {
            self.obligations_proved_safe += 1;
        } else {
            // Unbounded access
            self.findings.push(Finding {
                file: self.file_path.clone(),
                line: line_num,
                class: "cpp-vector-oob".into(),
                verdict: "possible-defect".into(),
                message: format!(
                    "Potential vector out of bounds: index '{}' without bounds check against '{}'",
                    idx_str, vname
                ),
            });
        }
    }

    fn check_division_by_zero(
        &mut self,
        line: &str,
        line_num: usize,
        int_vars: &HashMap<String, i64>,
        guarded_vars: &HashSet<String>,
    ) {
        // Skip comments, includes, templates
        if line.starts_with('#') || line.starts_with("//") {
            return;
        }

        // Look for '/' or '%'
        for (i, ch) in line.char_indices() {
            if (ch == '/' || ch == '%')
                && i + 1 < line.len()
                && line.as_bytes().get(i + 1) != Some(&b'/')
                && line.as_bytes().get(i + 1) != Some(&b'*')
                && line.as_bytes().get(i + 1) != Some(&b'=')
                && (i == 0 || line.as_bytes().get(i - 1) != Some(&b'/'))
                && (i == 0 || line.as_bytes().get(i - 1) != Some(&b'<'))
            {
                let after = line[i + 1..].trim_start();
                let divisor: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();

                if divisor.is_empty() {
                    continue;
                }

                self.obligations_total += 1;

                if divisor == "0" {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: line_num,
                        class: "cpp-div-by-zero".into(),
                        verdict: "proved-defect".into(),
                        message: "Division by zero: divisor is literal 0".into(),
                    });
                } else if let Some(&0) = int_vars.get(&divisor) {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: line_num,
                        class: "cpp-div-by-zero".into(),
                        verdict: "proved-defect".into(),
                        message: format!(
                            "Division by zero: divisor variable '{}' is 0",
                            divisor
                        ),
                    });
                } else if let Ok(val) = divisor.parse::<i64>() {
                    if val != 0 {
                        self.obligations_proved_safe += 1;
                    }
                } else if guarded_vars.contains(&divisor) {
                    self.obligations_proved_safe += 1;
                } else {
                    self.findings.push(Finding {
                        file: self.file_path.clone(),
                        line: line_num,
                        class: "cpp-div-by-zero".into(),
                        verdict: "possible-defect".into(),
                        message: format!(
                            "Potential division by zero: divisor '{}' is unconstrained and may evaluate to 0",
                            divisor
                        ),
                    });
                }
            }
        }
    }
}
