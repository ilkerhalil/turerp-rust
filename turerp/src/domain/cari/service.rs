//! Cari service for business logic
use rust_decimal::Decimal;
use validator::Validate;

use crate::common::pagination::PaginatedResult;
use crate::domain::cari::model::{Cari, CariResponse, CreateCari, UpdateCari};
use crate::domain::cari::repository::BoxCariRepository;
use crate::domain::company::service::ensure_company_owned;
use crate::domain::company::BoxCompanyRepository;
use crate::error::ApiError;
use tracing;

/// Cari service
#[derive(Clone)]
pub struct CariService {
    repo: BoxCariRepository,
    company_repo: BoxCompanyRepository,
}

impl CariService {
    pub fn new(repo: BoxCariRepository, company_repo: BoxCompanyRepository) -> Self {
        Self { repo, company_repo }
    }

    /// Create a new cari account
    #[tracing::instrument(skip(self, create), fields(tenant_id = create.tenant_id))]
    pub async fn create_cari(&self, create: CreateCari) -> Result<CariResponse, ApiError> {
        // Validate input
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;

        // Parent-ownership precheck: a body-controlled company_id must reference a
        // company owned by the caller's tenant (auth-overwritten in
        // create.tenant_id) before the INSERT. The legacy phantom `1` sentinel
        // ("no specific company"; column is NOT NULL DEFAULT 1, no FK) is skipped.
        ensure_company_owned(&self.company_repo, create.company_id, create.tenant_id).await?;

        // Check if code exists
        if self
            .repo
            .code_exists(&create.code, create.tenant_id)
            .await?
        {
            return Err(ApiError::Conflict(format!(
                "Cari code '{}' already exists",
                create.code
            )));
        }

        // Create cari
        let cari = self.repo.create(create).await?;

        Ok(cari.into())
    }

    /// Get cari by ID
    #[tracing::instrument(skip(self), fields(tenant_id = tenant_id))]
    pub async fn get_cari(&self, id: i64, tenant_id: i64) -> Result<CariResponse, ApiError> {
        let cari = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Cari {} not found", id)))?;

        Ok(cari.into())
    }

