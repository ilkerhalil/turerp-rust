//! Purchase service for business logic

use crate::common::pagination::PaginatedResult;
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
}

impl PurchaseService {
    pub fn new(
        order_repo: BoxPurchaseOrderRepository,
        order_line_repo: BoxPurchaseOrderLineRepository,
        receipt_repo: BoxGoodsReceiptRepository,
        receipt_line_repo: BoxGoodsReceiptLineRepository,
    ) -> Self {
        Self {
            order_repo,
            order_line_repo,
            receipt_repo,
            receipt_line_repo,
            request_repo: None,
            request_line_repo: None,
        }
    }

    /// Create service with purchase request support
    pub fn with_requests(
        order_repo: BoxPurchaseOrderRepository,
        order_line_repo: BoxPurchaseOrderLineRepository,
        receipt_repo: BoxGoodsReceiptRepository,
        receipt_line_repo: BoxGoodsReceiptLineRepository,
        request_repo: BoxPurchaseRequestRepository,
        request_line_repo: BoxPurchaseRequestLineRepository,
    ) -> Self {
        Self {
            order_repo,
            order_line_repo,
            receipt_repo,
            receipt_line_repo,
            request_repo: Some(request_repo),
            request_line_repo: Some(request_line_repo),
        }
    }

    // Purchase Order operations
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

        let order = self.order_repo.create(create.clone()).await?;
        let lines = self
            .order_line_repo
            .create_many(order.id, create.lines)
            .await?;

