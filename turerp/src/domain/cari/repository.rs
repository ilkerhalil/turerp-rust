//! Cari repository

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::cari::model::{Cari, CreateCari, UpdateCari};
use crate::error::ApiError;

/// Repository trait for Cari operations
#[async_trait]
pub trait CariRepository: Send + Sync {
    /// Create a new cari
    async fn create(&self, cari: CreateCari) -> Result<Cari, ApiError>;

    /// Find cari by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Cari>, ApiError>;

    /// Find cari by code
    async fn find_by_code(&self, code: &str, tenant_id: i64) -> Result<Option<Cari>, ApiError>;

    /// Find all cari accounts for a tenant
    async fn find_all(&self, tenant_id: i64) -> Result<Vec<Cari>, ApiError>;

    /// Find cari accounts by type
    async fn find_by_type(
        &self,
        cari_type: crate::domain::cari::model::CariType,
        tenant_id: i64,
    ) -> Result<Vec<Cari>, ApiError>;

    /// Search cari accounts by name or code
    async fn search(&self, query: &str, tenant_id: i64) -> Result<Vec<Cari>, ApiError>;

    /// Update a cari
    async fn update(&self, id: i64, tenant_id: i64, cari: UpdateCari) -> Result<Cari, ApiError>;

    /// Delete a cari
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Check if code exists
    async fn code_exists(&self, code: &str, tenant_id: i64) -> Result<bool, ApiError>;

    /// Update balance
    async fn update_balance(&self, id: i64, tenant_id: i64, amount: f64) -> Result<(), ApiError>;
}

/// Type alias for boxed repository
pub type BoxCariRepository = Arc<dyn CariRepository>;

/// In-memory cari repository for testing
pub struct InMemoryCariRepository {
    cari: std::sync::Mutex<std::collections::HashMap<i64, Cari>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryCariRepository {
    pub fn new() -> Self {
        Self {
            cari: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
        }
    }
}

impl Default for InMemoryCariRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CariRepository for InMemoryCariRepository {
    async fn create(&self, create: CreateCari) -> Result<Cari, ApiError> {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let now = chrono::Utc::now();

        let new_cari = Cari {
            id,
            code: create.code,
            name: create.name,
            cari_type: create.cari_type,
            tax_number: create.tax_number,
            tax_office: create.tax_office,
            identity_number: create.identity_number,
            email: create.email,
            phone: create.phone,
            address: create.address,
            city: create.city,
            country: create.country,
            postal_code: create.postal_code,
            credit_limit: create.credit_limit,
            current_balance: 0.0,
            status: crate::domain::cari::model::CariStatus::Active,
            tenant_id: create.tenant_id,
            created_by: create.created_by,
            created_at: now,
            updated_at: None,
        };

        self.cari.lock().unwrap().insert(id, new_cari.clone());
        Ok(new_cari)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Cari>, ApiError> {
        let cari = self.cari.lock().unwrap();
        Ok(cari
            .values()
            .find(|c| c.id == id && c.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_code(&self, code: &str, tenant_id: i64) -> Result<Option<Cari>, ApiError> {
        let cari = self.cari.lock().unwrap();
        Ok(cari
            .values()
            .find(|c| c.code == code && c.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<Cari>, ApiError> {
        let cari = self.cari.lock().unwrap();
        Ok(cari
            .values()
            .filter(|c| c.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_type(
        &self,
        cari_type: crate::domain::cari::model::CariType,
        tenant_id: i64,
    ) -> Result<Vec<Cari>, ApiError> {
        let cari = self.cari.lock().unwrap();
        Ok(cari
            .values()
            .filter(|c| c.tenant_id == tenant_id && c.cari_type == cari_type)
            .cloned()
            .collect())
    }

    async fn search(&self, query: &str, tenant_id: i64) -> Result<Vec<Cari>, ApiError> {
        let cari = self.cari.lock().unwrap();
        let query_lower = query.to_lowercase();
        Ok(cari
            .values()
            .filter(|c| {
                c.tenant_id == tenant_id
                    && (c.code.to_lowercase().contains(&query_lower)
                        || c.name.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect())
    }

    async fn update(&self, id: i64, tenant_id: i64, update: UpdateCari) -> Result<Cari, ApiError> {
        let mut cari_map = self.cari.lock().unwrap();

        let cari = cari_map
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Cari {} not found", id)))?;

        if cari.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Cari {} not found", id)));
        }

        if let Some(code) = update.code {
            cari.code = code;
        }
        if let Some(name) = update.name {
            cari.name = name;
        }
        if let Some(cari_type) = update.cari_type {
            cari.cari_type = cari_type;
        }
        if let Some(tax_number) = update.tax_number {
            cari.tax_number = Some(tax_number);
        }
        if let Some(tax_office) = update.tax_office {
            cari.tax_office = Some(tax_office);
        }
        if let Some(identity_number) = update.identity_number {
            cari.identity_number = Some(identity_number);
        }
        if let Some(email) = update.email {
            cari.email = Some(email);
        }
        if let Some(phone) = update.phone {
            cari.phone = Some(phone);
        }
        if let Some(address) = update.address {
            cari.address = Some(address);
        }
        if let Some(city) = update.city {
            cari.city = Some(city);
        }
        if let Some(country) = update.country {
            cari.country = Some(country);
        }
        if let Some(postal_code) = update.postal_code {
            cari.postal_code = Some(postal_code);
        }
        if let Some(credit_limit) = update.credit_limit {
            cari.credit_limit = credit_limit;
        }
        if let Some(status) = update.status {
            cari.status = status;
        }

        cari.updated_at = Some(chrono::Utc::now());

        Ok(cari.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut cari_map = self.cari.lock().unwrap();

        let cari = cari_map
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Cari {} not found", id)))?;

        if cari.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Cari {} not found", id)));
        }

        cari_map.remove(&id);
        Ok(())
    }

    async fn code_exists(&self, code: &str, tenant_id: i64) -> Result<bool, ApiError> {
        let cari = self.cari.lock().unwrap();
        Ok(cari
            .values()
            .any(|c| c.code == code && c.tenant_id == tenant_id))
    }

    async fn update_balance(&self, id: i64, tenant_id: i64, amount: f64) -> Result<(), ApiError> {
        let mut cari_map = self.cari.lock().unwrap();

        let cari = cari_map
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Cari {} not found", id)))?;

        if cari.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Cari {} not found", id)));
        }

        cari.current_balance += amount;
        cari.updated_at = Some(chrono::Utc::now());

        Ok(())
    }
}
