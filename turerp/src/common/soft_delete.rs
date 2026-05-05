//! Soft delete utilities for all domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Marker trait for entities that support soft deletion.
///
/// All find queries in repositories must filter: `WHERE deleted_at IS NULL`.
/// Admin-only endpoints enable listing deleted records and restoring them.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoftDeleteMeta {
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

/// Actions available for soft-deletable entities.
pub trait SoftDeletable {
    fn is_deleted(&self) -> bool;
    fn deleted_at(&self) -> Option<DateTime<Utc>>;
    fn deleted_by(&self) -> Option<i64>;
    fn mark_deleted(&mut self, by_user_id: i64);
    fn restore(&mut self);
}

impl SoftDeletable for SoftDeleteMeta {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }

    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }

    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }

    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}
