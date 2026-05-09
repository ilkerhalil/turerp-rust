#!/usr/bin/env python3
"""Comprehensive fix for company_id across all target domain models."""

import re
from pathlib import Path

ROOT = Path("/home/ilker/projects/turerp-rust/turerp/src/domain")

# All structs that need company_id in each domain
# Format: (struct_name, is_response, has_tenant_id)
DOMAINS = {
    "cari": [
        ("Cari", False, True),
        ("CreateCari", False, True),
        ("UpdateCari", False, True),
        ("CariResponse", True, True),
    ],
    "product": [
        ("Product", False, True),
        ("CreateProduct", False, True),
        ("UpdateProduct", False, True),
        ("ProductResponse", True, True),
        ("Category", False, True),
        ("CreateCategory", False, True),
        ("UpdateCategory", False, True),
        ("CategoryResponse", True, True),
        ("Unit", False, True),
        ("CreateUnit", False, True),
        ("UpdateUnit", False, True),
        ("UnitResponse", True, True),
    ],
    "stock": [
        ("Warehouse", False, True),
        ("CreateWarehouse", False, True),
        ("UpdateWarehouse", False, True),
        ("WarehouseResponse", True, True),
        ("StockMovement", False, False),  # no tenant_id
        ("CreateStockMovement", False, False),
        ("StockMovementResponse", True, False),
    ],
    "invoice": [
        ("Invoice", False, True),
        ("CreateInvoice", False, True),
        ("InvoiceResponse", True, True),
        ("Payment", False, True),
        ("CreatePayment", False, True),
        ("PaymentResponse", True, True),
    ],
    "sales": [
        ("SalesOrder", False, True),
        ("CreateSalesOrder", False, True),
        ("SalesOrderResponse", True, True),
        ("Quotation", False, True),
        ("CreateQuotation", False, True),
        ("QuotationResponse", True, True),
    ],
    "purchase": [
        ("PurchaseOrder", False, True),
        ("CreatePurchaseOrder", False, True),
        ("PurchaseOrderResponse", True, True),
        ("PurchaseRequest", False, True),
        ("CreatePurchaseRequest", False, True),
        ("PurchaseRequestResponse", True, True),
        ("GoodsReceipt", False, True),
        ("CreateGoodsReceipt", False, True),
        ("GoodsReceiptResponse", True, True),
    ],
    "accounting": [
        ("JournalEntry", False, True),
        ("CreateJournalEntry", False, True),
        ("JournalEntryResponse", True, True),
        ("Account", False, True),
        ("CreateAccount", False, True),
        ("UpdateAccount", False, True),
        ("AccountResponse", True, True),
    ],
    "assets": [
        ("Asset", False, True),
        ("CreateAsset", False, True),
        ("UpdateAsset", False, True),
        ("AssetResponse", True, True),
    ],
    "hr": [
        ("Employee", False, True),
        ("CreateEmployee", False, True),
        ("EmployeeResponse", True, True),
    ],
}

def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")

def write_text(path: Path, text: str):
    path.write_text(text, encoding="utf-8")

def add_company_id_to_struct(text: str, struct_name: str, is_response: bool, has_tenant_id: bool) -> str:
    """Add company_id to a struct if missing."""
    pattern = rf"(pub struct {re.escape(struct_name)} \{{[\s\S]*?)\n\}}"

    def repl(m):
        body = m.group(1)
        if "company_id" in body:
            return m.group(0)

        # Determine where to insert
        if has_tenant_id and "pub tenant_id: i64," in body:
            # Insert after tenant_id
            body = body.replace("pub tenant_id: i64,\n", "pub tenant_id: i64,\n    pub company_id: i64,\n")
        elif has_tenant_id and "tenant_id: i64," in body:
            body = body.replace("tenant_id: i64,\n", "tenant_id: i64,\n    pub company_id: i64,\n")
        else:
            # Insert after id if present, else at end
            if "pub id: i64,\n" in body:
                body = body.replace("pub id: i64,\n", "pub id: i64,\n    pub company_id: i64,\n")
            else:
                body = body.rstrip() + "\n    pub company_id: i64,"
        return body + "\n}"

    new_text = re.sub(pattern, repl, text)
    return new_text

def fix_from_impls(text: str) -> str:
    """Fix From impls to include company_id."""
    # Fix patterns like tenant_id: x.tenant_id, not followed by company_id
    # This is a simplified heuristic
    text = re.sub(
        r'(tenant_id: (\w+)\.tenant_id,)(?!\s*\n\s*company_id:)',
        r'\1\n            company_id: \2.company_id,',
        text
    )
    return text

def remove_duplicate_company_id(text: str) -> str:
    """Remove duplicate company_id lines in struct literals."""
    # Pattern: company_id: x.company_id,\n            company_id: x.company_id,
    text = re.sub(
        r'(\s+company_id: [\w.]+,)(\n\s+company_id: [\w.]+,)',
        r'\1',
        text
    )
    return text

def add_default_company_id(text: str) -> str:
    """Add default_company_id helper if missing."""
    if "fn default_company_id()" not in text:
        text = text.rstrip() + "\n\nfn default_company_id() -> i64 {\n    1\n}\n"
    return text

def fix_domain(domain: str, structs: list):
    path = ROOT / domain / "model.rs"
    if not path.exists():
        print(f"SKIP {domain}: model.rs not found")
        return

    text = read_text(path)
    original = text

    for struct_name, is_response, has_tenant_id in structs:
        text = add_company_id_to_struct(text, struct_name, is_response, has_tenant_id)

    text = fix_from_impls(text)
    text = remove_duplicate_company_id(text)
    text = add_default_company_id(text)

    if text != original:
        write_text(path, text)
        print(f"FIXED {domain}")
    else:
        print(f"OK    {domain}")

def main():
    for domain, structs in DOMAINS.items():
        fix_domain(domain, structs)

if __name__ == "__main__":
    main()
