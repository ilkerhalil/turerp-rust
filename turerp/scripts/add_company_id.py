#!/usr/bin/env python3
"""Mechanically add company_id to domain models, repositories, and postgres repos."""

import re
from pathlib import Path

ROOT = Path("/home/ilker/projects/turerp-rust/turerp/src/domain")

def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")

def write_text(path: Path, text: str):
    path.write_text(text, encoding="utf-8")

# Domain -> main entity struct names
domains = {
    "cari": {"entity": "Cari", "create": "CreateCari", "update": "UpdateCari", "response": "CariResponse"},
    "product": {"entity": "Product", "create": "CreateProduct", "update": "UpdateProduct", "response": "ProductResponse"},
    "stock": {"entity": "StockMovement", "create": "CreateStockMovement", "update": None, "response": "StockMovementResponse"},
    "invoice": {"entity": "Invoice", "create": "CreateInvoice", "update": None, "response": "InvoiceResponse"},
    "sales": {"entity": "SalesOrder", "create": "CreateSalesOrder", "update": None, "response": "SalesOrderResponse"},
    "purchase": {"entity": "PurchaseOrder", "create": "CreatePurchaseOrder", "update": None, "response": "PurchaseOrderResponse"},
    "accounting": {"entity": "JournalEntry", "create": "CreateJournalEntry", "update": None, "response": None},
    "assets": {"entity": "Asset", "create": "CreateAsset", "update": "UpdateAsset", "response": "AssetResponse"},
    "hr": {"entity": "Employee", "create": "CreateEmployee", "update": None, "response": "EmployeeResponse"},
}

def add_to_model(domain: str, info: dict):
    path = ROOT / domain / "model.rs"
    text = read_text(path)
    entity = info["entity"]
    create = info["create"]
    update = info.get("update")
    response = info.get("response")

    # Add company_id to entity struct after tenant_id line
    pattern = rf"(pub struct {entity} \{{[\s\S]*?pub tenant_id: i64;\n)"
    replacement = r"\1    #[serde(default = \"default_company_id\")]\n    pub company_id: i64,\n"
    text = re.sub(pattern, replacement, text)

    # Add company_id to create struct
    if create:
        pattern = rf"(pub struct {create} \{{[\s\S]*?pub tenant_id: i64;\n)"
        replacement = r"\1    #[serde(default = \"default_company_id\")]\n    pub company_id: i64,\n"
        text = re.sub(pattern, replacement, text)

    # Add company_id to update struct
    if update:
        pattern = rf"(pub struct {update} \{{[\s\S]*?)\}}"
        def repl(m):
            body = m.group(1)
            if "company_id" in body:
                return m.group(0)
            return body + "    #[serde(default)]\n    pub company_id: Option<i64>,\n}"
        text = re.sub(pattern, repl, text)

    # Add company_id to response struct
    if response:
        pattern = rf"(pub struct {response} \{{[\s\S]*?)\}}"
        def repl(m):
            body = m.group(1)
            if "company_id" in body:
                return m.group(0)
            # Insert after tenant_id if present, else at end
            if "pub tenant_id: i64;" in body:
                body = body.replace("pub tenant_id: i64;\n", "pub tenant_id: i64;\n    pub company_id: i64,\n")
            else:
                body = body.rstrip() + "\n    pub company_id: i64,\n"
            return body + "}"
        text = re.sub(pattern, repl, text)

        # Update From impl for response
        if f"From<{entity}> for {response}" in text:
            # Add company_id assignment after tenant_id
            text = text.replace("tenant_id: e.tenant_id,", "tenant_id: e.tenant_id,\n            company_id: e.company_id,")
            text = text.replace("tenant_id: order.tenant_id,", "tenant_id: order.tenant_id,\n            company_id: order.company_id,")
            text = text.replace("tenant_id: request.tenant_id,", "tenant_id: request.tenant_id,\n            company_id: request.company_id,")
            text = text.replace("tenant_id: asset.tenant_id,", "tenant_id: asset.tenant_id,\n            company_id: asset.company_id,")
            text = text.replace("tenant_id: p.tenant_id,", "tenant_id: p.tenant_id,\n            company_id: p.company_id,")
            text = text.replace("tenant_id: invoice.tenant_id,", "tenant_id: invoice.tenant_id,\n            company_id: invoice.company_id,")
            text = text.replace("tenant_id: cari.tenant_id,", "tenant_id: cari.tenant_id,\n            company_id: cari.company_id,")

    # Add default_company_id function if missing
    if "fn default_company_id()" not in text:
        text = text.rstrip() + "\n\nfn default_company_id() -> i64 {\n    1\n}\n"

    write_text(path, text)

