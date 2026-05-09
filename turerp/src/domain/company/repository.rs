//! Company repository

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::common::SoftDeletable;
use crate::domain::company::model::{Company, CreateCompany, UpdateCompany};
use crate::error::ApiError;
use chrono::Utc;

/// Repository trait for Company operations
#[async_trait]
pub trait CompanyRepository: Send + Sync {
    async fn create(&self, company: CreateCompany) -> Result<Company, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Company>, ApiError>;
    async fn find_by_code(&self, code: &str, tenant_id: i64) -> Result<Option<Company>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Company>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Company>, ApiError>;
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        company: UpdateCompany,
    ) -> Result<Company, ApiError>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Company, ApiError>;
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Company>, ApiError>;
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
    async fn code_exists(&self, code: &str, tenant_id: i64) -> Result<bool, ApiError>;
}

/// Type alias for boxed repository
pub type BoxCompanyRepository = Arc<dyn CompanyRepository>;

struct InMemoryCompanyInner {
    companies: std::collections::HashMap<i64, Company>,
    next_id: i64,
}

/// In-memory company repository for testing
pub struct InMemoryCompanyRepository {
    inner: Mutex<InMemoryCompanyInner>,
}

impl InMemoryCompanyRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryCompanyInner {
                companies: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryCompanyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CompanyRepository for InMemoryCompanyRepository {
    async fn create(&self, create: CreateCompany) -> Result<Company, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let company = Company::new(id, create.tenant_id, create.code, create.name);
        inner.companies.insert(id, company.clone());
        Ok(company)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Company>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .companies
            .get(&id)
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .cloned())
    }

    async fn find_by_code(&self, code: &str, tenant_id: i64) -> Result<Option<Company>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .companies
            .values()
            .find(|c| c.code == code && c.tenant_id == tenant_id && !c.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Company>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .companies
            .values()
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Company>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .companies
            .values()
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCompany,
    ) -> Result<Company, ApiError> {
        let mut inner = self.inner.lock();
        let company = inner
            .companies
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Company {} not found", id)))?;
        if company.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Company {} not found", id)));
        }
        if let Some(code) = update.code {
            company.code = code;
        }
        if let Some(name) = update.name {
            company.name = name;
        }
        if let Some(tax_number) = update.tax_number {
            company.tax_number = Some(tax_number);
        }
        if let Some(address) = update.address {
            company.address = Some(address);
        }
        if let Some(city) = update.city {
            company.city = Some(city);
        }
        if let Some(country) = update.country {
            company.country = Some(country);
        }
        if let Some(currency) = update.currency {
            company.currency = currency;
        }
        if let Some(is_active) = update.is_active {
            company.is_active = is_active;
        }
        company.updated_at = Some(Utc::now());
        Ok(company.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let company = inner
            .companies
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Company {} not found", id)))?;
        if company.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Company {} not found", id)));
        }
        company.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Company, ApiError> {
        let mut inner = self.inner.lock();
        let company = inner
            .companies
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Company {} not found", id)))?;
        if company.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Company {} not found", id)));
        }
        company.restore();
        Ok(company.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Company>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .companies
            .values()
            .filter(|c| c.tenant_id == tenant_id && c.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let company = inner
            .companies
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Company {} not found", id)))?;
        if company.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Company {} not found", id)));
        }
        inner.companies.remove(&id);
        Ok(())
    }

    async fn code_exists(&self, code: &str, tenant_id: i64) -> Result<bool, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .companies
            .values()
            .any(|c| c.code == code && c.tenant_id == tenant_id))
    }
}
