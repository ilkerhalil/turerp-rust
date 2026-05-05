//! GIB (Gelir İdaresi Başkanlığı) format generators for e-Defter
//!
//! This module provides XML generators conforming to the GIB specification
//! for Turkish electronic ledger submissions, including:
//! - Yevmiye defteri (journal ledger)
//! - Büyük defter (general ledger)
//! - Berat (certificate/signing)

pub mod berat;
pub mod buyuk_defter;
pub mod yevmiye;

// Re-exports
pub use berat::generate_berat_xml;
pub use buyuk_defter::generate_buyuk_defter_xml;
pub use yevmiye::generate_yevmiye_xml;
