//! GraphQL object types mapping domain entities

use async_graphql::*;
use chrono::{DateTime, Utc};

/// GraphQL Role enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum GraphQlRole {
    Admin,
    User,
    Viewer,
}

impl From<crate::domain::user::Role> for GraphQlRole {
    fn from(role: crate::domain::user::Role) -> Self {
        match role {
            crate::domain::user::Role::Admin => GraphQlRole::Admin,
            crate::domain::user::Role::User => GraphQlRole::User,
            crate::domain::user::Role::Viewer => GraphQlRole::Viewer,
        }
    }
}

/// GraphQL User type
#[derive(SimpleObject, Clone, Debug)]
pub struct GraphQlUser {
    pub id: ID,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub tenant_id: i64,
    pub role: GraphQlRole,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<crate::domain::user::UserResponse> for GraphQlUser {
    fn from(u: crate::domain::user::UserResponse) -> Self {
        Self {
            id: ID::from(u.id.to_string()),
            username: u.username,
            email: u.email,
            full_name: u.full_name,
            tenant_id: u.tenant_id,
            role: u.role.into(),
            is_active: u.is_active,
            created_at: u.created_at,
        }
    }
}

/// GraphQL Employee status enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum GraphQlEmployeeStatus {
    Active,
    OnLeave,
    Terminated,
    Suspended,
}

impl From<crate::domain::hr::EmployeeStatus> for GraphQlEmployeeStatus {
    fn from(s: crate::domain::hr::EmployeeStatus) -> Self {
        match s {
            crate::domain::hr::EmployeeStatus::Active => GraphQlEmployeeStatus::Active,
            crate::domain::hr::EmployeeStatus::OnLeave => GraphQlEmployeeStatus::OnLeave,
            crate::domain::hr::EmployeeStatus::Terminated => GraphQlEmployeeStatus::Terminated,
            crate::domain::hr::EmployeeStatus::Suspended => GraphQlEmployeeStatus::Suspended,
        }
    }
}

impl From<GraphQlEmployeeStatus> for crate::domain::hr::EmployeeStatus {
    fn from(s: GraphQlEmployeeStatus) -> Self {
        match s {
            GraphQlEmployeeStatus::Active => crate::domain::hr::EmployeeStatus::Active,
            GraphQlEmployeeStatus::OnLeave => crate::domain::hr::EmployeeStatus::OnLeave,
            GraphQlEmployeeStatus::Terminated => crate::domain::hr::EmployeeStatus::Terminated,
            GraphQlEmployeeStatus::Suspended => crate::domain::hr::EmployeeStatus::Suspended,
        }
    }
}

