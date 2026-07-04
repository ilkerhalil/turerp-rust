//! Purchase service for business logic

use crate::common::pagination::PaginatedResult;
use crate::domain::cari::repository::BoxCariRepository;
use crate::domain::product::repository::BoxProductRepository;
use crate::domain::purchase::model::{
    CreateGoodsReceipt, CreatePurchaseOrder, CreatePurchaseRequest, GoodsReceiptResponse,
    GoodsReceiptStatus, PurchaseOrderResponse, PurchaseOrderStatus, PurchaseRequestResponse,
    PurchaseRequestStatus, UpdatePurchaseRequest,
};
use crate::domain::purchase::repository::{
    BoxGoodsReceiptLineRepository, BoxGoodsReceiptRepository, BoxPurchaseOrderLineRepository,
    BoxPurchaseOrderRepository, BoxPurchaseRequestLineRepository, BoxPurchaseRequestRepository,
};
use crate::error::ApiError;
use rust_decimal::Decimal;

/// Purchase service
#[derive(Clone)]
pub struct PurchaseService {
    order_repo: BoxPurchaseOrderRepository,
    order_line_repo: BoxPurchaseOrderLineRepository,
    receipt_repo: BoxGoodsReceiptRepository,
    receipt_line_repo: BoxGoodsReceiptLineRepository,
    request_repo: Option<BoxPurchaseRequestRepository>,
    request_line_repo: Option<BoxPurchaseRequestLineRepository>,
    cari_repo: BoxCariRepository,
    product_repo: BoxProductRepository,
}

impl PurchaseService {
    pub fn new(
        order_repo: BoxPurchaseOrderRepository,
        order_line_repo: BoxPurchaseOrderLineRepository,
        receipt_repo: BoxGoodsReceiptRepository,
        receipt_line_repo: BoxGoodsReceiptLineRepository,
        cari_repo: BoxCariRepository,
        product_repo: BoxProductRepository,
    ) -> Self {
        Self {
            order_repo,
            order_line_repo,
            receipt_repo,
            receipt_line_repo,
            request_repo: None,
            request_line_repo: None,
            cari_repo,
            product_repo,
        }
    }

    /// Create service with purchase request support
    #[allow(clippy::too_many_arguments)]
    pub fn with_requests(
        order_repo: BoxPurchaseOrderRepository,
        order_line_repo: BoxPurchaseOrderLineRepository,
        receipt_repo: BoxGoodsReceiptRepository,
        receipt_line_repo: BoxGoodsReceiptLineRepository,
        request_repo: BoxPurchaseRequestRepository,
        request_line_repo: BoxPurchaseRequestLineRepository,
        cari_repo: BoxCariRepository,
        product_repo: BoxProductRepository,
    ) -> Self {
        Self {
            order_repo,
            order_line_repo,
            receipt_repo,
            receipt_line_repo,
            request_repo: Some(request_repo),
            request_line_repo: Some(request_line_repo),
            cari_repo,
            product_repo,
        }
    }

