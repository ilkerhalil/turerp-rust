//! POS service — business logic

use chrono::Utc;
use rust_decimal::Decimal;

use crate::error::ApiError;

use super::model::{
    CreatePosSale, CreatePosTerminal, CreateZReport, PaymentMethod, PosSaleLineResponse,
    PosSaleResponse, PosTerminalResponse, PosTerminalStatus, SyncQueueItem, SyncQueueStatus,
    UpdatePosTerminal, ZReportResponse, ZReportStatus,
};
use super::repository::{
    BoxPosSaleRepository, BoxPosTerminalRepository, BoxSyncQueueRepository, BoxZReportRepository,
    ZReportTotals,
};

#[derive(Clone)]
pub struct PosService {
    terminal_repo: BoxPosTerminalRepository,
    sale_repo: BoxPosSaleRepository,
    z_report_repo: BoxZReportRepository,
    sync_queue_repo: BoxSyncQueueRepository,
}

impl PosService {
    pub fn new(
        terminal_repo: BoxPosTerminalRepository,
        sale_repo: BoxPosSaleRepository,
        z_report_repo: BoxZReportRepository,
        sync_queue_repo: BoxSyncQueueRepository,
    ) -> Self {
        Self {
            terminal_repo,
            sale_repo,
            z_report_repo,
            sync_queue_repo,
        }
    }

    // --- Terminal operations ---

    pub async fn create_terminal(
        &self,
        input: CreatePosTerminal,
    ) -> Result<PosTerminalResponse, ApiError> {
        if input.terminal_code.trim().is_empty() {
            return Err(ApiError::Validation("Terminal code is required".into()));
        }
        if input.name.trim().is_empty() {
            return Err(ApiError::Validation("Terminal name is required".into()));
        }

        let terminal = self.terminal_repo.create(input).await?;
        Ok(terminal.into())
    }

    pub async fn get_terminal(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<PosTerminalResponse, ApiError> {
        let terminal = self
            .terminal_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("POS terminal not found".into()))?;
        Ok(terminal.into())
    }