/// GraphQL Employee type
#[derive(SimpleObject, Clone, Debug)]
pub struct GraphQlEmployee {
    pub id: ID,
    pub employee_number: String,
    pub first_name: String,
    pub last_name: String,
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub hire_date: DateTime<Utc>,
    pub status: GraphQlEmployeeStatus,
    pub salary: String,
    pub gross_salary: String,
    pub tenant_id: i64,
    pub company_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<crate::domain::hr::EmployeeResponse> for GraphQlEmployee {
    fn from(e: crate::domain::hr::EmployeeResponse) -> Self {
        Self {
            id: ID::from(e.id.to_string()),
            employee_number: e.employee_number,
            first_name: e.first_name,
            last_name: e.last_name,
            full_name: e.full_name,
            email: e.email,
            phone: e.phone,
            department: e.department,
            position: e.position,
            hire_date: e.hire_date,
            status: e.status.into(),
            salary: e.salary.to_string(),
            gross_salary: e.gross_salary.to_string(),
            tenant_id: e.tenant_id,
            company_id: e.company_id,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

/// GraphQL Invoice status enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum GraphQlInvoiceStatus {
    Draft,
    Pending,
    Approved,
    Sent,
    PartiallyPaid,
    Paid,
    Overdue,
    Cancelled,
    Refunded,
}

impl From<crate::domain::invoice::InvoiceStatus> for GraphQlInvoiceStatus {
    fn from(s: crate::domain::invoice::InvoiceStatus) -> Self {
        match s {
            crate::domain::invoice::InvoiceStatus::Draft => GraphQlInvoiceStatus::Draft,
            crate::domain::invoice::InvoiceStatus::Pending => GraphQlInvoiceStatus::Pending,
            crate::domain::invoice::InvoiceStatus::Approved => GraphQlInvoiceStatus::Approved,
            crate::domain::invoice::InvoiceStatus::Sent => GraphQlInvoiceStatus::Sent,
            crate::domain::invoice::InvoiceStatus::PartiallyPaid => {
                GraphQlInvoiceStatus::PartiallyPaid
            }
            crate::domain::invoice::InvoiceStatus::Paid => GraphQlInvoiceStatus::Paid,
            crate::domain::invoice::InvoiceStatus::Overdue => GraphQlInvoiceStatus::Overdue,
            crate::domain::invoice::InvoiceStatus::Cancelled => GraphQlInvoiceStatus::Cancelled,
            crate::domain::invoice::InvoiceStatus::Refunded => GraphQlInvoiceStatus::Refunded,
        }
    }
}

impl From<GraphQlInvoiceStatus> for crate::domain::invoice::InvoiceStatus {
    fn from(s: GraphQlInvoiceStatus) -> Self {
        match s {
            GraphQlInvoiceStatus::Draft => crate::domain::invoice::InvoiceStatus::Draft,
            GraphQlInvoiceStatus::Pending => crate::domain::invoice::InvoiceStatus::Pending,
            GraphQlInvoiceStatus::Approved => crate::domain::invoice::InvoiceStatus::Approved,
            GraphQlInvoiceStatus::Sent => crate::domain::invoice::InvoiceStatus::Sent,
            GraphQlInvoiceStatus::PartiallyPaid => {
                crate::domain::invoice::InvoiceStatus::PartiallyPaid
            }
            GraphQlInvoiceStatus::Paid => crate::domain::invoice::InvoiceStatus::Paid,
            GraphQlInvoiceStatus::Overdue => crate::domain::invoice::InvoiceStatus::Overdue,
            GraphQlInvoiceStatus::Cancelled => crate::domain::invoice::InvoiceStatus::Cancelled,
            GraphQlInvoiceStatus::Refunded => crate::domain::invoice::InvoiceStatus::Refunded,
        }
    }
}

/// GraphQL Invoice type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum GraphQlInvoiceType {
    SalesInvoice,
    PurchaseInvoice,
    SalesReturn,
    PurchaseReturn,
}

impl From<crate::domain::invoice::InvoiceType> for GraphQlInvoiceType {
    fn from(t: crate::domain::invoice::InvoiceType) -> Self {
        match t {
            crate::domain::invoice::InvoiceType::SalesInvoice => GraphQlInvoiceType::SalesInvoice,
            crate::domain::invoice::InvoiceType::PurchaseInvoice => {
                GraphQlInvoiceType::PurchaseInvoice
            }
            crate::domain::invoice::InvoiceType::SalesReturn => GraphQlInvoiceType::SalesReturn,
            crate::domain::invoice::InvoiceType::PurchaseReturn => {
                GraphQlInvoiceType::PurchaseReturn
            }
        }
    }
}

/// GraphQL Invoice line item
#[derive(SimpleObject, Clone, Debug)]
pub struct GraphQlInvoiceLine {
    pub id: ID,
    pub invoice_id: i64,
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: String,
    pub unit_price: String,
    pub tax_rate: String,
    pub discount_rate: String,
    pub line_total: String,
    pub sort_order: i32,
}

impl From<crate::domain::invoice::InvoiceLine> for GraphQlInvoiceLine {
    fn from(l: crate::domain::invoice::InvoiceLine) -> Self {
        Self {
            id: ID::from(l.id.to_string()),
            invoice_id: l.invoice_id,
            product_id: l.product_id,
            description: l.description,
            quantity: l.quantity.to_string(),
            unit_price: l.unit_price.to_string(),
            tax_rate: l.tax_rate.to_string(),
            discount_rate: l.discount_rate.to_string(),
            line_total: l.line_total.to_string(),
            sort_order: l.sort_order,
        }
    }
}

/// GraphQL Invoice type
#[derive(SimpleObject, Clone, Debug)]
pub struct GraphQlInvoice {
    pub id: ID,
    pub invoice_number: String,
    pub invoice_type: GraphQlInvoiceType,
    pub status: GraphQlInvoiceStatus,
    pub cari_id: i64,
    pub issue_date: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub subtotal: String,
    pub tax_amount: String,
    pub discount_amount: String,
    pub total_amount: String,
    pub paid_amount: String,
    pub currency: String,
    pub exchange_rate: String,
    pub notes: Option<String>,
    pub lines: Vec<GraphQlInvoiceLine>,
    pub company_id: i64,
}

impl From<crate::domain::invoice::InvoiceResponse> for GraphQlInvoice {
    fn from(i: crate::domain::invoice::InvoiceResponse) -> Self {
        Self {
            id: ID::from(i.id.to_string()),
            invoice_number: i.invoice_number,
            invoice_type: i.invoice_type.into(),
            status: i.status.into(),
            cari_id: i.cari_id,
            issue_date: i.issue_date,
            due_date: i.due_date,
            subtotal: i.subtotal.to_string(),
            tax_amount: i.tax_amount.to_string(),
            discount_amount: i.discount_amount.to_string(),
            total_amount: i.total_amount.to_string(),
            paid_amount: i.paid_amount.to_string(),
            currency: i.currency,
            exchange_rate: i.exchange_rate.to_string(),
            notes: i.notes,
            lines: i.lines.into_iter().map(Into::into).collect(),
            company_id: i.company_id,
        }
    }
}

/// GraphQL Product type
#[derive(SimpleObject, Clone, Debug)]
pub struct GraphQlProduct {
    pub id: ID,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub barcode: Option<String>,
    pub purchase_price: String,
    pub sale_price: String,
    pub tax_rate: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub company_id: i64,
}

impl From<crate::domain::product::ProductResponse> for GraphQlProduct {
    fn from(p: crate::domain::product::ProductResponse) -> Self {
        Self {
            id: ID::from(p.id.to_string()),
            code: p.code,
            name: p.name,
            description: p.description,
            category_id: p.category_id,
            unit_id: p.unit_id,
            barcode: p.barcode,
            purchase_price: p.purchase_price.to_string(),
            sale_price: p.sale_price.to_string(),
            tax_rate: p.tax_rate.to_string(),
            is_active: p.is_active,
            created_at: p.created_at,
            company_id: p.company_id,
        }
    }
}

impl From<crate::domain::product::Product> for GraphQlProduct {
    fn from(p: crate::domain::product::Product) -> Self {
        Self {
            id: ID::from(p.id.to_string()),
            code: p.code,
            name: p.name,
            description: p.description,
            category_id: p.category_id,
            unit_id: p.unit_id,
            barcode: p.barcode,
            purchase_price: p.purchase_price.to_string(),
            sale_price: p.sale_price.to_string(),
            tax_rate: p.tax_rate.to_string(),
            is_active: p.is_active,
            created_at: p.created_at,
            company_id: p.company_id,
        }
    }
}

/// GraphQL Cari type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum GraphQlCariType {
    Customer,
    Vendor,
    Both,
}

impl From<crate::domain::cari::CariType> for GraphQlCariType {
    fn from(t: crate::domain::cari::CariType) -> Self {
        match t {
            crate::domain::cari::CariType::Customer => GraphQlCariType::Customer,
            crate::domain::cari::CariType::Vendor => GraphQlCariType::Vendor,
            crate::domain::cari::CariType::Both => GraphQlCariType::Both,
        }
    }
}

impl From<GraphQlCariType> for crate::domain::cari::CariType {
    fn from(t: GraphQlCariType) -> Self {
        match t {
            GraphQlCariType::Customer => crate::domain::cari::CariType::Customer,
            GraphQlCariType::Vendor => crate::domain::cari::CariType::Vendor,
            GraphQlCariType::Both => crate::domain::cari::CariType::Both,
        }
    }
}

/// GraphQL Cari status enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum GraphQlCariStatus {
    Active,
    Passive,
    Blocked,
}

