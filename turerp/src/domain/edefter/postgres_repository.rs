//! PostgreSQL e-Defter repository implementation

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::db::error::map_sqlx_error;
use crate::domain::edefter::model::{
    BeratInfo, EDefterStatus, LedgerPeriod, LedgerType, YevmiyeEntry, YevmiyeLine,
};
use crate::domain::edefter::repository::{BoxEDefterRepository, EDefterRepository};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// LedgerPeriodRow / LedgerPeriod conversion
// ---------------------------------------------------------------------------

/// Database row representation for LedgerPeriod
#[derive(Debug, FromRow)]
struct LedgerPeriodRow {
    id: i64,
    tenant_id: i64,
    year: i32,
    month: i32,
    period_type: String,
    status: String,
    berat_signed_at: Option<DateTime<Utc>>,
    sent_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    total_count: Option<i64>,
}

impl From<LedgerPeriodRow> for LedgerPeriod {
    fn from(row: LedgerPeriodRow) -> Self {
        let period_type = row.period_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid period_type '{}' in database: {}, defaulting to YevmiyeDefteri",
                row.period_type,
                e
            );
            LedgerType::YevmiyeDefteri
        });

        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid e-Defter status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            EDefterStatus::Draft
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            year: row.year,
            month: row.month as u32,
            period_type,
            status,
            berat_signed_at: row.berat_signed_at,
            sent_at: row.sent_at,
            created_at: row.created_at,
        }
    }
}

// ---------------------------------------------------------------------------
// YevmiyeEntryRow / YevmiyeEntry conversion
// ---------------------------------------------------------------------------

/// Database row representation for YevmiyeEntry
#[derive(Debug, FromRow)]
struct YevmiyeEntryRow {
    id: i64,
    period_id: i64,
    entry_number: i64,
    entry_date: NaiveDate,
    explanation: String,
    debit_total: Decimal,
    credit_total: Decimal,
    lines: serde_json::Value,
}

impl From<YevmiyeEntryRow> for YevmiyeEntry {
    fn from(row: YevmiyeEntryRow) -> Self {
        let lines: Vec<YevmiyeLine> = serde_json::from_value(row.lines).unwrap_or_default();

        Self {
            id: row.id,
            period_id: row.period_id,
            entry_number: row.entry_number,
            entry_date: row.entry_date,
            explanation: row.explanation,
            debit_total: row.debit_total,
            credit_total: row.credit_total,
            lines,
        }
    }
}

// ---------------------------------------------------------------------------
// BeratInfoRow / BeratInfo conversion
// ---------------------------------------------------------------------------

/// Database row representation for BeratInfo
#[derive(Debug, FromRow)]
struct BeratInfoRow {
    period_id: i64,
    serial_number: String,
    sign_time: DateTime<Utc>,
    signer: String,
    digest_value: String,
    signature_value: String,
}

impl From<BeratInfoRow> for BeratInfo {
    fn from(row: BeratInfoRow) -> Self {
        Self {
            period_id: row.period_id,
            serial_number: row.serial_number,
            sign_time: row.sign_time,
            signer: row.signer,
            digest_value: row.digest_value,
            signature_value: row.signature_value,
        }
    }
}

// ===========================================================================
// PostgresEDefterRepository
// ===========================================================================

/// PostgreSQL e-Defter repository
pub struct PostgresEDefterRepository {
    pool: Arc<PgPool>,
}

impl PostgresEDefterRepository {
    /// Create a new PostgreSQL e-Defter repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxEDefterRepository {
        Arc::new(self) as BoxEDefterRepository
    }
}

/// Common column list for ledger_periods SELECT queries
const LEDGER_PERIOD_COLUMNS: &str = r#"
    id, tenant_id, year, month, period_type, status,
    berat_signed_at, sent_at, created_at
"#;

/// Common column list for yevmiye_entries SELECT queries
const YEVMIYE_ENTRY_COLUMNS: &str = r#"
    id, period_id, entry_number, entry_date, explanation,
    debit_total, credit_total, lines
"#;

