//! POS repository traits and InMemory implementations

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::error::ApiError;

use super::model::{
    CreatePosSale, CreatePosTerminal, CreateZReport, PosSale, PosSaleLine, PosTerminal,
    PosTerminalStatus, SyncQueueItem, SyncQueueStatus, UpdatePosTerminal, ZReport, ZReportStatus,
};

// --- Terminal repository ---

#[async_trait]
pub trait PosTerminalRepository: Send + Sync {
    async fn create(&self, input: CreatePosTerminal) -> Result<PosTerminal, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<PosTerminal>, ApiError>;
    async fn find_by_code(
        &self,
        code: &str,
        tenant_id: i64,
    ) -> Result<Option<PosTerminal>, ApiError>;
    async fn find_all(&self, tenant_id: i64) -> Result<Vec<PosTerminal>, ApiError>;
    async fn find_all_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<PosTerminal>, ApiError>;
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        input: UpdatePosTerminal,
    ) -> Result<PosTerminal, ApiError>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn update_last_sync(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

pub type BoxPosTerminalRepository = Arc<dyn PosTerminalRepository>;

pub struct InMemoryPosTerminalRepository {
    terminals: Mutex<HashMap<i64, PosTerminal>>,
    next_id: Mutex<i64>,
}

impl InMemoryPosTerminalRepository {
    pub fn new() -> Self {
        Self {
            terminals: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }
}

impl Default for InMemoryPosTerminalRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PosTerminalRepository for InMemoryPosTerminalRepository {
    async fn create(&self, input: CreatePosTerminal) -> Result<PosTerminal, ApiError> {
        let now = Utc::now();
        let mut id_guard = self.next_id.lock();
        let id = *id_guard;
        *id_guard += 1;

        if self
            .terminals
            .lock()
            .values()
            .any(|t| t.tenant_id == input.tenant_id && t.terminal_code == input.terminal_code)
        {
            return Err(ApiError::Conflict(
                "POS terminal with this code already exists".to_string(),
            ));
        }

        let terminal = PosTerminal {
            id,
            tenant_id: input.tenant_id,
            terminal_code: input.terminal_code,
            name: input.name,
            warehouse_id: input.warehouse_id,
            status: PosTerminalStatus::Active,
            store_name: input.store_name,
            last_sync_at: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        self.terminals.lock().insert(id, terminal.clone());
        Ok(terminal)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<PosTerminal>, ApiError> {
        Ok(self
            .terminals
            .lock()
            .get(&id)
            .filter(|t| t.tenant_id == tenant_id && t.deleted_at.is_none())
            .cloned())
    }

    async fn find_by_code(
        &self,
        code: &str,
        tenant_id: i64,
    ) -> Result<Option<PosTerminal>, ApiError> {
        Ok(self
            .terminals
            .lock()
            .values()
            .find(|t| t.tenant_id == tenant_id && t.terminal_code == code && t.deleted_at.is_none())
            .cloned())
    }

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<PosTerminal>, ApiError> {
        Ok(self
            .terminals
            .lock()
            .values()
            .filter(|t| t.tenant_id == tenant_id && t.deleted_at.is_none())
            .cloned()
            .collect())
    }

    async fn find_all_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<PosTerminal>, ApiError> {
        let all: Vec<PosTerminal> = self
            .terminals
            .lock()
            .values()
            .filter(|t| t.tenant_id == tenant_id && t.deleted_at.is_none())
            .cloned()
            .collect();

        let total = all.len() as u64;
        let offset = (page - 1) * per_page;
        let items: Vec<PosTerminal> = if (offset as usize) >= all.len() {
            Vec::new()
        } else {
            let end = std::cmp::min(offset as usize + per_page as usize, all.len());
            all[offset as usize..end].to_vec()
        };

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        input: UpdatePosTerminal,
    ) -> Result<PosTerminal, ApiError> {
        let mut terminals = self.terminals.lock();
        let terminal = terminals
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id && t.deleted_at.is_none())
            .ok_or_else(|| ApiError::NotFound("POS terminal not found".to_string()))?;

        if let Some(name) = input.name {
            terminal.name = name;
        }
        if let Some(warehouse_id) = input.warehouse_id {
            terminal.warehouse_id = warehouse_id;
        }
        if let Some(status) = input.status {
            terminal.status = status;
        }
        if let Some(store_name) = input.store_name {
            terminal.store_name = Some(store_name);
        }
        terminal.updated_at = Utc::now();

        let updated = terminal.clone();
        Ok(updated)
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut terminals = self.terminals.lock();
        let terminal = terminals
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id && t.deleted_at.is_none())
            .ok_or_else(|| ApiError::NotFound("POS terminal not found".to_string()))?;

        terminal.deleted_at = Some(Utc::now());
        terminal.deleted_by = Some(deleted_by);
        terminal.status = PosTerminalStatus::Inactive;
        Ok(())
    }

    async fn update_last_sync(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut terminals = self.terminals.lock();
        let terminal = terminals
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id && t.deleted_at.is_none())
            .ok_or_else(|| ApiError::NotFound("POS terminal not found".to_string()))?;

        terminal.last_sync_at = Some(Utc::now());
        terminal.updated_at = Utc::now();
        Ok(())
    }
}

// --- Sale repository ---

#[async_trait]
pub trait PosSaleRepository: Send + Sync {
    async fn create(
        &self,
        input: CreatePosSale,
        sale_number: String,
        subtotal: Decimal,
        tax_amount: Decimal,
        discount_amount: Decimal,
        total_amount: Decimal,
    ) -> Result<PosSale, ApiError>;
    async fn create_line(
        &self,
        sale_id: i64,
        tenant_id: i64,
        line: &super::model::CreatePosSaleLine,
        line_total: Decimal,
    ) -> Result<PosSaleLine, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<PosSale>, ApiError>;
    async fn find_by_number(
        &self,
        number: &str,
        tenant_id: i64,
    ) -> Result<Option<PosSale>, ApiError>;
    async fn find_lines(&self, sale_id: i64, tenant_id: i64) -> Result<Vec<PosSaleLine>, ApiError>;
    async fn find_all_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<PosSale>, ApiError>;
    async fn find_by_terminal(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<PosSale>, ApiError>;
    async fn find_by_z_report(
        &self,
        z_report_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<PosSale>, ApiError>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn assign_z_report(
        &self,
        sale_id: i64,
        z_report_id: i64,
        tenant_id: i64,
    ) -> Result<(), ApiError>;
}

pub type BoxPosSaleRepository = Arc<dyn PosSaleRepository>;

pub struct InMemoryPosSaleRepository {
    sales: Mutex<HashMap<i64, PosSale>>,
    lines: Mutex<HashMap<i64, PosSaleLine>>,
    next_sale_id: Mutex<i64>,
    next_line_id: Mutex<i64>,
}

impl InMemoryPosSaleRepository {
    pub fn new() -> Self {
        Self {
            sales: Mutex::new(HashMap::new()),
            lines: Mutex::new(HashMap::new()),
            next_sale_id: Mutex::new(1),
            next_line_id: Mutex::new(1),
        }
    }
}

impl Default for InMemoryPosSaleRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PosSaleRepository for InMemoryPosSaleRepository {
    async fn create(
        &self,
        input: CreatePosSale,
        sale_number: String,
        subtotal: Decimal,
        tax_amount: Decimal,
        discount_amount: Decimal,
        total_amount: Decimal,
    ) -> Result<PosSale, ApiError> {
        let now = Utc::now();
        let mut id_guard = self.next_sale_id.lock();
        let id = *id_guard;
        *id_guard += 1;

        let sale = PosSale {
            id,
            tenant_id: input.tenant_id,
            terminal_id: input.terminal_id,
            sale_number,
            cari_id: input.cari_id,
            sale_date: input.sale_date,
            subtotal,
            tax_amount,
            discount_amount,
            total_amount,
            payment_method: input.payment_method,
            z_report_id: None,
            notes: input.notes,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        self.sales.lock().insert(id, sale.clone());
        Ok(sale)
    }

    async fn create_line(
        &self,
        sale_id: i64,
        tenant_id: i64,
        line: &super::model::CreatePosSaleLine,
        line_total: Decimal,
    ) -> Result<PosSaleLine, ApiError> {
        let now = Utc::now();
        let mut id_guard = self.next_line_id.lock();
        let id = *id_guard;
        *id_guard += 1;

        let sale_line = PosSaleLine {
            id,
            tenant_id,
            sale_id,
            product_id: line.product_id,
            description: line.description.clone(),
            quantity: line.quantity,
            unit_price: line.unit_price,
            tax_rate: line.tax_rate,
            discount_amount: line.discount_amount.unwrap_or(Decimal::ZERO),
            line_total,
            created_at: now,
        };

        self.lines.lock().insert(id, sale_line.clone());
        Ok(sale_line)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<PosSale>, ApiError> {
        Ok(self
            .sales
            .lock()
            .get(&id)
            .filter(|s| s.tenant_id == tenant_id && s.deleted_at.is_none())
            .cloned())
    }

    async fn find_by_number(
        &self,
        number: &str,
        tenant_id: i64,
    ) -> Result<Option<PosSale>, ApiError> {
        Ok(self
            .sales
            .lock()
            .values()
            .find(|s| s.tenant_id == tenant_id && s.sale_number == number && s.deleted_at.is_none())
            .cloned())
    }

    async fn find_lines(&self, sale_id: i64, tenant_id: i64) -> Result<Vec<PosSaleLine>, ApiError> {
        Ok(self
            .lines
            .lock()
            .values()
            .filter(|l| l.sale_id == sale_id && l.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_all_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<PosSale>, ApiError> {
        let all: Vec<PosSale> = self
            .sales
            .lock()
            .values()
            .filter(|s| s.tenant_id == tenant_id && s.deleted_at.is_none())
            .cloned()
            .collect();

        let total = all.len() as u64;
        let offset = (page - 1) * per_page;
        let items: Vec<PosSale> = if (offset as usize) >= all.len() {
            Vec::new()
        } else {
            let end = std::cmp::min(offset as usize + per_page as usize, all.len());
            all[offset as usize..end].to_vec()
        };

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_terminal(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<PosSale>, ApiError> {
        Ok(self
            .sales
            .lock()
            .values()
            .filter(|s| {
                s.tenant_id == tenant_id && s.terminal_id == terminal_id && s.deleted_at.is_none()
            })
            .cloned()
            .collect())
    }

    async fn find_by_z_report(
        &self,
        z_report_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<PosSale>, ApiError> {
        Ok(self
            .sales
            .lock()
            .values()
            .filter(|s| {
                s.tenant_id == tenant_id
                    && s.z_report_id == Some(z_report_id)
                    && s.deleted_at.is_none()
            })
            .cloned()
            .collect())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut sales = self.sales.lock();
        let sale = sales
            .get_mut(&id)
            .filter(|s| s.tenant_id == tenant_id && s.deleted_at.is_none())
            .ok_or_else(|| ApiError::NotFound("POS sale not found".to_string()))?;

        sale.deleted_at = Some(Utc::now());
        sale.deleted_by = Some(deleted_by);
        Ok(())
    }

    async fn assign_z_report(
        &self,
        sale_id: i64,
        z_report_id: i64,
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        let mut sales = self.sales.lock();
        let sale = sales
            .get_mut(&sale_id)
            .filter(|s| s.tenant_id == tenant_id && s.deleted_at.is_none())
            .ok_or_else(|| ApiError::NotFound("POS sale not found".to_string()))?;

        sale.z_report_id = Some(z_report_id);
        sale.updated_at = Utc::now();
        Ok(())
    }
}

// --- Z-report repository ---

#[async_trait]
pub trait ZReportRepository: Send + Sync {
    async fn create(
        &self,
        input: CreateZReport,
        report_number: String,
    ) -> Result<ZReport, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ZReport>, ApiError>;
    async fn find_by_number(
        &self,
        number: &str,
        tenant_id: i64,
    ) -> Result<Option<ZReport>, ApiError>;
    async fn find_all_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<ZReport>, ApiError>;
    async fn find_open_by_terminal(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<Option<ZReport>, ApiError>;
    async fn find_by_terminal(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<ZReport>, ApiError>;
    async fn close(
        &self,
        id: i64,
        tenant_id: i64,
        totals: ZReportTotals,
    ) -> Result<ZReport, ApiError>;
    async fn reconcile(&self, id: i64, tenant_id: i64) -> Result<ZReport, ApiError>;
}

pub type BoxZReportRepository = Arc<dyn ZReportRepository>;

/// Totals for closing a Z-report
#[derive(Debug, Clone)]
pub struct ZReportTotals {
    pub total_sales: Decimal,
    pub total_cash: Decimal,
    pub total_card: Decimal,
    pub total_other: Decimal,
    pub total_tax: Decimal,
    pub total_discount: Decimal,
    pub transaction_count: u32,
}

pub struct InMemoryZReportRepository {
    reports: Mutex<HashMap<i64, ZReport>>,
    next_id: Mutex<i64>,
}

impl InMemoryZReportRepository {
    pub fn new() -> Self {
        Self {
            reports: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }
}

impl Default for InMemoryZReportRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ZReportRepository for InMemoryZReportRepository {
    async fn create(
        &self,
        input: CreateZReport,
        report_number: String,
    ) -> Result<ZReport, ApiError> {
        let now = Utc::now();
        let mut id_guard = self.next_id.lock();
        let id = *id_guard;
        *id_guard += 1;

        let report = ZReport {
            id,
            tenant_id: input.tenant_id,
            terminal_id: input.terminal_id,
            report_number,
            report_date: now,
            status: ZReportStatus::Open,
            total_sales: Decimal::ZERO,
            total_cash: Decimal::ZERO,
            total_card: Decimal::ZERO,
            total_other: Decimal::ZERO,
            total_tax: Decimal::ZERO,
            total_discount: Decimal::ZERO,
            transaction_count: 0,
            opened_at: now,
            closed_at: None,
            created_at: now,
            updated_at: now,
        };

        self.reports.lock().insert(id, report.clone());
        Ok(report)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ZReport>, ApiError> {
        Ok(self
            .reports
            .lock()
            .get(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_number(
        &self,
        number: &str,
        tenant_id: i64,
    ) -> Result<Option<ZReport>, ApiError> {
        Ok(self
            .reports
            .lock()
            .values()
            .find(|r| r.tenant_id == tenant_id && r.report_number == number)
            .cloned())
    }

    async fn find_all_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<ZReport>, ApiError> {
        let all: Vec<ZReport> = self
            .reports
            .lock()
            .values()
            .filter(|r| r.tenant_id == tenant_id)
            .cloned()
            .collect();

        let total = all.len() as u64;
        let offset = (page - 1) * per_page;
        let items: Vec<ZReport> = if (offset as usize) >= all.len() {
            Vec::new()
        } else {
            let end = std::cmp::min(offset as usize + per_page as usize, all.len());
            all[offset as usize..end].to_vec()
        };

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_open_by_terminal(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<Option<ZReport>, ApiError> {
        Ok(self
            .reports
            .lock()
            .values()
            .find(|r| {
                r.tenant_id == tenant_id
                    && r.terminal_id == terminal_id
                    && r.status == ZReportStatus::Open
            })
            .cloned())
    }

    async fn find_by_terminal(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<ZReport>, ApiError> {
        Ok(self
            .reports
            .lock()
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.terminal_id == terminal_id)
            .cloned()
            .collect())
    }

    async fn close(
        &self,
        id: i64,
        tenant_id: i64,
        totals: ZReportTotals,
    ) -> Result<ZReport, ApiError> {
        let mut reports = self.reports.lock();
        let report = reports
            .get_mut(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound("Z-report not found".to_string()))?;

        if report.status != ZReportStatus::Open {
            return Err(ApiError::BadRequest("Z-report is not open".to_string()));
        }

        report.status = ZReportStatus::Closed;
        report.total_sales = totals.total_sales;
        report.total_cash = totals.total_cash;
        report.total_card = totals.total_card;
        report.total_other = totals.total_other;
        report.total_tax = totals.total_tax;
        report.total_discount = totals.total_discount;
        report.transaction_count = totals.transaction_count;
        report.closed_at = Some(Utc::now());
        report.updated_at = Utc::now();

        let updated = report.clone();
        Ok(updated)
    }

    async fn reconcile(&self, id: i64, tenant_id: i64) -> Result<ZReport, ApiError> {
        let mut reports = self.reports.lock();
        let report = reports
            .get_mut(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound("Z-report not found".to_string()))?;

        if report.status != ZReportStatus::Closed {
            return Err(ApiError::BadRequest(
                "Z-report must be closed before reconciliation".to_string(),
            ));
        }

        report.status = ZReportStatus::Reconciled;
        report.updated_at = Utc::now();

        let updated = report.clone();
        Ok(updated)
    }
}

// --- Sync queue repository (simple in-memory) ---

#[async_trait]
pub trait SyncQueueRepository: Send + Sync {
    async fn enqueue(&self, item: SyncQueueItem) -> Result<SyncQueueItem, ApiError>;
    async fn find_pending(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SyncQueueItem>, ApiError>;
    async fn mark_synced(&self, id: i64) -> Result<(), ApiError>;
    async fn mark_failed(&self, id: i64, error: &str) -> Result<(), ApiError>;
}

pub type BoxSyncQueueRepository = Arc<dyn SyncQueueRepository>;

pub struct InMemorySyncQueueRepository {
    items: Mutex<HashMap<i64, SyncQueueItem>>,
    next_id: Mutex<i64>,
}

impl InMemorySyncQueueRepository {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }
}

impl Default for InMemorySyncQueueRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SyncQueueRepository for InMemorySyncQueueRepository {
    async fn enqueue(&self, mut item: SyncQueueItem) -> Result<SyncQueueItem, ApiError> {
        let mut id_guard = self.next_id.lock();
        let id = *id_guard;
        *id_guard += 1;
        item.id = id;
        self.items.lock().insert(id, item.clone());
        Ok(item)
    }

    async fn find_pending(
        &self,
        terminal_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SyncQueueItem>, ApiError> {
        Ok(self
            .items
            .lock()
            .values()
            .filter(|i| {
                i.terminal_id == terminal_id
                    && i.tenant_id == tenant_id
                    && i.status == SyncQueueStatus::Pending
            })
            .cloned()
            .collect())
    }

    async fn mark_synced(&self, id: i64) -> Result<(), ApiError> {
        let mut items = self.items.lock();
        let item = items
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Sync queue item not found".to_string()))?;
        item.status = SyncQueueStatus::Synced;
        item.synced_at = Some(Utc::now());
        Ok(())
    }

    async fn mark_failed(&self, id: i64, error: &str) -> Result<(), ApiError> {
        let mut items = self.items.lock();
        let item = items
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Sync queue item not found".to_string()))?;
        item.status = SyncQueueStatus::Failed;
        item.error_message = Some(error.to_string());
        Ok(())
    }
}
