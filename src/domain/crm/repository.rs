//! CRM repository

use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use crate::domain::crm::model::{
    Campaign, CampaignStatus, CreateCampaign, CreateLead, CreateOpportunity, CreateTicket, Lead,
    LeadStatus, Opportunity, OpportunityStatus, Ticket, TicketStatus,
};
use crate::error::ApiError;

#[async_trait]
pub trait LeadRepository: Send + Sync {
    async fn create(&self, lead: CreateLead) -> Result<Lead, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Lead>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Lead>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: LeadStatus,
    ) -> Result<Vec<Lead>, ApiError>;
    async fn update_status(&self, id: i64, status: LeadStatus) -> Result<Lead, ApiError>;
    async fn convert_to_customer(&self, id: i64, customer_id: i64) -> Result<Lead, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait OpportunityRepository: Send + Sync {
    async fn create(&self, opp: CreateOpportunity) -> Result<Opportunity, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Opportunity>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Opportunity>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: OpportunityStatus,
    ) -> Result<Vec<Opportunity>, ApiError>;
    async fn find_by_customer(&self, customer_id: i64) -> Result<Vec<Opportunity>, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        status: OpportunityStatus,
    ) -> Result<Opportunity, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait CampaignRepository: Send + Sync {
    async fn create(&self, campaign: CreateCampaign) -> Result<Campaign, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Campaign>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Campaign>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: CampaignStatus,
    ) -> Result<Vec<Campaign>, ApiError>;
    async fn update_status(&self, id: i64, status: CampaignStatus) -> Result<Campaign, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait TicketRepository: Send + Sync {
    async fn create(&self, ticket: CreateTicket) -> Result<Ticket, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Ticket>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Ticket>, ApiError>;
    async fn find_by_number(
        &self,
        tenant_id: i64,
        ticket_number: &str,
    ) -> Result<Option<Ticket>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: TicketStatus,
    ) -> Result<Vec<Ticket>, ApiError>;
    async fn find_by_assignee(&self, assignee_id: i64) -> Result<Vec<Ticket>, ApiError>;
    async fn update_status(&self, id: i64, status: TicketStatus) -> Result<Ticket, ApiError>;
    async fn resolve(&self, id: i64) -> Result<Ticket, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

pub type BoxLeadRepository = Arc<dyn LeadRepository>;
pub type BoxOpportunityRepository = Arc<dyn OpportunityRepository>;
pub type BoxCampaignRepository = Arc<dyn CampaignRepository>;
pub type BoxTicketRepository = Arc<dyn TicketRepository>;

// ==================== IN-MEMORY IMPLEMENTATIONS ====================

