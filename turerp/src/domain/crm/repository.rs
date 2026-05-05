//! CRM repository

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::common::SoftDeletable;
use crate::domain::crm::model::{
    Campaign, CampaignStatus, CreateCampaign, CreateLead, CreateOpportunity, CreateTicket, Lead,
    LeadStatus, Opportunity, OpportunityStatus, Ticket, TicketStatus,
};
use crate::error::ApiError;

#[async_trait]
pub trait LeadRepository: Send + Sync {
    async fn create(&self, lead: CreateLead) -> Result<Lead, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Lead>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Lead>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Lead>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: LeadStatus,
    ) -> Result<Vec<Lead>, ApiError>;
    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: LeadStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Lead>, ApiError>;
    async fn update_status(&self, id: i64, status: LeadStatus) -> Result<Lead, ApiError>;
    async fn convert_to_customer(&self, id: i64, customer_id: i64) -> Result<Lead, ApiError>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Lead, ApiError>;
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Lead>, ApiError>;
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait OpportunityRepository: Send + Sync {
    async fn create(&self, opp: CreateOpportunity) -> Result<Opportunity, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Opportunity>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Opportunity>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Opportunity>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: OpportunityStatus,
    ) -> Result<Vec<Opportunity>, ApiError>;
    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: OpportunityStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Opportunity>, ApiError>;
    async fn find_by_customer(&self, customer_id: i64) -> Result<Vec<Opportunity>, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        status: OpportunityStatus,
    ) -> Result<Opportunity, ApiError>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Opportunity, ApiError>;
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Opportunity>, ApiError>;
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait CampaignRepository: Send + Sync {
    async fn create(&self, campaign: CreateCampaign) -> Result<Campaign, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Campaign>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Campaign>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Campaign>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: CampaignStatus,
    ) -> Result<Vec<Campaign>, ApiError>;
    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: CampaignStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Campaign>, ApiError>;
    async fn update_status(&self, id: i64, status: CampaignStatus) -> Result<Campaign, ApiError>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Campaign, ApiError>;
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Campaign>, ApiError>;
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait TicketRepository: Send + Sync {
    async fn create(&self, ticket: CreateTicket) -> Result<Ticket, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Ticket>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Ticket>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Ticket>, ApiError>;
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
    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: TicketStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Ticket>, ApiError>;
    async fn find_by_assignee(&self, assignee_id: i64) -> Result<Vec<Ticket>, ApiError>;
    async fn update_status(&self, id: i64, status: TicketStatus) -> Result<Ticket, ApiError>;
    async fn resolve(&self, id: i64) -> Result<Ticket, ApiError>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Ticket, ApiError>;
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Ticket>, ApiError>;
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

pub type BoxLeadRepository = Arc<dyn LeadRepository>;
pub type BoxOpportunityRepository = Arc<dyn OpportunityRepository>;
pub type BoxCampaignRepository = Arc<dyn CampaignRepository>;
pub type BoxTicketRepository = Arc<dyn TicketRepository>;

// ==================== IN-MEMORY IMPLEMENTATIONS ====================

/// Inner state for InMemoryLeadRepository
struct InMemoryLeadInner {
    leads: std::collections::HashMap<i64, Lead>,
    next_id: i64,
}

pub struct InMemoryLeadRepository {
    inner: Mutex<InMemoryLeadInner>,
}

impl InMemoryLeadRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryLeadInner {
                leads: std::collections::HashMap::new(),
                next_id: 1,
            }),
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
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
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
            deleted_at: None,
            deleted_by: None,
        };
        inner.leads.insert(id, lead.clone());
        Ok(lead)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Lead>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .leads
            .get(&id)
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Lead>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .leads
            .values()
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Lead>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .leads
            .values()
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: LeadStatus,
    ) -> Result<Vec<Lead>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .leads
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: LeadStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Lead>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .leads
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status && !x.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update_status(&self, id: i64, status: LeadStatus) -> Result<Lead, ApiError> {
        let mut inner = self.inner.lock();
        let lead = inner
            .leads
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Lead not found".to_string()))?;
        lead.status = status;
        lead.updated_at = Utc::now();
        Ok(lead.clone())
    }

    async fn convert_to_customer(&self, id: i64, customer_id: i64) -> Result<Lead, ApiError> {
        let mut inner = self.inner.lock();
        let lead = inner
            .leads
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Lead not found".to_string()))?;
        lead.status = LeadStatus::Converted;
        lead.converted_to_customer_id = Some(customer_id);
        lead.updated_at = Utc::now();
        Ok(lead.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let lead = inner
            .leads
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Lead not found".to_string()))?;
        if lead.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Lead not found".to_string()));
        }
        lead.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Lead, ApiError> {
        let mut inner = self.inner.lock();
        let lead = inner
            .leads
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Lead not found".to_string()))?;
        if lead.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Lead not found".to_string()));
        }
        lead.restore();
        Ok(lead.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Lead>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .leads
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let lead = inner
            .leads
            .get(&id)
            .ok_or_else(|| ApiError::NotFound("Lead not found".to_string()))?;
        if lead.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Lead not found".to_string()));
        }
        inner.leads.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryOpportunityRepository
struct InMemoryOpportunityInner {
    opportunities: std::collections::HashMap<i64, Opportunity>,
    next_id: i64,
}

pub struct InMemoryOpportunityRepository {
    inner: Mutex<InMemoryOpportunityInner>,
}

impl InMemoryOpportunityRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryOpportunityInner {
                opportunities: std::collections::HashMap::new(),
                next_id: 1,
            }),
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
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
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
            deleted_at: None,
            deleted_by: None,
        };
        inner.opportunities.insert(id, opp.clone());
        Ok(opp)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Opportunity>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .opportunities
            .get(&id)
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Opportunity>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .opportunities
            .values()
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Opportunity>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .opportunities
            .values()
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: OpportunityStatus,
    ) -> Result<Vec<Opportunity>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .opportunities
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: OpportunityStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Opportunity>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .opportunities
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status && !x.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_customer(&self, customer_id: i64) -> Result<Vec<Opportunity>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .opportunities
            .values()
            .filter(|x| x.customer_id == Some(customer_id) && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn update_status(
        &self,
        id: i64,
        status: OpportunityStatus,
    ) -> Result<Opportunity, ApiError> {
        let mut inner = self.inner.lock();
        let opp = inner
            .opportunities
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Opportunity not found".to_string()))?;
        opp.status = status;
        opp.updated_at = Utc::now();
        Ok(opp.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let opp = inner
            .opportunities
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Opportunity not found".to_string()))?;
        if opp.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Opportunity not found".to_string()));
        }
        opp.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Opportunity, ApiError> {
        let mut inner = self.inner.lock();
        let opp = inner
            .opportunities
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Opportunity not found".to_string()))?;
        if opp.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Opportunity not found".to_string()));
        }
        opp.restore();
        Ok(opp.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Opportunity>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .opportunities
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let opp = inner
            .opportunities
            .get(&id)
            .ok_or_else(|| ApiError::NotFound("Opportunity not found".to_string()))?;
        if opp.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Opportunity not found".to_string()));
        }
        inner.opportunities.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryCampaignRepository
struct InMemoryCampaignInner {
    campaigns: std::collections::HashMap<i64, Campaign>,
    next_id: i64,
}

pub struct InMemoryCampaignRepository {
    inner: Mutex<InMemoryCampaignInner>,
}

impl InMemoryCampaignRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryCampaignInner {
                campaigns: std::collections::HashMap::new(),
                next_id: 1,
            }),
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
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let now = Utc::now();
        let campaign = Campaign {
            id,
            tenant_id: create.tenant_id,
            name: create.name,
            description: create.description,
            campaign_type: create.campaign_type,
            status: CampaignStatus::Draft,
            budget: create.budget,
            actual_cost: Decimal::ZERO,
            start_date: create.start_date,
            end_date: create.end_date,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };
        inner.campaigns.insert(id, campaign.clone());
        Ok(campaign)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Campaign>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .campaigns
            .get(&id)
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Campaign>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .campaigns
            .values()
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Campaign>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .campaigns
            .values()
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: CampaignStatus,
    ) -> Result<Vec<Campaign>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .campaigns
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: CampaignStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Campaign>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .campaigns
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status && !x.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update_status(&self, id: i64, status: CampaignStatus) -> Result<Campaign, ApiError> {
        let mut inner = self.inner.lock();
        let campaign = inner
            .campaigns
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Campaign not found".to_string()))?;
        campaign.status = status;
        campaign.updated_at = Utc::now();
        Ok(campaign.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let campaign = inner
            .campaigns
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Campaign not found".to_string()))?;
        if campaign.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Campaign not found".to_string()));
        }
        campaign.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Campaign, ApiError> {
        let mut inner = self.inner.lock();
        let campaign = inner
            .campaigns
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Campaign not found".to_string()))?;
        if campaign.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Campaign not found".to_string()));
        }
        campaign.restore();
        Ok(campaign.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Campaign>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .campaigns
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let campaign = inner
            .campaigns
            .get(&id)
            .ok_or_else(|| ApiError::NotFound("Campaign not found".to_string()))?;
        if campaign.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Campaign not found".to_string()));
        }
        inner.campaigns.remove(&id);
        Ok(())
    }
}

