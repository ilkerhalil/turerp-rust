#!/usr/bin/env python3
"""Fix company_id in model files for core domains."""

import re
from pathlib import Path

ROOT = Path("/home/ilker/projects/turerp-rust/turerp/src/domain")

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

def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")

def write_text(path: Path, text: str):
    path.write_text(text, encoding="utf-8")

def add_to_model(domain: str, info: dict):
    path = ROOT / domain / "model.rs"
    text = read_text(path)
    entity = info["entity"]
    create = info["create"]
    update = info.get("update")
    response = info.get("response")

    # 1. Add company_id to entity struct after tenant_id line
    # Use comma (not semicolon) since Rust struct fields use commas
    pattern = rf"(pub struct {entity} \{{[\s\S]*?pub tenant_id: i64,\n)"
    def repl(m):
        body = m.group(1)
        if "company_id" in body:
            return body
        return body + "    #[serde(default = \"default_company_id\")]\n    pub company_id: i64,\n"
    text = re.sub(pattern, repl, text)

    # 2. Add company_id to create struct
    if create:
        pattern = rf"(pub struct {create} \{{[\s\S]*?pub tenant_id: i64,\n)"
        def repl(m):
            body = m.group(1)
            if "company_id" in body:
                return body
            return body + "    #[serde(default = \"default_company_id\")]\n    pub company_id: i64,\n"
        text = re.sub(pattern, repl, text)

    # 3. Add company_id to update struct
    if update:
        pattern = rf"(pub struct {update} \{{[\s\S]*?)\n\}}"
        def repl(m):
            body = m.group(1)
            if "company_id" in body:
                return m.group(0)
            # Insert after tenant_id if present, else before closing }
            if "pub tenant_id: i64," in body:
                body = body.replace("pub tenant_id: i64,\n", "pub tenant_id: i64,\n    #[serde(default)]\n    pub company_id: Option<i64>,\n")
            else:
                body = body.rstrip() + "\n    #[serde(default)]\n    pub company_id: Option<i64>,\n"
            return body + "\n}"
        text = re.sub(pattern, repl, text)

    # 4. Add company_id to response struct
    if response:
        pattern = rf"(pub struct {response} \{{[\s\S]*?)\n\}}"
        def repl(m):
            body = m.group(1)
            if "company_id" in body:
                return m.group(0)
            # Insert after tenant_id if present, else at end
            if "pub tenant_id: i64," in body:
                body = body.replace("pub tenant_id: i64,\n", "pub tenant_id: i64,\n    pub company_id: i64,\n")
            else:
                body = body.rstrip() + "\n    pub company_id: i64,\n"
            return body + "\n}"
        text = re.sub(pattern, repl, text)

        # Update From impl for response
        if f"From<{entity}> for {response}" in text:
            text = text.replace("tenant_id: e.tenant_id,", "tenant_id: e.tenant_id,\n            company_id: e.company_id,")
            text = text.replace("tenant_id: order.tenant_id,", "tenant_id: order.tenant_id,\n            company_id: order.company_id,")
            text = text.replace("tenant_id: request.tenant_id,", "tenant_id: request.tenant_id,\n            company_id: request.company_id,")
            text = text.replace("tenant_id: asset.tenant_id,", "tenant_id: asset.tenant_id,\n            company_id: asset.company_id,")
            text = text.replace("tenant_id: p.tenant_id,", "tenant_id: p.tenant_id,\n            company_id: p.company_id,")
            text = text.replace("tenant_id: invoice.tenant_id,", "tenant_id: invoice.tenant_id,\n            company_id: invoice.company_id,")
            text = text.replace("tenant_id: cari.tenant_id,", "tenant_id: cari.tenant_id,\n            company_id: cari.company_id,")
            text = text.replace("tenant_id: employee.tenant_id,", "tenant_id: employee.tenant_id,\n            company_id: employee.company_id,")
            text = text.replace("tenant_id: movement.tenant_id,", "tenant_id: movement.tenant_id,\n            company_id: movement.company_id,")

    # 5. Add default_company_id function if missing
    if "fn default_company_id()" not in text:
        text = text.rstrip() + "\n\nfn default_company_id() -> i64 {\n    1\n}\n"

    write_text(path, text)
    print(f"Fixed {path}")

def main():
    for domain, info in domains.items():
        add_to_model(domain, info)

if __name__ == "__main__":
    main()
