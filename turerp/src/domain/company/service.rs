//! Company service

use validator::Validate;

use crate::common::pagination::PaginatedResult;
use crate::domain::company::model::{Company, CompanyResponse, CreateCompany, UpdateCompany};
use crate::domain::company::repository::BoxCompanyRepository;
use crate::error::ApiError;

/// Legacy "no specific company" sentinel. The `company_id` column was added by
/// migration 023+ as `BIGINT NOT NULL DEFAULT 1` with **no FK** to
/// `companies(id)` ("default 1 for backward compatibility"). Rows stamped with
/// `1` therefore need not reference an existing company. The parent-ownership
/// precheck (`ensure_company_owned`) skips this sentinel so legacy/default
/// callers are not rejected; any other `company_id` must be owned by the
/// caller's tenant.
pub const LEGACY_COMPANY_ID: i64 = 1;

/// Parent-ownership precheck for a body-controlled `company_id` (sentinel-aware).
///
/// Returns `NotFound` if `company_id` does not belong to `tenant_id`, **except**
/// the legacy phantom `1` (`LEGACY_COMPANY_ID`) which is accepted unchanged for
/// backward compatibility (the column is `NOT NULL DEFAULT 1` with no FK, so `1`
/// is a "no specific company" sentinel, not a real reference). Closes the
/// cross-tenant IDOR where a tenant-A caller stamps a tenant-B company id (or a
/// fabricated id) onto a row in their own tenant.
pub async fn ensure_company_owned(
    repo: &BoxCompanyRepository,
    company_id: i64,
    tenant_id: i64,
) -> Result<(), ApiError> {
    if company_id == LEGACY_COMPANY_ID {
        return Ok(());
    }
    repo.find_by_id(company_id, tenant_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Company not found".to_string()))?;
    Ok(())
}

#[derive(Clone)]
pub struct CompanyService {
    repo: BoxCompanyRepository,
}

impl CompanyService {
    pub fn new(repo: BoxCompanyRepository) -> Self {
        Self { repo }
    }

    #[tracing::instrument(skip(self))]
    pub async fn create_company(&self, create: CreateCompany) -> Result<CompanyResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;
        if self
            .repo
            .code_exists(&create.code, create.tenant_id)
            .await?
        {
            tracing::warn!(tenant_id = create.tenant_id, code = %create.code, "Company code already exists");
            return Err(ApiError::Conflict(format!(
                "Company code '{}' already exists",
                create.code
            )));
        }
        let tenant_id = create.tenant_id;
        let company = self.repo.create(create).await?;
        tracing::info!(tenant_id, "Created company");
        Ok(company.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_company(&self, id: i64, tenant_id: i64) -> Result<CompanyResponse, ApiError> {
        let company = self.repo.find_by_id(id, tenant_id).await?.ok_or_else(|| {
            tracing::warn!(tenant_id, company_id = id, "Company not found");
            ApiError::NotFound(format!("Company {} not found", id))
        })?;
        Ok(company.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_company_by_code(
        &self,
        code: &str,
        tenant_id: i64,
    ) -> Result<CompanyResponse, ApiError> {
        let company = self
            .repo
            .find_by_code(code, tenant_id)
            .await?
            .ok_or_else(|| {
                tracing::warn!(tenant_id, code, "Company not found by code");
                ApiError::NotFound(format!("Company {} not found", code))
            })?;
        Ok(company.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_all_companies(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<CompanyResponse>, ApiError> {
        let companies = self.repo.find_by_tenant(tenant_id).await?;
        Ok(companies.into_iter().map(|c| c.into()).collect())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_all_companies_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<CompanyResponse>, ApiError> {
        let result = self
            .repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await?;
        Ok(PaginatedResult::new(
            result.items.into_iter().map(|c| c.into()).collect(),
            result.page,
            result.per_page,
            result.total,
        ))
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_company(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCompany,
    ) -> Result<CompanyResponse, ApiError> {
        update
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;
        if let Some(ref code) = update.code {
            let existing = self.repo.find_by_code(code, tenant_id).await?;
            if let Some(c) = existing {
                if c.id != id {
                    tracing::warn!(tenant_id, code, "Company code already exists");
                    return Err(ApiError::Conflict(format!(
                        "Company code '{}' already exists",
                        code
                    )));
                }
            }
        }
        let company = self.repo.update(id, tenant_id, update).await?;
        tracing::info!(tenant_id, company_id = id, "Updated company");
        Ok(company.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_company(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await?;
        tracing::info!(tenant_id, company_id = id, "Deleted company");
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_company(&self, id: i64, tenant_id: i64) -> Result<Company, ApiError> {
        let company = self.repo.restore(id, tenant_id).await?;
        tracing::info!(tenant_id, company_id = id, "Restored company");
        Ok(company)
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_companies(&self, tenant_id: i64) -> Result<Vec<Company>, ApiError> {
        self.repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_company(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await?;
        tracing::info!(tenant_id, company_id = id, "Destroyed company");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::company::repository::InMemoryCompanyRepository;
    use std::sync::Arc;

    fn create_service() -> CompanyService {
        let repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;
        CompanyService::new(repo)
    }

    #[tokio::test]
    async fn test_create_company_success() {
        let service = create_service();
        let create = CreateCompany {
            code: "HQ".to_string(),
            name: "Headquarters".to_string(),
            tax_number: None,
            address: None,
            city: None,
            country: None,
            currency: "TRY".to_string(),
            tenant_id: 1,
        };
        let result = service.create_company(create).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().code, "HQ");
    }

    #[tokio::test]
    async fn test_create_company_duplicate_code() {
        let service = create_service();
        let create = CreateCompany {
            code: "HQ".to_string(),
            name: "Headquarters".to_string(),
            tax_number: None,
            address: None,
            city: None,
            country: None,
            currency: "TRY".to_string(),
            tenant_id: 1,
        };
        service.create_company(create.clone()).await.unwrap();
        let result = service.create_company(create).await;
        assert!(result.is_err());
    }
}
