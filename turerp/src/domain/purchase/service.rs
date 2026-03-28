//! Purchase service for business logic

#[allow(unused_imports)]
use std::sync::Arc;

#[allow(unused_imports)]
use crate::domain::purchase::model::{
    CreateGoodsReceipt, CreateGoodsReceiptLine, CreatePurchaseOrder, GoodsReceiptResponse,
    GoodsReceiptStatus, PurchaseOrderResponse, PurchaseOrderStatus,
};
use crate::domain::purchase::repository::{
    BoxGoodsReceiptLineRepository, BoxGoodsReceiptRepository, BoxPurchaseOrderLineRepository,
    BoxPurchaseOrderRepository,
};
use crate::error::ApiError;

/// Purchase service
#[derive(Clone)]
pub struct PurchaseService {
    order_repo: BoxPurchaseOrderRepository,
    order_line_repo: BoxPurchaseOrderLineRepository,
    receipt_repo: BoxGoodsReceiptRepository,
    receipt_line_repo: BoxGoodsReceiptLineRepository,
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

    pub async fn get_purchase_order(&self, id: i64) -> Result<PurchaseOrderResponse, ApiError> {
        let order = self
            .order_repo
            .find_by_id(id)
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

    pub async fn delete_order(&self, id: i64) -> Result<(), ApiError> {
        self.order_line_repo.delete_by_order(id).await?;
        self.order_repo.delete(id).await
    }

    // Goods Receipt operations
    pub async fn create_goods_receipt(
        &self,
        create: CreateGoodsReceipt,
    ) -> Result<GoodsReceiptResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        for line in &create.lines {
            line.validate()
                .map_err(|e| ApiError::Validation(e.join(", ")))?;
        }

        // Verify purchase order exists
        let _ = self
            .order_repo
            .find_by_id(create.purchase_order_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!(
                    "Purchase order {} not found",
                    create.purchase_order_id
                ))
            })?;

        let receipt = self.receipt_repo.create(create.clone()).await?;
        let lines = self
            .receipt_line_repo
            .create_many(receipt.id, create.lines)
            .await?;

        // Update order status based on receipt
        let all_receipts = self
            .receipt_repo
            .find_by_order(create.purchase_order_id)
            .await?;
        let order_lines = self
            .order_line_repo
            .find_by_order(create.purchase_order_id)
            .await?;

        let total_received: f64 = all_receipts
            .iter()
            .map(|_| {
                // Simplified calculation
                0.0
            })
            .sum();

        let order_total: f64 = order_lines.iter().map(|l| l.quantity).sum();

        let new_status = if total_received >= order_total {
            PurchaseOrderStatus::Received
        } else if total_received > 0.0 {
            PurchaseOrderStatus::PartialReceived
        } else {
            PurchaseOrderStatus::Approved
        };

        self.order_repo
            .update_status(create.purchase_order_id, new_status)
            .await?;

        Ok(GoodsReceiptResponse::from((receipt, lines)))
    }

    pub async fn get_goods_receipt(&self, id: i64) -> Result<GoodsReceiptResponse, ApiError> {
        let receipt = self
            .receipt_repo
            .find_by_id(id)
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

    pub async fn delete_receipt(&self, id: i64) -> Result<(), ApiError> {
        self.receipt_line_repo.delete_by_receipt(id).await?;
        self.receipt_repo.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::purchase::model::CreatePurchaseOrderLine;
    use crate::domain::purchase::repository::{
        InMemoryGoodsReceiptLineRepository, InMemoryGoodsReceiptRepository,
        InMemoryPurchaseOrderLineRepository, InMemoryPurchaseOrderRepository,
    };
    use chrono::Duration;
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

    #[tokio::test]
    async fn test_create_purchase_order() {
        let service = create_service();

        let create = CreatePurchaseOrder {
            tenant_id: 1,
            cari_id: 1,
            order_date: chrono::Utc::now(),
            expected_delivery_date: Some(chrono::Utc::now() + Duration::days(7)),
            notes: None,
            lines: vec![CreatePurchaseOrderLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: 10.0,
                unit_price: 50.0,
                tax_rate: 18.0,
                discount_rate: 5.0,
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
            expected_delivery_date: Some(chrono::Utc::now() + Duration::days(7)),
            notes: None,
            lines: vec![CreatePurchaseOrderLine {
                product_id: Some(1),
                description: "Test".to_string(),
                quantity: 10.0,
                unit_price: 50.0,
                tax_rate: 18.0,
                discount_rate: 0.0,
            }],
        };

        let order = service.create_purchase_order(order_create).await.unwrap();

        // Create goods receipt
        let receipt_create = CreateGoodsReceipt {
            tenant_id: 1,
            purchase_order_id: order.id,
            receipt_date: chrono::Utc::now(),
            notes: None,
            lines: vec![CreateGoodsReceiptLine {
                order_line_id: order.lines[0].id,
                product_id: Some(1),
                quantity: 10.0,
                condition: "Good".to_string(),
                notes: None,
            }],
        };

        let result = service.create_goods_receipt(receipt_create).await;
        assert!(result.is_ok());
    }
}
