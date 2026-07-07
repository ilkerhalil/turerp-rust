//! Import service trait and CSV implementation

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;

use crate::common::import::model::{
    CariImportRow, ChartAccountImportRow, EntityType, ImportError, ImportFormat, ImportResult,
    ImportStatus, ProductImportRow, StockMovementImportRow,
};
use crate::common::import::parser;
use crate::common::import::validator;
use crate::common::jobs::{CreateJob, JobScheduler, JobType};
use crate::common::pagination::PaginationParams;
use crate::domain::accounting::model::AccountType;
use crate::domain::cari::model::{CariType, CreateCari};
use crate::domain::cari::repository::BoxCariRepository;
use crate::domain::chart_of_accounts::model::{AccountGroup, CreateChartAccount};
use crate::domain::chart_of_accounts::repository::BoxChartAccountRepository;
use crate::domain::product::model::CreateProduct;
use crate::domain::product::repository::BoxProductRepository;
use crate::domain::stock::model::{CreateStockMovement, MovementType};
use crate::domain::stock::repository::BoxStockMovementRepository;
use crate::error::ApiError;

/// Parse optional ISO date strings (YYYY-MM-DD) into DateTime<Utc> range bounds.
#[allow(clippy::type_complexity)]
fn parse_date_range(
    from: Option<String>,
    to: Option<String>,
) -> Result<
    (
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    ),
    ApiError,
> {
    let from_dt = from
        .map(|s| {
            chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map_err(|e| ApiError::Validation(format!("Invalid from date: {}", e)))
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc())
        })
        .transpose()?;
    let to_dt = to
        .map(|s| {
            chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map_err(|e| ApiError::Validation(format!("Invalid to date: {}", e)))
                .map(|d| d.and_hms_opt(23, 59, 59).unwrap_or_default().and_utc())
        })
        .transpose()?;
    Ok((from_dt, to_dt))
}

/// Trait for import/export operations
#[async_trait]
pub trait ImportService: Send + Sync {
    /// Import data for a given entity type
    async fn import(
        &self,
        tenant_id: i64,
        company_id: i64,
        entity_type: EntityType,
        format: ImportFormat,
        data: Vec<u8>,
        created_by: i64,
    ) -> Result<ImportResult, ApiError>;

    /// Get import result by job ID, scoped to the caller's tenant.
    /// Returns `None` if the job does not exist OR belongs to another tenant
    /// (no cross-tenant read, no existence oracle beyond the caller's own 404).
    fn get_result(&self, job_id: i64, tenant_id: i64) -> Option<ImportResult>;

    /// Generate a template for an entity type
    fn generate_template(
        &self,
        entity_type: EntityType,
        format: ImportFormat,
    ) -> Result<Vec<u8>, ApiError>;

    /// Export data for a given entity type
    async fn export(
        &self,
        tenant_id: i64,
        entity_type: EntityType,
        format: ImportFormat,
        from: Option<String>,
        to: Option<String>,
    ) -> Result<Vec<u8>, ApiError>;

    /// Schedule an import as a background job
    async fn schedule_import(
        &self,
        tenant_id: i64,
        company_id: i64,
        entity_type: EntityType,
        format: ImportFormat,
        file_id: i64,
    ) -> Result<i64, ApiError>;
}

/// Type alias for boxed import service
pub type BoxImportService = Arc<dyn ImportService>;

/// CSV-based import service implementation
pub struct CsvImportService {
    product_repo: BoxProductRepository,
    cari_repo: BoxCariRepository,
    chart_account_repo: BoxChartAccountRepository,
    stock_movement_repo: BoxStockMovementRepository,
    job_scheduler: Arc<dyn JobScheduler>,
    results: Mutex<HashMap<i64, ImportResult>>,
    /// Owning tenant per import job, keyed by job_id. Populated alongside
    /// `results` so `get_result` can scope by the caller's tenant and prevent a
    /// cross-tenant read of another tenant's import result / row-level errors
    /// by enumerating the timestamp-based job_id.
    job_tenants: Mutex<HashMap<i64, i64>>,
}

