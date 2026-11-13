//! Formal verification and defect checks for SQL.
//!
//! Provides static verification of 5 critical SQL defect classes:
//! 1. `sql-division-by-zero`: Division `/` in select/expressions where divisor can evaluate to 0 without NULLIF or CASE guard.
//! 2. `sql-not-null-violation`: `INSERT` statements supplying `NULL` (or omitting columns) for columns declared `NOT NULL` without `DEFAULT`.
//! 3. `sql-recursive-cte-unbounded`: Recursive CTEs without termination condition or depth limit in the recursive leg.
//! 4. `sql-null-comparison`: Using `= NULL` or `<> NULL` instead of `IS NULL` / `IS NOT NULL`.
//! 5. `sql-unguarded-update-delete`: `UPDATE` or `DELETE` statements without a `WHERE` clause.

use crate::langaudit::Finding;
use crate::sql::ast::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct SchemaContext {
    pub file_tables: HashMap<String, HashMap<String, TableDef>>,
    pub dir_tables: HashMap<String, HashMap<String, TableDef>>,
    pub global_tables: HashMap<String, TableDef>,
}

#[derive(Debug, Clone)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub constraints: Vec<TableConstraint>,
}

impl SchemaContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_table(&mut self, file_str: &str, table: CreateTableStmt) {
        let key = normalize_name(&table.name);
        let table_def = TableDef {
            name: table.name,
            columns: table.columns,
            constraints: table.constraints,
        };

        self.file_tables
            .entry(file_str.to_string())
            .or_default()
            .insert(key.clone(), table_def.clone());

        let parent = std::path::Path::new(file_str)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        self.dir_tables
            .entry(parent)
            .or_default()
            .insert(key.clone(), table_def.clone());

        self.global_tables.insert(key, table_def);
    }

    pub fn alter_table(&mut self, file_str: &str, alter: &AlterTableStmt) {
        let key = normalize_name(&alter.table_name);
        let parent = std::path::Path::new(file_str)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        if let Some(tables) = self.file_tables.get_mut(file_str) {
            if let Some(t) = tables.get_mut(&key) {
                apply_alter_actions(t, &alter.actions);
            }
        }
        if let Some(tables) = self.dir_tables.get_mut(&parent) {
            if let Some(t) = tables.get_mut(&key) {
                apply_alter_actions(t, &alter.actions);
            }
        }
        if let Some(t) = self.global_tables.get_mut(&key) {
            apply_alter_actions(t, &alter.actions);
        }
    }

    pub fn get_table(&self, file_str: &str, name: &str) -> Option<&TableDef> {
        let key = normalize_name(name);

        // 1. File-level tables
        if let Some(tables) = self.file_tables.get(file_str) {
            if let Some(t) = tables.get(&key) {
                return Some(t);
            }
        }

        // 2. Directory-level tables
        let parent = std::path::Path::new(file_str)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Some(tables) = self.dir_tables.get(&parent) {
            if let Some(t) = tables.get(&key) {
                return Some(t);
            }
        }

        // 3. Global fallback
        self.global_tables.get(&key)
    }
}

fn apply_alter_actions(table: &mut TableDef, actions: &[AlterAction]) {
    for action in actions {
        match action {
            AlterAction::AddColumn(col) => {
                table.columns.push(col.clone());
            }
            AlterAction::DropColumn(name) => {
                let col_key = normalize_name(name);
                table.columns.retain(|c| normalize_name(&c.name) != col_key);
            }
            AlterAction::AddConstraint(c) => {
                table.constraints.push(c.clone());
            }
            AlterAction::DropConstraint(_) => {}
            AlterAction::ModifyColumn(col) => {
                let col_key = normalize_name(&col.name);
                if let Some(existing) = table.columns.iter_mut().find(|c| normalize_name(&c.name) == col_key) {
                    *existing = col.clone();
                } else {
                    table.columns.push(col.clone());
                }
            }
            _ => {}
        }
    }
}

pub fn normalize_name(name: &str) -> String {
    let clean = name.rsplit('.').next().unwrap_or(name);
    clean
        .trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']')
        .to_ascii_lowercase()
}

#[derive(Debug, Clone, Default)]
pub struct ExprGuardContext {
    pub guarded_vars: HashSet<String>,
}

pub struct Verifier<'a> {
    pub file_path: String,
    pub schema: &'a SchemaContext,
    pub functions_lowered: usize,
    pub obligations_total: usize,
    pub obligations_proved_safe: usize,
    pub findings: Vec<Finding>,
    last_insert_table: Option<String>,
}

