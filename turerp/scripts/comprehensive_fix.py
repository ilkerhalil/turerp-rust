#!/usr/bin/env python3
"""Comprehensive fix for company_id across all touched files."""

import re
from pathlib import Path

ROOT = Path("/home/ilker/projects/turerp-rust/turerp/src/domain")

def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")

def write_text(path: Path, text: str):
    path.write_text(text, encoding="utf-8")

def remove_duplicate_company_id_in_struct_literal(text: str) -> str:
    """Remove duplicate company_id: x.company_id, lines in struct literals."""
    # Match: tenant_id: row.tenant_id,\n            company_id: row.company_id,\n            company_id: row.company_id,
    text = re.sub(
        r'(tenant_id: [\w.]+,)(\s+company_id: [\w.]+,)(\s+company_id: [\w.]+,)',
        r'\1\2',
        text
    )
    return text

def add_company_id_to_from_row(text: str, entity_name: str) -> str:
    """Add company_id: row.company_id after tenant_id: row.tenant_id in From<Row>."""
    # Find From<XxxRow> for Xxx pattern and add company_id after tenant_id
    pattern = rf'(impl From<(\w+)Row> for (\w+) \{{\n    fn from\(row: \2Row\) -> Self \{{\n        Self \{{[\s\S]*?)(tenant_id: row\.tenant_id,)(?!\s*company_id:)'
    def repl(m):
        return m.group(1) + m.group(4) + '\n            company_id: row.company_id,'
    text = re.sub(pattern, repl, text)
    return text

def add_company_id_to_from_entity(text: str) -> str:
    """Add company_id after tenant_id in From<Entity> for Response."""
    # Match tenant_id: x.tenant_id, not followed by company_id
    text = re.sub(
        r'(tenant_id: [\w.]+,)(?!\s*company_id:)',
        r'\1\n            company_id: \2.company_id,',
        text
    )
    return text

def add_company_id_to_in_memory_repo(text: str, entity_name: str) -> str:
    """Add company_id to in-memory repository struct literals."""
    # Match patterns like tenant_id: create.tenant_id, not followed by company_id
    text = re.sub(
        r'(tenant_id: create\.tenant_id,)(?!\s*company_id:)',
        r'\1\n            company_id: create.company_id,',
        text
    )
    return text

def fix_model_rs(path: Path):
    text = read_text(path)
    original = text

    # Remove duplicate company_id in struct literals
    text = remove_duplicate_company_id_in_struct_literal(text)

    # Add company_id to From impls if missing
    text = re.sub(
        r'(tenant_id: (\w+)\.tenant_id,)(?!\s*company_id:)',
        r'\1\n            company_id: \2.company_id,',
        text
    )

    if text != original:
        write_text(path, text)
        print(f"FIXED model {path.name}")

def fix_repository_rs(path: Path):
    text = read_text(path)
    original = text

    # Remove duplicate company_id in struct literals
    text = remove_duplicate_company_id_in_struct_literal(text)

    # Add company_id to From impls if missing
    text = re.sub(
        r'(tenant_id: row\.tenant_id,)(?!\s*company_id:)',
        r'\1\n            company_id: row.company_id,',
        text
    )

    # Add company_id to in-memory repo constructors
    text = re.sub(
        r'(tenant_id: create\.tenant_id,)(?!\s*company_id:)',
        r'\1\n            company_id: create.company_id,',
        text
    )

    if text != original:
        write_text(path, text)
        print(f"FIXED repo {path.name}")

def fix_postgres_rs(path: Path):
    text = read_text(path)
    original = text

    # Remove duplicate company_id in struct literals
    text = remove_duplicate_company_id_in_struct_literal(text)

    # Add company_id to From impls if missing
    text = re.sub(
        r'(tenant_id: row\.tenant_id,)(?!\s*company_id:)',
        r'\1\n            company_id: row.company_id,',
        text
    )

    if text != original:
        write_text(path, text)
        print(f"FIXED pg {path.name}")

def main():
    for domain_dir in ROOT.iterdir():
        if not domain_dir.is_dir():
            continue
        model = domain_dir / "model.rs"
        repo = domain_dir / "repository.rs"
        pg = domain_dir / "postgres_repository.rs"

        if model.exists():
            fix_model_rs(model)
        if repo.exists():
            fix_repository_rs(repo)
        if pg.exists():
            fix_postgres_rs(pg)

if __name__ == "__main__":
    main()
