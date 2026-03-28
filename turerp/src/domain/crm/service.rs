//! CRM service for business logic

#[allow(unused_imports)]
use std::sync::Arc;

use crate::domain::crm::model::{
    Campaign, CampaignStatus, CreateCampaign, CreateLead, CreateOpportunity, CreateTicket, Lead,
    LeadStatus, Opportunity, OpportunityStatus, Ticket, TicketStatus,
};
use crate::domain::crm::repository::{
    BoxCampaignRepository, BoxLeadRepository, BoxOpportunityRepository, BoxTicketRepository,
};
use crate::error::ApiError;

#[derive(Clone)]
pub struct CrmService {
    lead_repo: BoxLeadRepository,
    opportunity_repo: BoxOpportunityRepository,
    campaign_repo: BoxCampaignRepository,
    ticket_repo: BoxTicketRepository,
}

impl CrmService {
    pub fn new(
        lead_repo: BoxLeadRepository,
        opportunity_repo: BoxOpportunityRepository,
        campaign_repo: BoxCampaignRepository,
        ticket_repo: BoxTicketRepository,
    ) -> Self {
        Self {
            lead_repo,
            opportunity_repo,
            campaign_repo,
            ticket_repo,
        }
    }

    // Lead methods
    pub async fn create_lead(&self, create: CreateLead) -> Result<Lead, ApiError> {
        self.lead_repo.create(create).await
    }

    pub async fn get_lead(&self, id: i64) -> Result<Lead, ApiError> {
        self.lead_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Lead not found".to_string()))
    }