impl<'a> Verifier<'a> {
    pub fn new(file_path: &str, schema: &'a SchemaContext) -> Self {
        Verifier {
            file_path: file_path.to_string(),
            schema,
            functions_lowered: 0,
            obligations_total: 0,
            obligations_proved_safe: 0,
            findings: Vec::new(),
            last_insert_table: None,
        }
    }

    pub fn verify_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::CreateTable(create) => {
                self.functions_lowered += 1;
                for col in &create.columns {
                    if let Some(ref expr) = col.check_expr {
                        let ctx = ExprGuardContext::default();
                        self.check_expr(expr, &ctx);
                    }
                }
                for c in &create.constraints {
                    if let TableConstraint::Check(ref expr) = c {
                        let ctx = ExprGuardContext::default();
                        self.check_expr(expr, &ctx);
                    }
                }
            }
            Statement::AlterTable(alter) => {
                for action in &alter.actions {
                    if let AlterAction::AddConstraint(TableConstraint::Check(ref expr)) = action {
                        let ctx = ExprGuardContext::default();
                        self.check_expr(expr, &ctx);
                    }
                }
            }
            Statement::CreateFunction(func) => {
                self.functions_lowered += 1;
                for inner in &func.body {
                    self.verify_statement(inner);
                }
            }
            Statement::CreateProcedure(proc) => {
                self.functions_lowered += 1;
                for inner in &proc.body {
                    self.verify_statement(inner);
                }
            }
            Statement::CreateTrigger(trig) => {
                self.functions_lowered += 1;
                for inner in &trig.body {
                    self.verify_statement(inner);
                }
            }
            Statement::CreateView(view) => {
                self.functions_lowered += 1;
                self.verify_select(&view.query);
            }
            Statement::Select(select) => {
                self.functions_lowered += 1;
                self.verify_select(select);
            }
            Statement::Insert(insert) => {
                // Count data loader batches
                let is_new_batch = match &self.last_insert_table {
                    Some(tbl) => tbl != &insert.table_name,
                    None => true,
                };
                if is_new_batch || insert.select_query.is_some() {
                    self.functions_lowered += 1;
                    self.last_insert_table = Some(insert.table_name.clone());
                }

                if let Some(ref q) = insert.select_query {
                    self.verify_select(q);
                }
                self.verify_insert(insert);
            }
            Statement::Update(update) => {
                self.functions_lowered += 1;
                self.verify_update(update);
            }
            Statement::Delete(delete) => {
                self.functions_lowered += 1;
                self.verify_delete(delete);
            }
            Statement::DropTable(_) | Statement::Other { .. } => {}
        }
    }

    fn verify_update(&mut self, update: &UpdateStmt) {
        // 5. sql-unguarded-update-delete
        self.obligations_total += 1;
        if update.where_clause.is_none() {
            self.findings.push(Finding {
                file: self.file_path.clone(),
                line: update.line,
                class: "sql-unguarded-update-delete".to_string(),
                verdict: "proved-defect".to_string(),
                message: format!(
                    "Unguarded UPDATE: statement modifies table '{}' without a WHERE clause",
                    update.table_name
                ),
            });
        } else {
            self.obligations_proved_safe += 1;
        }

        let mut ctx = ExprGuardContext::default();
        if let Some(ref wh) = update.where_clause {
            extract_guarded_vars(wh, &mut ctx.guarded_vars);
            self.check_expr(wh, &ctx);
        }

        for (_col, expr) in &update.assignments {
            self.check_expr(expr, &ctx);
        }
    }

    fn verify_delete(&mut self, delete: &DeleteStmt) {
        // 5. sql-unguarded-update-delete
        self.obligations_total += 1;
        if delete.where_clause.is_none() {
            self.findings.push(Finding {
                file: self.file_path.clone(),
                line: delete.line,
                class: "sql-unguarded-update-delete".to_string(),
                verdict: "proved-defect".to_string(),
                message: format!(
                    "Unguarded DELETE: statement truncates/deletes from table '{}' without a WHERE clause",
                    delete.table_name
                ),
            });
        } else {
            self.obligations_proved_safe += 1;
        }

        if let Some(ref wh) = delete.where_clause {
            let mut ctx = ExprGuardContext::default();
            extract_guarded_vars(wh, &mut ctx.guarded_vars);
            self.check_expr(wh, &ctx);
        }
    }

    fn verify_insert(&mut self, insert: &InsertStmt) {
        // 2. sql-not-null-violation
        let table_opt = self.schema.get_table(&self.file_path, &insert.table_name);

        if let Some(table) = table_opt {
            let mut violated = false;

            if !insert.columns.is_empty() {
                // Columns explicitly listed
                for row in &insert.values {
                    for (col_name, val_expr) in insert.columns.iter().zip(row.iter()) {
                        let col_key = normalize_name(col_name);
                        if let Some(col_def) = table.columns.iter().find(|c| normalize_name(&c.name) == col_key) {
                            if col_def.not_null && !col_def.is_auto_increment {
                                if val_expr.is_literal_null() {
                                    violated = true;
                                    self.findings.push(Finding {
                                        file: self.file_path.clone(),
                                        line: insert.line,
                                        class: "sql-not-null-violation".to_string(),
                                        verdict: "proved-defect".to_string(),
                                        message: format!(
                                            "NOT NULL violation: column '{}' in table '{}' is declared NOT NULL, but INSERT supplies explicit NULL",
                                            col_def.name, insert.table_name
                                        ),
                                    });
                                }
                            }
                        }
                    }

                    // Check for omitted non-default NOT NULL columns
                    for col_def in &table.columns {
                        if col_def.not_null && !col_def.has_default && !col_def.is_auto_increment {
                            let col_key = normalize_name(&col_def.name);
                            let present = insert.columns.iter().any(|c| normalize_name(c) == col_key);
                            if !present {
                                violated = true;
                                self.findings.push(Finding {
                                    file: self.file_path.clone(),
                                    line: insert.line,
                                    class: "sql-not-null-violation".to_string(),
                                    verdict: "proved-defect".to_string(),
                                    message: format!(
                                        "NOT NULL violation: non-nullable column '{}' in table '{}' without DEFAULT is omitted from INSERT column list",
                                        col_def.name, insert.table_name
                                    ),
                                });
                            }
                        }
                    }
                }
            } else if !insert.values.is_empty() {
                // Columns not listed: positional mapping
                for row in &insert.values {
                    for (i, val_expr) in row.iter().enumerate() {
                        if let Some(col_def) = table.columns.get(i) {
                            if col_def.not_null && !col_def.is_auto_increment {
                                if val_expr.is_literal_null() {
                                    violated = true;
                                    self.findings.push(Finding {
                                        file: self.file_path.clone(),
                                        line: insert.line,
                                        class: "sql-not-null-violation".to_string(),
                                        verdict: "proved-defect".to_string(),
                                        message: format!(
                                            "NOT NULL violation: column '{}' in table '{}' is declared NOT NULL, but INSERT supplies explicit NULL at position {}",
                                            col_def.name, insert.table_name, i + 1
                                        ),
                                    });
                                }
                            }
                        }
                    }

                    if row.len() < table.columns.len() {
                        for j in row.len()..table.columns.len() {
                            let col_def = &table.columns[j];
                            if col_def.not_null && !col_def.has_default && !col_def.is_auto_increment {
                                violated = true;
                                self.findings.push(Finding {
                                    file: self.file_path.clone(),
                                    line: insert.line,
                                    class: "sql-not-null-violation".to_string(),
                                    verdict: "proved-defect".to_string(),
                                    message: format!(
                                        "NOT NULL violation: non-nullable column '{}' in table '{}' without DEFAULT is omitted from values list",
                                        col_def.name, insert.table_name
                                    ),
                                });
                            }
                        }
                    }
                }
            }

            self.obligations_total += 1;
            if !violated {
                self.obligations_proved_safe += 1;
            }
        }

        // Check expressions in values
        let ctx = ExprGuardContext::default();
        for row in &insert.values {
            for expr in row {
                self.check_expr(expr, &ctx);
            }
        }
    }

    fn verify_select(&mut self, select: &SelectStmt) {
        // 3. sql-recursive-cte-unbounded
        if let Some(ref with_clause) = select.with_cte {
            for cte in &with_clause.ctes {
                let cte_norm = normalize_name(&cte.name);
                let is_recursive_cte = with_clause.recursive || is_query_recursive(&cte.query, &cte_norm);

                if is_recursive_cte {
                    self.obligations_total += 1;
                    let bounded = is_recursive_leg_bounded(&cte.query, &cte_norm, select.limit.is_some());
                    if bounded {
                        self.obligations_proved_safe += 1;
                    } else {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: cte.line,
                            class: "sql-recursive-cte-unbounded".to_string(),
                            verdict: "proved-defect".to_string(),
                            message: format!(
                                "Unbounded recursive CTE: CTE '{}' has recursive leg without WHERE termination condition or LIMIT",
                                cte.name
                            ),
                        });
                    }
                }
                self.verify_select(&cte.query);
            }
        }

        let mut ctx = ExprGuardContext::default();
        if let Some(ref wh) = select.where_clause {
            extract_guarded_vars(wh, &mut ctx.guarded_vars);
            self.check_expr(wh, &ctx);
        }

        for item in &select.projections {
            self.check_expr(&item.expr, &ctx);
        }

        for join in &select.joins {
            if let Some(ref on_expr) = join.on_clause {
                self.check_expr(on_expr, &ctx);
            }
        }

        for gb in &select.group_by {
            self.check_expr(gb, &ctx);
        }

        if let Some(ref having) = select.having {
            self.check_expr(having, &ctx);
        }

        if let Some(ref u_all) = select.union_all {
            self.verify_select(u_all);
        }

        if let Some(ref u) = select.union {
            self.verify_select(u);
        }
    }

    pub fn check_expr(&mut self, expr: &Expr, ctx: &ExprGuardContext) {
        match expr {
            Expr::Literal(_) => {}
            Expr::ColumnRef { .. } => {}
            Expr::BinaryOp {
                left,
                op,
                right,
                line,
            } => {
                // 1. sql-division-by-zero
                if *op == BinaryOpKind::Div {
                    self.obligations_total += 1;
                    if right.is_literal_zero() {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: *line,
                            class: "sql-division-by-zero".to_string(),
                            verdict: "proved-defect".to_string(),
                            message: "Division by zero: divisor evaluates to constant 0 without guard".to_string(),
                        });
                    } else if right.is_literal_nonzero() {
                        self.obligations_proved_safe += 1;
                    } else if right.is_nullif_guard() {
                        self.obligations_proved_safe += 1;
                    } else if let Expr::ColumnRef { ref column, .. } = **right {
                        let col_norm = normalize_name(column);
                        if ctx.guarded_vars.contains(&col_norm) {
                            self.obligations_proved_safe += 1;
                        } else {
                            self.findings.push(Finding {
                                file: self.file_path.clone(),
                                line: *line,
                                class: "sql-division-by-zero".to_string(),
                                verdict: "possible-defect".to_string(),
                                message: format!(
                                    "Potential division by zero: divisor '{}' can evaluate to 0 without NULLIF or CASE guard",
                                    column
                                ),
                            });
                        }
                    } else {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: *line,
                            class: "sql-division-by-zero".to_string(),
                            verdict: "possible-defect".to_string(),
                            message: "Potential division by zero: divisor in division expression can evaluate to 0 without NULLIF or CASE guard".to_string(),
                        });
                    }
                }

                // 4. sql-null-comparison
                if matches!(
                    *op,
                    BinaryOpKind::Eq
                        | BinaryOpKind::NotEq
                        | BinaryOpKind::Lt
                        | BinaryOpKind::LtEq
                        | BinaryOpKind::Gt
                        | BinaryOpKind::GtEq
                ) {
                    self.obligations_total += 1;
                    if left.is_literal_null() || right.is_literal_null() {
                        self.findings.push(Finding {
                            file: self.file_path.clone(),
                            line: *line,
                            class: "sql-null-comparison".to_string(),
                            verdict: "proved-defect".to_string(),
                            message: format!(
                                "Comparison with literal NULL using operator '{:?} NULL'; ANSI SQL three-valued logic requires 'IS NULL' or 'IS NOT NULL'",
                                op
                            ),
                        });
                    } else {
                        self.obligations_proved_safe += 1;
                    }
                }

                self.check_expr(left, ctx);
                self.check_expr(right, ctx);
            }
            Expr::UnaryOp { expr: inner, .. } => {
                self.check_expr(inner, ctx);
            }
            Expr::FunctionCall { args, .. } => {
                for arg in args {
                    self.check_expr(arg, ctx);
                }
            }
            Expr::Case {
                operand,
                when_clauses,
                else_clause,
                ..
            } => {
                if let Some(ref op) = operand {
                    self.check_expr(op, ctx);
                }
                for (cond, result) in when_clauses {
                    self.check_expr(cond, ctx);
                    let mut then_ctx = ctx.clone();
                    extract_guarded_vars(cond, &mut then_ctx.guarded_vars);
                    self.check_expr(result, &then_ctx);
                }
                if let Some(ref el) = else_clause {
                    let mut else_ctx = ctx.clone();
                    for (cond, _) in when_clauses {
                        extract_else_guarded_vars(cond, &mut else_ctx.guarded_vars);
                    }
                    self.check_expr(el, &else_ctx);
                }
            }
            Expr::IsNull { expr: inner, .. } => {
                self.check_expr(inner, ctx);
            }
            Expr::InList { expr: inner, list, .. } => {
                self.check_expr(inner, ctx);
                for item in list {
                    self.check_expr(item, ctx);
                }
            }
            Expr::InSubquery {
                expr: inner,
                subquery,
                ..
            } => {
                self.check_expr(inner, ctx);
                self.verify_select(subquery);
            }
            Expr::Between {
                expr: inner,
                low,
                high,
                ..
            } => {
                self.check_expr(inner, ctx);
                self.check_expr(low, ctx);
                self.check_expr(high, ctx);
            }
            Expr::Subquery(sub) => {
                self.verify_select(sub);
            }
            Expr::List(list) => {
                for item in list {
                    self.check_expr(item, ctx);
                }
            }
            Expr::Raw(_) => {}
        }
    }
}

