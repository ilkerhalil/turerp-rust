//! Inter-company Repository Unit Tests

use rust_decimal_macros::dec;

use turerp::domain::inter_company::{
    CreateInterCompanyInvoice, CreateInterCompanyStockTransfer, InMemoryInterCompanyRepository,
    InterCompanyInvoiceLine, InterCompanyRepository,
};

#[actix_web::test]
async fn test_repo_create_and_get_invoice() {
    let repo = InMemoryInterCompanyRepository::new();
    let invoice = repo
        .create_invoice(CreateInterCompanyInvoice {
            tenant_id: 1,
            seller_company_id: 10,
            buyer_company_id: 20,
            lines: vec![InterCompanyInvoiceLine {
                product_id: 1,
                description: "Widget".to_string(),
                quantity: dec!(5),
                unit_price: dec!(100),
                vat_rate: dec!(18),
            }],
            sales_invoice_id: 100,
            purchase_invoice_id: 200,
        })
        .await
        .unwrap();

    assert_eq!(invoice.id, 1);
    assert_eq!(invoice.tenant_id, 1);
    assert_eq!(invoice.seller_company_id, 10);
    assert_eq!(invoice.buyer_company_id, 20);
    assert_eq!(invoice.lines.len(), 1);

    let fetched = repo.get_invoice(1, 1).await.unwrap();
    assert!(fetched.is_some());
}

#[actix_web::test]
async fn test_repo_get_invoice_not_found() {
    let repo = InMemoryInterCompanyRepository::new();
    let fetched = repo.get_invoice(999, 1).await.unwrap();
    assert!(fetched.is_none());
}

#[actix_web::test]
async fn test_repo_get_invoice_wrong_tenant() {
    let repo = InMemoryInterCompanyRepository::new();
    repo.create_invoice(CreateInterCompanyInvoice {
        tenant_id: 1,
        seller_company_id: 10,
        buyer_company_id: 20,
        lines: vec![],
        sales_invoice_id: 100,
        purchase_invoice_id: 200,
    })
    .await
    .unwrap();

    let fetched = repo.get_invoice(1, 2).await.unwrap();
    assert!(fetched.is_none());
}

/// Regression for Phase 3 audit task 3.1: cross-tenant line items.
/// The Postgres repository enforces this via a JOIN in `fetch_invoice_lines`;
/// the InMemory repository enforces it by short-circuiting `get_invoice` to
/// `None` for foreign tenants, so lines can never be returned.
#[actix_web::test]
async fn test_repo_get_invoice_wrong_tenant_lines_not_exposed() {
    let repo = InMemoryInterCompanyRepository::new();
    repo.create_invoice(CreateInterCompanyInvoice {
        tenant_id: 1,
        seller_company_id: 10,
        buyer_company_id: 20,
        lines: vec![
            InterCompanyInvoiceLine {
                product_id: 1,
                description: "Sensitive line".to_string(),
                quantity: dec!(5),
                unit_price: dec!(100),
                vat_rate: dec!(18),
            },
            InterCompanyInvoiceLine {
                product_id: 2,
                description: "Another sensitive line".to_string(),
                quantity: dec!(3),
                unit_price: dec!(250),
                vat_rate: dec!(18),
            },
        ],
        sales_invoice_id: 100,
        purchase_invoice_id: 200,
    })
    .await
    .unwrap();

    // tenant 2 attempts to read tenant 1's invoice + its lines
    let fetched = repo.get_invoice(1, 2).await.unwrap();
    assert!(
        fetched.is_none(),
        "tenant 2 must not see tenant 1's invoice or its lines"
    );
}

#[actix_web::test]
async fn test_repo_list_invoices() {
    let repo = InMemoryInterCompanyRepository::new();
    repo.create_invoice(CreateInterCompanyInvoice {
        tenant_id: 1,
        seller_company_id: 10,
        buyer_company_id: 20,
        lines: vec![],
        sales_invoice_id: 1,
        purchase_invoice_id: 2,
    })
    .await
    .unwrap();

    repo.create_invoice(CreateInterCompanyInvoice {
        tenant_id: 1,
        seller_company_id: 30,
        buyer_company_id: 40,
        lines: vec![],
        sales_invoice_id: 3,
        purchase_invoice_id: 4,
    })
    .await
    .unwrap();

    repo.create_invoice(CreateInterCompanyInvoice {
        tenant_id: 2,
        seller_company_id: 50,
        buyer_company_id: 60,
        lines: vec![],
        sales_invoice_id: 5,
        purchase_invoice_id: 6,
    })
    .await
    .unwrap();

    assert_eq!(repo.list_invoices(1).await.unwrap().len(), 2);
    assert_eq!(repo.list_invoices(2).await.unwrap().len(), 1);
    assert!(repo.list_invoices(99).await.unwrap().is_empty());
}

