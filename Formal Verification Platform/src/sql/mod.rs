#![allow(dead_code, unused_variables, unused_imports)]

//! SQL frontend engine for Keller-VIR formal verification.
//!
//! Provides static formal verification and obligation tracking for 5 SQL defect classes:
//! 1. `sql-division-by-zero`: Division `/` in select/expressions where divisor column or value can evaluate to 0 without `NULLIF(col, 0)` or `CASE WHEN col <> 0` guard.
//! 2. `sql-not-null-violation`: `INSERT` statements supplying `NULL` (or omitting columns) for columns declared `NOT NULL` without `DEFAULT`.
//! 3. `sql-recursive-cte-unbounded`: Recursive Common Table Expressions (`WITH RECURSIVE`) without termination condition or depth limit in the recursive leg.
//! 4. `sql-null-comparison`: Using `= NULL` or `<> NULL` instead of `IS NULL` / `IS NOT NULL`.
//! 5. `sql-unguarded-update-delete`: `UPDATE` or `DELETE` statements without a `WHERE` clause.

pub mod ast;
pub mod parser;
pub mod token;
pub mod verifier;

use crate::langaudit::LangAudit;
use std::fs;
use std::path::Path;

/// Audit a path (.sql file or directory containing .sql files) in audit-only mode.
pub fn audit_path(path: &Path) -> Result<LangAudit, String> {
    let mut audit = LangAudit::new("sql", path);
    let files = crate::langaudit::collect_files(path, &["sql"]);
    audit.files_scanned = files.len();

    // Pass 1: Global Schema Discovery
    // Collect all table declarations and alterations to build schema context
    let mut schema = verifier::SchemaContext::new();
    let mut file_sources: Vec<(String, String)> = Vec::new();

    for file in &files {
        let file_str = file.display().to_string();
        let bytes = match fs::read(file) {
            Ok(b) => b,
            Err(e) => {
                audit.files_unparsed += 1;
                audit.notes.push(format!("{}: io error: {e}", file_str));
                continue;
            }
        };

        let src = String::from_utf8_lossy(&bytes).to_string();

        // Populate table schemas
        let mut p = parser::Parser::from_src(&src);
        while let Some(stmt) = p.next_statement() {
            match stmt {
                ast::Statement::CreateTable(create) => {
                    schema.add_table(&file_str, create);
                }
                ast::Statement::AlterTable(ref alter) => {
                    schema.alter_table(&file_str, alter);
                }
                _ => {}
            }
        }

        file_sources.push((file_str, src));
    }

    // Pass 2: Formal Verification Pass
    for (file_str, src) in file_sources {
        let mut verifier = verifier::Verifier::new(&file_str, &schema);
        let mut p = parser::Parser::from_src(&src);

        while let Some(stmt) = p.next_statement() {
            verifier.verify_statement(&stmt);
        }

        audit.functions_lowered += verifier.functions_lowered;
        audit.obligations_total += verifier.obligations_total;
        audit.obligations_proved_safe += verifier.obligations_proved_safe;
        audit.findings.extend(verifier.findings);
    }

    Ok(audit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verify_sql(src: &str, schema: &verifier::SchemaContext) -> verifier::Verifier<'static> {
        let leaked_schema: &'static verifier::SchemaContext = Box::leak(Box::new(schema.clone()));
        let mut verifier = verifier::Verifier::new("test.sql", leaked_schema);
        let mut p = parser::Parser::from_src(src);
        while let Some(stmt) = p.next_statement() {
            verifier.verify_statement(&stmt);
        }
        verifier
    }

    // ─── 1. sql-division-by-zero ─────────────────────────────────────────────

    #[test]
    fn test_division_by_zero_literal_zero_vulnerable() {
        let src = "SELECT a / 0 FROM items;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-division-by-zero").collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].verdict, "proved-defect");
    }

    #[test]
    fn test_division_by_zero_column_vulnerable() {
        let src = "SELECT a / b FROM items;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-division-by-zero").collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].verdict, "possible-defect");
    }

    #[test]
    fn test_division_by_zero_nullif_safe() {
        let src = "SELECT a / NULLIF(b, 0) FROM items;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-division-by-zero").collect();
        assert!(f.is_empty(), "expected NULLIF(b, 0) to be safe");
        assert!(v.obligations_proved_safe >= 1);
    }

    #[test]
    fn test_division_by_zero_case_safe() {
        let src = "SELECT CASE WHEN b <> 0 THEN a / b ELSE NULL END FROM items;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-division-by-zero").collect();
        assert!(f.is_empty(), "expected CASE WHEN b <> 0 to guard division");
        assert!(v.obligations_proved_safe >= 1);
    }

    #[test]
    fn test_division_by_zero_where_safe() {
        let src = "SELECT a / b FROM items WHERE b <> 0;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-division-by-zero").collect();
        assert!(f.is_empty(), "expected WHERE b <> 0 to guard division");
        assert!(v.obligations_proved_safe >= 1);
    }

    #[test]
    fn test_division_by_zero_constant_safe() {
        let src = "SELECT a / 100 FROM items;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-division-by-zero").collect();
        assert!(f.is_empty(), "expected non-zero constant divisor to be safe");
        assert!(v.obligations_proved_safe >= 1);
    }

    // ─── 2. sql-not-null-violation ───────────────────────────────────────────

    #[test]
    fn test_not_null_explicit_null_vulnerable() {
        let schema_src = "CREATE TABLE users (id INT NOT NULL, name VARCHAR(50) NOT NULL, bio TEXT);";
        let mut schema = verifier::SchemaContext::new();
        let mut p = parser::Parser::from_src(schema_src);
        if let Some(ast::Statement::CreateTable(c)) = p.next_statement() {
            schema.add_table("test.sql", c);
        }

        let insert_src = "INSERT INTO users (id, name, bio) VALUES (1, NULL, 'Developer');";
        let v = verify_sql(insert_src, &schema);
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-not-null-violation").collect();
        assert!(!f.is_empty(), "expected NOT NULL violation on explicit NULL");
        assert_eq!(f[0].verdict, "proved-defect");
    }

    #[test]
    fn test_not_null_omitted_column_vulnerable() {
        let schema_src = "CREATE TABLE users (id INT NOT NULL, name VARCHAR(50) NOT NULL, bio TEXT);";
        let mut schema = verifier::SchemaContext::new();
        let mut p = parser::Parser::from_src(schema_src);
        if let Some(ast::Statement::CreateTable(c)) = p.next_statement() {
            schema.add_table("test.sql", c);
        }

        let insert_src = "INSERT INTO users (id, bio) VALUES (1, 'Developer');";
        let v = verify_sql(insert_src, &schema);
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-not-null-violation").collect();
        assert!(!f.is_empty(), "expected NOT NULL violation on omitted non-nullable column");
        assert_eq!(f[0].verdict, "proved-defect");
    }

    #[test]
    fn test_not_null_positional_null_vulnerable() {
        let schema_src = "CREATE TABLE users (id INT NOT NULL, name VARCHAR(50) NOT NULL, bio TEXT);";
        let mut schema = verifier::SchemaContext::new();
        let mut p = parser::Parser::from_src(schema_src);
        if let Some(ast::Statement::CreateTable(c)) = p.next_statement() {
            schema.add_table("test.sql", c);
        }

        let insert_src = "INSERT INTO users VALUES (1, NULL, 'Developer');";
        let v = verify_sql(insert_src, &schema);
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-not-null-violation").collect();
        assert!(!f.is_empty(), "expected NOT NULL violation on positional NULL");
    }

    #[test]
    fn test_not_null_valid_insert_safe() {
        let schema_src = "CREATE TABLE users (id INT NOT NULL, name VARCHAR(50) NOT NULL, bio TEXT);";
        let mut schema = verifier::SchemaContext::new();
        let mut p = parser::Parser::from_src(schema_src);
        if let Some(ast::Statement::CreateTable(c)) = p.next_statement() {
            schema.add_table("test.sql", c);
        }

        let insert_src = "INSERT INTO users (id, name, bio) VALUES (1, 'Alice', 'Engineer');";
        let v = verify_sql(insert_src, &schema);
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-not-null-violation").collect();
        assert!(f.is_empty(), "expected valid insert to be safe");
        assert!(v.obligations_proved_safe >= 1);
    }

    #[test]
    fn test_not_null_default_column_omitted_safe() {
        let schema_src = "CREATE TABLE items (id INT NOT NULL, status VARCHAR(20) NOT NULL DEFAULT 'active');";
        let mut schema = verifier::SchemaContext::new();
        let mut p = parser::Parser::from_src(schema_src);
        if let Some(ast::Statement::CreateTable(c)) = p.next_statement() {
            schema.add_table("test.sql", c);
        }

        let insert_src = "INSERT INTO items (id) VALUES (1);";
        let v = verify_sql(insert_src, &schema);
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-not-null-violation").collect();
        assert!(f.is_empty(), "expected omitting NOT NULL column with DEFAULT to be safe");
        assert!(v.obligations_proved_safe >= 1);
    }

    // ─── 3. sql-recursive-cte-unbounded ──────────────────────────────────────

    #[test]
    fn test_recursive_cte_unbounded_vulnerable() {
        let src = r#"
        WITH RECURSIVE cnt(x) AS (
            SELECT 1
            UNION ALL
            SELECT x + 1 FROM cnt
        )
        SELECT * FROM cnt;
        "#;
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-recursive-cte-unbounded").collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].verdict, "proved-defect");
    }

    #[test]
    fn test_recursive_cte_bounded_where_safe() {
        let src = r#"
        WITH RECURSIVE cnt(x) AS (
            SELECT 1
            UNION ALL
            SELECT x + 1 FROM cnt WHERE x < 100
        )
        SELECT * FROM cnt;
        "#;
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-recursive-cte-unbounded").collect();
        assert!(f.is_empty(), "expected recursive CTE bounded by WHERE to be safe");
        assert!(v.obligations_proved_safe >= 1);
    }

    #[test]
    fn test_recursive_cte_bounded_limit_safe() {
        let src = r#"
        WITH RECURSIVE cnt(x) AS (
            SELECT 1
            UNION ALL
            SELECT x + 1 FROM cnt
        )
        SELECT * FROM cnt LIMIT 50;
        "#;
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-recursive-cte-unbounded").collect();
        assert!(f.is_empty(), "expected recursive CTE bounded by query LIMIT to be safe");
        assert!(v.obligations_proved_safe >= 1);
    }

    // ─── 4. sql-null-comparison ──────────────────────────────────────────────

    #[test]
    fn test_null_comparison_eq_vulnerable() {
        let src = "SELECT * FROM users WHERE status = NULL;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-null-comparison").collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].verdict, "proved-defect");
    }

    #[test]
    fn test_null_comparison_neq_vulnerable() {
        let src = "SELECT * FROM users WHERE status <> NULL;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-null-comparison").collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].verdict, "proved-defect");
    }

    #[test]
    fn test_null_comparison_safe_is_null() {
        let src = "SELECT * FROM users WHERE status IS NULL;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-null-comparison").collect();
        assert!(f.is_empty(), "expected IS NULL to be safe");
    }

    #[test]
    fn test_null_comparison_safe_is_not_null() {
        let src = "SELECT * FROM users WHERE status IS NOT NULL;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-null-comparison").collect();
        assert!(f.is_empty(), "expected IS NOT NULL to be safe");
    }

    // ─── 5. sql-unguarded-update-delete ──────────────────────────────────────

    #[test]
    fn test_unguarded_update_vulnerable() {
        let src = "UPDATE store SET manager_staff_id = 1;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-unguarded-update-delete").collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].verdict, "proved-defect");
    }

    #[test]
    fn test_guarded_update_safe() {
        let src = "UPDATE store SET manager_staff_id = 1 WHERE store_id = 10;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-unguarded-update-delete").collect();
        assert!(f.is_empty(), "expected UPDATE with WHERE to be safe");
        assert!(v.obligations_proved_safe >= 1);
    }

    #[test]
    fn test_postgres_sakila_schema() {
        let path = Path::new("benchmarks/repos/sql/sakila/postgres-sakila-db/postgres-sakila-schema.sql");
        if !path.exists() { return; }
        let src = fs::read_to_string(path).unwrap();
        let mut p = parser::Parser::from_src(&src);
        let mut count = 0;
        let mut has_payment = false;
        while let Some(stmt) = p.next_statement() {
            if let ast::Statement::CreateTable(c) = stmt {
                count += 1;
                if c.name.eq_ignore_ascii_case("payment") {
                    has_payment = true;
                }
            }
        }
        assert!(count >= 15);
        assert!(has_payment, "expected payment table to be found");
    }

    #[test]
    fn test_unguarded_delete_vulnerable() {
        let src = "DELETE FROM payment;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-unguarded-update-delete").collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].verdict, "proved-defect");
    }

    #[test]
    fn test_guarded_delete_safe() {
        let src = "DELETE FROM payment WHERE payment_id = 42;";
        let v = verify_sql(src, &verifier::SchemaContext::new());
        let f: Vec<_> = v.findings.iter().filter(|x| x.class == "sql-unguarded-update-delete").collect();
        assert!(f.is_empty(), "expected DELETE with WHERE to be safe");
        assert!(v.obligations_proved_safe >= 1);
    }
}
