//! Sales service for business logic
use crate::common::pagination::PaginatedResult;
use crate::domain::cari::repository::BoxCariRepository;
use crate::domain::product::repository::BoxProductRepository;
use crate::domain::sales::model::{
    CreateQuotation, CreateSalesOrder, CreateSalesOrderLine, Quotation, QuotationLine,
    QuotationResponse, QuotationStatus, SalesOrder, SalesOrderLine, SalesOrderResponse,
    SalesOrderStatus,
};
use crate::domain::sales::repository::{
    BoxQuotationLineRepository, BoxQuotationRepository, BoxSalesOrderLineRepository,
    BoxSalesOrderRepository,
};
use crate::error::ApiError;

/// Sales service
#[derive(Clone)]
pub struct SalesService {
    order_repo: BoxSalesOrderRepository,
    order_line_repo: BoxSalesOrderLineRepository,
    quotation_repo: BoxQuotationRepository,
    quotation_line_repo: BoxQuotationLineRepository,
    cari_repo: BoxCariRepository,
    product_repo: BoxProductRepository,
}

impl SalesService {
    pub fn new(
        order_repo: BoxSalesOrderRepository,
        order_line_repo: BoxSalesOrderLineRepository,
        quotation_repo: BoxQuotationRepository,
        quotation_line_repo: BoxQuotationLineRepository,
        cari_repo: BoxCariRepository,
        product_repo: BoxProductRepository,
    ) -> Self {
        Self {
            order_repo,
            order_line_repo,
            quotation_repo,
            quotation_line_repo,
            cari_repo,
            product_repo,
        }
    }

