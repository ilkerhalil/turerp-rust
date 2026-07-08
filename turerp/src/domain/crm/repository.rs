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
    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: LeadStatus,
    ) -> Result<Lead, ApiError>;
    async fn convert_to_customer(
        &self,
        id: i64,
        tenant_id: i64,
        customer_id: i64,
    ) -> Result<Lead, ApiError>;
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
    async fn find_by_customer(
        &self,
        customer_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Opportunity>, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
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
    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: CampaignStatus,
    ) -> Result<Campaign, ApiError>;
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
    async fn find_by_assignee(
        &self,
        assignee_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Ticket>, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: TicketStatus,
    ) -> Result<Ticket, ApiError>;
    async fn resolve(&self, id: i64, tenant_id: i64) -> Result<Ticket, ApiError>;
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

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: LeadStatus,
    ) -> Result<Lead, ApiError> {
        let mut inner = self.inner.lock();
        let lead = inner
            .leads
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound("Lead not found".to_string()))?;
        lead.status = status;
        lead.updated_at = Utc::now();
        Ok(lead.clone())
    }

    async fn convert_to_customer(
        &self,
        id: i64,
        tenant_id: i64,
        customer_id: i64,
    ) -> Result<Lead, ApiError> {
        let mut inner = self.inner.lock();
        let lead = inner
            .leads
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id)
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

    async fn find_by_customer(
        &self,
        customer_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Opportunity>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .opportunities
            .values()
            .filter(|x| {
                x.customer_id == Some(customer_id) && x.tenant_id == tenant_id && !x.is_deleted()
            })
            .cloned()
            .collect())
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: OpportunityStatus,
    ) -> Result<Opportunity, ApiError> {
        let mut inner = self.inner.lock();
        let opp = inner
            .opportunities
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id)
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

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: CampaignStatus,
    ) -> Result<Campaign, ApiError> {
        let mut inner = self.inner.lock();
        let campaign = inner
            .campaigns
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id)
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

    async fn find_by_assignee(
        &self,
        assignee_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Ticket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .values()
            .filter(|x| {
                x.assigned_to == Some(assignee_id) && x.tenant_id == tenant_id && !x.is_deleted()
            })
            .cloned()
            .collect())
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: TicketStatus,
    ) -> Result<Ticket, ApiError> {
        let mut inner = self.inner.lock();
        let ticket = inner
            .tickets
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound("Ticket not found".to_string()))?;
        ticket.status = status;
        ticket.updated_at = Utc::now();
        Ok(ticket.clone())
    }

    async fn resolve(&self, id: i64, tenant_id: i64) -> Result<Ticket, ApiError> {
        let mut inner = self.inner.lock();
        let ticket = inner
            .tickets
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::crm::model::{CreateOpportunity, CreateTicket, TicketPriority};

    fn make_opportunity(tenant_id: i64, customer_id: Option<i64>, name: &str) -> CreateOpportunity {
        CreateOpportunity {
            tenant_id,
            lead_id: None,
            name: name.to_string(),
            customer_id,
            value: Decimal::new(100, 0),
            probability: Decimal::new(50, 0),
            expected_close_date: None,
            assigned_to: None,
            notes: None,
        }
    }

    fn make_ticket(tenant_id: i64, assigned_to: Option<i64>, subject: &str) -> CreateTicket {
        CreateTicket {
            tenant_id,
            subject: subject.to_string(),
            description: "desc".to_string(),
            customer_id: None,
            assigned_to,
            priority: TicketPriority::Medium,
            category: None,
        }
    }

    #[tokio::test]
    async fn find_by_customer_is_tenant_scoped_and_excludes_deleted() {
        let repo = InMemoryOpportunityRepository::new();
        // Same customer_id (100) across two tenants + a soft-deleted tenant-1 row.
        let own = repo
            .create(make_opportunity(1, Some(100), "own"))
            .await
            .unwrap();
        let foreign = repo
            .create(make_opportunity(2, Some(100), "foreign"))
            .await
            .unwrap();
        let deleted = repo
            .create(make_opportunity(1, Some(100), "deleted"))
            .await
            .unwrap();
        repo.create(make_opportunity(1, Some(999), "other-customer"))
            .await
            .unwrap();
        repo.soft_delete(deleted.id, 1, 1).await.unwrap();

        // Tenant 1 sees only its own, non-deleted, customer=100 opportunity.
        let t1 = repo.find_by_customer(100, 1).await.unwrap();
        assert_eq!(t1.len(), 1);
        assert_eq!(t1[0].id, own.id);

        // Tenant 2 sees only its own — cross-tenant isolation (the bug being fixed).
        let t2 = repo.find_by_customer(100, 2).await.unwrap();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].id, foreign.id);

        // Unknown tenant sees nothing.
        assert!(repo.find_by_customer(100, 999).await.unwrap().is_empty());
        // Different customer_id on tenant 1 is not returned by customer=100.
        assert!(repo
            .find_by_customer(999, 1)
            .await
            .unwrap()
            .iter()
            .all(|o| o.customer_id == Some(999)));
    }

    #[tokio::test]
    async fn find_by_assignee_is_tenant_scoped_and_excludes_deleted() {
        let repo = InMemoryTicketRepository::new();
        // Same assignee (5) across two tenants + a soft-deleted tenant-1 row.
        let own = repo.create(make_ticket(1, Some(5), "own")).await.unwrap();
        let foreign = repo
            .create(make_ticket(2, Some(5), "foreign"))
            .await
            .unwrap();
        let deleted = repo
            .create(make_ticket(1, Some(5), "deleted"))
            .await
            .unwrap();
        repo.soft_delete(deleted.id, 1, 1).await.unwrap();

        let t1 = repo.find_by_assignee(5, 1).await.unwrap();
        assert_eq!(t1.len(), 1);
        assert_eq!(t1[0].id, own.id);

        let t2 = repo.find_by_assignee(5, 2).await.unwrap();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].id, foreign.id);

        assert!(repo.find_by_assignee(5, 999).await.unwrap().is_empty());
        // Unassigned tickets are never matched by an assignee lookup.
        repo.create(make_ticket(1, None, "unassigned"))
            .await
            .unwrap();
        assert_eq!(repo.find_by_assignee(5, 1).await.unwrap().len(), 1);
    }
}