/// Extract variables that are guarded against zero in a positive condition (`col <> 0`, `col != 0`, `col > 0`).
fn extract_guarded_vars(expr: &Expr, vars: &mut HashSet<String>) {
    match expr {
        Expr::BinaryOp {
            left, op, right, ..
        } => {
            if *op == BinaryOpKind::And {
                extract_guarded_vars(left, vars);
                extract_guarded_vars(right, vars);
            } else if *op == BinaryOpKind::NotEq {
                if right.is_literal_zero() {
                    if let Expr::ColumnRef { column, .. } = &**left {
                        vars.insert(normalize_name(column));
                    }
                } else if left.is_literal_zero() {
                    if let Expr::ColumnRef { column, .. } = &**right {
                        vars.insert(normalize_name(column));
                    }
                }
            } else if *op == BinaryOpKind::Gt {
                if right.is_literal_zero() {
                    if let Expr::ColumnRef { column, .. } = &**left {
                        vars.insert(normalize_name(column));
                    }
                }
            }
        }
        _ => {}
    }
}

/// Extract variables that are guarded in the ELSE branch of `CASE WHEN col = 0 THEN ... ELSE ... END`.
fn extract_else_guarded_vars(expr: &Expr, vars: &mut HashSet<String>) {
    if let Expr::BinaryOp {
        left, op, right, ..
    } = expr
    {
        if *op == BinaryOpKind::Eq {
            if right.is_literal_zero() {
                if let Expr::ColumnRef { column, .. } = &**left {
                    vars.insert(normalize_name(column));
                }
            } else if left.is_literal_zero() {
                if let Expr::ColumnRef { column, .. } = &**right {
                    vars.insert(normalize_name(column));
                }
            }
        }
    }
}