    /// Parent-ownership precheck: verify the customer (cari) belongs to the
    /// caller's tenant before any INSERT. A body-controlled `cari_id`
    /// referencing a foreign tenant's customer would otherwise create an
    /// orphaned sales order / quotation (cross-tenant IDOR).
    async fn ensure_cari_owned(&self, cari_id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.cari_repo
            .find_by_id(cari_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Cari {} not found", cari_id)))?;
        Ok(())
    }

    /// Parent-ownership precheck for line-level products: verify every line
    /// `product_id` (Some only — None is a legitimate no-product line) belongs
    /// to the caller's tenant. Uses a single batched `find_by_ids` lookup; a
    /// foreign product is simply not returned by the tenant-scoped query, so a
    /// count mismatch means at least one line references a foreign product.
    async fn ensure_line_products_owned(
        &self,
        product_ids: impl IntoIterator<Item = Option<i64>>,
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        let ids: Vec<i64> = {
            let mut set: std::collections::HashSet<i64> =
                product_ids.into_iter().flatten().collect();
            let mut v: Vec<i64> = set.drain().collect();
            v.sort_unstable();
            v
        };
        if ids.is_empty() {
            return Ok(());
        }
        let found = self.product_repo.find_by_ids(&ids, tenant_id).await?;
        if found.len() != ids.len() {
            let found_ids: std::collections::HashSet<i64> = found.iter().map(|p| p.id).collect();
            let missing = ids.iter().find(|id| !found_ids.contains(id)).copied();
            return Err(ApiError::NotFound(format!(
                "Product {} not found",
                missing.unwrap_or(-1)
            )));
        }
        Ok(())
    }

    // Sales Order operations
    #[tracing::instrument(skip(self))]
    pub async fn create_sales_order(
        &self,
        create: CreateSalesOrder,
    ) -> Result<SalesOrderResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        for line in &create.lines {
            line.validate()
                .map_err(|e| ApiError::Validation(e.join(", ")))?;
        }

        // Parent-ownership prechecks: reject body-controlled FKs that reference
        // a foreign tenant's customer or product before any INSERT.
        self.ensure_cari_owned(create.cari_id, create.tenant_id)
            .await?;
        self.ensure_line_products_owned(
            create.lines.iter().map(|l| l.product_id),
            create.tenant_id,
        )
        .await?;

        let order = self.order_repo.create(create.clone()).await?;
        let lines = self
            .order_line_repo
            .create_many(order.id, create.lines, create.tenant_id)
            .await?;

        Ok(SalesOrderResponse::from((order, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_sales_order(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<SalesOrderResponse, ApiError> {
        let order = self
            .order_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Sales order {} not found", id)))?;

        let lines = self.order_line_repo.find_by_order(id, tenant_id).await?;

        Ok(SalesOrderResponse::from((order, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_orders_by_tenant(&self, tenant_id: i64) -> Result<Vec<SalesOrder>, ApiError> {
        self.order_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_orders_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<SalesOrder>, ApiError> {
        crate::common::pagination::PaginationParams { page, per_page }
            .validate()
            .map_err(ApiError::Validation)?;
        self.order_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_orders_by_cari(
        &self,
        cari_id: i64,
    ) -> Result<Vec<crate::domain::sales::model::SalesOrder>, ApiError> {
        self.order_repo.find_by_cari(cari_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_orders_by_status(
        &self,
        tenant_id: i64,
        status: SalesOrderStatus,
    ) -> Result<Vec<SalesOrder>, ApiError> {
        self.order_repo.find_by_status(tenant_id, status).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_orders_by_status_paginated(
        &self,
        tenant_id: i64,
        status: SalesOrderStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<SalesOrder>, ApiError> {
        crate::common::pagination::PaginationParams { page, per_page }
            .validate()
            .map_err(ApiError::Validation)?;
        self.order_repo
            .find_by_status_paginated(tenant_id, status, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_order_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: SalesOrderStatus,
    ) -> Result<SalesOrder, ApiError> {
        self.order_repo.update_status(id, tenant_id, status).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_order(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.order_line_repo.delete_by_order(id, tenant_id).await?;
        self.order_repo.delete(id, tenant_id).await
    }

    /// Soft delete a sales order (sets deleted_at) and its lines
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_order(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        // Parent first: a tenant-scoped soft_delete gates NotFound for a foreign
        // order before any line is touched (closes the soft_delete-before-parent
        // leak). The order FK soft-delete is an UPDATE (no CASCADE), so the lines
        // are unaffected and can be soft-deleted afterwards.
        self.order_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await?;
        let lines = self.order_line_repo.find_by_order(id, tenant_id).await?;
        for line in lines {
            self.order_line_repo
                .soft_delete(line.id, id, deleted_by, tenant_id)
                .await?;
        }
        Ok(())
    }

    /// Restore a soft-deleted sales order and its lines
    #[tracing::instrument(skip(self))]
    pub async fn restore_order(&self, id: i64, tenant_id: i64) -> Result<SalesOrder, ApiError> {
        let order = self.order_repo.restore(id, tenant_id).await?;
        // Restore all soft-deleted lines (tenant-scoped)
        let deleted_lines = self
            .order_line_repo
            .find_deleted(order.id, tenant_id)
            .await?;
        for line in deleted_lines {
            let _ = self
                .order_line_repo
                .restore(line.id, order.id, tenant_id)
                .await
                .ok();
        }
        Ok(order)
    }

    /// Restore a soft-deleted sales order and return as response
    #[tracing::instrument(skip(self))]
    pub async fn restore_order_response(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<SalesOrderResponse, ApiError> {
        let order = self.restore_order(id, tenant_id).await?;
        let lines = self
            .order_line_repo
            .find_by_order(order.id, tenant_id)
            .await?;
        Ok(SalesOrderResponse::from((order, lines)))
    }

    /// List soft-deleted sales orders
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_orders(&self, tenant_id: i64) -> Result<Vec<SalesOrder>, ApiError> {
        self.order_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a sales order (admin only, after soft delete)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_order(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        // The tenant-scoped parent destroy gates NotFound for a foreign order.
        // The parent FK has ON DELETE CASCADE (003), which removes the order's
        // lines atomically with the parent -- the redundant unscoped
        // delete_by_order(id) call is dropped (mirror of the #194 invoice_lines
        // fix and 054): it ran on the raw caller id *before* the tenant-scoped
        // parent destroy, hard-deleting a foreign tenant's lines.
        self.order_repo.destroy(id, tenant_id).await
    }

    // Quotation operations
    #[tracing::instrument(skip(self))]
    pub async fn create_quotation(
        &self,
        create: CreateQuotation,
    ) -> Result<QuotationResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        for line in &create.lines {
            line.validate()
                .map_err(|e| ApiError::Validation(e.join(", ")))?;
        }

        // Parent-ownership prechecks: reject body-controlled FKs that reference
        // a foreign tenant's customer or product before any INSERT.
        self.ensure_cari_owned(create.cari_id, create.tenant_id)
            .await?;
        self.ensure_line_products_owned(
            create.lines.iter().map(|l| l.product_id),
            create.tenant_id,
        )
        .await?;

        let quotation = self.quotation_repo.create(create.clone()).await?;
        let lines = self
            .quotation_line_repo
            .create_many(quotation.id, create.lines, create.tenant_id)
            .await?;

        Ok(QuotationResponse::from((quotation, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_quotation(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<QuotationResponse, ApiError> {
        let quotation = self
            .quotation_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Quotation {} not found", id)))?;

        let lines = self
            .quotation_line_repo
            .find_by_quotation(id, tenant_id)
            .await?;

        Ok(QuotationResponse::from((quotation, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_quotations_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Quotation>, ApiError> {
        self.quotation_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_quotations_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Quotation>, ApiError> {
        crate::common::pagination::PaginationParams { page, per_page }
            .validate()
            .map_err(ApiError::Validation)?;
        self.quotation_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_quotations_by_cari(
        &self,
        cari_id: i64,
    ) -> Result<Vec<crate::domain::sales::model::Quotation>, ApiError> {
        self.quotation_repo.find_by_cari(cari_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_quotations_by_status(
        &self,
        tenant_id: i64,
        status: QuotationStatus,
    ) -> Result<Vec<Quotation>, ApiError> {
        self.quotation_repo.find_by_status(tenant_id, status).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_quotations_by_status_paginated(
        &self,
        tenant_id: i64,
        status: QuotationStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Quotation>, ApiError> {
        crate::common::pagination::PaginationParams { page, per_page }
            .validate()
            .map_err(ApiError::Validation)?;
        self.quotation_repo
            .find_by_status_paginated(tenant_id, status, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_quotation_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: QuotationStatus,
    ) -> Result<Quotation, ApiError> {
        self.quotation_repo
            .update_status(id, tenant_id, status)
            .await
    }

    /// Convert quotation to sales order
    #[tracing::instrument(skip(self))]
    pub async fn convert_quotation_to_order(
        &self,
        quotation_id: i64,
        tenant_id: i64,
    ) -> Result<SalesOrderResponse, ApiError> {
        let quotation = self
            .quotation_repo
            .find_by_id(quotation_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Quotation {} not found", quotation_id)))?;

        // Idempotency: if already converted, return the existing order
        if let Some(order_id) = quotation.sales_order_id {
            return self.get_sales_order(order_id, tenant_id).await;
        }

        if quotation.status == QuotationStatus::ConvertedToOrder {
            return Err(ApiError::BadRequest(
                "Quotation is already converted to order".to_string(),
            ));
        }

        // Get quotation lines
        let quote_lines = self
            .quotation_line_repo
            .find_by_quotation(quotation_id, tenant_id)
            .await?;

        // Create sales order lines
        let order_lines: Vec<CreateSalesOrderLine> = quote_lines
            .iter()
            .map(|l| CreateSalesOrderLine {
                product_id: l.product_id,
                description: l.description.clone(),
                quantity: l.quantity,
                unit_price: l.unit_price,
                tax_rate: l.tax_rate,
                discount_rate: l.discount_rate,
            })
            .collect();

        // Create sales order
        let order_lines_clone = order_lines.clone();
        let create = CreateSalesOrder {
            tenant_id: quotation.tenant_id,
            company_id: 0,
            cari_id: quotation.cari_id,
            order_date: chrono::Utc::now(),
            delivery_date: None,
            notes: quotation.notes.clone(),
            shipping_address: None,
            billing_address: None,
            currency: "TRY".to_string(),
            exchange_rate: rust_decimal::Decimal::ONE,
            lines: order_lines,
        };

        let order = self.order_repo.create(create).await?;

        let lines = match self
            .order_line_repo
            .create_many(order.id, order_lines_clone, tenant_id)
            .await
        {
            Ok(lines) => lines,
            Err(e) => {
                if let Err(rollback_err) =
                    self.order_repo.delete(order.id, quotation.tenant_id).await
                {
                    tracing::warn!(error = %rollback_err, "Failed to roll back orphan order {} after line creation failed", order.id);
                }
                return Err(e);
            }
        };

        // Update quotation status — if this fails, roll back order + lines
        if let Err(e) = self
            .quotation_repo
            .link_to_order(quotation_id, order.id)
            .await
        {
            if let Err(rollback_err) = self
                .order_line_repo
                .delete_by_order(order.id, tenant_id)
                .await
            {
                tracing::warn!(error = %rollback_err, "Failed to roll back order lines for order {} after quotation linking failed", order.id);
            }
            if let Err(rollback_err) = self.order_repo.delete(order.id, quotation.tenant_id).await {
                tracing::warn!(error = %rollback_err, "Failed to roll back orphan order {} after quotation linking failed", order.id);
            }
            return Err(e);
        }

        Ok(SalesOrderResponse::from((order, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_quotation(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.quotation_line_repo
            .delete_by_quotation(id, tenant_id)
            .await?;
        self.quotation_repo.delete(id, tenant_id).await
    }

    /// Soft delete a quotation (sets deleted_at) and its lines
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_quotation(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        // Parent first: a tenant-scoped soft_delete gates NotFound for a foreign
        // quotation before any line is touched (closes the soft_delete-before-
        // parent leak). The quotation soft-delete is an UPDATE (no CASCADE), so
        // the lines are unaffected and can be soft-deleted afterwards.
        self.quotation_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await?;
        let lines = self
            .quotation_line_repo
            .find_by_quotation(id, tenant_id)
            .await?;
        for line in lines {
            self.quotation_line_repo
                .soft_delete(line.id, id, deleted_by, tenant_id)
                .await?;
        }
        Ok(())
    }

    /// Restore a soft-deleted quotation and its lines
    #[tracing::instrument(skip(self))]
    pub async fn restore_quotation(&self, id: i64, tenant_id: i64) -> Result<Quotation, ApiError> {
        let quotation = self.quotation_repo.restore(id, tenant_id).await?;
        let deleted_lines = self
            .quotation_line_repo
            .find_deleted(quotation.id, tenant_id)
            .await?;
        for line in deleted_lines {
            let _ = self
                .quotation_line_repo
                .restore(line.id, quotation.id, tenant_id)
                .await
                .ok();
        }
        Ok(quotation)
    }

    /// Restore a soft-deleted quotation and return as response
    #[tracing::instrument(skip(self))]
    pub async fn restore_quotation_response(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<QuotationResponse, ApiError> {
        let quotation = self.restore_quotation(id, tenant_id).await?;
        let lines = self
            .quotation_line_repo
            .find_by_quotation(quotation.id, tenant_id)
            .await?;
        Ok(QuotationResponse::from((quotation, lines)))
    }

    /// List soft-deleted quotations
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_quotations(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Quotation>, ApiError> {
        self.quotation_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a quotation (admin only, after soft delete)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_quotation(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        // The tenant-scoped parent destroy gates NotFound for a foreign
        // quotation. The parent FK has ON DELETE CASCADE (003), which removes the
        // quotation's lines atomically with the parent -- the redundant unscoped
        // delete_by_quotation(id) call is dropped (mirror of the #194
        // invoice_lines fix and 054/057): it ran on the raw caller id *before* the
        // tenant-scoped parent destroy, hard-deleting a foreign tenant's lines.
        self.quotation_repo.destroy(id, tenant_id).await
    }

    // Sales Order Line operations

    /// Soft delete a sales order line (admin only, tenant-scoped)
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_order_line(
        &self,
        line_id: i64,
        order_id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        // Parent-ownership precheck: a tenant-scoped find_by_id gates NotFound
        // for a foreign order before the line is touched. This plus the repo's
        // `AND order_id = $N AND tenant_id = $N` filter closes the per-line IDOR
        // (a tenant-A admin could formerly soft-delete tenant-B's line by
        // guessing line_id, since the Postgres impl ignored order_id entirely).
        self.order_repo
            .find_by_id(order_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Sales order {} not found", order_id)))?;
        self.order_line_repo
            .soft_delete(line_id, order_id, deleted_by, tenant_id)
            .await
    }

    /// Restore a soft-deleted sales order line (admin only, tenant-scoped)
    #[tracing::instrument(skip(self))]
    pub async fn restore_order_line(
        &self,
        line_id: i64,
        order_id: i64,
        tenant_id: i64,
    ) -> Result<SalesOrderLine, ApiError> {
        self.order_repo
            .find_by_id(order_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Sales order {} not found", order_id)))?;
        self.order_line_repo
            .restore(line_id, order_id, tenant_id)
            .await
    }

    /// List soft-deleted sales order lines (admin only, tenant-scoped)
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_order_lines(
        &self,
        order_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SalesOrderLine>, ApiError> {
        self.order_repo
            .find_by_id(order_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Sales order {} not found", order_id)))?;
        self.order_line_repo.find_deleted(order_id, tenant_id).await
    }

    /// Permanently delete a sales order line (admin only, tenant-scoped)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_order_line(
        &self,
        line_id: i64,
        order_id: i64,
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        self.order_repo
            .find_by_id(order_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Sales order {} not found", order_id)))?;
        self.order_line_repo
            .destroy(line_id, order_id, tenant_id)
            .await
    }

    // Quotation Line operations

    /// Soft delete a quotation line (admin only, tenant-scoped)
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_quotation_line(
        &self,
        line_id: i64,
        quotation_id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        // Parent-ownership precheck (mirror of the order-line path): a
        // tenant-scoped find_by_id gates NotFound for a foreign quotation before
        // the line is touched, closing the per-line IDOR.
        self.quotation_repo
            .find_by_id(quotation_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Quotation {} not found", quotation_id)))?;
        self.quotation_line_repo
            .soft_delete(line_id, quotation_id, deleted_by, tenant_id)
            .await
    }

    /// Restore a soft-deleted quotation line (admin only, tenant-scoped)
    #[tracing::instrument(skip(self))]
    pub async fn restore_quotation_line(
        &self,
        line_id: i64,
        quotation_id: i64,
        tenant_id: i64,
    ) -> Result<QuotationLine, ApiError> {
        self.quotation_repo
            .find_by_id(quotation_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Quotation {} not found", quotation_id)))?;
        self.quotation_line_repo
            .restore(line_id, quotation_id, tenant_id)
            .await
    }

    /// List soft-deleted quotation lines (admin only, tenant-scoped)
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_quotation_lines(
        &self,
        quotation_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<QuotationLine>, ApiError> {
        self.quotation_repo
            .find_by_id(quotation_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Quotation {} not found", quotation_id)))?;
        self.quotation_line_repo
            .find_deleted(quotation_id, tenant_id)
            .await
    }

    /// Permanently delete a quotation line (admin only, tenant-scoped)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_quotation_line(
        &self,
        line_id: i64,
        quotation_id: i64,
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        self.quotation_repo
            .find_by_id(quotation_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Quotation {} not found", quotation_id)))?;
        self.quotation_line_repo
            .destroy(line_id, quotation_id, tenant_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cari::model::CreateCari;
    use crate::domain::cari::repository::InMemoryCariRepository;
    use crate::domain::product::model::CreateProduct;
    use crate::domain::product::repository::InMemoryProductRepository;
    use crate::domain::sales::model::CreateQuotationLine;
    use crate::domain::sales::repository::{
        InMemoryQuotationLineRepository, InMemoryQuotationRepository,
        InMemorySalesOrderLineRepository, InMemorySalesOrderRepository,
    };
    use chrono::Duration;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    async fn create_service() -> SalesService {
        let order_repo = Arc::new(InMemorySalesOrderRepository::new()) as BoxSalesOrderRepository;
        let order_line_repo =
            Arc::new(InMemorySalesOrderLineRepository::new()) as BoxSalesOrderLineRepository;
        let quotation_repo = Arc::new(InMemoryQuotationRepository::new()) as BoxQuotationRepository;
        let quotation_line_repo =
            Arc::new(InMemoryQuotationLineRepository::new()) as BoxQuotationLineRepository;

        // Seed the parent entities the create-* prechecks validate against.
        // InMemory repo create() auto-assigns ids starting at 1, matching the
        // cari_id=1 / product_id=Some(1) / tenant_id=1 used by the happy-path
        // tests below. A second cari/product is seeded on tenant 2 (ids 2) so
        // the cross-tenant IDOR rejection tests can reference a foreign parent.
        let cari_repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        cari_repo
            .create(CreateCari {
                code: "C1".to_string(),
                name: "Test Cari T1".to_string(),
                tenant_id: 1,
                created_by: 1,
                ..Default::default()
            })
            .await
            .expect("seed cari t1");
        cari_repo
            .create(CreateCari {
                code: "C2".to_string(),
                name: "Test Cari T2".to_string(),
                tenant_id: 2,
                created_by: 1,
                ..Default::default()
            })
            .await
            .expect("seed cari t2");

        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        product_repo
            .create(CreateProduct {
                tenant_id: 1,
                code: "P1".to_string(),
                name: "Test Product T1".to_string(),
                purchase_price: Decimal::ZERO,
                sale_price: Decimal::ZERO,
                tax_rate: Decimal::ZERO,
                ..Default::default()
            })
            .await
            .expect("seed product t1");
        product_repo
            .create(CreateProduct {
                tenant_id: 2,
                code: "P2".to_string(),
                name: "Test Product T2".to_string(),
                purchase_price: Decimal::ZERO,
                sale_price: Decimal::ZERO,
                tax_rate: Decimal::ZERO,
                ..Default::default()
            })
            .await
            .expect("seed product t2");

        SalesService::new(
            order_repo,
            order_line_repo,
            quotation_repo,
            quotation_line_repo,
            cari_repo,
            product_repo,
        )
    }

    #[tokio::test]
    async fn test_create_sales_order() {
        let service = create_service().await;

        let create = CreateSalesOrder {
            tenant_id: 1,
            company_id: 1,
            cari_id: 1,
            order_date: chrono::Utc::now(),
            delivery_date: Some(chrono::Utc::now() + Duration::days(7)),
            notes: None,
            shipping_address: Some("Shipping address".to_string()),
            billing_address: Some("Billing address".to_string()),
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            lines: vec![CreateSalesOrderLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: dec!(2),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };

        let result = service.create_sales_order(create).await;
        assert!(result.is_ok());
        let order = result.unwrap();
        assert_eq!(order.lines.len(), 1);
    }

    #[tokio::test]
    async fn test_create_quotation() {
        let service = create_service().await;

        let create = CreateQuotation {
            tenant_id: 1,
            company_id: 1,
            cari_id: 1,
            valid_until: chrono::Utc::now() + Duration::days(30),
            notes: None,
            terms: Some("Net 30".to_string()),
            lines: vec![CreateQuotationLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(10),
            }],
        };

        let result = service.create_quotation(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_convert_quotation_to_order() {
        let service = create_service().await;

        // Create quotation
        let quote_create = CreateQuotation {
            tenant_id: 1,
            company_id: 1,
            cari_id: 1,
            valid_until: chrono::Utc::now() + Duration::days(30),
            notes: None,
            terms: None,
            lines: vec![CreateQuotationLine {
                product_id: Some(1),
                description: "Test".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };

        let quotation = service.create_quotation(quote_create).await.unwrap();

        // Convert to order
        let order_result = service.convert_quotation_to_order(quotation.id, 1).await;
        assert!(order_result.is_ok());

        // Check quotation status updated
        let updated_quote = service.get_quotation(quotation.id, 1).await.unwrap();
        assert_eq!(updated_quote.status, QuotationStatus::ConvertedToOrder);
        assert!(updated_quote.sales_order_id.is_some());
    }

    // Cross-tenant IDOR rejection: a tenant-1 caller must NOT be able to
    // create a sales order referencing a tenant-2 customer (cari_id=2 exists
    // but belongs to tenant 2). The parent-ownership precheck must reject it
    // with NotFound before any INSERT.
    #[tokio::test]
    async fn test_create_sales_order_rejects_foreign_cari() {
        let service = create_service().await;
        let create = CreateSalesOrder {
            tenant_id: 1,
            company_id: 1,
            cari_id: 2, // exists, but belongs to tenant 2
            order_date: chrono::Utc::now(),
            delivery_date: None,
            notes: None,
            shipping_address: None,
            billing_address: None,
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            lines: vec![CreateSalesOrderLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };
        let result = service.create_sales_order(create).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    // Cross-tenant IDOR rejection: a tenant-1 caller must NOT be able to
    // create a sales order whose line references a tenant-2 product
    // (product_id=2 exists but belongs to tenant 2).
    #[tokio::test]
    async fn test_create_sales_order_rejects_foreign_product() {
        let service = create_service().await;
        let create = CreateSalesOrder {
            tenant_id: 1,
            company_id: 1,
            cari_id: 1, // own-tenant cari
            order_date: chrono::Utc::now(),
            delivery_date: None,
            notes: None,
            shipping_address: None,
            billing_address: None,
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            lines: vec![CreateSalesOrderLine {
                product_id: Some(2), // exists, but belongs to tenant 2
                description: "Foreign Product".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };
        let result = service.create_sales_order(create).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    // A line with no product (product_id: None) is a legitimate no-product
    // line and must NOT be rejected by the product precheck.
    #[tokio::test]
    async fn test_create_sales_order_allows_no_product_line() {
        let service = create_service().await;
        let create = CreateSalesOrder {
            tenant_id: 1,
            company_id: 1,
            cari_id: 1,
            order_date: chrono::Utc::now(),
            delivery_date: None,
            notes: None,
            shipping_address: None,
            billing_address: None,
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            lines: vec![CreateSalesOrderLine {
                product_id: None, // legitimate no-product line
                description: "Service line".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };
        let result = service.create_sales_order(create).await;
        assert!(result.is_ok());
    }
}
