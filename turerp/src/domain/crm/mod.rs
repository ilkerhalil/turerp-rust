//! CRM domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    Campaign, CampaignStatus, CreateCampaign, CreateLead, CreateOpportunity, CreateTicket, Lead,
    LeadStatus, Opportunity, OpportunityStatus, Ticket, TicketPriority, TicketStatus,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{
    PostgresCampaignRepository, PostgresLeadRepository, PostgresOpportunityRepository,
    PostgresTicketRepository,
};
pub use repository::{
    BoxCampaignRepository, BoxLeadRepository, BoxOpportunityRepository, BoxTicketRepository,
    CampaignRepository, InMemoryCampaignRepository, InMemoryLeadRepository,
    InMemoryOpportunityRepository, InMemoryTicketRepository, LeadRepository, OpportunityRepository,
    TicketRepository,
};
pub use service::CrmService;