#[async_trait]
impl EDefterRepository for PostgresEDefterRepository {
    async fn create_period(&self, period: LedgerPeriod) -> Result<LedgerPeriod, ApiError> {
        let period_type = period.period_type.to_string();
        let status = period.status.to_string();

        let row: LedgerPeriodRow = sqlx::query_as(&format!(
            r#"
            INSERT INTO ledger_periods (tenant_id, year, month, period_type, status,
                                        berat_signed_at, sent_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING {LEDGER_PERIOD_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(period.tenant_id)
        .bind(period.year)
        .bind(period.month as i32)
        .bind(&period_type)
        .bind(&status)
        .bind(period.berat_signed_at)
        .bind(period.sent_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "LedgerPeriod"))?;

        Ok(row.into())
    }

    async fn find_period_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<LedgerPeriod>, ApiError> {
        let result: Option<LedgerPeriodRow> = sqlx::query_as(&format!(
            r#"
            SELECT {LEDGER_PERIOD_COLUMNS}, 0 as total_count
            FROM ledger_periods
            WHERE id = $1 AND tenant_id = $2
            "#,
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find ledger period: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_periods(
        &self,
        tenant_id: i64,
        year: Option<i32>,
        period_type: Option<LedgerType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<LedgerPeriod>, ApiError> {
        let offset = params.offset() as i64;
        let per_page = params.per_page as i64;

        match (year, period_type) {
            (Some(y), Some(pt)) => {
                let period_type_str = pt.to_string();
                let rows: Vec<LedgerPeriodRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {LEDGER_PERIOD_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM ledger_periods
                    WHERE tenant_id = $1 AND year = $2 AND period_type = $3
                    ORDER BY year DESC, month DESC
                    LIMIT $4 OFFSET $5
                    "#,
                ))
                .bind(tenant_id)
                .bind(y)
                .bind(&period_type_str)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "LedgerPeriod"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<LedgerPeriod> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            (Some(y), None) => {
                let rows: Vec<LedgerPeriodRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {LEDGER_PERIOD_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM ledger_periods
                    WHERE tenant_id = $1 AND year = $2
                    ORDER BY year DESC, month DESC
                    LIMIT $3 OFFSET $4
                    "#,
                ))
                .bind(tenant_id)
                .bind(y)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "LedgerPeriod"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<LedgerPeriod> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            (None, Some(pt)) => {
                let period_type_str = pt.to_string();
                let rows: Vec<LedgerPeriodRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {LEDGER_PERIOD_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM ledger_periods
                    WHERE tenant_id = $1 AND period_type = $2
                    ORDER BY year DESC, month DESC
                    LIMIT $3 OFFSET $4
                    "#,
                ))
                .bind(tenant_id)
                .bind(&period_type_str)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "LedgerPeriod"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<LedgerPeriod> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            (None, None) => {
                let rows: Vec<LedgerPeriodRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {LEDGER_PERIOD_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM ledger_periods
                    WHERE tenant_id = $1
                    ORDER BY year DESC, month DESC
                    LIMIT $2 OFFSET $3
                    "#,
                ))
                .bind(tenant_id)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "LedgerPeriod"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<LedgerPeriod> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
        }
    }

    async fn update_period_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: EDefterStatus,
    ) -> Result<LedgerPeriod, ApiError> {
        let status_str = status.to_string();

        // Update berat_signed_at when status becomes Signed
        let row: LedgerPeriodRow = sqlx::query_as(&format!(
            r#"
            UPDATE ledger_periods
            SET status = $1,
                berat_signed_at = CASE WHEN $1 = 'Signed' THEN NOW() ELSE berat_signed_at END,
                sent_at = CASE WHEN $1 = 'Sent' THEN NOW() ELSE sent_at END
            WHERE id = $2 AND tenant_id = $3
            RETURNING {LEDGER_PERIOD_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(&status_str)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "LedgerPeriod"))?;

        Ok(row.into())
    }

    async fn add_entry(&self, entry: YevmiyeEntry) -> Result<YevmiyeEntry, ApiError> {
        let lines_json =
            serde_json::to_value(&entry.lines).unwrap_or(serde_json::Value::Array(vec![]));

        let row: YevmiyeEntryRow = sqlx::query_as(&format!(
            r#"
            INSERT INTO yevmiye_entries (period_id, entry_number, entry_date, explanation,
                                         debit_total, credit_total, lines)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING {YEVMIYE_ENTRY_COLUMNS}
            "#,
        ))
        .bind(entry.period_id)
        .bind(entry.entry_number)
        .bind(entry.entry_date)
        .bind(&entry.explanation)
        .bind(entry.debit_total)
        .bind(entry.credit_total)
        .bind(lines_json)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "YevmiyeEntry"))?;

        Ok(row.into())
    }

    async fn find_entries(&self, period_id: i64) -> Result<Vec<YevmiyeEntry>, ApiError> {
        let rows: Vec<YevmiyeEntryRow> = sqlx::query_as(&format!(
            r#"
            SELECT {YEVMIYE_ENTRY_COLUMNS}
            FROM yevmiye_entries
            WHERE period_id = $1
            ORDER BY entry_number ASC
            "#,
        ))
        .bind(period_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find yevmiye entries: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_berat(&self, period_id: i64, berat: BeratInfo) -> Result<(), ApiError> {
        // UPSERT: insert or update berat info for the period
        let result = sqlx::query(
            r#"
            INSERT INTO berat_info (period_id, serial_number, sign_time, signer, digest_value, signature_value)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (period_id)
            DO UPDATE SET serial_number = $2, sign_time = $3, signer = $4,
                          digest_value = $5, signature_value = $6
            "#,
        )
        .bind(berat.period_id)
        .bind(&berat.serial_number)
        .bind(berat.sign_time)
        .bind(&berat.signer)
        .bind(&berat.digest_value)
        .bind(&berat.signature_value)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "BeratInfo"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Ledger period {} not found for berat update",
                period_id
            )));
        }

        Ok(())
    }

    async fn get_berat(&self, period_id: i64) -> Result<Option<BeratInfo>, ApiError> {
        let result: Option<BeratInfoRow> = sqlx::query_as(
            r#"
            SELECT period_id, serial_number, sign_time, signer, digest_value, signature_value
            FROM berat_info
            WHERE period_id = $1
            "#,
        )
        .bind(period_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get berat info: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }
}