    /// Get cari by code
    pub async fn get_cari_by_code(
        &self,
        code: &str,
        tenant_id: i64,
    ) -> Result<CariResponse, ApiError> {
        let cari = self
            .repo
            .find_by_code(code, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Cari {} not found", code)))?;

        Ok(cari.into())
    }

    /// Get all cari accounts for a tenant
    pub async fn get_all_cari(&self, tenant_id: i64) -> Result<Vec<CariResponse>, ApiError> {
        let cari_list = self.repo.find_all(tenant_id).await?;
        Ok(cari_list.into_iter().map(|c| c.into()).collect())
    }

    /// Get cari accounts by type
    pub async fn get_cari_by_type(
        &self,
        cari_type: crate::domain::cari::model::CariType,
        tenant_id: i64,
    ) -> Result<Vec<CariResponse>, ApiError> {
        let cari_list = self.repo.find_by_type(cari_type, tenant_id).await?;
        Ok(cari_list.into_iter().map(|c| c.into()).collect())
    }

    /// Search cari accounts
    pub async fn search_cari(
        &self,
        query: &str,
        tenant_id: i64,
    ) -> Result<Vec<CariResponse>, ApiError> {
        let cari_list = self.repo.search(query, tenant_id).await?;
        Ok(cari_list.into_iter().map(|c| c.into()).collect())
    }

    /// Get all cari accounts for a tenant with pagination
    pub async fn get_all_cari_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<CariResponse>, ApiError> {
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

    /// Get cari accounts by type with pagination
    pub async fn get_cari_by_type_paginated(
        &self,
        cari_type: crate::domain::cari::model::CariType,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<CariResponse>, ApiError> {
        let result = self
            .repo
            .find_by_type_paginated(cari_type, tenant_id, page, per_page)
            .await?;
        Ok(PaginatedResult::new(
            result.items.into_iter().map(|c| c.into()).collect(),
            result.page,
            result.per_page,
            result.total,
        ))
    }

    /// Search cari accounts with pagination
    pub async fn search_cari_paginated(
        &self,
        query: &str,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<CariResponse>, ApiError> {
        let result = self
            .repo
            .search_paginated(query, tenant_id, page, per_page)
            .await?;
        Ok(PaginatedResult::new(
            result.items.into_iter().map(|c| c.into()).collect(),
            result.page,
            result.per_page,
            result.total,
        ))
    }

    /// Update a cari account
    #[tracing::instrument(skip(self), fields(tenant_id = tenant_id))]
    pub async fn update_cari(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCari,
    ) -> Result<CariResponse, ApiError> {
        // Validate input
        update
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;

        // Check if code changed and exists
        if let Some(ref code) = update.code {
            let existing = self.repo.find_by_code(code, tenant_id).await?;
            if let Some(c) = existing {
                if c.id != id {
                    return Err(ApiError::Conflict(format!(
                        "Cari code '{}' already exists",
                        code
                    )));
                }
            }
        }

        let cari = self.repo.update(id, tenant_id, update).await?;
        Ok(cari.into())
    }

    /// Delete a cari account
    pub async fn delete_cari(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted cari (admin only)
    pub async fn restore_cari(&self, id: i64, tenant_id: i64) -> Result<Cari, ApiError> {
        self.repo.restore(id, tenant_id).await
    }

    /// List soft-deleted cari accounts (admin only)
    pub async fn list_deleted_cari(&self, tenant_id: i64) -> Result<Vec<Cari>, ApiError> {
        self.repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a cari (admin only, after soft delete)
    pub async fn destroy_cari(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await
    }

    /// Update cari balance (for financial transactions)
    pub async fn update_balance(
        &self,
        id: i64,
        tenant_id: i64,
        amount: Decimal,
    ) -> Result<(), ApiError> {
        // Verify cari exists
        let cari = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Cari {} not found", id)))?;

        // Check credit limit for negative amounts (debit operations)
        if amount < Decimal::ZERO {
            let new_balance = cari.current_balance + amount;
            if new_balance < -cari.credit_limit {
                return Err(ApiError::BadRequest(format!(
                    "Credit limit exceeded: current balance {:.2}, amount {:.2}, credit limit {:.2}",
                    cari.current_balance, amount, cari.credit_limit
                )));
            }
        }

        self.repo.update_balance(id, tenant_id, amount).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cari::model::CariType;
    use crate::domain::cari::repository::InMemoryCariRepository;
    use crate::domain::company::model::CreateCompany;
    use crate::domain::company::repository::InMemoryCompanyRepository;
    use crate::domain::company::BoxCompanyRepository;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_service() -> CariService {
        let repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        let company_repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;
        CariService::new(repo, company_repo)
    }

    /// Helper: seed a real company on a tenant via the repo and return its id
    /// (used to exercise the non-sentinel branch of the company_id precheck).
    async fn seed_company(company_repo: &BoxCompanyRepository, tenant_id: i64) -> i64 {
        let company = company_repo
            .create(CreateCompany {
                code: format!("COMP-{}", tenant_id),
                name: format!("Company T{}", tenant_id),
                tax_number: None,
                address: None,
                city: None,
                country: None,
                currency: "TRY".to_string(),
                tenant_id,
            })
            .await
            .unwrap();
        company.id
    }

    #[tokio::test]
    async fn test_create_cari_success() {
        let service = create_service();

        let create = CreateCari {
            code: "C001".to_string(),
            name: "Test Customer".to_string(),
            cari_type: CariType::Customer,
            tax_number: Some("1234567890".to_string()),
            tax_office: None,
            identity_number: None,
            email: Some("test@example.com".to_string()),
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: dec!(1000),
            tenant_id: 1,
            company_id: 1,
            created_by: 1,
            default_currency: "TRY".to_string(),
        };

        let result = service.create_cari(create).await;
        assert!(result.is_ok());
        let cari = result.unwrap();
        assert_eq!(cari.code, "C001");
        assert_eq!(cari.name, "Test Customer");
    }

    #[tokio::test]
    async fn test_create_cari_duplicate_code() {
        let service = create_service();

        let create = CreateCari {
            code: "C001".to_string(),
            name: "Test Customer".to_string(),
            cari_type: CariType::Customer,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            tenant_id: 1,
            company_id: 1,
            created_by: 1,
            default_currency: "TRY".to_string(),
        };

        service.create_cari(create.clone()).await.unwrap();

        let result = service.create_cari(create).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_get_cari_by_id() {
        let service = create_service();

        let create = CreateCari {
            code: "C001".to_string(),
            name: "Test Customer".to_string(),
            cari_type: CariType::Customer,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            tenant_id: 1,
            company_id: 1,
            created_by: 1,
            default_currency: "TRY".to_string(),
        };

        let created = service.create_cari(create).await.unwrap();

        let result = service.get_cari(created.id, 1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().code, "C001");
    }

    #[tokio::test]
    async fn test_get_cari_not_found() {
        let service = create_service();

        let result = service.get_cari(999, 1).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_search_cari() {
        let service = create_service();

        // Create multiple cari accounts
        let create1 = CreateCari {
            code: "C001".to_string(),
            name: "ABC Company".to_string(),
            cari_type: CariType::Customer,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            tenant_id: 1,
            company_id: 1,
            created_by: 1,
            default_currency: "TRY".to_string(),
        };

        let create2 = CreateCari {
            code: "V001".to_string(),
            name: "XYZ Vendor".to_string(),
            cari_type: CariType::Vendor,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            tenant_id: 1,
            company_id: 1,
            created_by: 1,
            default_currency: "TRY".to_string(),
        };

        service.create_cari(create1).await.unwrap();
        service.create_cari(create2).await.unwrap();

        // Search
        let result = service.search_cari("abc", 1).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "ABC Company");
    }

    #[tokio::test]
    async fn test_update_cari() {
        let service = create_service();

        let create = CreateCari {
            code: "C001".to_string(),
            name: "Test Customer".to_string(),
            cari_type: CariType::Customer,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            tenant_id: 1,
            company_id: 1,
            created_by: 1,
            default_currency: "TRY".to_string(),
        };

        let created = service.create_cari(create).await.unwrap();

        let update = UpdateCari {
            name: Some("Updated Name".to_string()),
            credit_limit: Some(dec!(5000)),
            ..Default::default()
        };

        let result = service.update_cari(created.id, 1, update).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Updated Name");
    }

    #[tokio::test]
    async fn test_delete_cari() {
        let service = create_service();

        let create = CreateCari {
            code: "C001".to_string(),
            name: "Test Customer".to_string(),
            cari_type: CariType::Customer,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            tenant_id: 1,
            company_id: 1,
            created_by: 1,
            default_currency: "TRY".to_string(),
        };

        let created = service.create_cari(create).await.unwrap();

        let result = service.delete_cari(created.id, 1, 1).await;
        assert!(result.is_ok());

        // Verify deleted
        let result = service.get_cari(created.id, 1).await;
        assert!(result.is_err());
    }

    /// A cari referencing a company owned by another tenant must be rejected
    /// before the INSERT (cross-tenant IDOR closure on company_id).
    #[tokio::test]
    async fn test_create_cari_rejects_foreign_company() {
        let service = create_service();
        // Seed an owned company on tenant 1 first so the InMemory auto-id
        // counter advances past the legacy phantom `1` (the first company
        // created would otherwise get id == LEGACY_COMPANY_ID and be skipped by
        // the sentinel-aware precheck). The tenant-2 company then gets a real
        // non-sentinel id.
        let _owned = seed_company(&service.company_repo, 1).await;
        let foreign_company_id = seed_company(&service.company_repo, 2).await;
        assert_ne!(
            foreign_company_id,
            crate::domain::company::service::LEGACY_COMPANY_ID
        );

        let create = CreateCari {
            code: "C-FOREIGN".to_string(),
            name: "Foreign Company Cari".to_string(),
            cari_type: CariType::Customer,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            tenant_id: 1,
            company_id: foreign_company_id,
            created_by: 1,
            default_currency: "TRY".to_string(),
        };

        let result = service.create_cari(create).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::NotFound(msg) if msg == "Company not found"
        ));
    }

    /// A cari referencing an owned company must be accepted.
    #[tokio::test]
    async fn test_create_cari_accepts_owned_company() {
        let service = create_service();
        // Advance the InMemory auto-id counter past the legacy phantom `1` so
        // the owned company gets a real non-sentinel id (exercising the actual
        // precheck success path, not the sentinel skip).
        let _phantom = seed_company(&service.company_repo, 2).await;
        let owned_company_id = seed_company(&service.company_repo, 1).await;
        assert_ne!(
            owned_company_id,
            crate::domain::company::service::LEGACY_COMPANY_ID
        );

        let create = CreateCari {
            code: "C-OWNED".to_string(),
            name: "Owned Company Cari".to_string(),
            cari_type: CariType::Customer,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            tenant_id: 1,
            company_id: owned_company_id,
            created_by: 1,
            default_currency: "TRY".to_string(),
        };

        let result = service.create_cari(create).await;
        assert!(result.is_ok());
    }

    /// The legacy phantom `company_id == 1` sentinel ("no specific company") is
    /// accepted unchanged (backward compatibility) — not a real reference.
    #[tokio::test]
    async fn test_create_cari_accepts_legacy_company_sentinel() {
        let service = create_service();

        let create = CreateCari {
            code: "C-PHANTOM".to_string(),
            name: "Phantom Company Cari".to_string(),
            cari_type: CariType::Customer,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            tenant_id: 1,
            company_id: crate::domain::company::service::LEGACY_COMPANY_ID,
            created_by: 1,
            default_currency: "TRY".to_string(),
        };

        let result = service.create_cari(create).await;
        assert!(result.is_ok());
    }
}