impl CsvImportService {
    pub fn new(
        product_repo: BoxProductRepository,
        cari_repo: BoxCariRepository,
        chart_account_repo: BoxChartAccountRepository,
        stock_movement_repo: BoxStockMovementRepository,
        job_scheduler: Arc<dyn JobScheduler>,
    ) -> Self {
        Self {
            product_repo,
            cari_repo,
            chart_account_repo,
            stock_movement_repo,
            job_scheduler,
            results: Mutex::new(HashMap::new()),
            job_tenants: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ImportService for CsvImportService {
    async fn import(
        &self,
        tenant_id: i64,
        company_id: i64,
        entity_type: EntityType,
        format: ImportFormat,
        data: Vec<u8>,
        created_by: i64,
    ) -> Result<ImportResult, ApiError> {
        let job_id = chrono::Utc::now().timestamp_millis();
        let mut result = ImportResult::new(job_id, entity_type);
        result.status = ImportStatus::Processing;

        match entity_type {
            EntityType::Product => {
                let rows = parser::parse_rows::<ProductImportRow>(&data, format)?;
                result.total_rows = rows.len();
                for (row_num, row) in rows {
                    let errors = validator::validate_product_row(row_num, &row);
                    if !errors.is_empty() {
                        result.errors.extend(errors);
                        result.failed_rows += 1;
                        continue;
                    }
                    // Duplicate detection
                    if let Ok(Some(_)) = self.product_repo.find_by_code(tenant_id, &row.code).await
                    {
                        result.errors.push(ImportError {
                            row: row_num,
                            field: Some("code".to_string()),
                            message: format!("Product with code '{}' already exists", row.code),
                        });
                        result.failed_rows += 1;
                        continue;
                    }
                    let unit_price = row
                        .unit_price
                        .trim()
                        .parse::<rust_decimal::Decimal>()
                        .map_err(|e| ApiError::Validation(format!("Invalid unit_price: {}", e)))?;
                    let create = CreateProduct {
                        tenant_id,
                        company_id,
                        code: row.code,
                        name: row.name,
                        description: None,
                        category_id: None,
                        unit_id: None,
                        barcode: None,
                        purchase_price: unit_price,
                        sale_price: unit_price,
                        tax_rate: rust_decimal::Decimal::ZERO,
                    };
                    match self.product_repo.create(create).await {
                        Ok(_) => result.add_success(),
                        Err(e) => {
                            result.errors.push(ImportError {
                                row: row_num,
                                field: None,
                                message: e.to_string(),
                            });
                            result.failed_rows += 1;
                        }
                    }
                }
            }
            EntityType::Cari => {
                let rows = parser::parse_rows::<CariImportRow>(&data, format)?;
                result.total_rows = rows.len();
                for (row_num, row) in rows {
                    let errors = validator::validate_cari_row(row_num, &row);
                    if !errors.is_empty() {
                        result.errors.extend(errors);
                        result.failed_rows += 1;
                        continue;
                    }
                    if let Ok(Some(_)) = self.cari_repo.find_by_code(&row.code, tenant_id).await {
                        result.errors.push(ImportError {
                            row: row_num,
                            field: Some("code".to_string()),
                            message: format!("Cari with code '{}' already exists", row.code),
                        });
                        result.failed_rows += 1;
                        continue;
                    }
                    let cari_type = row
                        .cari_type
                        .parse::<CariType>()
                        .map_err(ApiError::Validation)?;
                    let create = CreateCari {
                        code: row.code,
                        company_id,
                        name: row.name,
                        cari_type,
                        tax_number: row.tax_number,
                        tax_office: None,
                        identity_number: None,
                        email: row.email,
                        phone: None,
                        address: None,
                        city: None,
                        country: None,
                        postal_code: None,
                        credit_limit: rust_decimal::Decimal::ZERO,
                        default_currency: "TRY".to_string(),
                        tenant_id,
                        created_by,
                    };
                    match self.cari_repo.create(create).await {
                        Ok(_) => result.add_success(),
                        Err(e) => {
                            result.errors.push(ImportError {
                                row: row_num,
                                field: None,
                                message: e.to_string(),
                            });
                            result.failed_rows += 1;
                        }
                    }
                }
            }
            EntityType::ChartOfAccounts => {
                let rows = parser::parse_rows::<ChartAccountImportRow>(&data, format)?;
                result.total_rows = rows.len();
                for (row_num, row) in rows {
                    let errors = validator::validate_chart_account_row(row_num, &row);
                    if !errors.is_empty() {
                        result.errors.extend(errors);
                        result.failed_rows += 1;
                        continue;
                    }
                    if let Ok(Some(_)) = self
                        .chart_account_repo
                        .find_by_code(&row.code, tenant_id)
                        .await
                    {
                        result.errors.push(ImportError {
                            row: row_num,
                            field: Some("code".to_string()),
                            message: format!("Account with code '{}' already exists", row.code),
                        });
                        result.failed_rows += 1;
                        continue;
                    }
                    let account_type = row
                        .account_type
                        .parse::<AccountType>()
                        .map_err(ApiError::Validation)?;
                    let create = CreateChartAccount {
                        code: row.code,
                        name: row.name,
                        account_type,
                        group: AccountGroup::DonenVarliklar,
                        parent_code: row.parent_code,
                        allow_posting: true,
                    };
                    match self.chart_account_repo.create(create, tenant_id).await {
                        Ok(_) => result.add_success(),
                        Err(e) => {
                            result.errors.push(ImportError {
                                row: row_num,
                                field: None,
                                message: e.to_string(),
                            });
                            result.failed_rows += 1;
                        }
                    }
                }
            }
            EntityType::StockMovement => {
                let rows = parser::parse_rows::<StockMovementImportRow>(&data, format)?;
                result.total_rows = rows.len();
                for (row_num, row) in rows {
                    let errors = validator::validate_stock_movement_row(row_num, &row);
                    if !errors.is_empty() {
                        result.errors.extend(errors);
                        result.failed_rows += 1;
                        continue;
                    }
                    let warehouse_id = row.warehouse_id.parse::<i64>().map_err(|e| {
                        ApiError::Validation(format!("Invalid warehouse_id: {}", e))
                    })?;
                    let quantity = row
                        .quantity
                        .parse::<rust_decimal::Decimal>()
                        .map_err(|e| ApiError::Validation(format!("Invalid quantity: {}", e)))?;
                    let movement_type = if let Ok(mt) = row.direction.parse::<MovementType>() {
                        mt
                    } else {
                        match row.direction.to_lowercase().as_str() {
                            "in" => MovementType::Purchase,
                            "out" => MovementType::Sale,
                            _ => {
                                result.errors.push(ImportError {
                                    row: row_num,
                                    field: Some("direction".to_string()),
                                    message: format!("Invalid direction: {}", row.direction),
                                });
                                result.failed_rows += 1;
                                continue;
                            }
                        }
                    };
                    // Find product by code to get id
                    let product = match self
                        .product_repo
                        .find_by_code(tenant_id, &row.product_code)
                        .await
                    {
                        Ok(Some(p)) => p,
                        Ok(None) => {
                            result.errors.push(ImportError {
                                row: row_num,
                                field: Some("product_code".to_string()),
                                message: format!("Product '{}' not found", row.product_code),
                            });
                            result.failed_rows += 1;
                            continue;
                        }
                        Err(e) => {
                            result.errors.push(ImportError {
                                row: row_num,
                                field: Some("product_code".to_string()),
                                message: e.to_string(),
                            });
                            result.failed_rows += 1;
                            continue;
                        }
                    };
                    let create = CreateStockMovement {
                        tenant_id,
                        warehouse_id,
                        company_id,
                        product_id: product.id,
                        movement_type,
                        quantity,
                        reference_type: None,
                        reference_id: None,
                        notes: None,
                        created_by,
                    };
                    match self.stock_movement_repo.create(create).await {
                        Ok(_) => result.add_success(),
                        Err(e) => {
                            result.errors.push(ImportError {
                                row: row_num,
                                field: None,
                                message: e.to_string(),
                            });
                            result.failed_rows += 1;
                        }
                    }
                }
            }
        }

        result.status = ImportStatus::Completed;
        result.completed_at = Some(Utc::now());
        self.results.lock().insert(job_id, result.clone());
        // Record the owning tenant so get_result can scope by caller tenant.
        self.job_tenants.lock().insert(job_id, tenant_id);
        Ok(result)
    }

    fn get_result(&self, job_id: i64, tenant_id: i64) -> Option<ImportResult> {
        // Reject before lookup if the job belongs to another tenant — a
        // foreign job_id yields None (404) with no data read and no oracle.
        if self.job_tenants.lock().get(&job_id) != Some(&tenant_id) {
            return None;
        }
        self.results.lock().get(&job_id).cloned()
    }

    fn generate_template(
        &self,
        entity_type: EntityType,
        format: ImportFormat,
    ) -> Result<Vec<u8>, ApiError> {
        match (entity_type, format) {
            (EntityType::Product, ImportFormat::Csv) => {
                parser::build_csv_template(&["code", "name", "unit_price", "category", "unit"])
            }
            (EntityType::Product, ImportFormat::Json) => {
                parser::build_json_template(serde_json::json!({
                    "code": "P001",
                    "name": "Product Name",
                    "unit_price": "100.00",
                    "category": "Category Name",
                    "unit": "piece"
                }))
            }
            (EntityType::Cari, ImportFormat::Csv) => {
                parser::build_csv_template(&["code", "name", "cari_type", "tax_number", "email"])
            }
            (EntityType::Cari, ImportFormat::Json) => {
                parser::build_json_template(serde_json::json!({
                    "code": "C001",
                    "name": "Customer Name",
                    "cari_type": "customer",
                    "tax_number": "1234567890",
                    "email": "customer@example.com"
                }))
            }
            (EntityType::ChartOfAccounts, ImportFormat::Csv) => {
                parser::build_csv_template(&["code", "name", "account_type", "parent_code"])
            }
            (EntityType::ChartOfAccounts, ImportFormat::Json) => {
                parser::build_json_template(serde_json::json!({
                    "code": "100",
                    "name": "Cash",
                    "account_type": "Asset",
                    "parent_code": ""
                }))
            }
            (EntityType::StockMovement, ImportFormat::Csv) => parser::build_csv_template(&[
                "product_code",
                "warehouse_id",
                "quantity",
                "direction",
            ]),
            (EntityType::StockMovement, ImportFormat::Json) => {
                parser::build_json_template(serde_json::json!({
                    "product_code": "P001",
                    "warehouse_id": "1",
                    "quantity": "10",
                    "direction": "in"
                }))
            }
        }
    }

    async fn export(
        &self,
        tenant_id: i64,
        entity_type: EntityType,
        format: ImportFormat,
        from: Option<String>,
        to: Option<String>,
    ) -> Result<Vec<u8>, ApiError> {
        let (from_dt, to_dt) = parse_date_range(from, to)?;
        match entity_type {
            EntityType::Product => {
                let products = self.product_repo.find_by_tenant(tenant_id).await?;
                let records: Vec<Vec<String>> = products
                    .into_iter()
                    .filter(|p| {
                        from_dt.is_none_or(|d| p.created_at >= d)
                            && to_dt.is_none_or(|d| p.created_at <= d)
                    })
                    .map(|p| {
                        vec![
                            p.code,
                            p.name,
                            p.purchase_price.to_string(),
                            "".to_string(),
                            "".to_string(),
                        ]
                    })
                    .collect();
                match format {
                    ImportFormat::Csv => parser::write_csv_records(
                        &["code", "name", "purchase_price", "category", "unit"],
                        records,
                    ),
                    ImportFormat::Json => parser::write_json_records(&records),
                }
            }
            EntityType::Cari => {
                let caris = self.cari_repo.find_all(tenant_id).await?;
                let records: Vec<Vec<String>> = caris
                    .into_iter()
                    .filter(|c| {
                        from_dt.is_none_or(|d| c.created_at >= d)
                            && to_dt.is_none_or(|d| c.created_at <= d)
                    })
                    .map(|c| {
                        vec![
                            c.code,
                            c.name,
                            c.cari_type.to_string(),
                            c.tax_number.unwrap_or_default(),
                            c.email.unwrap_or_default(),
                        ]
                    })
                    .collect();
                match format {
                    ImportFormat::Csv => parser::write_csv_records(
                        &["code", "name", "cari_type", "tax_number", "email"],
                        records,
                    ),
                    ImportFormat::Json => parser::write_json_records(&records),
                }
            }
            EntityType::ChartOfAccounts => {
                let accounts = self
                    .chart_account_repo
                    .find_all(tenant_id, None, PaginationParams::default())
                    .await?;
                let records: Vec<Vec<String>> = accounts
                    .items
                    .into_iter()
                    .filter(|a| {
                        from_dt.is_none_or(|d| a.created_at >= d)
                            && to_dt.is_none_or(|d| a.created_at <= d)
                    })
                    .map(|a| {
                        vec![
                            a.code,
                            a.name,
                            a.account_type.to_string(),
                            a.parent_code.unwrap_or_default(),
                        ]
                    })
                    .collect();
                match format {
                    ImportFormat::Csv => parser::write_csv_records(
                        &["code", "name", "account_type", "parent_code"],
                        records,
                    ),
                    ImportFormat::Json => parser::write_json_records(&records),
                }
            }
            EntityType::StockMovement => {
                let movements = self.stock_movement_repo.find_by_tenant(tenant_id).await?;
                let records: Vec<Vec<String>> = movements
                    .into_iter()
                    .filter(|m| {
                        from_dt.is_none_or(|d| m.created_at >= d)
                            && to_dt.is_none_or(|d| m.created_at <= d)
                    })
                    .map(|m| {
                        vec![
                            m.product_id.to_string(),
                            m.warehouse_id.to_string(),
                            m.quantity.to_string(),
                            m.movement_type.to_string(),
                        ]
                    })
                    .collect();
                match format {
                    ImportFormat::Csv => parser::write_csv_records(
                        &["product_code", "warehouse_id", "quantity", "direction"],
                        records,
                    ),
                    ImportFormat::Json => parser::write_json_records(&records),
                }
            }
        }
    }

    async fn schedule_import(
        &self,
        tenant_id: i64,
        company_id: i64,
        entity_type: EntityType,
        format: ImportFormat,
        file_id: i64,
    ) -> Result<i64, ApiError> {
        let job = CreateJob::new(
            JobType::Import {
                file_id,
                entity_type: entity_type.to_string(),
                tenant_id,
                company_id,
                format: format.to_string(),
            },
            tenant_id,
        );
        let job = self
            .job_scheduler
            .schedule(job)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to schedule import job: {}", e)))?;
        Ok(job.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::InMemoryJobScheduler;
    use crate::domain::cari::repository::InMemoryCariRepository;
    use crate::domain::chart_of_accounts::repository::InMemoryChartAccountRepository;
    use crate::domain::product::repository::InMemoryProductRepository;
    use crate::domain::stock::repository::InMemoryStockMovementRepository;

    fn create_test_service() -> CsvImportService {
        CsvImportService::new(
            Arc::new(InMemoryProductRepository::new()),
            Arc::new(InMemoryCariRepository::new()),
            Arc::new(InMemoryChartAccountRepository::new()),
            Arc::new(InMemoryStockMovementRepository::new()),
            Arc::new(InMemoryJobScheduler::new()),
        )
    }

    #[tokio::test]
    async fn test_import_products_csv() {
        let svc = create_test_service();
        let data = b"code,name,unit_price\nP001,Product 1,100.00\nP002,Product 2,200.00";
        let result = svc
            .import(
                1,
                1,
                EntityType::Product,
                ImportFormat::Csv,
                data.to_vec(),
                1,
            )
            .await
            .unwrap();
        assert_eq!(result.total_rows, 2);
        assert_eq!(result.success_rows, 2);
        assert_eq!(result.failed_rows, 0);
    }

    #[tokio::test]
    async fn test_import_duplicate_detection() {
        let svc = create_test_service();
        let data = b"code,name,unit_price\nP001,Product 1,100.00";
        svc.import(
            1,
            1,
            EntityType::Product,
            ImportFormat::Csv,
            data.to_vec(),
            1,
        )
        .await
        .unwrap();
        let result = svc
            .import(
                1,
                1,
                EntityType::Product,
                ImportFormat::Csv,
                data.to_vec(),
                1,
            )
            .await
            .unwrap();
        assert_eq!(result.total_rows, 1);
        assert_eq!(result.failed_rows, 1);
        assert!(result.errors[0].message.contains("already exists"));
    }

    #[tokio::test]
    async fn test_generate_template() {
        let svc = create_test_service();
        let template = svc
            .generate_template(EntityType::Product, ImportFormat::Csv)
            .unwrap();
        let s = String::from_utf8(template).unwrap();
        assert!(s.contains("code"));
        assert!(s.contains("name"));
    }

    #[tokio::test]
    async fn test_get_result() {
        let svc = create_test_service();
        let data = b"code,name,unit_price\nP001,Product 1,100.00";
        let result = svc
            .import(
                1,
                1,
                EntityType::Product,
                ImportFormat::Csv,
                data.to_vec(),
                1,
            )
            .await
            .unwrap();
        let fetched = svc.get_result(result.job_id, 1);
        assert!(fetched.is_some());
    }

    #[tokio::test]
    async fn test_get_result_rejects_foreign_tenant() {
        // Cross-tenant IDOR guard: a job imported by tenant 1 must not be
        // readable by tenant 2 via get_result — the job_id is timestamp-based
        // and enumerable, so without the tenant scope a caller could read
        // another tenant's import counts and row-level validation errors.
        let svc = create_test_service();
        let data = b"code,name,unit_price\nP001,Product 1,100.00";
        let result = svc
            .import(
                1,
                1,
                EntityType::Product,
                ImportFormat::Csv,
                data.to_vec(),
                1,
            )
            .await
            .unwrap();
        // Same tenant can read.
        assert!(svc.get_result(result.job_id, 1).is_some());
        // Foreign tenant cannot.
        assert!(svc.get_result(result.job_id, 2).is_none());
        // Unknown job_id yields None regardless of tenant.
        assert!(svc.get_result(9999999999, 1).is_none());
    }

    #[tokio::test]
    async fn test_import_respects_company_id() {
        let svc = create_test_service();
        let data = b"code,name,unit_price\nP001,Product 1,100.00";
        svc.import(
            1,
            42,
            EntityType::Product,
            ImportFormat::Csv,
            data.to_vec(),
            1,
        )
        .await
        .unwrap();

        let product = svc
            .product_repo
            .find_by_code(1, "P001")
            .await
            .unwrap()
            .expect("Product should exist");
        assert_eq!(product.company_id, 42);
    }
}
