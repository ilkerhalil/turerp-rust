//! PostgreSQL SGK repository implementation
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::hr::sgk::model::{EmployeeBonus, SgkConfig, SgkEmployeeRegistration};
use crate::domain::hr::sgk::repository::{
    BoxEmployeeBonusRepository, BoxSgkConfigRepository, BoxSgkEmployeeRegistrationRepository,
    EmployeeBonusRepository, SgkConfigRepository, SgkEmployeeRegistrationRepository,
};
use crate::error::ApiError;

// --- SgkEmployeeRegistration ---

#[derive(Debug, FromRow)]
struct SgkEmployeeRegistrationRow {
    id: i64,
    employee_id: i64,
    tenant_id: i64,
    tc_kimlik_no: String,
    sgk_sicil_no: String,
    workplace_code: String,
    profession_code: String,
    registration_date: chrono::DateTime<chrono::Utc>,
    termination_date: Option<chrono::DateTime<chrono::Utc>>,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<SgkEmployeeRegistrationRow> for SgkEmployeeRegistration {
    fn from(row: SgkEmployeeRegistrationRow) -> Self {
        Self {
            id: row.id,
            employee_id: row.employee_id,
            tenant_id: row.tenant_id,
            tc_kimlik_no: row.tc_kimlik_no,
            sgk_sicil_no: row.sgk_sicil_no,
            workplace_code: row.workplace_code,
            profession_code: row.profession_code,
            registration_date: row.registration_date,
            termination_date: row.termination_date,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL SGK employee registration repository
pub struct PostgresSgkEmployeeRegistrationRepository {
    pool: Arc<PgPool>,
}

impl PostgresSgkEmployeeRegistrationRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxSgkEmployeeRegistrationRepository {
        Arc::new(self) as BoxSgkEmployeeRegistrationRepository
    }
}

#[async_trait]
impl SgkEmployeeRegistrationRepository for PostgresSgkEmployeeRegistrationRepository {
    async fn create(
        &self,
        reg: SgkEmployeeRegistration,
    ) -> Result<SgkEmployeeRegistration, ApiError> {
        let row: SgkEmployeeRegistrationRow = sqlx::query_as(
            r#"
            INSERT INTO sgk_employee_registrations (
                employee_id, tenant_id, tc_kimlik_no, sgk_sicil_no,
                workplace_code, profession_code, registration_date, termination_date, is_active, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
            RETURNING id, employee_id, tenant_id, tc_kimlik_no, sgk_sicil_no, workplace_code, profession_code,
                    registration_date, termination_date, is_active, created_at, updated_at
            "#,
        )
        .bind(reg.employee_id)
        .bind(reg.tenant_id)
        .bind(&reg.tc_kimlik_no)
        .bind(&reg.sgk_sicil_no)
        .bind(&reg.workplace_code)
        .bind(&reg.profession_code)
        .bind(reg.registration_date)
        .bind(reg.termination_date)
        .bind(reg.is_active)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SgkEmployeeRegistration"))?;

        Ok(row.into())
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<SgkEmployeeRegistration>, ApiError> {
        let result: Option<SgkEmployeeRegistrationRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, tenant_id, tc_kimlik_no, sgk_sicil_no, workplace_code, profession_code,
                   registration_date, termination_date, is_active, created_at, updated_at
            FROM sgk_employee_registrations
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find SGK registration: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Vec<SgkEmployeeRegistration>, ApiError> {
        let rows: Vec<SgkEmployeeRegistrationRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, tenant_id, tc_kimlik_no, sgk_sicil_no, workplace_code, profession_code,
                   registration_date, termination_date, is_active, created_at, updated_at
            FROM sgk_employee_registrations
            WHERE employee_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(employee_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find SGK registrations: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_active_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Option<SgkEmployeeRegistration>, ApiError> {
        let result: Option<SgkEmployeeRegistrationRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, tenant_id, tc_kimlik_no, sgk_sicil_no, workplace_code, profession_code,
                   registration_date, termination_date, is_active, created_at, updated_at
            FROM sgk_employee_registrations
            WHERE employee_id = $1 AND is_active = true
            ORDER BY registration_date DESC
            LIMIT 1
            "#,
        )
        .bind(employee_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find active SGK registration: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        reg: SgkEmployeeRegistration,
    ) -> Result<SgkEmployeeRegistration, ApiError> {
        let row: SgkEmployeeRegistrationRow = sqlx::query_as(
            r#"
            UPDATE sgk_employee_registrations
            SET
                employee_id = $1,
                tc_kimlik_no = $2,
                sgk_sicil_no = $3,
                workplace_code = $4,
                profession_code = $5,
                registration_date = $6,
                termination_date = $7,
                is_active = $8,
                updated_at = NOW()
            WHERE id = $9 AND tenant_id = $10
            RETURNING id, employee_id, tenant_id, tc_kimlik_no, sgk_sicil_no, workplace_code, profession_code,
                    registration_date, termination_date, is_active, created_at, updated_at
            "#,
        )
        .bind(reg.employee_id)
        .bind(&reg.tc_kimlik_no)
        .bind(&reg.sgk_sicil_no)
        .bind(&reg.workplace_code)
        .bind(&reg.profession_code)
        .bind(reg.registration_date)
        .bind(reg.termination_date)
        .bind(reg.is_active)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SgkEmployeeRegistration"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM sgk_employee_registrations
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete SGK registration: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("SGK registration not found".to_string()));
        }

        Ok(())
    }
}

// --- SgkConfig ---

#[derive(Debug, FromRow)]
struct SgkConfigRow {
    id: i64,
    tenant_id: i64,
    year: i32,
    min_wage: Decimal,
    sgk_earnings_ceiling: Decimal,
    sgk_worker_rate: Decimal,
    unemployment_worker_rate: Decimal,
    stamp_tax_rate: Decimal,
    agi_amount_single: Decimal,
    agi_amount_married: Decimal,
    agi_per_child: Decimal,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<SgkConfigRow> for SgkConfig {
    fn from(row: SgkConfigRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            year: row.year,
            min_wage: row.min_wage,
            sgk_earnings_ceiling: row.sgk_earnings_ceiling,
            sgk_worker_rate: row.sgk_worker_rate,
            unemployment_worker_rate: row.unemployment_worker_rate,
            stamp_tax_rate: row.stamp_tax_rate,
            agi_amount_single: row.agi_amount_single,
            agi_amount_married: row.agi_amount_married,
            agi_per_child: row.agi_per_child,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL SGK config repository
pub struct PostgresSgkConfigRepository {
    pool: Arc<PgPool>,
}

impl PostgresSgkConfigRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxSgkConfigRepository {
        Arc::new(self) as BoxSgkConfigRepository
    }
}

#[async_trait]
impl SgkConfigRepository for PostgresSgkConfigRepository {
    async fn create(&self, config: SgkConfig) -> Result<SgkConfig, ApiError> {
        let row: SgkConfigRow = sqlx::query_as(
            r#"
            INSERT INTO sgk_configs (
                tenant_id, year, min_wage, sgk_earnings_ceiling, sgk_worker_rate,
                unemployment_worker_rate, stamp_tax_rate, agi_amount_single,
                agi_amount_married, agi_per_child, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
            RETURNING id, tenant_id, year, min_wage, sgk_earnings_ceiling, sgk_worker_rate,
                    unemployment_worker_rate, stamp_tax_rate, agi_amount_single,
                    agi_amount_married, agi_per_child, created_at, updated_at
            "#,
        )
        .bind(config.tenant_id)
        .bind(config.year)
        .bind(config.min_wage)
        .bind(config.sgk_earnings_ceiling)
        .bind(config.sgk_worker_rate)
        .bind(config.unemployment_worker_rate)
        .bind(config.stamp_tax_rate)
        .bind(config.agi_amount_single)
        .bind(config.agi_amount_married)
        .bind(config.agi_per_child)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SgkConfig"))?;

        Ok(row.into())
    }

    async fn find_by_tenant_and_year(
        &self,
        tenant_id: i64,
        year: i32,
    ) -> Result<Option<SgkConfig>, ApiError> {
        let result: Option<SgkConfigRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, year, min_wage, sgk_earnings_ceiling, sgk_worker_rate,
                   unemployment_worker_rate, stamp_tax_rate, agi_amount_single,
                   agi_amount_married, agi_per_child, created_at, updated_at
            FROM sgk_configs
            WHERE tenant_id = $1 AND year = $2
            "#,
        )
        .bind(tenant_id)
        .bind(year)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find SGK config: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        config: SgkConfig,
    ) -> Result<SgkConfig, ApiError> {
        let row: SgkConfigRow = sqlx::query_as(
            r#"
            UPDATE sgk_configs
            SET
                min_wage = $1,
                sgk_earnings_ceiling = $2,
                sgk_worker_rate = $3,
                unemployment_worker_rate = $4,
                stamp_tax_rate = $5,
                agi_amount_single = $6,
                agi_amount_married = $7,
                agi_per_child = $8,
                updated_at = NOW()
            WHERE id = $9 AND tenant_id = $10
            RETURNING id, tenant_id, year, min_wage, sgk_earnings_ceiling, sgk_worker_rate,
                    unemployment_worker_rate, stamp_tax_rate, agi_amount_single,
                    agi_amount_married, agi_per_child, created_at, updated_at
            "#,
        )
        .bind(config.min_wage)
        .bind(config.sgk_earnings_ceiling)
        .bind(config.sgk_worker_rate)
        .bind(config.unemployment_worker_rate)
        .bind(config.stamp_tax_rate)
        .bind(config.agi_amount_single)
        .bind(config.agi_amount_married)
        .bind(config.agi_per_child)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SgkConfig"))?;

        Ok(row.into())
    }
}

// --- EmployeeBonus ---

#[derive(Debug, FromRow)]
struct EmployeeBonusRow {
    id: i64,
    employee_id: i64,
    tenant_id: i64,
    bonus_type: String,
    amount: Decimal,
    bonus_month: i32,
    bonus_year: i32,
    description: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<EmployeeBonusRow> for EmployeeBonus {
    fn from(row: EmployeeBonusRow) -> Self {
        Self {
            id: row.id,
            employee_id: row.employee_id,
            tenant_id: row.tenant_id,
            bonus_type: row.bonus_type,
            amount: row.amount,
            bonus_month: row.bonus_month,
            bonus_year: row.bonus_year,
            description: row.description,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL employee bonus repository
pub struct PostgresEmployeeBonusRepository {
    pool: Arc<PgPool>,
}

impl PostgresEmployeeBonusRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxEmployeeBonusRepository {
        Arc::new(self) as BoxEmployeeBonusRepository
    }
}

#[async_trait]
impl EmployeeBonusRepository for PostgresEmployeeBonusRepository {
    async fn create(&self, bonus: EmployeeBonus) -> Result<EmployeeBonus, ApiError> {
        let row: EmployeeBonusRow = sqlx::query_as(
            r#"
            INSERT INTO employee_bonuses (
                employee_id, tenant_id, bonus_type, amount, bonus_month, bonus_year, description, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING id, employee_id, tenant_id, bonus_type, amount, bonus_month, bonus_year, description, created_at
            "#,
        )
        .bind(bonus.employee_id)
        .bind(bonus.tenant_id)
        .bind(&bonus.bonus_type)
        .bind(bonus.amount)
        .bind(bonus.bonus_month)
        .bind(bonus.bonus_year)
        .bind(&bonus.description)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "EmployeeBonus"))?;

        Ok(row.into())
    }

    async fn find_by_employee_and_year_month(
        &self,
        employee_id: i64,
        year: i32,
        month: i32,
    ) -> Result<Vec<EmployeeBonus>, ApiError> {
        let rows: Vec<EmployeeBonusRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, tenant_id, bonus_type, amount, bonus_month, bonus_year, description, created_at
            FROM employee_bonuses
            WHERE employee_id = $1 AND bonus_year = $2 AND bonus_month = $3
            ORDER BY created_at DESC
            "#,
        )
        .bind(employee_id)
        .bind(year)
        .bind(month)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find employee bonuses: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM employee_bonuses
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete employee bonus: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Employee bonus not found".to_string()));
        }

        Ok(())
    }
}