impl From<crate::domain::cari::CariStatus> for GraphQlCariStatus {
    fn from(s: crate::domain::cari::CariStatus) -> Self {
        match s {
            crate::domain::cari::CariStatus::Active => GraphQlCariStatus::Active,
            crate::domain::cari::CariStatus::Passive => GraphQlCariStatus::Passive,
            crate::domain::cari::CariStatus::Blocked => GraphQlCariStatus::Blocked,
        }
    }
}

/// GraphQL Cari type
#[derive(SimpleObject, Clone, Debug)]
pub struct GraphQlCari {
    pub id: ID,
    pub code: String,
    pub name: String,
    pub cari_type: GraphQlCariType,
    pub tax_number: Option<String>,
    pub tax_office: Option<String>,
    pub identity_number: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub credit_limit: String,
    pub current_balance: String,
    pub default_currency: String,
    pub status: GraphQlCariStatus,
    pub tenant_id: i64,
    pub company_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<crate::domain::cari::CariResponse> for GraphQlCari {
    fn from(c: crate::domain::cari::CariResponse) -> Self {
        Self {
            id: ID::from(c.id.to_string()),
            code: c.code,
            name: c.name,
            cari_type: c.cari_type.into(),
            tax_number: c.tax_number,
            tax_office: c.tax_office,
            identity_number: c.identity_number,
            email: c.email,
            phone: c.phone,
            address: c.address,
            city: c.city,
            country: c.country,
            postal_code: c.postal_code,
            credit_limit: c.credit_limit.to_string(),
            current_balance: c.current_balance.to_string(),
            default_currency: c.default_currency,
            status: c.status.into(),
            tenant_id: c.tenant_id,
            company_id: c.company_id,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

/// Pagination info for GraphQL connections
#[derive(SimpleObject, Clone, Debug)]
pub struct PageInfo {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

impl PageInfo {
    pub fn from_paginated<T>(result: &crate::common::pagination::PaginatedResult<T>) -> Self {
        Self {
            page: result.page,
            per_page: result.per_page,
            total: result.total,
            total_pages: result.total_pages,
            has_next_page: result.has_next_page(),
            has_previous_page: result.has_previous_page(),
        }
    }
}

/// User connection with pagination
#[derive(SimpleObject, Clone, Debug)]
pub struct UserConnection {
    pub items: Vec<GraphQlUser>,
    pub page_info: PageInfo,
}

/// Employee connection with pagination
#[derive(SimpleObject, Clone, Debug)]
pub struct EmployeeConnection {
    pub items: Vec<GraphQlEmployee>,
    pub page_info: PageInfo,
}

/// Invoice connection with pagination
#[derive(SimpleObject, Clone, Debug)]
pub struct InvoiceConnection {
    pub items: Vec<GraphQlInvoice>,
    pub page_info: PageInfo,
}

/// Product connection with pagination
#[derive(SimpleObject, Clone, Debug)]
pub struct ProductConnection {
    pub items: Vec<GraphQlProduct>,
    pub page_info: PageInfo,
}

/// Cari connection with pagination
#[derive(SimpleObject, Clone, Debug)]
pub struct CariConnection {
    pub items: Vec<GraphQlCari>,
    pub page_info: PageInfo,
}
