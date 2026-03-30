//! Sales service for business logic
use crate::domain::sales::model::{
    CreateQuotation, CreateSalesOrder, CreateSalesOrderLine, QuotationResponse, QuotationStatus,
    SalesOrderResponse, SalesOrderStatus,
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
}

impl SalesService {
    pub fn new(
        order_repo: BoxSalesOrderRepository,
        order_line_repo: BoxSalesOrderLineRepository,
        quotation_repo: BoxQuotationRepository,
        quotation_line_repo: BoxQuotationLineRepository,
    ) -> Self {
        Self {
            order_repo,
            order_line_repo,
            quotation_repo,
            quotation_line_repo,
        }
    }

    // Sales Order operations
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

        let order = self.order_repo.create(create.clone()).await?;
        let lines = self
            .order_line_repo
            .create_many(order.id, create.lines)
            .await?;

        Ok(SalesOrderResponse::from((order, lines)))
    }

    pub async fn get_sales_order(&self, id: i64) -> Result<SalesOrderResponse, ApiError> {
        let order = self
            .order_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Sales order {} not found", id)))?;

        let lines = self.order_line_repo.find_by_order(id).await?;

        Ok(SalesOrderResponse::from((order, lines)))
    }

    pub async fn get_orders_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::sales::model::SalesOrder>, ApiError> {
        self.order_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_orders_by_cari(
        &self,
        cari_id: i64,
    ) -> Result<Vec<crate::domain::sales::model::SalesOrder>, ApiError> {
        self.order_repo.find_by_cari(cari_id).await
    }

    pub async fn get_orders_by_status(
        &self,
        tenant_id: i64,
        status: SalesOrderStatus,
    ) -> Result<Vec<crate::domain::sales::model::SalesOrder>, ApiError> {
        self.order_repo.find_by_status(tenant_id, status).await
    }

    pub async fn update_order_status(
        &self,
        id: i64,
        status: SalesOrderStatus,
    ) -> Result<crate::domain::sales::model::SalesOrder, ApiError> {
        self.order_repo.update_status(id, status).await
    }

    pub async fn delete_order(&self, id: i64) -> Result<(), ApiError> {
        self.order_line_repo.delete_by_order(id).await?;
        self.order_repo.delete(id).await
    }

    // Quotation operations
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

        let quotation = self.quotation_repo.create(create.clone()).await?;
        let lines = self
            .quotation_line_repo
            .create_many(quotation.id, create.lines)
            .await?;

        Ok(QuotationResponse::from((quotation, lines)))
    }

    pub async fn get_quotation(&self, id: i64) -> Result<QuotationResponse, ApiError> {
        let quotation = self
            .quotation_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Quotation {} not found", id)))?;

        let lines = self.quotation_line_repo.find_by_quotation(id).await?;

        Ok(QuotationResponse::from((quotation, lines)))
    }

    pub async fn get_quotations_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::sales::model::Quotation>, ApiError> {
        self.quotation_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_quotations_by_cari(
        &self,
        cari_id: i64,
    ) -> Result<Vec<crate::domain::sales::model::Quotation>, ApiError> {
        self.quotation_repo.find_by_cari(cari_id).await
    }

    pub async fn get_quotations_by_status(
        &self,
        tenant_id: i64,
        status: QuotationStatus,
    ) -> Result<Vec<crate::domain::sales::model::Quotation>, ApiError> {
        self.quotation_repo.find_by_status(tenant_id, status).await
    }

    pub async fn update_quotation_status(
        &self,
        id: i64,
        status: QuotationStatus,
    ) -> Result<crate::domain::sales::model::Quotation, ApiError> {
        self.quotation_repo.update_status(id, status).await
    }

    /// Convert quotation to sales order
    pub async fn convert_quotation_to_order(
        &self,
        quotation_id: i64,
    ) -> Result<SalesOrderResponse, ApiError> {
        let quotation = self
            .quotation_repo
            .find_by_id(quotation_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Quotation {} not found", quotation_id)))?;

        if quotation.status == QuotationStatus::ConvertedToOrder {
            return Err(ApiError::BadRequest(
                "Quotation is already converted to order".to_string(),
            ));
        }

        // Get quotation lines
        let quote_lines = self
            .quotation_line_repo
            .find_by_quotation(quotation_id)
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
            cari_id: quotation.cari_id,
            order_date: chrono::Utc::now(),
            delivery_date: None,
            notes: quotation.notes.clone(),
            shipping_address: None,
            billing_address: None,
            lines: order_lines,
        };

        let order = self.order_repo.create(create).await?;
        let lines = self
            .order_line_repo
            .create_many(order.id, order_lines_clone)
            .await?;

        // Update quotation status
        self.quotation_repo
            .link_to_order(quotation_id, order.id)
            .await?;

        Ok(SalesOrderResponse::from((order, lines)))
    }

    pub async fn delete_quotation(&self, id: i64) -> Result<(), ApiError> {
        self.quotation_line_repo.delete_by_quotation(id).await?;
        self.quotation_repo.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::sales::model::CreateQuotationLine;
    use crate::domain::sales::repository::{
        InMemoryQuotationLineRepository, InMemoryQuotationRepository,
        InMemorySalesOrderLineRepository, InMemorySalesOrderRepository,
    };
    use chrono::Duration;
    use std::sync::Arc;

    fn create_service() -> SalesService {
        let order_repo = Arc::new(InMemorySalesOrderRepository::new()) as BoxSalesOrderRepository;
        let order_line_repo =
            Arc::new(InMemorySalesOrderLineRepository::new()) as BoxSalesOrderLineRepository;
        let quotation_repo = Arc::new(InMemoryQuotationRepository::new()) as BoxQuotationRepository;
        let quotation_line_repo =
            Arc::new(InMemoryQuotationLineRepository::new()) as BoxQuotationLineRepository;
        SalesService::new(
            order_repo,
            order_line_repo,
            quotation_repo,
            quotation_line_repo,
        )
    }

    #[tokio::test]
    async fn test_create_sales_order() {
        let service = create_service();

        let create = CreateSalesOrder {
            tenant_id: 1,
            cari_id: 1,
            order_date: chrono::Utc::now(),
            delivery_date: Some(chrono::Utc::now() + Duration::days(7)),
            notes: None,
            shipping_address: Some("Shipping address".to_string()),
            billing_address: Some("Billing address".to_string()),
            lines: vec![CreateSalesOrderLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: 2.0,
                unit_price: 100.0,
                tax_rate: 18.0,
                discount_rate: 0.0,
            }],
        };

        let result = service.create_sales_order(create).await;
        assert!(result.is_ok());
        let order = result.unwrap();
        assert_eq!(order.lines.len(), 1);
    }

    #[tokio::test]
    async fn test_create_quotation() {
        let service = create_service();

        let create = CreateQuotation {
            tenant_id: 1,
            cari_id: 1,
            valid_until: chrono::Utc::now() + Duration::days(30),
            notes: None,
            terms: Some("Net 30".to_string()),
            lines: vec![CreateQuotationLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: 1.0,
                unit_price: 100.0,
                tax_rate: 18.0,
                discount_rate: 10.0,
            }],
        };

        let result = service.create_quotation(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_convert_quotation_to_order() {
        let service = create_service();

        // Create quotation
        let quote_create = CreateQuotation {
            tenant_id: 1,
            cari_id: 1,
            valid_until: chrono::Utc::now() + Duration::days(30),
            notes: None,
            terms: None,
            lines: vec![CreateQuotationLine {
                product_id: Some(1),
                description: "Test".to_string(),
                quantity: 1.0,
                unit_price: 100.0,
                tax_rate: 18.0,
                discount_rate: 0.0,
            }],
        };

        let quotation = service.create_quotation(quote_create).await.unwrap();

        // Convert to order
        let order_result = service.convert_quotation_to_order(quotation.id).await;
        assert!(order_result.is_ok());

        // Check quotation status updated
        let updated_quote = service.get_quotation(quotation.id).await.unwrap();
        assert_eq!(updated_quote.status, QuotationStatus::ConvertedToOrder);
        assert!(updated_quote.sales_order_id.is_some());
    }
}
