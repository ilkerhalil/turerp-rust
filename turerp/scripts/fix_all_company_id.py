#!/usr/bin/env python3
"""Fix duplicate and malformed company_id insertions across all postgres repos."""

from pathlib import Path
import re

ROOT = Path("/home/ilker/projects/turerp-rust/turerp/src/domain")

def fix_file(path: Path):
    text = path.read_text(encoding="utf-8")
    original = text

    # 1. Remove duplicate #[sqlx(default = "default_company_id")]\n    company_id: i64, blocks
    dup_block = '    #[sqlx(default = "default_company_id")]\n    company_id: i64,\n    #[sqlx(default = "default_company_id")]\n    company_id: i64,'
    single_block = '    #[sqlx(default = "default_company_id")]\n    company_id: i64,'
    while dup_block in text:
        text = text.replace(dup_block, single_block)

    # 2. Fix From impls: "tenant_id: row.tenant_id, company_id," followed by "company_id: row.company_id,"
    text = re.sub(
        r'(tenant_id: \w+\.tenant_id),?\s*company_id,\n(\s+company_id: \w+\.company_id,)',
        r'\1,\n\2',
        text
    )

    # 3. Remove duplicate company_id, in SQL column lists
    while "company_id, company_id," in text:
        text = text.replace("company_id, company_id,", "company_id,")

    # 4. Replace multiline SQL duplicates
    text = re.sub(
        r'company_id,\s+company_id,',
        'company_id,',
        text
    )

    if text != original:
        path.write_text(text, encoding="utf-8")
        print(f"Fixed {path}")
    else:
        print(f"OK    {path}")

def main():
    for domain_dir in ROOT.iterdir():
        if not domain_dir.is_dir():
            continue
        pg_repo = domain_dir / "postgres_repository.rs"
        if pg_repo.exists():
            fix_file(pg_repo)

if __name__ == "__main__":
    main()
