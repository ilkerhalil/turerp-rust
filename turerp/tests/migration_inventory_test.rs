//! Cross-check: every table referenced by a Postgres repository has a
//! `CREATE TABLE` in some migration. This is a regression guard added in
//! the production-readiness hardening PR to prevent the kind of drift
//! where a repository is wired into `AppState` but the underlying table
//! was never migrated.
//!
//! Implementation: scan `turerp/src/domain/*/postgres_repository.rs` and
//! `turerp/migrations/*.sql`, extracting identifiers that appear after
//! `FROM`, `JOIN`, `INTO`, `UPDATE` keywords in SQL string literals.
//! Column names like `document_id`, `project_id`, `deleted_at` are
//! filtered out because they don't appear as the *first* identifier
//! after a SQL keyword (they appear in column lists or WHERE clauses).

use std::collections::BTreeSet;
use std::fs;

/// Read all `*.sql` files under `migrations/` and collect every name
/// that appears after `CREATE TABLE` (with or without `IF NOT EXISTS`).
fn collect_migrated_tables() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Ok(entries) = fs::read_dir("migrations") else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for line in content.lines() {
            // Skip pure SQL comments and blank lines
            let trimmed = line.trim_start();
            if trimmed.starts_with("--") || trimmed.is_empty() {
                continue;
            }
            // Find "CREATE TABLE" anywhere in the line and read the next identifier.
            // SQL allows whitespace/comments between tokens, so tokenize the
            // remainder and skip the optional IF NOT EXISTS clause.
            let upper = line.to_uppercase();
            let Some(idx) = upper.find("CREATE TABLE") else {
                continue;
            };
            let after = &trimmed[idx + "CREATE TABLE".len()..];
            let tokens = after.split_whitespace();
            // Skip optional `IF`, `NOT`, `EXISTS`
            let mut first_real: Option<&str> = None;
            for tok in tokens {
                let up = tok.to_uppercase();
                if up == "IF" || up == "NOT" || up == "EXISTS" {
                    continue;
                }
                first_real = Some(tok);
                break;
            }
            if let Some(name) = first_real {
                // Strip trailing punctuation like `(`
                let name = name.trim_end_matches('(');
                let name: String = name
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    out.insert(name.to_lowercase());
                }
            }
        }
    }
    out
}

/// Read all `postgres_repository.rs` files under `src/domain/` and
/// collect every name that appears as a *table reference* in a SQL
/// string literal. A "table reference" is the first identifier after
/// `FROM`, `JOIN`, `INTO`, or `UPDATE` in a line that consists primarily
/// of SQL (no Rust code like `let` or `match`).
///
/// This is a heuristic parser; it is deliberately conservative and
/// only matches patterns that we know appear in our codebase.
fn collect_referenced_tables() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Ok(entries) = fs::read_dir("src/domain") else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path().join("postgres_repository.rs");
        if !path.exists() {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for line in content.lines() {
            // Skip lines that contain Rust code: variable bindings, type
            // annotations, function calls, comments, etc. The signal we
            // use is the presence of `//`, `let `, `fn `, `match `, or
            // `=>`. SQL fragments are usually indented to match the
            // surrounding string literal and contain only SQL keywords
            // + identifiers.
            if line.contains("//")
                || line.contains(" let ")
                || line.contains(" fn ")
                || line.contains(" match ")
                || line.contains("=>")
                || line.contains("Self")
            {
                continue;
            }
            let trimmed = line.trim_start();
            for keyword in ["FROM ", "JOIN ", "INTO ", "UPDATE "] {
                if let Some(rest) = trimmed.strip_prefix(keyword) {
                    let name: String = rest
                        .chars()
                        .skip_while(|c| *c == '(' || c.is_ascii_whitespace())
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if name.is_empty() {
                        continue;
                    }
                    // Reject obvious SQL keywords and Rust types
                    let upper = name.to_uppercase();
                    if matches!(
                        upper.as_str(),
                        "SELECT"
                            | "WHERE"
                            | "ON"
                            | "AS"
                            | "AND"
                            | "OR"
                            | "SET"
                            | "VALUES"
                            | "RETURNING"
                            | "DELETE"
                            | "EXISTS"
                            | "IF"
                            | "NOT"
                            | "NULL"
                            | "TRUE"
                            | "FALSE"
                            | "DEFAULT"
                            | "PRIMARY"
                            | "FOREIGN"
                            | "KEY"
                            | "REFERENCES"
                            | "CASCADE"
                            | "ONLY"
                            | "ALL"
                            | "DISTINCT"
                            | "CASE"
                            | "WHEN"
                            | "THEN"
                            | "ELSE"
                            | "END"
                            | "COUNT"
                            | "SUM"
                            | "AVG"
                            | "MIN"
                            | "MAX"
                            | "NOW"
                    ) {
                        continue;
                    }
                    // Table names in our migrations are snake_case and
                    // at least 3 characters; this filters out short
                    // column-name fragments.
                    if name.len() < 3 || name.len() > 40 {
                        continue;
                    }
                    out.insert(name.to_lowercase());
                }
            }
        }
    }
    out
}

fn referenced_allowlist() -> BTreeSet<String> {
    // Common Table Expression (CTE) names that look like table references
    // but are scoped to the WITH clause of a single query.
    ["cash_in", "cash_out", "filtered", "ranked", "totals"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn all_postgres_tables_have_migrations() {
    let referenced = collect_referenced_tables();
    let migrated = collect_migrated_tables();
    let allow = referenced_allowlist();

    let missing: Vec<&String> = referenced
        .iter()
        .filter(|n| !migrated.contains(*n) && !allow.contains(*n))
        .collect();

    assert!(
        missing.is_empty(),
        "Postgres repositories reference {} table(s) with no `CREATE TABLE` \
         migration: {:?}\n\nThis drift causes production crashes with \
         `relation \"...\" does not exist`.\n\nIf a new table is intentional, \
         add it to the next migration (e.g., 036_*.sql) AND register it in \
         `turerp/src/db/pool.rs` MIGRATIONS list.",
        missing.len(),
        missing,
    );
}
