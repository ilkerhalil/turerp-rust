#!/usr/bin/env python3
"""Remove badly-indented company_id method parameters from postgres repos."""

import re
from pathlib import Path

ROOT = Path("/home/ilker/projects/turerp-rust/turerp/src/domain")

def fix_file(path: Path):
    text = path.read_text(encoding="utf-8")
    original = text

    # Pattern: inside fn signature, after tenant_id: i64, with wrong indent (4 spaces)
    # Match: tenant_id: i64,\n    company_id: i64,\n        param...
    text = re.sub(
        r'(tenant_id: i64,)\n    company_id: i64,\n(        [^\n]+,)',
        r'\1\n\2',
        text
    )

    # Also handle case where tenant_id is not the last param before company_id
    # More general: inside async fn ...(&self,\n... ) ->, find badly indented company_id
    # Remove lines that are exactly "    company_id: i64," in fn signatures
    # But we need to be careful not to remove struct fields.
    # Struct fields appear inside struct NAME { ... }, fn params inside fn NAME(...).
    # We can identify fn signatures and remove from them.

    # Let's use a more targeted approach: find all async fn ... blocks and clean them
    def clean_fn(m):
        fn_sig = m.group(1)
        # Remove "    company_id: i64," from fn signature
        cleaned = re.sub(r'\n    company_id: i64,', '', fn_sig)
        return 'async fn' + cleaned

    text = re.sub(r'async fn([\s\S]*?\n    \) ->)', clean_fn, text)

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
