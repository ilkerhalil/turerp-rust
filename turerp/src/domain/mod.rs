//! Domain layer

pub mod accounting;
pub mod api_key;
pub mod archive;
pub mod assets;
pub mod audit;
pub mod auth;
pub mod bank;
pub mod barcode;
pub mod cari;
pub mod chart_of_accounts;
pub mod company;
pub mod cost_center;
pub mod crm;
pub mod currency;
pub mod custom_field;
pub mod customer_portal;
pub mod dashboard;
pub mod document;
pub mod earchive;
pub mod edefter;
pub mod efatura;
pub mod feature;
pub mod file;
pub mod forecasting;
pub mod hr;
pub mod inter_company;
pub mod invoice;
pub mod ip_whitelist;
pub mod job;
pub mod ldap;
pub mod manufacturing;
pub mod mfa;
pub mod notification;
pub mod observability;
pub mod product;
pub mod project;
pub mod purchase;
pub mod quality_control;
pub mod sales;
pub mod settings;
pub mod shift;
pub mod stock;
pub mod subscription;
pub mod tax;
pub mod tenant;
pub mod user;
pub mod vendor_portal;
pub mod webhook;
pub mod workflow;

#[cfg(test)]
// Test: is file module visible?
pub type __TestFileVisibility = crate::domain::file::model::FileRecord;