fn generate_ticket_number(count: i64) -> String {
    format!("TKT-{:06}", count)
}

/// Inner state for InMemoryTicketRepository
struct InMemoryTicketInner {
    tickets: std::collections::HashMap<i64, Ticket>,
    next_id: i64,
}

pub struct InMemoryTicketRepository {
    inner: Mutex<InMemoryTicketInner>,
}

impl InMemoryTicketRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryTicketInner {
                tickets: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}
impl Default for InMemoryTicketRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TicketRepository for InMemoryTicketRepository {
    async fn create(&self, create: CreateTicket) -> Result<Ticket, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
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
            deleted_at: None,
            deleted_by: None,
        };
        inner.tickets.insert(id, ticket.clone());
        Ok(ticket)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Ticket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .get(&id)
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Ticket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .values()
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Ticket>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .tickets
            .values()
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_number(
        &self,
        tenant_id: i64,
        ticket_number: &str,
    ) -> Result<Option<Ticket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .values()
            .find(|x| {
                x.tenant_id == tenant_id && x.ticket_number == ticket_number && !x.is_deleted()
            })
            .cloned())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: TicketStatus,
    ) -> Result<Vec<Ticket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: TicketStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Ticket>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .tickets
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status && !x.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_assignee(&self, assignee_id: i64) -> Result<Vec<Ticket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .values()
            .filter(|x| x.assigned_to == Some(assignee_id) && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn update_status(&self, id: i64, status: TicketStatus) -> Result<Ticket, ApiError> {
        let mut inner = self.inner.lock();
        let ticket = inner
            .tickets
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Ticket not found".to_string()))?;
        ticket.status = status;
        ticket.updated_at = Utc::now();
        Ok(ticket.clone())
    }

    async fn resolve(&self, id: i64) -> Result<Ticket, ApiError> {
        let mut inner = self.inner.lock();
        let ticket = inner
            .tickets
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Ticket not found".to_string()))?;
        ticket.status = TicketStatus::Resolved;
        ticket.resolved_at = Some(Utc::now());
        ticket.updated_at = Utc::now();
        Ok(ticket.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let ticket = inner
            .tickets
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Ticket not found".to_string()))?;
        if ticket.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Ticket not found".to_string()));
        }
        ticket.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Ticket, ApiError> {
        let mut inner = self.inner.lock();
        let ticket = inner
            .tickets
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Ticket not found".to_string()))?;
        if ticket.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Ticket not found".to_string()));
        }
        ticket.restore();
        Ok(ticket.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Ticket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let ticket = inner
            .tickets
            .get(&id)
            .ok_or_else(|| ApiError::NotFound("Ticket not found".to_string()))?;
        if ticket.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Ticket not found".to_string()));
        }
        inner.tickets.remove(&id);
        Ok(())
    }
}
