//! CRM domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ==================== LEADS ====================

/// Lead status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LeadStatus {
    New,
    Contacted,
    Qualified,
    Unqualified,
    Converted,
}

/// Lead entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub company: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub source: String,
    pub status: LeadStatus,
    pub assigned_to: Option<i64>,
    pub converted_to_customer_id: Option<i64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create lead request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLead {
    pub tenant_id: i64,
    pub name: String,
    pub company: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub source: String,
    pub assigned_to: Option<i64>,
    pub notes: Option<String>,
}

impl CreateLead {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Lead name is required".to_string());
        }
        if self.source.trim().is_empty() {
            errors.push("Lead source is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ==================== OPPORTUNITIES ====================

/// Opportunity status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OpportunityStatus {
    Open,
    Won,
    Lost,
    Cancelled,
}

/// Opportunity entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: i64,
    pub tenant_id: i64,
    pub lead_id: Option<i64>,
    pub name: String,
    pub customer_id: Option<i64>,
    pub value: f64,
    pub probability: f64,
    pub expected_close_date: Option<DateTime<Utc>>,
    pub status: OpportunityStatus,
    pub assigned_to: Option<i64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create opportunity request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOpportunity {
    pub tenant_id: i64,
    pub lead_id: Option<i64>,
    pub name: String,
    pub customer_id: Option<i64>,
    pub value: f64,
    pub probability: f64,
    pub expected_close_date: Option<DateTime<Utc>>,
    pub assigned_to: Option<i64>,
    pub notes: Option<String>,
}

impl CreateOpportunity {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Opportunity name is required".to_string());
        }
        if self.value < 0.0 {
            errors.push("Value cannot be negative".to_string());
        }
        if self.probability < 0.0 || self.probability > 100.0 {
            errors.push("Probability must be between 0 and 100".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ==================== CAMPAIGNS ====================

/// Campaign status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CampaignStatus {
    Draft,
    Scheduled,
    Active,
    Completed,
    Cancelled,
}

/// Campaign entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub campaign_type: String,
    pub status: CampaignStatus,
    pub budget: f64,
    pub actual_cost: f64,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create campaign request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCampaign {
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub campaign_type: String,
    pub budget: f64,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

impl CreateCampaign {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Campaign name is required".to_string());
        }
        if self.campaign_type.trim().is_empty() {
            errors.push("Campaign type is required".to_string());
        }
        if self.budget < 0.0 {
            errors.push("Budget cannot be negative".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ==================== TICKETS ====================

/// Ticket status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TicketStatus {
    Open,
    InProgress,
    Pending,
    Resolved,
    Closed,
}

/// Ticket priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Ticket entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: i64,
    pub tenant_id: i64,
    pub ticket_number: String,
    pub subject: String,
    pub description: String,
    pub customer_id: Option<i64>,
    pub assigned_to: Option<i64>,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub category: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create ticket request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTicket {
    pub tenant_id: i64,
    pub subject: String,
    pub description: String,
    pub customer_id: Option<i64>,
    pub assigned_to: Option<i64>,
    pub priority: TicketPriority,
    pub category: Option<String>,
}

impl CreateTicket {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.subject.trim().is_empty() {
            errors.push("Ticket subject is required".to_string());
        }
        if self.description.trim().is_empty() {
            errors.push("Ticket description is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_lead_validation() {
        let valid = CreateLead {
            tenant_id: 1,
            name: "John Doe".to_string(),
            company: Some("Acme Inc".to_string()),
            email: Some("john@acme.com".to_string()),
            phone: Some("+1234567890".to_string()),
            source: "Website".to_string(),
            assigned_to: None,
            notes: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateLead {
            tenant_id: 1,
            name: "".to_string(),
            company: None,
            email: None,
            phone: None,
            source: "".to_string(),
            assigned_to: None,
            notes: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_opportunity_validation() {
        let valid = CreateOpportunity {
            tenant_id: 1,
            lead_id: None,
            name: "Big Deal".to_string(),
            customer_id: Some(1),
            value: 50000.0,
            probability: 75.0,
            expected_close_date: Some(Utc::now()),
            assigned_to: None,
            notes: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateOpportunity {
            tenant_id: 1,
            lead_id: None,
            name: "".to_string(),
            customer_id: None,
            value: -100.0,
            probability: 150.0,
            expected_close_date: None,
            assigned_to: None,
            notes: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_campaign_validation() {
        let valid = CreateCampaign {
            tenant_id: 1,
            name: "Summer Sale".to_string(),
            description: Some("Annual summer campaign".to_string()),
            campaign_type: "Email".to_string(),
            budget: 10000.0,
            start_date: Some(Utc::now()),
            end_date: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateCampaign {
            tenant_id: 1,
            name: "".to_string(),
            description: None,
            campaign_type: "".to_string(),
            budget: -500.0,
            start_date: None,
            end_date: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_ticket_validation() {
        let valid = CreateTicket {
            tenant_id: 1,
            subject: "Login issue".to_string(),
            description: "Cannot login to the system".to_string(),
            customer_id: Some(1),
            assigned_to: None,
            priority: TicketPriority::High,
            category: Some("Technical".to_string()),
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateTicket {
            tenant_id: 1,
            subject: "".to_string(),
            description: "".to_string(),
            customer_id: None,
            assigned_to: None,
            priority: TicketPriority::Low,
            category: None,
        };
        assert!(invalid.validate().is_err());
    }
}
