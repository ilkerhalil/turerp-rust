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

/// Collect the on-disk up-migration stems (`migrations/*.sql` minus the
/// `.sql` suffix). The `down/` subdirectory is skipped (it holds
/// `*.down.sql` files, not up migrations).
fn disk_up_migration_stems() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Ok(entries) = fs::read_dir("migrations") else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }
        let Some(fname) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Some(stem) = fname.strip_suffix(".sql") {
            out.insert(stem.to_string());
        }
    }
    out
}

/// Collect the on-disk down-migration stems (`migrations/down/*.down.sql`
/// minus the `.down.sql` suffix).
fn disk_down_migration_stems() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Ok(entries) = fs::read_dir("migrations/down") else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }
        let Some(fname) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Some(stem) = fname.strip_suffix(".down.sql") {
            out.insert(stem.to_string());
        }
    }
    out
}

/// Extract the `version: "..."` strings from a named `const` migration
/// array in `src/db/pool.rs` (e.g. `MIGRATIONS` or `DOWN_MIGRATIONS`).
/// The array is `const NAME: &[Migration] = &[ ... ];` — we scan from the
/// `&[` after the name to the first `];` and pull every `version: "..."`
/// line. Version strings are filename stems, so they must match the
/// on-disk file stems exactly.
fn pool_migration_versions(array_name: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Ok(content) = fs::read_to_string("src/db/pool.rs") else {
        return out;
    };
    let needle = format!("const {array_name}: &[Migration] = &[");
    let Some(start) = content.find(&needle) else {
        return out;
    };
    let body_start = start + needle.len();
    let Some(end) = content[body_start..].find("];") else {
        return out;
    };
    let body = &content[body_start..body_start + end];
    for line in body.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("version:") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('"') else {
            continue;
        };
        if let Some(end_q) = rest.find('"') {
            out.insert(rest[..end_q].to_string());
        }
    }
    out
}

/// Regression guard for the bug fixed in PR #206 / issue #205: the app's
/// custom migration runner (`src/db/pool.rs` `MIGRATIONS` array, applied at
/// startup) is the SOLE migrator in production (Dockerfile = app binary,
/// no external `sqlx migrate`). A migration file on disk that is NOT
/// registered in the array is silently never applied in production — which
/// is how 047-058 (the entire #162 cross-tenant leak-audit) ended up absent
/// in prod. CI's Test job has no database, so the gap never surfaced there.
/// This test asserts the array ≡ the disk set so the next unregistered
/// migration fails CI instead of prod.
#[test]
fn pool_migrations_array_matches_disk_up() {
    let disk = disk_up_migration_stems();
    let array = pool_migration_versions("MIGRATIONS");
    let not_registered: Vec<_> = disk.difference(&array).collect();
    let dangling: Vec<_> = array.difference(&disk).collect();
    assert!(
        not_registered.is_empty() && dangling.is_empty(),
        "src/db/pool.rs `MIGRATIONS` array is out of sync with `migrations/*.sql`.\n\
         On disk but NOT in the array (would NOT be applied in production — the \
         app is the sole migrator): {:?}\n\
         In the array but NO file on disk (dangling include_str!): {:?}\n\
         Every new migration file MUST be registered in `src/db/pool.rs` \
         `MIGRATIONS` (ascending) AND `DOWN_MIGRATIONS` (descending).",
        not_registered,
        dangling,
    );
}

/// Mirror of the up guard for the `DOWN_MIGRATIONS` array vs
/// `migrations/down/*.down.sql`. A down file on disk but unregistered in
/// the array would be skipped by `run_migrations_down`; an array entry
/// with no file is a dangling include_str! (compile-caught, but flagged
/// here too for a single-point parity report).
#[test]
fn pool_migrations_array_matches_disk_down() {
    let disk = disk_down_migration_stems();
    let array = pool_migration_versions("DOWN_MIGRATIONS");
    let not_registered: Vec<_> = disk.difference(&array).collect();
    let dangling: Vec<_> = array.difference(&disk).collect();
    assert!(
        not_registered.is_empty() && dangling.is_empty(),
        "src/db/pool.rs `DOWN_MIGRATIONS` array is out of sync with \
         `migrations/down/*.down.sql`.\n\
         On disk but NOT in the array: {:?}\n\
         In the array but NO file on disk: {:?}\n\
         Every new down-migration MUST be registered in `DOWN_MIGRATIONS` \
         (descending) AND its up counterpart in `MIGRATIONS` (ascending).",
        not_registered,
        dangling,
    );
}
