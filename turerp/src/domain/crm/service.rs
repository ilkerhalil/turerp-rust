//! CRM service for business logic

use rust_decimal::Decimal;

use crate::common::pagination::PaginatedResult;
use crate::domain::crm::model::{
    Campaign, CampaignStatus, CreateCampaign, CreateLead, CreateOpportunity, CreateTicket, Lead,
    LeadStatus, Opportunity, OpportunityStatus, Ticket, TicketStatus,
};
use crate::domain::crm::repository::{
    BoxCampaignRepository, BoxLeadRepository, BoxOpportunityRepository, BoxTicketRepository,
};
use crate::domain::user::repository::BoxUserRepository;
use crate::domain::user::service::ensure_user_owned;
use crate::error::ApiError;

#[derive(Clone)]
pub struct CrmService {
    lead_repo: BoxLeadRepository,
    opportunity_repo: BoxOpportunityRepository,
    campaign_repo: BoxCampaignRepository,
    ticket_repo: BoxTicketRepository,
    user_repo: BoxUserRepository,
}

impl CrmService {
    pub fn new(
        lead_repo: BoxLeadRepository,
        opportunity_repo: BoxOpportunityRepository,
        campaign_repo: BoxCampaignRepository,
        ticket_repo: BoxTicketRepository,
        user_repo: BoxUserRepository,
    ) -> Self {
        Self {
            lead_repo,
            opportunity_repo,
            campaign_repo,
            ticket_repo,
            user_repo,
        }
    }