    pub async fn list_terminals(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<crate::common::pagination::PaginatedResult<PosTerminalResponse>, ApiError> {
        let paginated = self
            .terminal_repo
            .find_all_paginated(tenant_id, page, per_page)
            .await?;
        let items = paginated.items.into_iter().map(Into::into).collect();
        Ok(crate::common::pagination::PaginatedResult::new(
            items,
            paginated.page,
            paginated.per_page,
            paginated.total,
        ))
    }

    pub async fn update_terminal(
        &self,
        id: i64,
        tenant_id: i64,
        input: UpdatePosTerminal,
    ) -> Result<PosTerminalResponse, ApiError> {
        let terminal = self.terminal_repo.update(id, tenant_id, input).await?;
        Ok(terminal.into())
    }

    pub async fn delete_terminal(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.terminal_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    pub async fn sync_terminal(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<PosTerminalResponse, ApiError> {
        self.terminal_repo.update_last_sync(id, tenant_id).await?;
        let terminal = self
            .terminal_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("POS terminal not found".into()))?;
        Ok(terminal.into())
    }

    // --- Sale operations ---

    pub async fn create_sale(&self, input: CreatePosSale) -> Result<PosSaleResponse, ApiError> {
        // Verify terminal exists and is active
        let terminal = self
            .terminal_repo
            .find_by_id(input.terminal_id, input.tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("POS terminal not found".into()))?;

        if terminal.status != PosTerminalStatus::Active {
            return Err(ApiError::BadRequest("Terminal is not active".to_string()));
        }

        if input.lines.is_empty() {
            return Err(ApiError::Validation(
                "Sale must have at least one line".into(),
            ));
        }

        // Calculate totals
        let mut subtotal = Decimal::ZERO;
        let mut tax_amount = Decimal::ZERO;
        let mut lines_discount = Decimal::ZERO;

        for line in &input.lines {
            let line_subtotal = line.quantity * line.unit_price;
            let line_discount = line.discount_amount.unwrap_or(Decimal::ZERO);
            let line_taxable = line_subtotal - line_discount;
            let line_tax = line_taxable * line.tax_rate / Decimal::from(100);
            subtotal += line_subtotal;
            tax_amount += line_tax;
            lines_discount += line_discount;
        }

        let header_discount = input.discount_amount.unwrap_or(Decimal::ZERO);
        let discount_amount = lines_discount + header_discount;
        let total_amount = subtotal + tax_amount - header_discount;

        // Generate sale number
        let sale_number = format!("POS-{}", Utc::now().timestamp());

        let sale = self
            .sale_repo
            .create(
                input.clone(),
                sale_number,
                subtotal,
                tax_amount,
                discount_amount,
                total_amount,
            )
            .await?;

        // Create lines
        let mut line_responses = Vec::new();
        for line in &input.lines {
            let line_subtotal = line.quantity * line.unit_price;
            let line_discount = line.discount_amount.unwrap_or(Decimal::ZERO);
            let line_total = line_subtotal - line_discount;

            let sale_line = self
                .sale_repo
                .create_line(sale.id, input.tenant_id, line, line_total)
                .await?;

            line_responses.push(PosSaleLineResponse {
                id: sale_line.id,
                product_id: sale_line.product_id,
                description: sale_line.description,
                quantity: sale_line.quantity,
                unit_price: sale_line.unit_price,
                tax_rate: sale_line.tax_rate,
                discount_amount: sale_line.discount_amount,
                line_total: sale_line.line_total,
            });
        }

        Ok(PosSaleResponse {
            id: sale.id,
            terminal_id: sale.terminal_id,
            sale_number: sale.sale_number,
            cari_id: sale.cari_id,
            sale_date: sale.sale_date,
            subtotal: sale.subtotal,
            tax_amount: sale.tax_amount,
            discount_amount: sale.discount_amount,
            total_amount: sale.total_amount,
            payment_method: sale.payment_method.to_string(),
            z_report_id: sale.z_report_id,
            notes: sale.notes,
            lines: line_responses,
            created_at: sale.created_at,
        })
    }

    pub async fn get_sale(&self, id: i64, tenant_id: i64) -> Result<PosSaleResponse, ApiError> {
        let sale = self
            .sale_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("POS sale not found".into()))?;

        let lines = self.sale_repo.find_lines(id, tenant_id).await?;
        let line_responses: Vec<PosSaleLineResponse> = lines
            .into_iter()
            .map(|l| PosSaleLineResponse {
                id: l.id,
                product_id: l.product_id,
                description: l.description,
                quantity: l.quantity,
                unit_price: l.unit_price,
                tax_rate: l.tax_rate,
                discount_amount: l.discount_amount,
                line_total: l.line_total,
            })
            .collect();

        Ok(PosSaleResponse {
            id: sale.id,
            terminal_id: sale.terminal_id,
            sale_number: sale.sale_number,
            cari_id: sale.cari_id,
            sale_date: sale.sale_date,
            subtotal: sale.subtotal,
            tax_amount: sale.tax_amount,
            discount_amount: sale.discount_amount,
            total_amount: sale.total_amount,
            payment_method: sale.payment_method.to_string(),
            z_report_id: sale.z_report_id,
            notes: sale.notes,
            lines: line_responses,
            created_at: sale.created_at,
        })
    }

    pub async fn list_sales(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<crate::common::pagination::PaginatedResult<PosSaleResponse>, ApiError> {
        let paginated = self
            .sale_repo
            .find_all_paginated(tenant_id, page, per_page)
            .await?;

        let mut items = Vec::new();
        for sale in paginated.items {
            let lines = self.sale_repo.find_lines(sale.id, tenant_id).await?;
            let line_responses: Vec<PosSaleLineResponse> = lines
                .into_iter()
                .map(|l| PosSaleLineResponse {
                    id: l.id,
                    product_id: l.product_id,
                    description: l.description,
                    quantity: l.quantity,
                    unit_price: l.unit_price,
                    tax_rate: l.tax_rate,
                    discount_amount: l.discount_amount,
                    line_total: l.line_total,
                })
                .collect();

            items.push(PosSaleResponse {
                id: sale.id,
                terminal_id: sale.terminal_id,
                sale_number: sale.sale_number,
                cari_id: sale.cari_id,
                sale_date: sale.sale_date,
                subtotal: sale.subtotal,
                tax_amount: sale.tax_amount,
                discount_amount: sale.discount_amount,
                total_amount: sale.total_amount,
                payment_method: sale.payment_method.to_string(),
                z_report_id: sale.z_report_id,
                notes: sale.notes,
                lines: line_responses,
                created_at: sale.created_at,
            });
        }

        Ok(crate::common::pagination::PaginatedResult::new(
            items,
            paginated.page,
            paginated.per_page,
            paginated.total,
        ))
    }

    pub async fn list_sales_by_terminal(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<PosSaleResponse>, ApiError> {
        let sales = self
            .sale_repo
            .find_by_terminal(terminal_id, tenant_id)
            .await?;
        let mut responses = Vec::new();
        for sale in sales {
            let lines = self.sale_repo.find_lines(sale.id, tenant_id).await?;
            let line_responses: Vec<PosSaleLineResponse> = lines
                .into_iter()
                .map(|l| PosSaleLineResponse {
                    id: l.id,
                    product_id: l.product_id,
                    description: l.description,
                    quantity: l.quantity,
                    unit_price: l.unit_price,
                    tax_rate: l.tax_rate,
                    discount_amount: l.discount_amount,
                    line_total: l.line_total,
                })
                .collect();

            responses.push(PosSaleResponse {
                id: sale.id,
                terminal_id: sale.terminal_id,
                sale_number: sale.sale_number,
                cari_id: sale.cari_id,
                sale_date: sale.sale_date,
                subtotal: sale.subtotal,
                tax_amount: sale.tax_amount,
                discount_amount: sale.discount_amount,
                total_amount: sale.total_amount,
                payment_method: sale.payment_method.to_string(),
                z_report_id: sale.z_report_id,
                notes: sale.notes,
                lines: line_responses,
                created_at: sale.created_at,
            });
        }
        Ok(responses)
    }

    pub async fn delete_sale(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.sale_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    // --- Z-report operations ---

    pub async fn create_z_report(&self, input: CreateZReport) -> Result<ZReportResponse, ApiError> {
        // Verify terminal exists
        let _terminal = self
            .terminal_repo
            .find_by_id(input.terminal_id, input.tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("POS terminal not found".into()))?;

        // Check if there's already an open Z-report for this terminal
        if let Some(_existing) = self
            .z_report_repo
            .find_open_by_terminal(input.terminal_id, input.tenant_id)
            .await?
        {
            return Err(ApiError::Conflict(
                "Terminal already has an open Z-report".to_string(),
            ));
        }

        let report_number = format!("Z-{}", Utc::now().timestamp());
        let report = self.z_report_repo.create(input, report_number).await?;
        Ok(report.into())
    }

    pub async fn get_z_report(&self, id: i64, tenant_id: i64) -> Result<ZReportResponse, ApiError> {
        let report = self
            .z_report_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Z-report not found".into()))?;
        Ok(report.into())
    }

    pub async fn list_z_reports(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<crate::common::pagination::PaginatedResult<ZReportResponse>, ApiError> {
        let paginated = self
            .z_report_repo
            .find_all_paginated(tenant_id, page, per_page)
            .await?;
        let items = paginated.items.into_iter().map(Into::into).collect();
        Ok(crate::common::pagination::PaginatedResult::new(
            items,
            paginated.page,
            paginated.per_page,
            paginated.total,
        ))
    }

    pub async fn close_z_report(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<ZReportResponse, ApiError> {
        // Find all sales for this terminal with no z_report_id assigned
        let report = self
            .z_report_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Z-report not found".into()))?;

        if report.status != ZReportStatus::Open {
            return Err(ApiError::BadRequest("Z-report is not open".into()));
        }

        let sales = self
            .sale_repo
            .find_by_terminal(report.terminal_id, tenant_id)
            .await?;

        // Assign unassigned sales to this Z-report and compute totals
        let mut total_sales = Decimal::ZERO;
        let mut total_cash = Decimal::ZERO;
        let mut total_card = Decimal::ZERO;
        let mut total_other = Decimal::ZERO;
        let mut total_tax = Decimal::ZERO;
        let mut total_discount = Decimal::ZERO;
        let mut transaction_count = 0u32;

        for sale in &sales {
            if sale.z_report_id.is_none() {
                self.sale_repo
                    .assign_z_report(sale.id, id, tenant_id)
                    .await?;
                transaction_count += 1;
                total_sales += sale.total_amount;
                total_tax += sale.tax_amount;
                total_discount += sale.discount_amount;

                match sale.payment_method {
                    PaymentMethod::Cash => total_cash += sale.total_amount,
                    PaymentMethod::CreditCard | PaymentMethod::DebitCard => {
                        total_card += sale.total_amount
                    }
                    PaymentMethod::MobilePayment
                    | PaymentMethod::Voucher
                    | PaymentMethod::Other => total_other += sale.total_amount,
                }
            }
        }

        let totals = ZReportTotals {
            total_sales,
            total_cash,
            total_card,
            total_other,
            total_tax,
            total_discount,
            transaction_count,
        };

        let closed = self.z_report_repo.close(id, tenant_id, totals).await?;
        Ok(closed.into())
    }

    pub async fn reconcile_z_report(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<ZReportResponse, ApiError> {
        let report = self.z_report_repo.reconcile(id, tenant_id).await?;
        Ok(report.into())
    }

    pub async fn list_z_reports_by_terminal(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<ZReportResponse>, ApiError> {
        let reports = self
            .z_report_repo
            .find_by_terminal(terminal_id, tenant_id)
            .await?;
        Ok(reports.into_iter().map(Into::into).collect())
    }

    // --- Sync queue operations ---

    pub async fn enqueue_sync(
        &self,
        tenant_id: i64,
        terminal_id: i64,
        payload: String,
    ) -> Result<i64, ApiError> {
        let item = SyncQueueItem {
            id: 0,
            tenant_id,
            terminal_id,
            payload,
            status: SyncQueueStatus::Pending,
            error_message: None,
            created_at: Utc::now(),
            synced_at: None,
        };
        let saved = self.sync_queue_repo.enqueue(item).await?;
        Ok(saved.id)
    }

    pub async fn pending_sync_count(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<usize, ApiError> {
        let items = self
            .sync_queue_repo
            .find_pending(terminal_id, tenant_id)
            .await?;
        Ok(items.len())
    }
}