pub struct InMemoryLeadRepository {
    leads: std::sync::Mutex<std::collections::HashMap<i64, Lead>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryLeadRepository {
    pub fn new() -> Self {
        Self {
            leads: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
        }
    }
}
impl Default for InMemoryLeadRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LeadRepository for InMemoryLeadRepository {
    async fn create(&self, create: CreateLead) -> Result<Lead, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let now = Utc::now();
        let lead = Lead {
            id,
            tenant_id: create.tenant_id,
            name: create.name,
            company: create.company,
            email: create.email,
            phone: create.phone,
            source: create.source,
            status: LeadStatus::New,
            assigned_to: create.assigned_to,
            converted_to_customer_id: None,
            notes: create.notes,
            created_at: now,
            updated_at: now,
        };
        self.leads.lock().unwrap().insert(id, lead.clone());
        Ok(lead)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Lead>, ApiError> {
        Ok(self.leads.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Lead>, ApiError> {
        let l = self.leads.lock().unwrap();
        Ok(l.values()
            .filter(|x| x.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: LeadStatus,
    ) -> Result<Vec<Lead>, ApiError> {
        let l = self.leads.lock().unwrap();
        Ok(l.values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status)
            .cloned()
            .collect())
    }

    async fn update_status(&self, id: i64, status: LeadStatus) -> Result<Lead, ApiError> {
        let mut l = self.leads.lock().unwrap();
        let lead = l
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Lead not found".to_string()))?;
        lead.status = status;
        lead.updated_at = Utc::now();
        Ok(lead.clone())
    }

    async fn convert_to_customer(&self, id: i64, customer_id: i64) -> Result<Lead, ApiError> {
        let mut l = self.leads.lock().unwrap();
        let lead = l
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Lead not found".to_string()))?;
        lead.status = LeadStatus::Converted;
        lead.converted_to_customer_id = Some(customer_id);
        lead.updated_at = Utc::now();
        Ok(lead.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.leads.lock().unwrap().remove(&id);
        Ok(())
    }
}

pub struct InMemoryOpportunityRepository {
    opportunities: std::sync::Mutex<std::collections::HashMap<i64, Opportunity>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryOpportunityRepository {
    pub fn new() -> Self {
        Self {
            opportunities: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
        }
    }
}
impl Default for InMemoryOpportunityRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpportunityRepository for InMemoryOpportunityRepository {
    async fn create(&self, create: CreateOpportunity) -> Result<Opportunity, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let now = Utc::now();
        let opp = Opportunity {
            id,
            tenant_id: create.tenant_id,
            lead_id: create.lead_id,
            name: create.name,
            customer_id: create.customer_id,
            value: create.value,
            probability: create.probability,
            expected_close_date: create.expected_close_date,
            status: OpportunityStatus::Open,
            assigned_to: create.assigned_to,
            notes: create.notes,
            created_at: now,
            updated_at: now,
        };
        self.opportunities.lock().unwrap().insert(id, opp.clone());
        Ok(opp)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Opportunity>, ApiError> {
        Ok(self.opportunities.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Opportunity>, ApiError> {
        let o = self.opportunities.lock().unwrap();
        Ok(o.values()
            .filter(|x| x.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: OpportunityStatus,
    ) -> Result<Vec<Opportunity>, ApiError> {
        let o = self.opportunities.lock().unwrap();
        Ok(o.values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status)
            .cloned()
            .collect())
    }

    async fn find_by_customer(&self, customer_id: i64) -> Result<Vec<Opportunity>, ApiError> {
        let o = self.opportunities.lock().unwrap();
        Ok(o.values()
            .filter(|x| x.customer_id == Some(customer_id))
            .cloned()
            .collect())
    }

    async fn update_status(
        &self,
        id: i64,
        status: OpportunityStatus,
    ) -> Result<Opportunity, ApiError> {
        let mut o = self.opportunities.lock().unwrap();
        let opp = o
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Opportunity not found".to_string()))?;
        opp.status = status;
        opp.updated_at = Utc::now();
        Ok(opp.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.opportunities.lock().unwrap().remove(&id);
        Ok(())
    }
}

pub struct InMemoryCampaignRepository {
    campaigns: std::sync::Mutex<std::collections::HashMap<i64, Campaign>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryCampaignRepository {
    pub fn new() -> Self {
        Self {
            campaigns: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
        }
    }
}
impl Default for InMemoryCampaignRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CampaignRepository for InMemoryCampaignRepository {
    async fn create(&self, create: CreateCampaign) -> Result<Campaign, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let now = Utc::now();
        let campaign = Campaign {
            id,
            tenant_id: create.tenant_id,
            name: create.name,
            description: create.description,
            campaign_type: create.campaign_type,
            status: CampaignStatus::Draft,
            budget: create.budget,
            actual_cost: 0.0,
            start_date: create.start_date,
            end_date: create.end_date,
            created_at: now,
            updated_at: now,
        };
        self.campaigns.lock().unwrap().insert(id, campaign.clone());
        Ok(campaign)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Campaign>, ApiError> {
        Ok(self.campaigns.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Campaign>, ApiError> {
        let c = self.campaigns.lock().unwrap();
        Ok(c.values()
            .filter(|x| x.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: CampaignStatus,
    ) -> Result<Vec<Campaign>, ApiError> {
        let c = self.campaigns.lock().unwrap();
        Ok(c.values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status)
            .cloned()
            .collect())
    }

    async fn update_status(&self, id: i64, status: CampaignStatus) -> Result<Campaign, ApiError> {
        let mut c = self.campaigns.lock().unwrap();
        let campaign = c
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Campaign not found".to_string()))?;
        campaign.status = status;
        campaign.updated_at = Utc::now();
        Ok(campaign.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.campaigns.lock().unwrap().remove(&id);
        Ok(())
    }
}

pub struct InMemoryTicketRepository {
    tickets: std::sync::Mutex<std::collections::HashMap<i64, Ticket>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryTicketRepository {
    pub fn new() -> Self {
        Self {
            tickets: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
        }
    }
}
impl Default for InMemoryTicketRepository {
    fn default() -> Self {
        Self::new()
    }
}

fn generate_ticket_number(count: i64) -> String {
    format!("TKT-{:06}", count)
}

#[async_trait]
impl TicketRepository for InMemoryTicketRepository {
    async fn create(&self, create: CreateTicket) -> Result<Ticket, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let ticket_number = generate_ticket_number(id);
        let now = Utc::now();
        let ticket = Ticket {
            id,
            tenant_id: create.tenant_id,
            ticket_number,
            subject: create.subject,
            description: create.description,
            customer_id: create.customer_id,
            assigned_to: create.assigned_to,
            status: TicketStatus::Open,
            priority: create.priority,
            category: create.category,
            resolved_at: None,
            created_at: now,
            updated_at: now,
        };
        self.tickets.lock().unwrap().insert(id, ticket.clone());
        Ok(ticket)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Ticket>, ApiError> {
        Ok(self.tickets.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Ticket>, ApiError> {
        let t = self.tickets.lock().unwrap();
        Ok(t.values()
            .filter(|x| x.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_number(
        &self,
        tenant_id: i64,
        ticket_number: &str,
    ) -> Result<Option<Ticket>, ApiError> {
        let t = self.tickets.lock().unwrap();
        Ok(t.values()
            .filter(|x| x.tenant_id == tenant_id && x.ticket_number == ticket_number)
            .cloned()
            .collect::<Vec<_>>()
            .pop())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: TicketStatus,
    ) -> Result<Vec<Ticket>, ApiError> {
        let t = self.tickets.lock().unwrap();
        Ok(t.values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status)
            .cloned()
            .collect())
    }

    async fn find_by_assignee(&self, assignee_id: i64) -> Result<Vec<Ticket>, ApiError> {
        let t = self.tickets.lock().unwrap();
        Ok(t.values()
            .filter(|x| x.assigned_to == Some(assignee_id))
            .cloned()
            .collect())
    }

    async fn update_status(&self, id: i64, status: TicketStatus) -> Result<Ticket, ApiError> {
        let mut t = self.tickets.lock().unwrap();
        let ticket = t
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Ticket not found".to_string()))?;
        ticket.status = status;
        ticket.updated_at = Utc::now();
        Ok(ticket.clone())
    }

    async fn resolve(&self, id: i64) -> Result<Ticket, ApiError> {
        let mut t = self.tickets.lock().unwrap();
        let ticket = t
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Ticket not found".to_string()))?;
        ticket.status = TicketStatus::Resolved;
        ticket.resolved_at = Some(Utc::now());
        ticket.updated_at = Utc::now();
        Ok(ticket.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.tickets.lock().unwrap().remove(&id);
        Ok(())
    }
}
