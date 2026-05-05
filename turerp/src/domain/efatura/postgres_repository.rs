//! PostgreSQL e-Fatura repository implementation

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::db::error::map_sqlx_error;
use crate::domain::efatura::model::{
    AddressInfo, EFatura, EFaturaLine, EFaturaProfile, EFaturaStatus, MonetaryTotal, PartyInfo,
    TaxSubtotal,
};
use crate::domain::efatura::repository::{BoxEFaturaRepository, EFaturaRepository};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// EfaturaRow / EFatura conversion
// ---------------------------------------------------------------------------

/// Database row representation for e-Fatura
#[derive(Debug, FromRow)]
struct EfaturaRow {
    id: i64,
    tenant_id: i64,
    invoice_id: Option<i64>,
    uuid: String,
    document_number: String,
    issue_date: NaiveDate,
    profile_id: String,
    // Sender
    sender_vkn_tckn: String,
    sender_name: String,
    sender_tax_office: String,
    sender_street: String,
    sender_district: Option<String>,
    sender_city: String,
    sender_country: Option<String>,
    sender_postal_code: Option<String>,
    sender_email: Option<String>,
    sender_phone: Option<String>,
    sender_register_number: Option<String>,
    sender_mersis_number: Option<String>,
    // Receiver
    receiver_vkn_tckn: String,
    receiver_name: String,
    receiver_tax_office: String,
    receiver_street: String,
    receiver_district: Option<String>,
    receiver_city: String,
    receiver_country: Option<String>,
    receiver_postal_code: Option<String>,
    receiver_email: Option<String>,
    receiver_phone: Option<String>,
    receiver_register_number: Option<String>,
    receiver_mersis_number: Option<String>,
    // Status
    status: String,
    response_code: Option<String>,
    response_desc: Option<String>,
    xml_content: Option<String>,
    // JSONB columns
    lines: serde_json::Value,
    tax_totals: serde_json::Value,
    legal_monetary_total: serde_json::Value,
    // Timestamps
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    total_count: Option<i64>,
}

impl From<EfaturaRow> for EFatura {
    fn from(row: EfaturaRow) -> Self {
        let profile_id = row.profile_id.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid profile_id '{}' in database: {}, defaulting to TemelFatura",
                row.profile_id,
                e
            );
            EFaturaProfile::TemelFatura
        });

        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid e-Fatura status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            EFaturaStatus::Draft
        });

        let lines: Vec<EFaturaLine> = serde_json::from_value(row.lines).unwrap_or_default();
        let tax_totals: Vec<TaxSubtotal> =
            serde_json::from_value(row.tax_totals).unwrap_or_default();
        let legal_monetary_total: MonetaryTotal = serde_json::from_value(row.legal_monetary_total)
            .unwrap_or(MonetaryTotal {
                line_extension_amount: rust_decimal::Decimal::ZERO,
                tax_exclusive_amount: rust_decimal::Decimal::ZERO,
                tax_inclusive_amount: rust_decimal::Decimal::ZERO,
                allowance_total_amount: None,
                payable_amount: rust_decimal::Decimal::ZERO,
            });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            invoice_id: row.invoice_id,
            uuid: row.uuid,
            document_number: row.document_number,
            issue_date: row.issue_date,
            profile_id,
            sender: PartyInfo {
                vkn_tckn: row.sender_vkn_tckn,
                name: row.sender_name,
                tax_office: row.sender_tax_office,
                address: AddressInfo {
                    street: row.sender_street,
                    district: row.sender_district,
                    city: row.sender_city,
                    country: row.sender_country,
                    postal_code: row.sender_postal_code,
                },
                email: row.sender_email,
                phone: row.sender_phone,
                register_number: row.sender_register_number,
                mersis_number: row.sender_mersis_number,
            },
            receiver: PartyInfo {
                vkn_tckn: row.receiver_vkn_tckn,
                name: row.receiver_name,
                tax_office: row.receiver_tax_office,
                address: AddressInfo {
                    street: row.receiver_street,
                    district: row.receiver_district,
                    city: row.receiver_city,
                    country: row.receiver_country,
                    postal_code: row.receiver_postal_code,
                },
                email: row.receiver_email,
                phone: row.receiver_phone,
                register_number: row.receiver_register_number,
                mersis_number: row.receiver_mersis_number,
            },
            lines,
            tax_totals,
            legal_monetary_total,
            status,
            response_code: row.response_code,
            response_desc: row.response_desc,
            xml_content: row.xml_content,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ===========================================================================