/// Regression for Phase 3 audit task 3.1: cross-tenant list must not return
/// foreign-tenant invoice lines. Postgres enforces this via the JOIN in
/// `fetch_invoice_lines`; InMemory enforces it by only indexing invoices
/// under the creating tenant_id.
#[actix_web::test]
async fn test_repo_list_invoices_wrong_tenant_lines_not_exposed() {
    let repo = InMemoryInterCompanyRepository::new();
    repo.create_invoice(CreateInterCompanyInvoice {
        tenant_id: 1,
        seller_company_id: 10,
        buyer_company_id: 20,
        lines: vec![InterCompanyInvoiceLine {
            product_id: 1,
            description: "Tenant 1 line".to_string(),
            quantity: dec!(5),
            unit_price: dec!(100),
            vat_rate: dec!(18),
        }],
        sales_invoice_id: 1,
        purchase_invoice_id: 2,
    })
    .await
    .unwrap();

    repo.create_invoice(CreateInterCompanyInvoice {
        tenant_id: 2,
        seller_company_id: 50,
        buyer_company_id: 60,
        lines: vec![InterCompanyInvoiceLine {
            product_id: 99,
            description: "Tenant 2 line — must not appear for tenant 1".to_string(),
            quantity: dec!(1),
            unit_price: dec!(9999),
            vat_rate: dec!(18),
        }],
        sales_invoice_id: 5,
        purchase_invoice_id: 6,
    })
    .await
    .unwrap();

    // tenant 1's list must contain exactly its own invoice + its own line only
    let t1_invoices = repo.list_invoices(1).await.unwrap();
    assert_eq!(t1_invoices.len(), 1);
    assert_eq!(t1_invoices[0].tenant_id, 1);
    assert_eq!(t1_invoices[0].lines.len(), 1);
    assert_eq!(t1_invoices[0].lines[0].product_id, 1);

    // tenant 2's list must not leak tenant 1's invoice
    let t2_invoices = repo.list_invoices(2).await.unwrap();
    assert_eq!(t2_invoices.len(), 1);
    assert_eq!(t2_invoices[0].tenant_id, 2);
    assert_eq!(t2_invoices[0].lines[0].product_id, 99);
}

#[actix_web::test]
async fn test_repo_create_and_get_stock_transfer() {
    let repo = InMemoryInterCompanyRepository::new();
    let transfer = repo
        .create_stock_transfer(CreateInterCompanyStockTransfer {
            tenant_id: 1,
            from_company_id: 10,
            to_company_id: 20,
            product_id: 1,
            warehouse_id: 5,
            quantity: dec!(10),
            out_movement_id: 100,
            in_movement_id: 200,
            created_by: 1,
        })
        .await
        .unwrap();

    assert_eq!(transfer.id, 1);
    assert_eq!(transfer.tenant_id, 1);
    assert_eq!(transfer.from_company_id, 10);

    let fetched = repo.get_stock_transfer(1, 1).await.unwrap();
    assert!(fetched.is_some());
}

#[actix_web::test]
async fn test_repo_get_stock_transfer_not_found() {
    let repo = InMemoryInterCompanyRepository::new();
    assert!(repo.get_stock_transfer(999, 1).await.unwrap().is_none());
}

#[actix_web::test]
async fn test_repo_get_stock_transfer_wrong_tenant() {
    let repo = InMemoryInterCompanyRepository::new();
    repo.create_stock_transfer(CreateInterCompanyStockTransfer {
        tenant_id: 1,
        from_company_id: 10,
        to_company_id: 20,
        product_id: 1,
        warehouse_id: 5,
        quantity: dec!(10),
        out_movement_id: 100,
        in_movement_id: 200,
        created_by: 1,
    })
    .await
    .unwrap();

    assert!(repo.get_stock_transfer(1, 2).await.unwrap().is_none());
}

#[actix_web::test]
async fn test_repo_list_stock_transfers() {
    let repo = InMemoryInterCompanyRepository::new();
    for (tid, fid, tid2) in [(1, 10, 20), (1, 30, 40), (2, 50, 60)] {
        repo.create_stock_transfer(CreateInterCompanyStockTransfer {
            tenant_id: tid,
            from_company_id: fid,
            to_company_id: tid2,
            product_id: 1,
            warehouse_id: 5,
            quantity: dec!(10),
            out_movement_id: 1,
            in_movement_id: 2,
            created_by: 1,
        })
        .await
        .unwrap();
    }

    assert_eq!(repo.list_stock_transfers(1).await.unwrap().len(), 2);
    assert_eq!(repo.list_stock_transfers(2).await.unwrap().len(), 1);
    assert!(repo.list_stock_transfers(99).await.unwrap().is_empty());
}
