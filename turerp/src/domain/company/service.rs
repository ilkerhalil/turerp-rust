//! Company service

use validator::Validate;

use crate::common::pagination::PaginatedResult;
use crate::domain::company::model::{Company, CompanyResponse, CreateCompany, UpdateCompany};
use crate::domain::company::repository::BoxCompanyRepository;
use crate::error::ApiError;

#[derive(Clone)]
pub struct CompanyService {
    repo: BoxCompanyRepository,
}

impl CompanyService {
    pub fn new(repo: BoxCompanyRepository) -> Self {
        Self { repo }
    }

    pub async fn create_company(&self, create: CreateCompany) -> Result<CompanyResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;
        if self
            .repo
            .code_exists(&create.code, create.tenant_id)
            .await?
        {
            return Err(ApiError::Conflict(format!(
                "Company code '{}' already exists",
                create.code
            )));
        }
        let company = self.repo.create(create).await?;
        Ok(company.into())
    }

    pub async fn get_company(&self, id: i64, tenant_id: i64) -> Result<CompanyResponse, ApiError> {
        let company = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Company {} not found", id)))?;
        Ok(company.into())
    }

    pub async fn get_company_by_code(
        &self,
        code: &str,
        tenant_id: i64,
    ) -> Result<CompanyResponse, ApiError> {
        let company = self
            .repo
            .find_by_code(code, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Company {} not found", code)))?;
        Ok(company.into())
    }

    pub async fn get_all_companies(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<CompanyResponse>, ApiError> {
        let companies = self.repo.find_by_tenant(tenant_id).await?;
        Ok(companies.into_iter().map(|c| c.into()).collect())
    }

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
                    return Err(ApiError::Conflict(format!(
                        "Company code '{}' already exists",
                        code
                    )));
                }
            }
        }
        let company = self.repo.update(id, tenant_id, update).await?;
        Ok(company.into())
    }

    pub async fn delete_company(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await
    }

    pub async fn restore_company(&self, id: i64, tenant_id: i64) -> Result<Company, ApiError> {
        self.repo.restore(id, tenant_id).await
    }

    pub async fn list_deleted_companies(&self, tenant_id: i64) -> Result<Vec<Company>, ApiError> {
        self.repo.find_deleted(tenant_id).await
    }

    pub async fn destroy_company(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await
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