    /// Parent-ownership precheck: verify the vendor (cari) belongs to the
    /// caller's tenant before any INSERT. A body-controlled `cari_id`
    /// referencing a foreign tenant's vendor would otherwise create an
    /// orphaned purchase order (cross-tenant IDOR).
    async fn ensure_cari_owned(&self, cari_id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.cari_repo
            .find_by_id(cari_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Cari {} not found", cari_id)))?;
        Ok(())
    }

    /// Parent-ownership precheck for line-level products: verify every line
    /// `product_id` (Some only — None is a legitimate no-product line) belongs
    /// to the caller's tenant. Single batched `find_by_ids` lookup; a foreign
    /// product is not returned by the tenant-scoped query, so a count mismatch
    /// means at least one line references a foreign product.
    async fn ensure_line_products_owned(
        &self,
        product_ids: impl IntoIterator<Item = Option<i64>>,
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        let ids: Vec<i64> = {
            let set: std::collections::HashSet<i64> = product_ids.into_iter().flatten().collect();
            let mut v: Vec<i64> = set.into_iter().collect();
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

    // Purchase Order operations
    #[tracing::instrument(skip(self))]
    pub async fn create_purchase_order(
        &self,
        create: CreatePurchaseOrder,
    ) -> Result<PurchaseOrderResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        for line in &create.lines {
            line.validate()
                .map_err(|e| ApiError::Validation(e.join(", ")))?;
        }

        // Parent-ownership prechecks: reject body-controlled FKs that reference
        // a foreign tenant's vendor or product before any INSERT.
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

        Ok(PurchaseOrderResponse::from((order, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_purchase_order(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<PurchaseOrderResponse, ApiError> {
        let order = self
            .order_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Purchase order {} not found", id)))?;

        let lines = self.order_line_repo.find_by_order(id, tenant_id).await?;

        Ok(PurchaseOrderResponse::from((order, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_orders_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseOrder>, ApiError> {
        self.order_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_orders_by_cari(
        &self,
        cari_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseOrder>, ApiError> {
        self.order_repo.find_by_cari(cari_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_orders_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseOrderStatus,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseOrder>, ApiError> {
        self.order_repo.find_by_status(tenant_id, status).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_order_status(
        &self,
        id: i64,
        status: PurchaseOrderStatus,
        tenant_id: i64,
    ) -> Result<crate::domain::purchase::model::PurchaseOrder, ApiError> {
        self.order_repo.update_status(id, status, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_order(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.order_line_repo.delete_by_order(id, tenant_id).await?;
        self.order_repo.delete(id, tenant_id).await
    }

    /// Soft delete a purchase order (sets deleted_at)
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_order(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        // Parent-first: the tenant-scoped parent soft_delete gates the operation
        // (returns NotFound for a foreign-tenant order) before any line rows are
        // touched, so a caller cannot soft-delete another tenant's lines by
        // guessing the order id. The line soft-delete is additionally scoped by
        // tenant_id as a SQL-level backstop.
        self.order_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await?;
        self.order_line_repo
            .soft_delete_by_order(id, deleted_by, tenant_id)
            .await
    }

    /// Restore a soft-deleted purchase order
    #[tracing::instrument(skip(self))]
    pub async fn restore_order(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<crate::domain::purchase::model::PurchaseOrder, ApiError> {
        let order = self.order_repo.restore(id, tenant_id).await?;
        self.order_line_repo.restore_by_order(id, tenant_id).await?;
        Ok(order)
    }

    /// Restore a soft-deleted purchase order and return as response
    #[tracing::instrument(skip(self))]
    pub async fn restore_order_response(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<PurchaseOrderResponse, ApiError> {
        let order = self.restore_order(id, tenant_id).await?;
        let lines = self
            .order_line_repo
            .find_by_order(order.id, tenant_id)
            .await?;
        Ok(PurchaseOrderResponse::from((order, lines)))
    }

    /// List soft-deleted purchase orders
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_orders(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseOrder>, ApiError> {
        self.order_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a purchase order (admin only, after soft delete)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_order(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        // Rely on the tenant-scoped parent destroy + the ON DELETE CASCADE on
        // purchase_order_lines.order_id to remove the lines. The previous
        // unscoped delete_by_order(id) ran BEFORE the tenant-scoped parent
        // destroy and hard-deleted another tenant's lines by guessing the order
        // id (the #194 destroy-ordering leak); it is dropped here.
        self.order_repo.destroy(id, tenant_id).await
    }

    // Goods Receipt operations
    #[tracing::instrument(skip(self))]
    pub async fn create_goods_receipt(
        &self,
        mut create: CreateGoodsReceipt,
        tenant_id: i64,
    ) -> Result<GoodsReceiptResponse, ApiError> {
        // Force the auth-derived tenant onto the request body so the receipt row
        // and the tenant-scoped internal calls below cannot be misattributed to
        // another tenant via a client-supplied `tenant_id` field.
        create.tenant_id = tenant_id;

        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        for line in &create.lines {
            line.validate()
                .map_err(|e| ApiError::Validation(e.join(", ")))?;
        }

        // Verify purchase order exists
        let order = self
            .order_repo
            .find_by_id(create.purchase_order_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!(
                    "Purchase order {} not found",
                    create.purchase_order_id
                ))
            })?;

        // Verify order is in a valid state for receiving
        if order.status != PurchaseOrderStatus::Approved
            && order.status != PurchaseOrderStatus::SentToVendor
            && order.status != PurchaseOrderStatus::PartialReceived
        {
            return Err(ApiError::Validation(format!(
                "Cannot receive goods for order in {} status",
                order.status
            )));
        }

        // Parent-ownership prechecks on the receipt lines. Load the PO's own
        // lines once (tenant-scoped) and validate every receipt line's
        // `order_line_id` actually belongs to THIS PO (HashSet membership —
        // stronger than a per-line tenant check, which would accept a line
        // from a different PO of the same tenant). Also validate each line's
        // `product_id` (Some only) belongs to the caller's tenant.
        let order_lines = self
            .order_line_repo
            .find_by_order(create.purchase_order_id, tenant_id)
            .await?;
        let valid_line_ids: std::collections::HashSet<i64> =
            order_lines.iter().map(|l| l.id).collect();
        for line in &create.lines {
            if !valid_line_ids.contains(&line.order_line_id) {
                return Err(ApiError::NotFound(format!(
                    "Purchase order line {} not found on purchase order {}",
                    line.order_line_id, create.purchase_order_id
                )));
            }
        }
        self.ensure_line_products_owned(create.lines.iter().map(|l| l.product_id), tenant_id)
            .await?;

        let receipt = self.receipt_repo.create(create.clone()).await?;
        let lines = self
            .receipt_line_repo
            .create_many(receipt.id, create.lines, tenant_id)
            .await?;

        // Update order status based on receipt quantities
        let all_receipts = self
            .receipt_repo
            .find_by_order(create.purchase_order_id, tenant_id)
            .await?;

        // Calculate total received quantities from all receipts
        let mut total_received = Decimal::ZERO;
        for r in &all_receipts {
            let receipt_lines = self
                .receipt_line_repo
                .find_by_receipt(r.id, tenant_id)
                .await?;
            for l in receipt_lines {
                total_received += l.quantity;
            }
        }

        let order_total: Decimal = order_lines.iter().map(|l| l.quantity).sum();

        let new_status = if total_received >= order_total && order_total > Decimal::ZERO {
            PurchaseOrderStatus::Received
        } else if total_received > Decimal::ZERO {
            PurchaseOrderStatus::PartialReceived
        } else {
            order.status
        };

        self.order_repo
            .update_status(create.purchase_order_id, new_status, tenant_id)
            .await?;

        Ok(GoodsReceiptResponse::from((receipt, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_goods_receipt(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<GoodsReceiptResponse, ApiError> {
        let receipt = self
            .receipt_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Goods receipt {} not found", id)))?;

        let lines = self
            .receipt_line_repo
            .find_by_receipt(id, tenant_id)
            .await?;

        Ok(GoodsReceiptResponse::from((receipt, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_receipts_by_order(
        &self,
        order_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::GoodsReceipt>, ApiError> {
        self.receipt_repo.find_by_order(order_id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_receipt_status(
        &self,
        id: i64,
        status: GoodsReceiptStatus,
        tenant_id: i64,
    ) -> Result<crate::domain::purchase::model::GoodsReceipt, ApiError> {
        self.receipt_repo.update_status(id, status, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_receipt(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.receipt_line_repo
            .delete_by_receipt(id, tenant_id)
            .await?;
        self.receipt_repo.delete(id, tenant_id).await
    }

    /// Soft delete a goods receipt (sets deleted_at)
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_receipt(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        // Parent-first: the tenant-scoped parent soft_delete gates the operation
        // (returns NotFound for a foreign-tenant receipt) before any line rows
        // are touched. The line soft-delete is additionally scoped by tenant_id
        // as a SQL-level backstop.
        self.receipt_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await?;
        self.receipt_line_repo
            .soft_delete_by_receipt(id, deleted_by, tenant_id)
            .await
    }

    /// Restore a soft-deleted goods receipt
    #[tracing::instrument(skip(self))]
    pub async fn restore_receipt(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<crate::domain::purchase::model::GoodsReceipt, ApiError> {
        let receipt = self.receipt_repo.restore(id, tenant_id).await?;
        self.receipt_line_repo
            .restore_by_receipt(id, tenant_id)
            .await?;
        Ok(receipt)
    }

    /// Restore a soft-deleted goods receipt and return as response
    #[tracing::instrument(skip(self))]
    pub async fn restore_receipt_response(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<GoodsReceiptResponse, ApiError> {
        let receipt = self.restore_receipt(id, tenant_id).await?;
        let lines = self
            .receipt_line_repo
            .find_by_receipt(receipt.id, tenant_id)
            .await?;
        Ok(GoodsReceiptResponse::from((receipt, lines)))
    }

    /// List soft-deleted goods receipts
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_receipts(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::GoodsReceipt>, ApiError> {
        self.receipt_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a goods receipt (admin only, after soft delete)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_receipt(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        // Rely on the tenant-scoped parent destroy + the ON DELETE CASCADE on
        // goods_receipt_lines.receipt_id to remove the lines. The previous
        // unscoped delete_by_receipt(id) ran BEFORE the tenant-scoped parent
        // destroy and hard-deleted another tenant's lines by guessing the
        // receipt id (the #194 destroy-ordering leak); it is dropped here.
        self.receipt_repo.destroy(id, tenant_id).await
    }

    // Purchase Request operations
    #[tracing::instrument(skip(self))]
    pub async fn create_purchase_request(
        &self,
        create: CreatePurchaseRequest,
    ) -> Result<PurchaseRequestResponse, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        let request_line_repo = self.request_line_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request line repository not configured".to_string())
        })?;

        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        for line in &create.lines {
            line.validate()
                .map_err(|e| ApiError::Validation(e.join(", ")))?;
        }

        // Parent-ownership precheck: reject body-controlled line `product_id`s
        // that reference a foreign tenant's product before any INSERT.
        // `requested_by` is a user-id FK, out of scope (separate class).
        self.ensure_line_products_owned(
            create.lines.iter().map(|l| l.product_id),
            create.tenant_id,
        )
        .await?;

        let request = request_repo.create(create.clone()).await?;
        let lines = request_line_repo
            .create_many(request.id, create.lines, create.tenant_id)
            .await?;

        Ok(PurchaseRequestResponse::from((request, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_purchase_request(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<PurchaseRequestResponse, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        let request_line_repo = self.request_line_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request line repository not configured".to_string())
        })?;

        let request = request_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Purchase request {} not found", id)))?;

        let lines = request_line_repo.find_by_request(id, tenant_id).await?;

        Ok(PurchaseRequestResponse::from((request, lines)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_requests_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseRequest>, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo.find_by_tenant(tenant_id).await
    }

    /// Get purchase requests by tenant with pagination
    #[tracing::instrument(skip(self))]
    pub async fn get_requests_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<crate::domain::purchase::model::PurchaseRequest>, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_requests_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseRequest>, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo.find_by_status(tenant_id, status).await
    }

    /// Get purchase requests by status with pagination
    #[tracing::instrument(skip(self))]
    pub async fn get_requests_by_status_paginated(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<crate::domain::purchase::model::PurchaseRequest>, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo
            .find_by_status_paginated(tenant_id, status, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_requests_by_requester(
        &self,
        requested_by: i64,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseRequest>, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo.find_by_requester(requested_by).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_purchase_request(
        &self,
        id: i64,
        update: UpdatePurchaseRequest,
        tenant_id: i64,
    ) -> Result<crate::domain::purchase::model::PurchaseRequest, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo.update(id, update, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_request_status(
        &self,
        id: i64,
        status: PurchaseRequestStatus,
        tenant_id: i64,
    ) -> Result<crate::domain::purchase::model::PurchaseRequest, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        // Get current request to validate status transition
        let current_request = request_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Purchase request {} not found", id)))?;

        // Validate status transition
        if !current_request.status.can_transition_to(&status) {
            return Err(ApiError::Validation(format!(
                "Cannot transition from {} to {}. Valid transitions: {:?}",
                current_request.status,
                status,
                current_request.status.valid_next_statuses()
            )));
        }

        request_repo.update_status(id, status, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_purchase_request(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        let request_line_repo = self.request_line_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request line repository not configured".to_string())
        })?;

        request_line_repo.delete_by_request(id, tenant_id).await?;
        request_repo.delete(id, tenant_id).await
    }

    /// Soft delete a purchase request (sets deleted_at)
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_request(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        let request_line_repo = self.request_line_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request line repository not configured".to_string())
        })?;

        // Parent-first: the tenant-scoped parent soft_delete gates the operation
        // (returns NotFound for a foreign-tenant request) before any line rows
        // are touched. The line soft-delete is additionally scoped by tenant_id
        // as a SQL-level backstop.
        request_repo.soft_delete(id, tenant_id, deleted_by).await?;
        request_line_repo
            .soft_delete_by_request(id, deleted_by, tenant_id)
            .await
    }

    /// Restore a soft-deleted purchase request
    #[tracing::instrument(skip(self))]
    pub async fn restore_request(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<crate::domain::purchase::model::PurchaseRequest, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        let request_line_repo = self.request_line_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request line repository not configured".to_string())
        })?;

        let request = request_repo.restore(id, tenant_id).await?;
        request_line_repo.restore_by_request(id, tenant_id).await?;
        Ok(request)
    }

    /// Restore a soft-deleted purchase request and return as response
    #[tracing::instrument(skip(self))]
    pub async fn restore_request_response(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<PurchaseRequestResponse, ApiError> {
        let request = self.restore_request(id, tenant_id).await?;

        let request_line_repo = self.request_line_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request line repository not configured".to_string())
        })?;

        let lines = request_line_repo
            .find_by_request(request.id, tenant_id)
            .await?;
        Ok(PurchaseRequestResponse::from((request, lines)))
    }

    /// List soft-deleted purchase requests
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_requests(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseRequest>, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a purchase request (admin only, after soft delete)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_request(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        // Rely on the tenant-scoped parent destroy + the ON DELETE CASCADE on
        // purchase_request_lines.request_id to remove the lines. The previous
        // unscoped delete_by_request(id) ran BEFORE the tenant-scoped parent
        // destroy and hard-deleted another tenant's lines by guessing the
        // request id (the #194 destroy-ordering leak); it is dropped here.
        request_repo.destroy(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cari::model::CreateCari;
    use crate::domain::cari::repository::InMemoryCariRepository;
    use crate::domain::product::model::CreateProduct;
    use crate::domain::product::repository::InMemoryProductRepository;
    use crate::domain::purchase::model::{
        CreateGoodsReceiptLine, CreatePurchaseOrderLine, CreatePurchaseRequestLine,
    };
    use crate::domain::purchase::repository::{
        InMemoryGoodsReceiptLineRepository, InMemoryGoodsReceiptRepository,
        InMemoryPurchaseOrderLineRepository, InMemoryPurchaseOrderRepository,
        InMemoryPurchaseRequestLineRepository, InMemoryPurchaseRequestRepository,
    };
    use chrono::Duration;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    // Seed cari + product on tenants 1 and 2 (InMemory create() auto-assigns
    // ids from 1: cari 1/2, product 1/2) so the happy-path tests (cari_id=1 /
    // product_id=Some(1) / tenant_id=1) pass the parent-ownership prechecks
    // and the cross-tenant IDOR rejection tests can reference a foreign parent.
    async fn seed_parents() -> (BoxCariRepository, BoxProductRepository) {
        let cari_repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        cari_repo
            .create(CreateCari {
                code: "C1".to_string(),
                name: "Vendor T1".to_string(),
                tenant_id: 1,
                created_by: 1,
                ..Default::default()
            })
            .await
            .expect("seed cari t1");
        cari_repo
            .create(CreateCari {
                code: "C2".to_string(),
                name: "Vendor T2".to_string(),
                tenant_id: 2,
                created_by: 1,
                ..Default::default()
            })
            .await
            .expect("seed cari t2");

        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        for tenant in [1, 2] {
            product_repo
                .create(CreateProduct {
                    tenant_id: tenant,
                    code: format!("P{}", tenant),
                    name: format!("Product T{}", tenant),
                    purchase_price: Decimal::ZERO,
                    sale_price: Decimal::ZERO,
                    tax_rate: Decimal::ZERO,
                    ..Default::default()
                })
                .await
                .expect("seed product");
        }
        (cari_repo, product_repo)
    }

    async fn create_service() -> PurchaseService {
        let order_repo =
            Arc::new(InMemoryPurchaseOrderRepository::new()) as BoxPurchaseOrderRepository;
        let order_line_repo =
            Arc::new(InMemoryPurchaseOrderLineRepository::new()) as BoxPurchaseOrderLineRepository;
        let receipt_repo =
            Arc::new(InMemoryGoodsReceiptRepository::new()) as BoxGoodsReceiptRepository;
        let receipt_line_repo =
            Arc::new(InMemoryGoodsReceiptLineRepository::new()) as BoxGoodsReceiptLineRepository;
        let (cari_repo, product_repo) = seed_parents().await;
        PurchaseService::new(
            order_repo,
            order_line_repo,
            receipt_repo,
            receipt_line_repo,
            cari_repo,
            product_repo,
        )
    }

    async fn create_service_with_requests() -> PurchaseService {
        let order_repo =
            Arc::new(InMemoryPurchaseOrderRepository::new()) as BoxPurchaseOrderRepository;
        let order_line_repo =
            Arc::new(InMemoryPurchaseOrderLineRepository::new()) as BoxPurchaseOrderLineRepository;
        let receipt_repo =
            Arc::new(InMemoryGoodsReceiptRepository::new()) as BoxGoodsReceiptRepository;
        let receipt_line_repo =
            Arc::new(InMemoryGoodsReceiptLineRepository::new()) as BoxGoodsReceiptLineRepository;
        let request_repo =
            Arc::new(InMemoryPurchaseRequestRepository::new()) as BoxPurchaseRequestRepository;
        let request_line_repo = Arc::new(InMemoryPurchaseRequestLineRepository::new())
            as BoxPurchaseRequestLineRepository;
        let (cari_repo, product_repo) = seed_parents().await;
        PurchaseService::with_requests(
            order_repo,
            order_line_repo,
            receipt_repo,
            receipt_line_repo,
            request_repo,
            request_line_repo,
            cari_repo,
            product_repo,
        )
    }

    #[tokio::test]
    async fn test_create_purchase_order() {
        let service = create_service().await;

        let create = CreatePurchaseOrder {
            tenant_id: 1,
            company_id: 1,
            cari_id: 1,
            order_date: chrono::Utc::now(),
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            expected_delivery_date: Some(chrono::Utc::now() + Duration::days(7)),
            notes: None,
            lines: vec![CreatePurchaseOrderLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: dec!(10),
                unit_price: dec!(50),
                tax_rate: dec!(18),
                discount_rate: dec!(5),
            }],
        };

        let result = service.create_purchase_order(create).await;
        assert!(result.is_ok());
        let order = result.unwrap();
        assert_eq!(order.lines.len(), 1);
    }

    #[tokio::test]
    async fn test_create_goods_receipt() {
        let service = create_service().await;

        // Create purchase order first
        let order_create = CreatePurchaseOrder {
            tenant_id: 1,
            company_id: 1,
            cari_id: 1,
            order_date: chrono::Utc::now(),
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            expected_delivery_date: Some(chrono::Utc::now() + Duration::days(7)),
            notes: None,
            lines: vec![CreatePurchaseOrderLine {
                product_id: Some(1),
                description: "Test".to_string(),
                quantity: dec!(10),
                unit_price: dec!(50),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };

        let order = service.create_purchase_order(order_create).await.unwrap();

        // Update order status to Approved (required before receiving goods)
        service
            .update_order_status(order.id, PurchaseOrderStatus::Approved, 1)
            .await
            .unwrap();

        // Create goods receipt
        let receipt_create = CreateGoodsReceipt {
            tenant_id: 1,
            company_id: 1,
            purchase_order_id: order.id,
            receipt_date: chrono::Utc::now(),
            notes: None,
            lines: vec![CreateGoodsReceiptLine {
                order_line_id: order.lines[0].id,
                product_id: Some(1),
                quantity: dec!(10),
                condition: "Good".to_string(),
                notes: None,
            }],
        };

        let result = service.create_goods_receipt(receipt_create, 1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_purchase_request() {
        let service = create_service_with_requests().await;

        let create = CreatePurchaseRequest {
            tenant_id: 1,
            company_id: 1,
            requested_by: 1,
            department: Some("IT".to_string()),
            priority: "High".to_string(),
            reason: Some("New equipment needed".to_string()),
            lines: vec![CreatePurchaseRequestLine {
                product_id: Some(1),
                description: "Laptop".to_string(),
                quantity: dec!(5),
                notes: Some("For new developers".to_string()),
            }],
        };

        let result = service.create_purchase_request(create).await;
        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.lines.len(), 1);
        assert_eq!(request.status, PurchaseRequestStatus::Draft);
    }

    #[tokio::test]
    async fn test_update_request_status() {
        let service = create_service_with_requests().await;

        // Create request
        let create = CreatePurchaseRequest {
            tenant_id: 1,
            company_id: 1,
            requested_by: 1,
            department: None,
            priority: "Medium".to_string(),
            reason: None,
            lines: vec![CreatePurchaseRequestLine {
                product_id: None,
                description: "Office supplies".to_string(),
                quantity: dec!(100),
                notes: None,
            }],
        };

        let request = service.create_purchase_request(create).await.unwrap();
        assert_eq!(request.status, PurchaseRequestStatus::Draft);

        // Update status
        let updated = service
            .update_request_status(request.id, PurchaseRequestStatus::PendingApproval, 1)
            .await;
        assert!(updated.is_ok());
        assert_eq!(
            updated.unwrap().status,
            PurchaseRequestStatus::PendingApproval
        );
    }

    #[tokio::test]
    async fn test_delete_purchase_request() {
        let service = create_service_with_requests().await;

        let create = CreatePurchaseRequest {
            tenant_id: 1,
            company_id: 1,
            requested_by: 1,
            department: None,
            priority: "Low".to_string(),
            reason: None,
            lines: vec![CreatePurchaseRequestLine {
                product_id: None,
                description: "Test item".to_string(),
                quantity: dec!(1),
                notes: None,
            }],
        };

        let request = service.create_purchase_request(create).await.unwrap();

        // Delete
        let result = service.delete_purchase_request(request.id, 1).await;
        assert!(result.is_ok());

        // Verify deletion
        let result = service.get_purchase_request(request.id, 1).await;
        assert!(result.is_err());
    }

    // Cross-tenant IDOR rejection: a tenant-1 caller must NOT be able to
    // create a purchase order referencing a tenant-2 vendor (cari_id=2 exists
    // but belongs to tenant 2).
    #[tokio::test]
    async fn test_create_purchase_order_rejects_foreign_cari() {
        let service = create_service().await;
        let create = CreatePurchaseOrder {
            tenant_id: 1,
            company_id: 1,
            cari_id: 2, // exists, but belongs to tenant 2
            order_date: chrono::Utc::now(),
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            expected_delivery_date: None,
            notes: None,
            lines: vec![CreatePurchaseOrderLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: dec!(1),
                unit_price: dec!(50),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };
        let result = service.create_purchase_order(create).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    // Cross-tenant IDOR rejection: a tenant-1 caller must NOT be able to
    // create a purchase order whose line references a tenant-2 product.
    #[tokio::test]
    async fn test_create_purchase_order_rejects_foreign_product() {
        let service = create_service().await;
        let create = CreatePurchaseOrder {
            tenant_id: 1,
            company_id: 1,
            cari_id: 1, // own-tenant vendor
            order_date: chrono::Utc::now(),
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            expected_delivery_date: None,
            notes: None,
            lines: vec![CreatePurchaseOrderLine {
                product_id: Some(2), // exists, but belongs to tenant 2
                description: "Foreign Product".to_string(),
                quantity: dec!(1),
                unit_price: dec!(50),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };
        let result = service.create_purchase_order(create).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    // Cross-tenant IDOR rejection: a tenant-1 caller must NOT be able to
    // create a purchase request whose line references a tenant-2 product.
    #[tokio::test]
    async fn test_create_purchase_request_rejects_foreign_product() {
        let service = create_service_with_requests().await;
        let create = CreatePurchaseRequest {
            tenant_id: 1,
            company_id: 1,
            requested_by: 1,
            department: None,
            priority: "High".to_string(),
            reason: None,
            lines: vec![CreatePurchaseRequestLine {
                product_id: Some(2), // exists, but belongs to tenant 2
                description: "Foreign Product".to_string(),
                quantity: dec!(1),
                notes: None,
            }],
        };
        let result = service.create_purchase_request(create).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    // Cross-tenant IDOR rejection: a goods-receipt line must NOT be able to
    // reference an order_line_id that does not belong to the receipt's own
    // purchase order (even if the line exists on a different PO of the same
    // tenant). The precheck validates membership in THIS PO's line set.
    #[tokio::test]
    async fn test_create_goods_receipt_rejects_foreign_order_line() {
        let service = create_service().await;

        // Create PO #1 (tenant 1) and approve it.
        let order_create = CreatePurchaseOrder {
            tenant_id: 1,
            company_id: 1,
            cari_id: 1,
            order_date: chrono::Utc::now(),
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            expected_delivery_date: None,
            notes: None,
            lines: vec![CreatePurchaseOrderLine {
                product_id: Some(1),
                description: "Line A".to_string(),
                quantity: dec!(10),
                unit_price: dec!(50),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };
        let order_a = service.create_purchase_order(order_create).await.unwrap();
        service
            .update_order_status(order_a.id, PurchaseOrderStatus::Approved, 1)
            .await
            .unwrap();

        // Create PO #2 (tenant 1) and approve it; its line has a different id.
        let order_b_create = CreatePurchaseOrder {
            tenant_id: 1,
            company_id: 1,
            cari_id: 1,
            order_date: chrono::Utc::now(),
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            expected_delivery_date: None,
            notes: None,
            lines: vec![CreatePurchaseOrderLine {
                product_id: Some(1),
                description: "Line B".to_string(),
                quantity: dec!(10),
                unit_price: dec!(50),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };
        let order_b = service.create_purchase_order(order_b_create).await.unwrap();
        service
            .update_order_status(order_b.id, PurchaseOrderStatus::Approved, 1)
            .await
            .unwrap();

        // Receipt against PO #1, but referencing PO #2's line id.
        let receipt_create = CreateGoodsReceipt {
            tenant_id: 1,
            company_id: 1,
            purchase_order_id: order_a.id,
            receipt_date: chrono::Utc::now(),
            notes: None,
            lines: vec![CreateGoodsReceiptLine {
                order_line_id: order_b.lines[0].id, // belongs to PO #2, not PO #1
                product_id: Some(1),
                quantity: dec!(10),
                condition: "Good".to_string(),
                notes: None,
            }],
        };
        let result = service.create_goods_receipt(receipt_create, 1).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }
}