    // Lead methods
    #[tracing::instrument(skip(self))]
    pub async fn create_lead(&self, create: CreateLead) -> Result<Lead, ApiError> {
        // Parent-ownership precheck: a body-supplied `assigned_to` user id
        // must belong to the caller's tenant. `None` is a legitimate
        // "unassigned" lead and is NOT rejected.
        if let Some(id) = create.assigned_to {
            ensure_user_owned(&self.user_repo, id, create.tenant_id).await?;
        }
        self.lead_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_lead(&self, id: i64, tenant_id: i64) -> Result<Lead, ApiError> {
        self.lead_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Lead not found".to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_leads_by_tenant(&self, tenant_id: i64) -> Result<Vec<Lead>, ApiError> {
        self.lead_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_leads_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Lead>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.lead_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_leads_by_status(
        &self,
        tenant_id: i64,
        status: LeadStatus,
    ) -> Result<Vec<Lead>, ApiError> {
        self.lead_repo.find_by_status(tenant_id, status).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_leads_by_status_paginated(
        &self,
        tenant_id: i64,
        status: LeadStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Lead>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.lead_repo
            .find_by_status_paginated(tenant_id, status, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_lead_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: LeadStatus,
    ) -> Result<Lead, ApiError> {
        self.lead_repo.update_status(id, tenant_id, status).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn convert_lead_to_customer(
        &self,
        id: i64,
        tenant_id: i64,
        customer_id: i64,
    ) -> Result<Lead, ApiError> {
        self.lead_repo
            .convert_to_customer(id, tenant_id, customer_id)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_lead(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.lead_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_lead(&self, id: i64, tenant_id: i64) -> Result<Lead, ApiError> {
        self.lead_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_leads(&self, tenant_id: i64) -> Result<Vec<Lead>, ApiError> {
        self.lead_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_lead(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.lead_repo.destroy(id, tenant_id).await
    }

    // Opportunity methods
    #[tracing::instrument(skip(self))]
    pub async fn create_opportunity(
        &self,
        create: CreateOpportunity,
    ) -> Result<Opportunity, ApiError> {
        // Parent-ownership precheck: a body-supplied `assigned_to` user id
        // must belong to the caller's tenant. `None` is a legitimate
        // "unassigned" opportunity and is NOT rejected.
        if let Some(id) = create.assigned_to {
            ensure_user_owned(&self.user_repo, id, create.tenant_id).await?;
        }
        self.opportunity_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_opportunity(&self, id: i64, tenant_id: i64) -> Result<Opportunity, ApiError> {
        self.opportunity_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Opportunity not found".to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_opportunities_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Opportunity>, ApiError> {
        self.opportunity_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_opportunities_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Opportunity>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.opportunity_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_opportunities_by_status_paginated(
        &self,
        tenant_id: i64,
        status: OpportunityStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Opportunity>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.opportunity_repo
            .find_by_status_paginated(tenant_id, status, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_opportunities_by_status(
        &self,
        tenant_id: i64,
        status: OpportunityStatus,
    ) -> Result<Vec<Opportunity>, ApiError> {
        self.opportunity_repo
            .find_by_status(tenant_id, status)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_opportunities_by_customer(
        &self,
        customer_id: i64,
    ) -> Result<Vec<Opportunity>, ApiError> {
        self.opportunity_repo.find_by_customer(customer_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_opportunity_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: OpportunityStatus,
    ) -> Result<Opportunity, ApiError> {
        self.opportunity_repo
            .update_status(id, tenant_id, status)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_opportunity(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.opportunity_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_opportunity(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Opportunity, ApiError> {
        self.opportunity_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_opportunities(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Opportunity>, ApiError> {
        self.opportunity_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_opportunity(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.opportunity_repo.destroy(id, tenant_id).await
    }

    // Campaign methods
    #[tracing::instrument(skip(self))]
    pub async fn create_campaign(&self, create: CreateCampaign) -> Result<Campaign, ApiError> {
        self.campaign_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_campaign(&self, id: i64, tenant_id: i64) -> Result<Campaign, ApiError> {
        self.campaign_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Campaign not found".to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_campaigns_by_tenant(&self, tenant_id: i64) -> Result<Vec<Campaign>, ApiError> {
        self.campaign_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_campaigns_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Campaign>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.campaign_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_campaigns_by_status_paginated(
        &self,
        tenant_id: i64,
        status: CampaignStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Campaign>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.campaign_repo
            .find_by_status_paginated(tenant_id, status, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_campaigns_by_status(
        &self,
        tenant_id: i64,
        status: CampaignStatus,
    ) -> Result<Vec<Campaign>, ApiError> {
        self.campaign_repo.find_by_status(tenant_id, status).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_campaign_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: CampaignStatus,
    ) -> Result<Campaign, ApiError> {
        self.campaign_repo
            .update_status(id, tenant_id, status)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_campaign(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.campaign_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_campaign(&self, id: i64, tenant_id: i64) -> Result<Campaign, ApiError> {
        self.campaign_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_campaigns(&self, tenant_id: i64) -> Result<Vec<Campaign>, ApiError> {
        self.campaign_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_campaign(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.campaign_repo.destroy(id, tenant_id).await
    }

    // Ticket methods
    #[tracing::instrument(skip(self))]
    pub async fn create_ticket(&self, create: CreateTicket) -> Result<Ticket, ApiError> {
        // Parent-ownership precheck: a body-supplied `assigned_to` user id
        // must belong to the caller's tenant. `None` is a legitimate
        // "unassigned" ticket and is NOT rejected.
        if let Some(id) = create.assigned_to {
            ensure_user_owned(&self.user_repo, id, create.tenant_id).await?;
        }
        self.ticket_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_ticket(&self, id: i64, tenant_id: i64) -> Result<Ticket, ApiError> {
        self.ticket_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Ticket not found".to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_ticket_by_number(
        &self,
        tenant_id: i64,
        ticket_number: &str,
    ) -> Result<Option<Ticket>, ApiError> {
        self.ticket_repo
            .find_by_number(tenant_id, ticket_number)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_tickets_by_tenant(&self, tenant_id: i64) -> Result<Vec<Ticket>, ApiError> {
        self.ticket_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_tickets_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Ticket>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.ticket_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_tickets_by_status_paginated(
        &self,
        tenant_id: i64,
        status: TicketStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Ticket>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.ticket_repo
            .find_by_status_paginated(tenant_id, status, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_tickets_by_status(
        &self,
        tenant_id: i64,
        status: TicketStatus,
    ) -> Result<Vec<Ticket>, ApiError> {
        self.ticket_repo.find_by_status(tenant_id, status).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_tickets_by_assignee(&self, assignee_id: i64) -> Result<Vec<Ticket>, ApiError> {
        self.ticket_repo.find_by_assignee(assignee_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_ticket_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: TicketStatus,
    ) -> Result<Ticket, ApiError> {
        self.ticket_repo.update_status(id, tenant_id, status).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn resolve_ticket(&self, id: i64, tenant_id: i64) -> Result<Ticket, ApiError> {
        self.ticket_repo.resolve(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_ticket(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.ticket_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_ticket(&self, id: i64, tenant_id: i64) -> Result<Ticket, ApiError> {
        self.ticket_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_tickets(&self, tenant_id: i64) -> Result<Vec<Ticket>, ApiError> {
        self.ticket_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_ticket(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.ticket_repo.destroy(id, tenant_id).await
    }

    // Dashboard metrics
    #[tracing::instrument(skip(self))]
    pub async fn get_sales_pipeline_value(&self, tenant_id: i64) -> Result<Decimal, ApiError> {
        let opportunities = self.opportunity_repo.find_by_tenant(tenant_id).await?;
        let open_opps: Vec<_> = opportunities
            .into_iter()
            .filter(|o| o.status == OpportunityStatus::Open)
            .collect();
        let weighted_value: Decimal = open_opps
            .iter()
            .map(|o| o.value * (o.probability / Decimal::ONE_HUNDRED))
            .sum();
        Ok(weighted_value)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_open_tickets_count(&self, tenant_id: i64) -> Result<usize, ApiError> {
        let tickets = self.ticket_repo.find_by_tenant(tenant_id).await?;
        let open_count = tickets
            .iter()
            .filter(|t| t.status == TicketStatus::Open || t.status == TicketStatus::InProgress)
            .count();
        Ok(open_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::crm::model::TicketPriority;
    use crate::domain::crm::repository::{
        InMemoryCampaignRepository, InMemoryLeadRepository, InMemoryOpportunityRepository,
        InMemoryTicketRepository,
    };
    use crate::domain::user::model::{CreateUser, Role};
    use crate::domain::user::repository::InMemoryUserRepository;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    async fn create_service() -> CrmService {
        let lead_repo = Arc::new(InMemoryLeadRepository::new()) as BoxLeadRepository;
        let opp_repo = Arc::new(InMemoryOpportunityRepository::new()) as BoxOpportunityRepository;
        let campaign_repo = Arc::new(InMemoryCampaignRepository::new()) as BoxCampaignRepository;
        let ticket_repo = Arc::new(InMemoryTicketRepository::new()) as BoxTicketRepository;
        let user_repo = Arc::new(InMemoryUserRepository::new()) as BoxUserRepository;
        // Seed a tenant-1 user (auto-id 1) so the existing happy-path tests that
        // stamp `assigned_to: Some(1)` resolve against the caller's tenant.
        user_repo
            .create(
                CreateUser {
                    username: "t1user".to_string(),
                    email: "t1@example.com".to_string(),
                    full_name: "Tenant 1 user".to_string(),
                    password: "password123456".to_string(),
                    tenant_id: 1,
                    role: Some(Role::User),
                },
                "hash".to_string(),
            )
            .await
            .unwrap();
        // Seed a tenant-2 user (auto-id 2) used as the foreign referent by the
        // reject tests below.
        user_repo
            .create(
                CreateUser {
                    username: "t2user".to_string(),
                    email: "t2@example.com".to_string(),
                    full_name: "Tenant 2 user".to_string(),
                    password: "password123456".to_string(),
                    tenant_id: 2,
                    role: Some(Role::User),
                },
                "hash".to_string(),
            )
            .await
            .unwrap();
        CrmService::new(lead_repo, opp_repo, campaign_repo, ticket_repo, user_repo)
    }

    #[tokio::test]
    async fn test_create_lead() {
        let service = create_service().await;
        let create = CreateLead {
            tenant_id: 1,
            name: "John Doe".to_string(),
            company: Some("Acme".to_string()),
            email: Some("john@acme.com".to_string()),
            phone: None,
            source: "Website".to_string(),
            assigned_to: None,
            notes: None,
        };
        let result = service.create_lead(create).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, LeadStatus::New);
    }

    #[tokio::test]
    async fn test_create_opportunity() {
        let service = create_service().await;
        let create = CreateOpportunity {
            tenant_id: 1,
            lead_id: None,
            name: "Big Deal".to_string(),
            customer_id: Some(1),
            value: dec!(50000),
            probability: dec!(75),
            expected_close_date: Some(chrono::Utc::now()),
            assigned_to: None,
            notes: None,
        };
        let result = service.create_opportunity(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_campaign() {
        let service = create_service().await;
        let create = CreateCampaign {
            tenant_id: 1,
            name: "Summer Sale".to_string(),
            description: Some("Annual campaign".to_string()),
            campaign_type: "Email".to_string(),
            budget: dec!(10000),
            start_date: Some(chrono::Utc::now()),
            end_date: None,
        };
        let result = service.create_campaign(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_ticket() {
        let service = create_service().await;
        let create = CreateTicket {
            tenant_id: 1,
            subject: "Login issue".to_string(),
            description: "Cannot login".to_string(),
            customer_id: Some(1),
            assigned_to: None,
            priority: TicketPriority::High,
            category: Some("Technical".to_string()),
        };
        let result = service.create_ticket(create).await;
        assert!(result.is_ok());
        assert!(result.unwrap().ticket_number.starts_with("TKT-"));
    }

    #[tokio::test]
    async fn test_sales_pipeline_value() {
        let service = create_service().await;

        // Create opportunity with 50% probability and $100,000 value
        service
            .create_opportunity(CreateOpportunity {
                tenant_id: 1,
                lead_id: None,
                name: "Deal 1".to_string(),
                customer_id: None,
                value: dec!(100000),
                probability: dec!(50),
                expected_close_date: None,
                assigned_to: None,
                notes: None,
            })
            .await
            .unwrap();

        let value = service.get_sales_pipeline_value(1).await.unwrap();
        assert_eq!(value, dec!(50000)); // 100000 * 50%
    }

    #[tokio::test]
    async fn test_resolve_ticket() {
        let service = create_service().await;
        let ticket = service
            .create_ticket(CreateTicket {
                tenant_id: 1,
                subject: "Issue".to_string(),
                description: "Problem".to_string(),
                customer_id: None,
                assigned_to: Some(1),
                priority: TicketPriority::Medium,
                category: None,
            })
            .await
            .unwrap();

        let resolved = service.resolve_ticket(ticket.id, 1).await.unwrap();
        assert_eq!(resolved.status, TicketStatus::Resolved);
        assert!(resolved.resolved_at.is_some());
    }

    // Cross-tenant IDOR guard: a tenant-1 caller must not be able to attribute
    // a lead/opportunity/ticket to a tenant-2 user via a client-supplied
    // `assigned_to`. The tenant-2 user is seeded with auto-id 2 (see
    // `create_service`); the caller's tenant is 1.
    #[tokio::test]
    async fn test_create_lead_rejects_foreign_assigned_to() {
        let service = create_service().await;
        let create = CreateLead {
            tenant_id: 1,
            name: "Foreign assignee".to_string(),
            company: None,
            email: None,
            phone: None,
            source: "Website".to_string(),
            assigned_to: Some(2),
            notes: None,
        };
        let result = service.create_lead(create).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_create_opportunity_rejects_foreign_assigned_to() {
        let service = create_service().await;
        let create = CreateOpportunity {
            tenant_id: 1,
            lead_id: None,
            name: "Foreign assignee".to_string(),
            customer_id: None,
            value: dec!(50000),
            probability: dec!(75),
            expected_close_date: None,
            assigned_to: Some(2),
            notes: None,
        };
        let result = service.create_opportunity(create).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_create_ticket_rejects_foreign_assigned_to() {
        let service = create_service().await;
        let create = CreateTicket {
            tenant_id: 1,
            subject: "Foreign assignee".to_string(),
            description: "Problem".to_string(),
            customer_id: None,
            assigned_to: Some(2),
            priority: TicketPriority::High,
            category: None,
        };
        let result = service.create_ticket(create).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }
}
