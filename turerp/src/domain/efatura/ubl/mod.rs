//! UBL-TR module for e-Fatura XML mapping and validation
//!
//! Provides conversion between EFatura domain objects and UBL-TR XML format
//! (Turkey's e-invoicing standard based on UBL 2.1), plus structural
//! validation of generated XML documents.

pub mod mapper;
pub mod validator;

pub use mapper::{efatura_to_ubl_xml, ubl_xml_to_efatura_partial, UblPartialInvoice};
pub use validator::{validate_efatura, validate_ubl_xml, ValidationResult};