    pub async fn get_leads_by_tenant(&self, tenant_id: i64) -> Result<Vec<Lead>, ApiError> {
        self.lead_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_leads_by_status(
        &self,
        tenant_id: i64,
        status: LeadStatus,
    ) -> Result<Vec<Lead>, ApiError> {
        self.lead_repo.find_by_status(tenant_id, status).await
    }

    pub async fn update_lead_status(&self, id: i64, status: LeadStatus) -> Result<Lead, ApiError> {
        self.lead_repo.update_status(id, status).await
    }

    pub async fn convert_lead_to_customer(
        &self,
        id: i64,
        customer_id: i64,
    ) -> Result<Lead, ApiError> {
        self.lead_repo.convert_to_customer(id, customer_id).await
    }

    // Opportunity methods
    pub async fn create_opportunity(
        &self,
        create: CreateOpportunity,
    ) -> Result<Opportunity, ApiError> {
        self.opportunity_repo.create(create).await
    }

    pub async fn get_opportunity(&self, id: i64) -> Result<Opportunity, ApiError> {
        self.opportunity_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Opportunity not found".to_string()))
    }

    pub async fn get_opportunities_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Opportunity>, ApiError> {
        self.opportunity_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_opportunities_by_status(
        &self,
        tenant_id: i64,
        status: OpportunityStatus,
    ) -> Result<Vec<Opportunity>, ApiError> {
        self.opportunity_repo
            .find_by_status(tenant_id, status)
            .await
    }

    pub async fn get_opportunities_by_customer(
        &self,
        customer_id: i64,
    ) -> Result<Vec<Opportunity>, ApiError> {
        self.opportunity_repo.find_by_customer(customer_id).await
    }

    pub async fn update_opportunity_status(
        &self,
        id: i64,
        status: OpportunityStatus,
    ) -> Result<Opportunity, ApiError> {
        self.opportunity_repo.update_status(id, status).await
    }

    // Campaign methods
    pub async fn create_campaign(&self, create: CreateCampaign) -> Result<Campaign, ApiError> {
        self.campaign_repo.create(create).await
    }

    pub async fn get_campaign(&self, id: i64) -> Result<Campaign, ApiError> {
        self.campaign_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Campaign not found".to_string()))
    }

    pub async fn get_campaigns_by_tenant(&self, tenant_id: i64) -> Result<Vec<Campaign>, ApiError> {
        self.campaign_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_campaigns_by_status(
        &self,
        tenant_id: i64,
        status: CampaignStatus,
    ) -> Result<Vec<Campaign>, ApiError> {
        self.campaign_repo.find_by_status(tenant_id, status).await
    }

    pub async fn update_campaign_status(
        &self,
        id: i64,
        status: CampaignStatus,
    ) -> Result<Campaign, ApiError> {
        self.campaign_repo.update_status(id, status).await
    }

    // Ticket methods
    pub async fn create_ticket(&self, create: CreateTicket) -> Result<Ticket, ApiError> {
        self.ticket_repo.create(create).await
    }

    pub async fn get_ticket(&self, id: i64) -> Result<Ticket, ApiError> {
        self.ticket_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Ticket not found".to_string()))
    }

    pub async fn get_ticket_by_number(
        &self,
        tenant_id: i64,
        ticket_number: &str,
    ) -> Result<Option<Ticket>, ApiError> {
        self.ticket_repo
            .find_by_number(tenant_id, ticket_number)
            .await
    }

    pub async fn get_tickets_by_tenant(&self, tenant_id: i64) -> Result<Vec<Ticket>, ApiError> {
        self.ticket_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_tickets_by_status(
        &self,
        tenant_id: i64,
        status: TicketStatus,
    ) -> Result<Vec<Ticket>, ApiError> {
        self.ticket_repo.find_by_status(tenant_id, status).await
    }

    pub async fn get_tickets_by_assignee(&self, assignee_id: i64) -> Result<Vec<Ticket>, ApiError> {
        self.ticket_repo.find_by_assignee(assignee_id).await
    }

    pub async fn update_ticket_status(
        &self,
        id: i64,
        status: TicketStatus,
    ) -> Result<Ticket, ApiError> {
        self.ticket_repo.update_status(id, status).await
    }

    pub async fn resolve_ticket(&self, id: i64) -> Result<Ticket, ApiError> {
        self.ticket_repo.resolve(id).await
    }

    // Dashboard metrics
    pub async fn get_sales_pipeline_value(&self, tenant_id: i64) -> Result<f64, ApiError> {
        let opportunities = self.opportunity_repo.find_by_tenant(tenant_id).await?;
        let open_opps: Vec<_> = opportunities
            .into_iter()
            .filter(|o| o.status == OpportunityStatus::Open)
            .collect();
        let weighted_value: f64 = open_opps
            .iter()
            .map(|o| o.value * (o.probability / 100.0))
            .sum();
        Ok(weighted_value)
    }

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

    fn create_service() -> CrmService {
        let lead_repo = Arc::new(InMemoryLeadRepository::new()) as BoxLeadRepository;
        let opp_repo = Arc::new(InMemoryOpportunityRepository::new()) as BoxOpportunityRepository;
        let campaign_repo = Arc::new(InMemoryCampaignRepository::new()) as BoxCampaignRepository;
        let ticket_repo = Arc::new(InMemoryTicketRepository::new()) as BoxTicketRepository;
        CrmService::new(lead_repo, opp_repo, campaign_repo, ticket_repo)
    }

    #[tokio::test]
    async fn test_create_lead() {
        let service = create_service();
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
        let service = create_service();
        let create = CreateOpportunity {
            tenant_id: 1,
            lead_id: None,
            name: "Big Deal".to_string(),
            customer_id: Some(1),
            value: 50000.0,
            probability: 75.0,
            expected_close_date: Some(chrono::Utc::now()),
            assigned_to: None,
            notes: None,
        };
        let result = service.create_opportunity(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_campaign() {
        let service = create_service();
        let create = CreateCampaign {
            tenant_id: 1,
            name: "Summer Sale".to_string(),
            description: Some("Annual campaign".to_string()),
            campaign_type: "Email".to_string(),
            budget: 10000.0,
            start_date: Some(chrono::Utc::now()),
            end_date: None,
        };
        let result = service.create_campaign(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_ticket() {
        let service = create_service();
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
        let service = create_service();

        // Create opportunity with 50% probability and $100,000 value
        service
            .create_opportunity(CreateOpportunity {
                tenant_id: 1,
                lead_id: None,
                name: "Deal 1".to_string(),
                customer_id: None,
                value: 100000.0,
                probability: 50.0,
                expected_close_date: None,
                assigned_to: None,
                notes: None,
            })
            .await
            .unwrap();

        let value = service.get_sales_pipeline_value(1).await.unwrap();
        assert_eq!(value, 50000.0); // 100000 * 50%
    }

    #[tokio::test]
    async fn test_resolve_ticket() {
        let service = create_service();
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

        let resolved = service.resolve_ticket(ticket.id).await.unwrap();
        assert_eq!(resolved.status, TicketStatus::Resolved);
        assert!(resolved.resolved_at.is_some());
    }
}
