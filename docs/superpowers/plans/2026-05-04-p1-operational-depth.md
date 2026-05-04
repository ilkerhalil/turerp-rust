# P1: Operational Depth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add operational depth features: Lot/Serial tracking, Expiry dates, WMS bin locations, MRP, Reorder points, Quality management, Multi-unit conversion, Cost accounting.

**Architecture:** Extend existing stock/manufacturing domains with new fields and services. New `wms` and `mrp` domain modules. Multi-unit and cost accounting added to product and accounting domains.

**Tech Stack:** Rust, Actix-web, SQLx, rust_decimal, chrono, utoipa

---

## Task 1: Lot & Serial Tracking — Stock Model Extensions

**Files:**
- Modify: `src/domain/stock/model.rs` (add Lot/Serial models)
- Modify: `src/domain/stock/repository.rs` (add LotRepository trait)
- Modify: `src/domain/stock/service.rs` (add lot methods)

- [ ] **Step 1: Add Lot and Serial models to stock/model.rs**

```rust
/// Lot/batch tracking for stock items
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockLot {
    pub id: i64,
    pub tenant_id: i64,
    pub product_id: i64,
    pub warehouse_id: i64,
    pub lot_number: String,
    pub serial_number: Option<String>,
    pub manufacturing_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub quantity: Decimal,
    pub reserved_quantity: Decimal,
    pub supplier_ref: Option<String>,
    pub certificate_no: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateStockLot {
    pub product_id: i64,
    pub warehouse_id: i64,
    pub lot_number: String,
    pub serial_number: Option<String>,
    pub manufacturing_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub quantity: Decimal,
    pub supplier_ref: Option<String>,
    pub certificate_no: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockLotResponse {
    pub id: i64,
    pub product_id: i64,
    pub warehouse_id: i64,
    pub lot_number: String,
    pub serial_number: Option<String>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub quantity: Decimal,
    pub reserved_quantity: Decimal,
    pub is_active: bool,
}

/// Expiry alert for lots approaching expiry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExpiryAlert {
    pub lot_id: i64,
    pub product_id: i64,
    pub lot_number: String,
    pub expiry_date: chrono::NaiveDate,
    pub days_remaining: i64,
    pub quantity: Decimal,
}
```

- [ ] **Step 2: Add LotRepository trait to stock/repository.rs**

Methods: `create_lot`, `find_by_lot_number`, `find_by_product`, `find_expiring_soon`, `reserve_lot`, `release_reservation`, `soft_delete`.

- [ ] **Step 3: Add lot methods to StockService**

`create_lot`, `get_lot`, `list_lots`, `get_expiry_alerts(days_threshold)`, `reserve_lot_quantity`, `release_lot_reservation`.

- [ ] **Step 4: Write unit tests**

Test: lot creation, expiry detection, reservation/release, serial uniqueness per product.

- [ ] **Step 5: Run tests**

Run: `cargo test stock::lot`
Expected: All lot tests pass

- [ ] **Step 6: Commit**

```bash
git add src/domain/stock/
git commit -m "feat(stock): add lot/serial tracking with expiry date support"
```

---

## Task 2: Lot & Serial — API Endpoints & PostgreSQL

**Files:**
- Modify: `src/api/v1/stock.rs` (add lot endpoints)
- Create: `migrations/013_stock_lots.sql`
- Create: `src/domain/stock/postgres_lot_repository.rs`

- [ ] **Step 1: Add lot API endpoints to stock.rs**

```
POST /api/v1/stock/lots                    — Create lot
GET  /api/v1/stock/lots                    — List lots (product_id, warehouse_id filters)
GET  /api/v1/stock/lots/{id}               — Get lot
GET  /api/v1/stock/lots/expiring           — Expiring lots (?days=30)
POST /api/v1/stock/lots/{id}/reserve       — Reserve lot quantity
POST /api/v1/stock/lots/{id}/release       — Release reservation
```

- [ ] **Step 2: Write migration SQL**

Table: `stock_lots` with product_id, warehouse_id, lot_number, serial_number, dates, quantity, reserved_quantity. Unique index: (tenant_id, product_id, lot_number, serial_number).

- [ ] **Step 3: Write PostgresLotRepository**

- [ ] **Step 4: Run cargo check + cargo test**

- [ ] **Step 5: Commit**

