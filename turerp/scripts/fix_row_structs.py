#!/usr/bin/env python3
"""Add company_id to Row structs in postgres repos for target domains."""

import re
from pathlib import Path

ROOT = Path("/home/ilker/projects/turerp-rust/turerp/src/domain")

TARGET_DOMAINS = [
    "cari", "product", "stock", "invoice", "sales", "purchase",
    "accounting", "assets", "hr"
]

def fix_file(path: Path):
    text = path.read_text(encoding="utf-8")
    original = text

    # Pattern: #[derive(Debug, FromRow)]\nstruct XxxRow { ... tenant_id: i64,\n ... }
    # Add company_id after tenant_id if missing
    pattern = r'(#[derive\(Debug, FromRow\)\]\nstruct (\w+)Row(?:WithTotal)? \{[\s\S]*?tenant_id: i64,\n)(?!\s*company_id:)'

    def repl(m):
        return m.group(1) + '    company_id: i64,\n'

    text = re.sub(pattern, repl, text)

    if text != original:
        path.write_text(text, encoding="utf-8")
        print(f"Fixed {path}")
    else:
        print(f"OK    {path}")

def main():
    for domain in TARGET_DOMAINS:
        path = ROOT / domain / "postgres_repository.rs"
        if path.exists():
            fix_file(path)

if __name__ == "__main__":
    main()