fn is_query_recursive(query: &SelectStmt, cte_name: &str) -> bool {
    if query_references_table(query, cte_name) {
        return true;
    }
    if let Some(ref u_all) = query.union_all {
        if query_references_table(u_all, cte_name) {
            return true;
        }
    }
    if let Some(ref u) = query.union {
        if query_references_table(u, cte_name) {
            return true;
        }
    }
    false
}

fn query_references_table(query: &SelectStmt, target_table: &str) -> bool {
    let target = normalize_name(target_table);
    for t in &query.from {
        if normalize_name(&t.name) == target {
            return true;
        }
    }
    for j in &query.joins {
        if normalize_name(&j.table.name) == target {
            return true;
        }
    }
    false
}

fn is_recursive_leg_bounded(query: &SelectStmt, cte_name: &str, outer_bounded: bool) -> bool {
    if outer_bounded || query.limit.is_some() {
        return true;
    }

    let target = normalize_name(cte_name);

    // Look in UNION ALL recursive leg
    if let Some(ref rec_query) = query.union_all {
        if query_references_table(rec_query, &target) {
            return rec_query.where_clause.is_some() || rec_query.limit.is_some();
        }
    }
    if let Some(ref rec_query) = query.union {
        if query_references_table(rec_query, &target) {
            return rec_query.where_clause.is_some() || rec_query.limit.is_some();
        }
    }

    if query_references_table(query, &target) {
        return query.where_clause.is_some() || query.limit.is_some();
    }

    true
}