```bash
git add src/api/v1/stock.rs migrations/013_stock_lots.sql src/domain/stock/
git commit -m "feat(stock): add lot/serial API endpoints and PostgreSQL repository"
```

---

## Task 3: WMS Bin Location — Model & Repository

**Files:**
- Create: `src/domain/wms/mod.rs`
- Create: `src/domain/wms/model.rs`
- Create: `src/domain/wms/repository.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: Write WMS models**

```rust
/// Bin location within a warehouse
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BinLocation {
    pub id: i64,
    pub tenant_id: i64,
    pub warehouse_id: i64,
    pub zone: String,          // Zone (A, B, C...)
    pub aisle: String,         // Koridor (01, 02...)
    pub rack: String,          // Raf (R1, R2...)
    pub shelf: String,         // Rafyeri (S1, S2...)
    pub bin_code: String,      // Full code: A-01-R1-S1
    pub description: Option<String>,
    pub capacity: Option<Decimal>,
    pub current_quantity: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Stock in a specific bin
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BinStock {
    pub id: i64,
    pub tenant_id: i64,
    pub bin_id: i64,
    pub product_id: i64,
    pub lot_id: Option<i64>,
    pub quantity: Decimal,
    pub reserved_quantity: Decimal,
}
```

- [ ] **Step 2: Write BinLocationRepository trait**

Methods: `create_bin`, `find_by_code`, `find_by_warehouse`, `update_bin`, `get_bin_stock`, `transfer_between_bins`, `find_available_bins`.

- [ ] **Step 3: Write InMemoryBinLocationRepository**

- [ ] **Step 4: Register in domain/mod.rs**

- [ ] **Step 5: Run cargo check**

- [ ] **Step 6: Commit**

```bash
git add src/domain/wms/ src/domain/mod.rs
git commit -m "feat(wms): add bin location model and repository"
```

---

## Task 4: WMS — Service, API & PostgreSQL

**Files:**
- Create: `src/domain/wms/service.rs`
- Create: `src/api/v1/wms.rs`
- Create: `src/domain/wms/postgres_repository.rs`
- Create: `migrations/014_wms.sql`
- Modify: `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`

- [ ] **Step 1: Write WmsService**

Methods: `create_bin`, `get_bin`, `list_bins`, `put_stock_in_bin`, `transfer_stock`, `get_bin_contents`, `find_available_bins_for_product`.

- [ ] **Step 2: Write API endpoints**

```
POST /api/v1/wms/bins                      — Create bin
GET  /api/v1/wms/bins                      — List bins (warehouse_id filter)
GET  /api/v1/wms/bins/{id}                 — Get bin
PUT  /api/v1/wms/bins/{id}                 — Update bin
POST /api/v1/wms/bins/{id}/put             — Put stock in bin
POST /api/v1/wms/bins/transfer             — Transfer between bins
GET  /api/v1/wms/bins/{id}/stock           — Get bin contents
```

- [ ] **Step 3: Write migration + PostgreSQL repo**

- [ ] **Step 4: Wire into AppState, OpenAPI, main.rs**

- [ ] **Step 5: Run cargo check + cargo test**

- [ ] **Step 6: Commit**

```bash
git add src/domain/wms/ src/api/v1/wms.rs migrations/014_wms.sql src/api/ src/main.rs
git commit -m "feat(wms): add service, API, and PostgreSQL repository"
```

---

## Task 5: Multi-Unit Conversion

**Files:**
- Modify: `src/domain/product/model.rs` (add UnitConversion)
- Modify: `src/domain/product/repository.rs` (add conversion methods)
- Modify: `src/domain/product/service.rs` (add conversion calculation)
- Modify: `src/api/v1/product_variants.rs` or new stock endpoint

- [ ] **Step 1: Add UnitConversion model**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UnitConversion {
    pub id: i64,
    pub tenant_id: i64,
    pub product_id: i64,
    pub from_unit_id: i64,
    pub to_unit_id: i64,
    pub factor: Decimal,    // 1 from_unit = factor to_unit
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateUnitConversion {
    pub product_id: i64,
    pub from_unit_id: i64,
    pub to_unit_id: i64,
    pub factor: Decimal,
}
```

- [ ] **Step 2: Add conversion methods to ProductService**

`create_conversion`, `convert_quantity(from_unit, to_unit, qty)`, `list_conversions`.

- [ ] **Step 3: Add API endpoint**

`POST /api/v1/products/{id}/unit-conversions`, `GET /api/v1/products/{id}/unit-conversions`, `POST /api/v1/products/convert-quantity`

- [ ] **Step 4: Write unit tests**

Test: conversion calculation, bidirectional conversion, factor validation (must be > 0).

- [ ] **Step 5: Run cargo check + cargo test**

- [ ] **Step 6: Commit**

```bash
git add src/domain/product/ src/api/
git commit -m "feat(product): add multi-unit conversion with factor-based calculation"
```

---

## Task 6: Reorder Points & Safety Stock

**Files:**
- Modify: `src/domain/stock/model.rs` (add ReorderPoint)
- Modify: `src/domain/stock/service.rs` (add reorder logic)
- Modify: `src/api/v1/stock.rs` (add reorder endpoints)

- [ ] **Step 1: Add ReorderPoint model**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReorderPoint {
    pub id: i64,
    pub tenant_id: i64,
    pub product_id: i64,
    pub warehouse_id: i64,
    pub min_stock: Decimal,       // Minimum stock level
    pub max_stock: Decimal,       // Maximum stock level
    pub safety_stock: Decimal,    // Safety stock buffer
    pub reorder_qty: Decimal,     // Order quantity when below min
    pub lead_time_days: u32,      // Supplier lead time
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReorderAlert {
    pub product_id: i64,
    pub warehouse_id: i64,
    pub product_name: String,
    pub current_stock: Decimal,
    pub min_stock: Decimal,
    pub safety_stock: Decimal,
    pub suggested_order_qty: Decimal,
}
```

- [ ] **Step 2: Add reorder methods to StockService**

`set_reorder_point`, `get_reorder_alerts`, `calculate_suggested_order`.

- [ ] **Step 3: Add API endpoints**

`POST /api/v1/stock/reorder-points`, `GET /api/v1/stock/reorder-points`, `GET /api/v1/stock/reorder-alerts`

- [ ] **Step 4: Write unit tests**

- [ ] **Step 5: Run cargo check + cargo test**

- [ ] **Step 6: Commit**

```bash
git add src/domain/stock/ src/api/v1/stock.rs
git commit -m "feat(stock): add reorder points, safety stock, and reorder alerts"
```

---

## Task 7: MRP (Material Requirements Planning)

**Files:**
- Create: `src/domain/mrp/mod.rs`
- Create: `src/domain/mrp/model.rs`
- Create: `src/domain/mrp/repository.rs`
- Create: `src/domain/mrp/service.rs`
- Create: `src/api/v1/mrp.rs`
- Modify: `src/domain/mod.rs`, `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`

- [ ] **Step 1: Write MRP models**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MrpPlan {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub planning_horizon_days: u32,
    pub status: MrpPlanStatus,
    pub created_at: DateTime<Utc>,
}

pub enum MrpPlanStatus { Draft, Calculated, Approved, Closed }

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MrpItem {
    pub id: i64,
    pub plan_id: i64,
    pub product_id: i64,
    pub gross_requirement: Decimal,
    pub scheduled_receipts: Decimal,
    pub on_hand: Decimal,
    pub net_requirement: Decimal,
    pub planned_order: Decimal,
    pub order_date: Option<chrono::NaiveDate>,
    pub need_date: Option<chrono::NaiveDate>,
}
```

- [ ] **Step 2: Write MrpService**

`create_plan`, `run_mrp(product_ids, horizon)` — uses BOM explosion + stock levels + open orders to calculate net requirements, `get_plan_items`, `approve_plan`.

- [ ] **Step 3: Add API endpoints**

```
POST /api/v1/mrp/plans                  — Create MRP plan
POST /api/v1/mrp/plans/{id}/run        — Run MRP calculation
GET  /api/v1/mrp/plans/{id}            — Get plan
GET  /api/v1/mrp/plans/{id}/items      — Get plan items
POST /api/v1/mrp/plans/{id}/approve    — Approve plan
```

- [ ] **Step 4: Wire and test**

- [ ] **Step 5: Commit**

```bash
git add src/domain/mrp/ src/api/v1/mrp.rs src/api/ src/main.rs src/domain/mod.rs
git commit -m "feat(mrp): add Material Requirements Planning with BOM explosion"
```

---

## Task 8: Quality Management System (QMS) Enhancement

**Files:**
- Modify: `src/domain/manufacturing/model.rs` (add QMS models)
- Modify: `src/domain/manufacturing/service.rs` (add QMS workflow)
- Modify: `src/api/v1/manufacturing.rs` (add QMS endpoints)

- [ ] **Step 1: Add QMS process models**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QualityControlPlan {
    pub id: i64,
    pub tenant_id: i64,
    pub product_id: i64,
    pub name: String,
    pub inspection_points: Vec<InspectionPoint>,
    pub is_active: bool,
}

pub struct InspectionPoint {
    pub sequence: u32,
    pub inspection_type: String,
    pub criteria: String,
    pub tolerance_min: Option<Decimal>,
    pub tolerance_max: Option<Decimal>,
    pub is_mandatory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CorrectiveAction {
    pub id: i64,
    pub tenant_id: i64,
    pub ncr_id: i64,
    pub description: String,
    pub assigned_to: Option<String>,
    pub due_date: Option<chrono::NaiveDate>,
    pub status: CorrectiveActionStatus,
    pub completed_at: Option<DateTime<Utc>>,
}
```

- [ ] **Step 2: Add QMS methods to ManufacturingService**

`create_qc_plan`, `create_inspection_from_plan`, `add_corrective_action`, `close_corrective_action`.

- [ ] **Step 3: Add API endpoints**

```
POST /api/v1/manufacturing/qc-plans              — Create QC plan
GET  /api/v1/manufacturing/qc-plans              — List QC plans
GET  /api/v1/manufacturing/qc-plans/{id}         — Get QC plan
POST /api/v1/manufacturing/ncr/{id}/corrective   — Add corrective action
PUT  /api/v1/manufacturing/corrective/{id}/close  — Close corrective action
```

- [ ] **Step 4: Write tests + commit**

```bash
git add src/domain/manufacturing/ src/api/v1/manufacturing.rs
git commit -m "feat(manufacturing): add QMS process management with corrective actions"
```

---

## Task 9: Cost Accounting

**Files:**
- Create: `src/domain/cost_accounting/mod.rs`
- Create: `src/domain/cost_accounting/model.rs`
- Create: `src/domain/cost_accounting/service.rs`
- Modify: `src/domain/mod.rs`
- Modify: `src/api/v1/accounting.rs` (add cost endpoints)

- [ ] **Step 1: Write cost accounting models**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CostCenter {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub parent_id: Option<i64>,
    pub is_active: bool,
}

pub enum CostAllocationMethod {
    Direct,           // Doğrudan
    StepDown,         // Adım dağıtım
    Reciprocal,       // Karşılıklı
    ActivityBased,    // Faaliyete dayalı
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CostAllocation {
    pub id: i64,
    pub tenant_id: i64,
    pub from_cost_center_id: i64,
    pub to_cost_center_id: i64,
    pub amount: Decimal,
    pub method: CostAllocationMethod,
    pub allocation_basis: Option<String>,
    pub period_year: i32,
    pub period_month: u32,
}
```

- [ ] **Step 2: Write CostAccountingService**

`create_cost_center`, `allocate_costs`, `get_cost_center_summary`.

- [ ] **Step 3: Add API endpoints to accounting**

```
POST /api/v1/accounting/cost-centers          — Create cost center
GET  /api/v1/accounting/cost-centers          — List cost centers
POST /api/v1/accounting/cost-centers/allocate — Allocate costs
GET  /api/v1/accounting/cost-centers/{id}/summary — Cost center summary
```

- [ ] **Step 4: Wire, test, commit**

```bash
git add src/domain/cost_accounting/ src/domain/mod.rs src/api/v1/accounting.rs
git commit -m "feat(cost-accounting): add cost centers and allocation methods"
```

---

## Summary

| Task | Feature | New Endpoints |
|------|---------|---------------|
| 1-2 | Lot/Serial + Expiry | 6 |
| 3-4 | WMS Bin Location | 7 |
| 5 | Multi-Unit Conversion | 3 |
| 6 | Reorder Points | 3 |
| 7 | MRP | 5 |
| 8 | QMS Enhancement | 5 |
| 9 | Cost Accounting | 4 |
| **Total** | | **33** |