def add_to_repository(domain: str, info: dict):
    path = ROOT / domain / "repository.rs"
    text = read_text(path)
    entity = info["entity"]
    create = info["create"]

    # Update in-memory create to include company_id from create DTO
    # Find pattern like: tenant_id: create.tenant_id, and add company_id after it in the struct literal
    # This is heuristic
    text = text.replace("tenant_id: create.tenant_id,", "tenant_id: create.tenant_id,\n            company_id: create.company_id,")
    text = text.replace("tenant_id: create.tenant_id\n            }", "tenant_id: create.tenant_id,\n            company_id: create.company_id\n            }")

    # For invoice create, it's `tenant_id: create.tenant_id,`
    text = text.replace("tenant_id: create.tenant_id,", "tenant_id: create.tenant_id,\n            company_id: create.company_id,")

    write_text(path, text)

def add_to_postgres_repository(domain: str, info: dict):
    path = ROOT / domain / "postgres_repository.rs"
    if not path.exists():
        return
    text = read_text(path)
    entity = info["entity"]
    create = info["create"]
    update = info.get("update")

    # Add default_company_id helper if missing
    if "fn default_company_id()" not in text:
        text = text.replace("use crate::error::ApiError;", "use crate::error::ApiError;\n\nfn default_company_id() -> i64 { 1 }\n")

    # Add company_id to Row structs (with #[sqlx(default = "default_company_id")])
    # Find struct XxxRow { ... } and add field after tenant_id
    pattern = r"(#[derive\(Debug, FromRow\)\]\nstruct (\w+)Row \{[\s\S]*?tenant_id: i64,\n)"
    def repl(m):
        if "company_id" in m.group(0):
            return m.group(0)
        return m.group(1) + "    #[sqlx(default = \"default_company_id\")]\n    company_id: i64,\n"
    text = re.sub(pattern, repl, text)

    # Same for RowWithTotal structs
    pattern = r"(#[derive\(Debug, FromRow\)\]\nstruct (\w+)RowWithTotal \{[\s\S]*?tenant_id: i64,\n)"
    def repl(m):
        if "company_id" in m.group(0):
            return m.group(0)
        return m.group(1) + "    #[sqlx(default = \"default_company_id\")]\n    company_id: i64,\n"
    text = re.sub(pattern, repl, text)

    # Update From<Row> for Entity to include company_id
    # Heuristic: add line after tenant_id assignment in From impl
    text = text.replace("tenant_id: row.tenant_id,", "tenant_id: row.tenant_id,\n            company_id: row.company_id,")
    text = text.replace("tenant_id: row.tenant_id\n        }", "tenant_id: row.tenant_id,\n            company_id: row.company_id\n        }")

    # Update INSERT queries to include company_id column
    # Pattern: INSERT INTO table (... columns ...) VALUES (...)
    # We need to add company_id before the first closing paren of columns and a bind before the first closing paren of values
    # This is hard with regex; let's do a simpler approach: add company_id to the RETURNING clause and the column list for the main entity table.
    # Since each repo is different, we'll do targeted replacements for common patterns.

    # Add company_id to INSERT column list after tenant_id
    text = text.replace("tenant_id,", "tenant_id, company_id,")
    # Add company_id to INSERT values after tenant_id bind
    text = text.replace(".bind(create.tenant_id)", ".bind(create.tenant_id)\n        .bind(create.company_id)")

    # Add company_id to UPDATE set list
    text = text.replace("tenant_id = COALESCE($1, tenant_id),", "tenant_id = COALESCE($1, tenant_id),\n                company_id = COALESCE($X, company_id),")  # placeholder

    # Add company_id to RETURNING clauses
    text = text.replace("RETURNING id, tenant_id,", "RETURNING id, tenant_id, company_id,")

    # Add company_id to SELECT clauses (simple heuristic)
    text = text.replace("SELECT id, tenant_id,", "SELECT id, tenant_id, company_id,")

    write_text(path, text)

def main():
    for domain, info in domains.items():
        print(f"Processing {domain}...")
        add_to_model(domain, info)
        add_to_repository(domain, info)
        add_to_postgres_repository(domain, info)
    print("Done.")

if __name__ == "__main__":
    main()
