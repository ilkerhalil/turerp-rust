//! PostgreSQL CRM repository implementations

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::domain::crm::model::{
    Campaign, CampaignStatus, CreateCampaign, CreateLead, CreateOpportunity, CreateTicket, Lead,
    LeadStatus, Opportunity, OpportunityStatus, Ticket, TicketPriority, TicketStatus,
};
use crate::domain::crm::repository::{
    BoxCampaignRepository, BoxLeadRepository, BoxOpportunityRepository, BoxTicketRepository,
    CampaignRepository, LeadRepository, OpportunityRepository, TicketRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types
fn map_sqlx_error(e: sqlx::Error, entity: &str) -> ApiError {
    match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("{} not found", entity)),
        _ => {
            let msg = e.to_string();
            if msg.contains("duplicate key") || msg.contains("unique constraint") {
                ApiError::Conflict(format!("{} already exists", entity))
            } else {
                ApiError::Database(format!("Failed to operate on {}: {}", entity, e))
            }
        }
    }
}

// ==================== LEAD ====================

/// Database row representation for Lead
#[derive(Debug, FromRow)]
struct LeadRow {
    id: i64,
    tenant_id: i64,
    name: String,
    company: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    source: String,
    status: String,
    assigned_to: Option<i64>,
    converted_to_customer_id: Option<i64>,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<LeadRow> for Lead {
    fn from(row: LeadRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}' in database: {}, defaulting to New",
                row.status,
                e
            );
            LeadStatus::New
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            company: row.company,
            email: row.email,
            phone: row.phone,
            source: row.source,
            status,
            assigned_to: row.assigned_to,
            converted_to_customer_id: row.converted_to_customer_id,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL lead repository
pub struct PostgresLeadRepository {
    pool: Arc<PgPool>,
}

impl PostgresLeadRepository {
    /// Create a new PostgreSQL lead repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxLeadRepository {
        Arc::new(self) as BoxLeadRepository
    }
}

