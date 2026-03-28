//! CRM domain module

pub mod model;
pub mod repository;
pub mod service;

pub use model::{
    Campaign, CampaignStatus, CreateCampaign, CreateLead, CreateOpportunity, CreateTicket, Lead,
    LeadStatus, Opportunity, OpportunityStatus, Ticket, TicketPriority, TicketStatus,
};
pub use repository::{
    BoxCampaignRepository, BoxLeadRepository, BoxOpportunityRepository, BoxTicketRepository,
    CampaignRepository, InMemoryCampaignRepository, InMemoryLeadRepository,
    InMemoryOpportunityRepository, InMemoryTicketRepository, LeadRepository, OpportunityRepository,
    TicketRepository,
};
pub use service::CrmService;