// PostgresEFaturaRepository
// ===========================================================================

/// PostgreSQL e-Fatura repository
pub struct PostgresEFaturaRepository {
    pool: Arc<PgPool>,
}

impl PostgresEFaturaRepository {
    /// Create a new PostgreSQL e-Fatura repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxEFaturaRepository {
        Arc::new(self) as BoxEFaturaRepository
    }
}

/// Common column list for efatura SELECT queries
const EFATURA_COLUMNS: &str = r#"
    id, tenant_id, invoice_id, uuid, document_number, issue_date, profile_id,
    sender_vkn_tckn, sender_name, sender_tax_office, sender_street,
    sender_district, sender_city, sender_country, sender_postal_code,
    sender_email, sender_phone, sender_register_number, sender_mersis_number,
    receiver_vkn_tckn, receiver_name, receiver_tax_office, receiver_street,
    receiver_district, receiver_city, receiver_country, receiver_postal_code,
    receiver_email, receiver_phone, receiver_register_number, receiver_mersis_number,
    status, response_code, response_desc, xml_content,
    lines, tax_totals, legal_monetary_total,
    created_at, updated_at
"#;

#[async_trait]
impl EFaturaRepository for PostgresEFaturaRepository {
    async fn create(&self, fatura: EFatura) -> Result<EFatura, ApiError> {
        let profile_id = fatura.profile_id.to_string();
        let status = fatura.status.to_string();
        let lines_json =
            serde_json::to_value(&fatura.lines).unwrap_or(serde_json::Value::Array(vec![]));
        let tax_totals_json =
            serde_json::to_value(&fatura.tax_totals).unwrap_or(serde_json::Value::Array(vec![]));
        let monetary_total_json = serde_json::to_value(&fatura.legal_monetary_total)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let row: EfaturaRow = sqlx::query_as(&format!(
            r#"
            INSERT INTO efatura (
                tenant_id, invoice_id, uuid, document_number, issue_date, profile_id,
                sender_vkn_tckn, sender_name, sender_tax_office, sender_street,
                sender_district, sender_city, sender_country, sender_postal_code,
                sender_email, sender_phone, sender_register_number, sender_mersis_number,
                receiver_vkn_tckn, receiver_name, receiver_tax_office, receiver_street,
                receiver_district, receiver_city, receiver_country, receiver_postal_code,
                receiver_email, receiver_phone, receiver_register_number, receiver_mersis_number,
                status, response_code, response_desc, xml_content,
                lines, tax_totals, legal_monetary_total
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                    $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26,
                    $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37)
            RETURNING {EFATURA_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(fatura.tenant_id)
        .bind(fatura.invoice_id)
        .bind(&fatura.uuid)
        .bind(&fatura.document_number)
        .bind(fatura.issue_date)
        .bind(&profile_id)
        // Sender
        .bind(&fatura.sender.vkn_tckn)
        .bind(&fatura.sender.name)
        .bind(&fatura.sender.tax_office)
        .bind(&fatura.sender.address.street)
        .bind(&fatura.sender.address.district)
        .bind(&fatura.sender.address.city)
        .bind(&fatura.sender.address.country)
        .bind(&fatura.sender.address.postal_code)
        .bind(&fatura.sender.email)
        .bind(&fatura.sender.phone)
        .bind(&fatura.sender.register_number)
        .bind(&fatura.sender.mersis_number)
        // Receiver
        .bind(&fatura.receiver.vkn_tckn)
        .bind(&fatura.receiver.name)
        .bind(&fatura.receiver.tax_office)
        .bind(&fatura.receiver.address.street)
        .bind(&fatura.receiver.address.district)
        .bind(&fatura.receiver.address.city)
        .bind(&fatura.receiver.address.country)
        .bind(&fatura.receiver.address.postal_code)
        .bind(&fatura.receiver.email)
        .bind(&fatura.receiver.phone)
        .bind(&fatura.receiver.register_number)
        .bind(&fatura.receiver.mersis_number)
        // Status & content
        .bind(&status)
        .bind(&fatura.response_code)
        .bind(&fatura.response_desc)
        .bind(&fatura.xml_content)
        // JSONB
        .bind(lines_json)
        .bind(tax_totals_json)
        .bind(monetary_total_json)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "EFatura"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<EFatura>, ApiError> {
        let result: Option<EfaturaRow> = sqlx::query_as(&format!(
            r#"
            SELECT {EFATURA_COLUMNS}, 0 as total_count
            FROM efatura
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find e-Fatura: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_uuid(&self, uuid: &str, tenant_id: i64) -> Result<Option<EFatura>, ApiError> {
        let result: Option<EfaturaRow> = sqlx::query_as(&format!(
            r#"
            SELECT {EFATURA_COLUMNS}, 0 as total_count
            FROM efatura
            WHERE uuid = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        ))
        .bind(uuid)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find e-Fatura by UUID: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_invoice_id(
        &self,
        invoice_id: i64,
        tenant_id: i64,
    ) -> Result<Option<EFatura>, ApiError> {
        let result: Option<EfaturaRow> = sqlx::query_as(&format!(
            r#"
            SELECT {EFATURA_COLUMNS}, 0 as total_count
            FROM efatura
            WHERE invoice_id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        ))
        .bind(invoice_id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find e-Fatura by invoice ID: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        status: Option<EFaturaStatus>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<EFatura>, ApiError> {
        let offset = params.offset() as i64;
        let per_page = params.per_page as i64;

        match status {
            Some(s) => {
                let status_str = s.to_string();
                let rows: Vec<EfaturaRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {EFATURA_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM efatura
                    WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL
                    ORDER BY issue_date DESC, id DESC
                    LIMIT $3 OFFSET $4
                    "#,
                ))
                .bind(tenant_id)
                .bind(&status_str)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "EFatura"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<EFatura> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            None => {
                let rows: Vec<EfaturaRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {EFATURA_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM efatura
                    WHERE tenant_id = $1 AND deleted_at IS NULL
                    ORDER BY issue_date DESC, id DESC
                    LIMIT $2 OFFSET $3
                    "#,
                ))
                .bind(tenant_id)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "EFatura"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<EFatura> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
        }
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: EFaturaStatus,
        response_code: Option<String>,
        response_desc: Option<String>,
    ) -> Result<EFatura, ApiError> {
        let status_str = status.to_string();

        let row: EfaturaRow = sqlx::query_as(&format!(
            r#"
            UPDATE efatura
            SET status = $1,
                response_code = $2,
                response_desc = $3,
                updated_at = NOW()
            WHERE id = $4 AND tenant_id = $5 AND deleted_at IS NULL
            RETURNING {EFATURA_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(&status_str)
        .bind(&response_code)
        .bind(&response_desc)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "EFatura"))?;

        Ok(row.into())
    }

    async fn update_xml(
        &self,
        id: i64,
        tenant_id: i64,
        xml_content: String,
    ) -> Result<EFatura, ApiError> {
        let row: EfaturaRow = sqlx::query_as(&format!(
            r#"
            UPDATE efatura
            SET xml_content = $1,
                updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3 AND deleted_at IS NULL
            RETURNING {EFATURA_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(&xml_content)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "EFatura"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE efatura
            SET deleted_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete e-Fatura: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("e-Fatura not found".to_string()));
        }

        Ok(())
    }
}