#[async_trait]
impl LeadRepository for PostgresLeadRepository {
    async fn create(&self, create: CreateLead) -> Result<Lead, ApiError> {
        let status = LeadStatus::New.to_string();

        let row: LeadRow = sqlx::query_as(
            r#"
            INSERT INTO leads (tenant_id, name, company, email, phone, source,
                              status, assigned_to, converted_to_customer_id, notes, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, $9, NOW(), NOW())
            RETURNING id, tenant_id, name, company, email, phone, source,
                      status, assigned_to, converted_to_customer_id, notes, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.name)
        .bind(&create.company)
        .bind(&create.email)
        .bind(&create.phone)
        .bind(&create.source)
        .bind(&status)
        .bind(create.assigned_to)
        .bind(&create.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Lead"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Lead>, ApiError> {
        let result: Option<LeadRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, company, email, phone, source,
                   status, assigned_to, converted_to_customer_id, notes, created_at, updated_at
            FROM leads
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find lead by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Lead>, ApiError> {
        let rows: Vec<LeadRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, company, email, phone, source,
                   status, assigned_to, converted_to_customer_id, notes, created_at, updated_at
            FROM leads
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find leads by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: LeadStatus,
    ) -> Result<Vec<Lead>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<LeadRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, company, email, phone, source,
                   status, assigned_to, converted_to_customer_id, notes, created_at, updated_at
            FROM leads
            WHERE tenant_id = $1 AND status = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find leads by status: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(&self, id: i64, status: LeadStatus) -> Result<Lead, ApiError> {
        let status_str = status.to_string();

        let row: LeadRow = sqlx::query_as(
            r#"
            UPDATE leads
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, tenant_id, name, company, email, phone, source,
                      status, assigned_to, converted_to_customer_id, notes, created_at, updated_at
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Lead"))?;

        Ok(row.into())
    }

    async fn convert_to_customer(&self, id: i64, customer_id: i64) -> Result<Lead, ApiError> {
        let row: LeadRow = sqlx::query_as(
            r#"
            UPDATE leads
            SET status = 'Converted', converted_to_customer_id = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, tenant_id, name, company, email, phone, source,
                      status, assigned_to, converted_to_customer_id, notes, created_at, updated_at
            "#,
        )
        .bind(customer_id)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Lead"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM leads
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete lead: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Lead not found".to_string()));
        }

        Ok(())
    }
}

// ==================== OPPORTUNITY ====================

/// Database row representation for Opportunity
#[derive(Debug, FromRow)]
struct OpportunityRow {
    id: i64,
    tenant_id: i64,
    lead_id: Option<i64>,
    name: String,
    customer_id: Option<i64>,
    value: Decimal,
    probability: Decimal,
    expected_close_date: Option<chrono::DateTime<chrono::Utc>>,
    status: String,
    assigned_to: Option<i64>,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<OpportunityRow> for Opportunity {
    fn from(row: OpportunityRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}' in database: {}, defaulting to Open",
                row.status,
                e
            );
            OpportunityStatus::Open
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            lead_id: row.lead_id,
            name: row.name,
            customer_id: row.customer_id,
            value: row.value,
            probability: row.probability,
            expected_close_date: row.expected_close_date,
            status,
            assigned_to: row.assigned_to,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL opportunity repository
pub struct PostgresOpportunityRepository {
    pool: Arc<PgPool>,
}

impl PostgresOpportunityRepository {
    /// Create a new PostgreSQL opportunity repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxOpportunityRepository {
        Arc::new(self) as BoxOpportunityRepository
    }
}

#[async_trait]
impl OpportunityRepository for PostgresOpportunityRepository {
    async fn create(&self, create: CreateOpportunity) -> Result<Opportunity, ApiError> {
        let status = OpportunityStatus::Open.to_string();

        let row: OpportunityRow = sqlx::query_as(
            r#"
            INSERT INTO opportunities (tenant_id, lead_id, name, customer_id, value, probability,
                                        expected_close_date, status, assigned_to, notes, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
            RETURNING id, tenant_id, lead_id, name, customer_id, value, probability,
                      expected_close_date, status, assigned_to, notes, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.lead_id)
        .bind(&create.name)
        .bind(create.customer_id)
        .bind(create.value)
        .bind(create.probability)
        .bind(create.expected_close_date)
        .bind(&status)
        .bind(create.assigned_to)
        .bind(&create.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Opportunity"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Opportunity>, ApiError> {
        let result: Option<OpportunityRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, lead_id, name, customer_id, value, probability,
                   expected_close_date, status, assigned_to, notes, created_at, updated_at
            FROM opportunities
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find opportunity by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Opportunity>, ApiError> {
        let rows: Vec<OpportunityRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, lead_id, name, customer_id, value, probability,
                   expected_close_date, status, assigned_to, notes, created_at, updated_at
            FROM opportunities
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find opportunities by tenant: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: OpportunityStatus,
    ) -> Result<Vec<Opportunity>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<OpportunityRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, lead_id, name, customer_id, value, probability,
                   expected_close_date, status, assigned_to, notes, created_at, updated_at
            FROM opportunities
            WHERE tenant_id = $1 AND status = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find opportunities by status: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_customer(&self, customer_id: i64) -> Result<Vec<Opportunity>, ApiError> {
        let rows: Vec<OpportunityRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, lead_id, name, customer_id, value, probability,
                   expected_close_date, status, assigned_to, notes, created_at, updated_at
            FROM opportunities
            WHERE customer_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(customer_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find opportunities by customer: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(
        &self,
        id: i64,
        status: OpportunityStatus,
    ) -> Result<Opportunity, ApiError> {
        let status_str = status.to_string();

        let row: OpportunityRow = sqlx::query_as(
            r#"
            UPDATE opportunities
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, tenant_id, lead_id, name, customer_id, value, probability,
                      expected_close_date, status, assigned_to, notes, created_at, updated_at
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Opportunity"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM opportunities
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete opportunity: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Opportunity not found".to_string()));
        }

        Ok(())
    }
}

// ==================== CAMPAIGN ====================

/// Database row representation for Campaign
#[derive(Debug, FromRow)]
struct CampaignRow {
    id: i64,
    tenant_id: i64,
    name: String,
    description: Option<String>,
    campaign_type: String,
    status: String,
    budget: Decimal,
    actual_cost: Decimal,
    start_date: Option<chrono::DateTime<chrono::Utc>>,
    end_date: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<CampaignRow> for Campaign {
    fn from(row: CampaignRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            CampaignStatus::Draft
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            description: row.description,
            campaign_type: row.campaign_type,
            status,
            budget: row.budget,
            actual_cost: row.actual_cost,
            start_date: row.start_date,
            end_date: row.end_date,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL campaign repository
pub struct PostgresCampaignRepository {
    pool: Arc<PgPool>,
}

impl PostgresCampaignRepository {
    /// Create a new PostgreSQL campaign repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxCampaignRepository {
        Arc::new(self) as BoxCampaignRepository
    }
}

#[async_trait]
impl CampaignRepository for PostgresCampaignRepository {
    async fn create(&self, create: CreateCampaign) -> Result<Campaign, ApiError> {
        let status = CampaignStatus::Draft.to_string();

        let row: CampaignRow = sqlx::query_as(
            r#"
            INSERT INTO campaigns (tenant_id, name, description, campaign_type, status,
                                   budget, actual_cost, start_date, end_date, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
            RETURNING id, tenant_id, name, description, campaign_type, status,
                      budget, actual_cost, start_date, end_date, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.name)
        .bind(&create.description)
        .bind(&create.campaign_type)
        .bind(&status)
        .bind(create.budget)
        .bind(Decimal::ZERO)
        .bind(create.start_date)
        .bind(create.end_date)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Campaign"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Campaign>, ApiError> {
        let result: Option<CampaignRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, campaign_type, status,
                   budget, actual_cost, start_date, end_date, created_at, updated_at
            FROM campaigns
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find campaign by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Campaign>, ApiError> {
        let rows: Vec<CampaignRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, campaign_type, status,
                   budget, actual_cost, start_date, end_date, created_at, updated_at
            FROM campaigns
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find campaigns by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: CampaignStatus,
    ) -> Result<Vec<Campaign>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<CampaignRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, campaign_type, status,
                   budget, actual_cost, start_date, end_date, created_at, updated_at
            FROM campaigns
            WHERE tenant_id = $1 AND status = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find campaigns by status: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(&self, id: i64, status: CampaignStatus) -> Result<Campaign, ApiError> {
        let status_str = status.to_string();

        let row: CampaignRow = sqlx::query_as(
            r#"
            UPDATE campaigns
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, tenant_id, name, description, campaign_type, status,
                      budget, actual_cost, start_date, end_date, created_at, updated_at
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Campaign"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM campaigns
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete campaign: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Campaign not found".to_string()));
        }

        Ok(())
    }
}

// ==================== TICKET ====================

/// Database row representation for Ticket
#[derive(Debug, FromRow)]
struct TicketRow {
    id: i64,
    tenant_id: i64,
    ticket_number: String,
    subject: String,
    description: String,
    customer_id: Option<i64>,
    assigned_to: Option<i64>,
    status: String,
    priority: String,
    category: Option<String>,
    resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<TicketRow> for Ticket {
    fn from(row: TicketRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}' in database: {}, defaulting to Open",
                row.status,
                e
            );
            TicketStatus::Open
        });

        let priority = row.priority.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid priority '{}' in database: {}, defaulting to Medium",
                row.priority,
                e
            );
            TicketPriority::Medium
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            ticket_number: row.ticket_number,
            subject: row.subject,
            description: row.description,
            customer_id: row.customer_id,
            assigned_to: row.assigned_to,
            status,
            priority,
            category: row.category,
            resolved_at: row.resolved_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL ticket repository
pub struct PostgresTicketRepository {
    pool: Arc<PgPool>,
}

impl PostgresTicketRepository {
    /// Create a new PostgreSQL ticket repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxTicketRepository {
        Arc::new(self) as BoxTicketRepository
    }
}

#[async_trait]
impl TicketRepository for PostgresTicketRepository {
    async fn create(&self, create: CreateTicket) -> Result<Ticket, ApiError> {
        let ticket_number = format!("TKT-{}", chrono::Utc::now().timestamp());
        let status = TicketStatus::Open.to_string();
        let priority = create.priority.to_string();

        let row: TicketRow = sqlx::query_as(
            r#"
            INSERT INTO tickets (tenant_id, ticket_number, subject, description,
                                 customer_id, assigned_to, status, priority, category,
                                 resolved_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL, NOW(), NOW())
            RETURNING id, tenant_id, ticket_number, subject, description,
                      customer_id, assigned_to, status, priority, category,
                      resolved_at, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&ticket_number)
        .bind(&create.subject)
        .bind(&create.description)
        .bind(create.customer_id)
        .bind(create.assigned_to)
        .bind(&status)
        .bind(&priority)
        .bind(&create.category)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Ticket"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Ticket>, ApiError> {
        let result: Option<TicketRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, ticket_number, subject, description,
                   customer_id, assigned_to, status, priority, category,
                   resolved_at, created_at, updated_at
            FROM tickets
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find ticket by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Ticket>, ApiError> {
        let rows: Vec<TicketRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, ticket_number, subject, description,
                   customer_id, assigned_to, status, priority, category,
                   resolved_at, created_at, updated_at
            FROM tickets
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find tickets by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_number(
        &self,
        tenant_id: i64,
        ticket_number: &str,
    ) -> Result<Option<Ticket>, ApiError> {
        let result: Option<TicketRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, ticket_number, subject, description,
                   customer_id, assigned_to, status, priority, category,
                   resolved_at, created_at, updated_at
            FROM tickets
            WHERE tenant_id = $1 AND ticket_number = $2
            "#,
        )
        .bind(tenant_id)
        .bind(ticket_number)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find ticket by number: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: TicketStatus,
    ) -> Result<Vec<Ticket>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<TicketRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, ticket_number, subject, description,
                   customer_id, assigned_to, status, priority, category,
                   resolved_at, created_at, updated_at
            FROM tickets
            WHERE tenant_id = $1 AND status = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find tickets by status: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_assignee(&self, assignee_id: i64) -> Result<Vec<Ticket>, ApiError> {
        let rows: Vec<TicketRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, ticket_number, subject, description,
                   customer_id, assigned_to, status, priority, category,
                   resolved_at, created_at, updated_at
            FROM tickets
            WHERE assigned_to = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(assignee_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find tickets by assignee: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(&self, id: i64, status: TicketStatus) -> Result<Ticket, ApiError> {
        let status_str = status.to_string();

        let row: TicketRow = sqlx::query_as(
            r#"
            UPDATE tickets
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, tenant_id, ticket_number, subject, description,
                      customer_id, assigned_to, status, priority, category,
                      resolved_at, created_at, updated_at
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Ticket"))?;

        Ok(row.into())
    }

    async fn resolve(&self, id: i64) -> Result<Ticket, ApiError> {
        let row: TicketRow = sqlx::query_as(
            r#"
            UPDATE tickets
            SET status = 'Resolved', resolved_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING id, tenant_id, ticket_number, subject, description,
                      customer_id, assigned_to, status, priority, category,
                      resolved_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Ticket"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM tickets
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete ticket: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Ticket not found".to_string()));
        }

        Ok(())
    }
}