        Ok(PurchaseOrderResponse::from((order, lines)))
    }

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

        let lines = self.order_line_repo.find_by_order(id).await?;

        Ok(PurchaseOrderResponse::from((order, lines)))
    }

    pub async fn get_orders_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseOrder>, ApiError> {
        self.order_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_orders_by_cari(
        &self,
        cari_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseOrder>, ApiError> {
        self.order_repo.find_by_cari(cari_id).await
    }

    pub async fn get_orders_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseOrderStatus,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseOrder>, ApiError> {
        self.order_repo.find_by_status(tenant_id, status).await
    }

    pub async fn update_order_status(
        &self,
        id: i64,
        status: PurchaseOrderStatus,
    ) -> Result<crate::domain::purchase::model::PurchaseOrder, ApiError> {
        self.order_repo.update_status(id, status).await
    }

    pub async fn delete_order(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.order_line_repo.delete_by_order(id).await?;
        self.order_repo.delete(id, tenant_id).await
    }

    /// Soft delete a purchase order (sets deleted_at)
    pub async fn soft_delete_order(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.order_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted purchase order
    pub async fn restore_order(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<crate::domain::purchase::model::PurchaseOrder, ApiError> {
        self.order_repo.restore(id, tenant_id).await
    }

    /// Restore a soft-deleted purchase order and return as response
    pub async fn restore_order_response(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<PurchaseOrderResponse, ApiError> {
        let order = self.order_repo.restore(id, tenant_id).await?;
        let lines = self.order_line_repo.find_by_order(order.id).await?;
        Ok(PurchaseOrderResponse::from((order, lines)))
    }

    /// List soft-deleted purchase orders
    pub async fn list_deleted_orders(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseOrder>, ApiError> {
        self.order_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a purchase order (admin only, after soft delete)
    pub async fn destroy_order(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.order_line_repo.delete_by_order(id).await?;
        self.order_repo.destroy(id, tenant_id).await
    }

    // Goods Receipt operations
    pub async fn create_goods_receipt(
        &self,
        create: CreateGoodsReceipt,
        tenant_id: i64,
    ) -> Result<GoodsReceiptResponse, ApiError> {
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

        let receipt = self.receipt_repo.create(create.clone()).await?;
        let lines = self
            .receipt_line_repo
            .create_many(receipt.id, create.lines)
            .await?;

        // Update order status based on receipt quantities
        let all_receipts = self
            .receipt_repo
            .find_by_order(create.purchase_order_id)
            .await?;
        let order_lines = self
            .order_line_repo
            .find_by_order(create.purchase_order_id)
            .await?;

        // Calculate total received quantities from all receipts
        let mut total_received = Decimal::ZERO;
        for r in &all_receipts {
            let receipt_lines = self.receipt_line_repo.find_by_receipt(r.id).await?;
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
            .update_status(create.purchase_order_id, new_status)
            .await?;

        Ok(GoodsReceiptResponse::from((receipt, lines)))
    }

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

        let lines = self.receipt_line_repo.find_by_receipt(id).await?;

        Ok(GoodsReceiptResponse::from((receipt, lines)))
    }

    pub async fn get_receipts_by_order(
        &self,
        order_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::GoodsReceipt>, ApiError> {
        self.receipt_repo.find_by_order(order_id).await
    }

    pub async fn update_receipt_status(
        &self,
        id: i64,
        status: GoodsReceiptStatus,
    ) -> Result<crate::domain::purchase::model::GoodsReceipt, ApiError> {
        self.receipt_repo.update_status(id, status).await
    }

    pub async fn delete_receipt(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.receipt_line_repo.delete_by_receipt(id).await?;
        self.receipt_repo.delete(id, tenant_id).await
    }

    /// Soft delete a goods receipt (sets deleted_at)
    pub async fn soft_delete_receipt(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.receipt_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    /// Restore a soft-deleted goods receipt
    pub async fn restore_receipt(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<crate::domain::purchase::model::GoodsReceipt, ApiError> {
        self.receipt_repo.restore(id, tenant_id).await
    }

    /// Restore a soft-deleted goods receipt and return as response
    pub async fn restore_receipt_response(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<GoodsReceiptResponse, ApiError> {
        let receipt = self.receipt_repo.restore(id, tenant_id).await?;
        let lines = self.receipt_line_repo.find_by_receipt(receipt.id).await?;
        Ok(GoodsReceiptResponse::from((receipt, lines)))
    }

    /// List soft-deleted goods receipts
    pub async fn list_deleted_receipts(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::purchase::model::GoodsReceipt>, ApiError> {
        self.receipt_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a goods receipt (admin only, after soft delete)
    pub async fn destroy_receipt(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.receipt_line_repo.delete_by_receipt(id).await?;
        self.receipt_repo.destroy(id, tenant_id).await
    }

    // Purchase Request operations
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

        let request = request_repo.create(create.clone()).await?;
        let lines = request_line_repo
            .create_many(request.id, create.lines)
            .await?;

        Ok(PurchaseRequestResponse::from((request, lines)))
    }

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

        let lines = request_line_repo.find_by_request(id).await?;

        Ok(PurchaseRequestResponse::from((request, lines)))
    }

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

    pub async fn get_requests_by_requester(
        &self,
        requested_by: i64,
    ) -> Result<Vec<crate::domain::purchase::model::PurchaseRequest>, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo.find_by_requester(requested_by).await
    }

    pub async fn update_purchase_request(
        &self,
        id: i64,
        update: UpdatePurchaseRequest,
    ) -> Result<crate::domain::purchase::model::PurchaseRequest, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo.update(id, update).await
    }

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

        request_repo.update_status(id, status).await
    }

    pub async fn delete_purchase_request(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        let request_line_repo = self.request_line_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request line repository not configured".to_string())
        })?;

        request_line_repo.delete_by_request(id).await?;
        request_repo.delete(id, tenant_id).await
    }

    /// Soft delete a purchase request (sets deleted_at)
    pub async fn soft_delete_request(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted purchase request
    pub async fn restore_request(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<crate::domain::purchase::model::PurchaseRequest, ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        request_repo.restore(id, tenant_id).await
    }

    /// Restore a soft-deleted purchase request and return as response
    pub async fn restore_request_response(
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

        let request = request_repo.restore(id, tenant_id).await?;
        let lines = request_line_repo.find_by_request(request.id).await?;
        Ok(PurchaseRequestResponse::from((request, lines)))
    }

    /// List soft-deleted purchase requests
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
    pub async fn destroy_request(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let request_repo = self.request_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request repository not configured".to_string())
        })?;

        let request_line_repo = self.request_line_repo.as_ref().ok_or_else(|| {
            ApiError::Internal("Purchase request line repository not configured".to_string())
        })?;

        request_line_repo.delete_by_request(id).await?;
        request_repo.destroy(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn create_service() -> PurchaseService {
        let order_repo =
            Arc::new(InMemoryPurchaseOrderRepository::new()) as BoxPurchaseOrderRepository;
        let order_line_repo =
            Arc::new(InMemoryPurchaseOrderLineRepository::new()) as BoxPurchaseOrderLineRepository;
        let receipt_repo =
            Arc::new(InMemoryGoodsReceiptRepository::new()) as BoxGoodsReceiptRepository;
        let receipt_line_repo =
            Arc::new(InMemoryGoodsReceiptLineRepository::new()) as BoxGoodsReceiptLineRepository;
        PurchaseService::new(order_repo, order_line_repo, receipt_repo, receipt_line_repo)
    }

    fn create_service_with_requests() -> PurchaseService {
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
        PurchaseService::with_requests(
            order_repo,
            order_line_repo,
            receipt_repo,
            receipt_line_repo,
            request_repo,
            request_line_repo,
        )
    }

    #[tokio::test]
    async fn test_create_purchase_order() {
        let service = create_service();

        let create = CreatePurchaseOrder {
            tenant_id: 1,
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
        let service = create_service();

        // Create purchase order first
        let order_create = CreatePurchaseOrder {
            tenant_id: 1,
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
            .update_order_status(order.id, PurchaseOrderStatus::Approved)
            .await
            .unwrap();

        // Create goods receipt
        let receipt_create = CreateGoodsReceipt {
            tenant_id: 1,
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
        let service = create_service_with_requests();

        let create = CreatePurchaseRequest {
            tenant_id: 1,
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
        let service = create_service_with_requests();

        // Create request
        let create = CreatePurchaseRequest {
            tenant_id: 1,
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
        let service = create_service_with_requests();

        let create = CreatePurchaseRequest {
            tenant_id: 1,
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
}
