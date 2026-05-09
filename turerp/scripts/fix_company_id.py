#!/usr/bin/env python3
"""Fix duplicate and malformed company_id insertions caused by the add script."""

from pathlib import Path
import re

ROOT = Path("/home/ilker/projects/turerp-rust/turerp/src/domain")

def fix_file(path: Path):
    text = path.read_text(encoding="utf-8")
    original = text

    # 1. Remove duplicate company_id field blocks in structs
    # Pattern: #[sqlx(default = "default_company_id")]\n    company_id: i64,\n    #[sqlx(default = "default_company_id")]\n    company_id: i64,
    text = re.sub(
        r'    #\[sqlx\(default = "default_company_id"\)\]\n    company_id: i64,\n    #\[sqlx\(default = "default_company_id"\)\]\n    company_id: i64,',
        '    #[sqlx(default = "default_company_id")]\n    company_id: i64,',
        text
    )

    # 2. Fix From impls: "tenant_id: row.tenant_id, company_id," -> "tenant_id: row.tenant_id,"
    # Also handle variations like "tenant_id: order.tenant_id, company_id," etc.
    text = re.sub(
        r'(tenant_id: \w+\.tenant_id,),?\s*company_id,',
        r'\1',
        text
    )

    # 3. Remove duplicate company_id, in SQL SELECT / RETURNING / column lists
    # Be careful not to affect "company_id = " in UPDATE SET
    # Replace "company_id, company_id," with "company_id,"
    while "company_id, company_id," in text:
        text = text.replace("company_id, company_id,", "company_id,")

    # 4. Also remove duplicate RETURNING / SELECT where there might be "company_id, company_id"
    # Already handled by the while loop above

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
