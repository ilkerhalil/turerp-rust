# Turerp ERP - Module Documentation

## Overview

This documentation covers all modules in the Turerp ERP system. Each module is designed for specific business functions within a multi-tenant SaaS architecture.

---

## Core Modules

| Module | Description | Documentation |
|--------|-------------|----------------|
| **Cari** | Customer/Vendor accounts | [cari.md](modules/core/cari.md) |
| **Products** | Product catalog & categories | [products.md](modules/core/products.md) |
| **Stock** | Warehouse & inventory management | [stock.md](modules/core/stock.md) |
| **Invoices** | Customer invoices & payments | [invoices.md](modules/core/invoices.md) |

---

## Sales & Purchase

| Module | Description | Documentation |
|--------|-------------|----------------|
| **Sales** | Sales orders & quotations | [sales.md](modules/sales-purchase/sales.md) |
| **Purchase** | Purchase requests, orders, receipts | [purchase.md](modules/sales-purchase/purchase.md) |

---

## Human Resources

| Module | Description | Documentation |
|--------|-------------|----------------|
| **HR** | Employees, payroll, attendance, leaves | [hr.md](modules/hr.md) |

---

## Finance

| Module | Description | Documentation |
|--------|-------------|----------------|
| **Accounting** | Chart of accounts, journal entries, reports | [accounting.md](modules/accounting.md) |
| **Assets** | Fixed assets, depreciation, maintenance | [assets.md](modules/assets.md) |

---

## Projects

| Module | Description | Documentation |
|--------|-------------|----------------|
| **Projects** | Project management, WBS, clients | [projects.md](modules/projects/projects.md) |
| **Project Finance** | Project costs, budgets, profitability | [project-finance.md](modules/projects/project-finance.md) |

---

## Manufacturing

| Module | Description | Documentation |
|--------|-------------|----------------|
| **Manufacturing** | Work orders, routing, production | [manufacturing.md](modules/manufacturing/manufacturing.md) |
| **BOM** | Bill of Materials management | [bom.md](modules/manufacturing/bom.md) |
| **Quality Control** | Inspections, NCRs, checklists | [quality-control.md](modules/manufacturing/quality-control.md) |
| **Shop Floor** | QR codes, real-time data capture | [shop-floor.md](modules/manufacturing/shop-floor.md) |
| **Bank Integration** | Bank accounts, statements, MT940/CAMT.053/XML parsers, auto-reconciliation | [bank.md](modules/finance/bank.md) |
| **Cost Center** | Cost centers, profit centers, allocations, profitability reporting | [cost-center.md](modules/finance/cost-center.md) |
| **Subscription Billing** | Subscription plans, recurring subscriptions, billing cycles, subscription invoices | [subscription.md](modules/finance/subscription.md) |

---

## CRM

| Module | Description | Documentation |
|--------|-------------|----------------|
| **CRM** | Leads, opportunities, campaigns, tickets | [crm.md](modules/crm.md) |

---

## System Administration

| Module | Description | Documentation |
|--------|-------------|----------------|
| **Admin** | Tenants, users, feature flags, config | [admin.md](modules/system/admin.md) |
| **Workflow Engine** | Approval workflows, templates, instances, steps | [workflow.md](modules/system/workflow.md) |

---

## Infrastructure & Operations

| Module | Description | Documentation |
|--------|-------------|---------------|
| **Event Bus** | Outbox pattern, Redis Streams, DLQ, background processing | [event-bus.md](modules/infrastructure/event-bus.md) |
| **Redis Cache** | Repository-level caching with TTL and invalidation | [redis-cache.md](modules/infrastructure/redis-cache.md) |
| **File Storage** | S3/MinIO + local backends, presigned URLs | [file-storage.md](modules/infrastructure/file-storage.md) |
| **CDC** | PostgreSQL LISTEN/NOTIFY triggers, real-time streaming | [cdc.md](modules/infrastructure/cdc.md) |
| **Job Scheduler** | Background jobs with priority, retry, cron scheduling | [jobs.md](modules/infrastructure/jobs.md) |
| **Circuit Breaker** | Per-service circuit breaker registry with state monitoring | [resilience.md](modules/infrastructure/resilience.md) |
| **Retry** | Exponential backoff retry with jitter and statistics | [retry.md](modules/infrastructure/retry.md) |

---

## Analytics & Reporting

| Module | Description | Documentation |
|--------|-------------|---------------|
| **BI Dashboard** | KPI widgets, chart data, time-series aggregation | [dashboard.md](modules/analytics/dashboard.md) |
| **Import/Export** | Bulk CSV/JSON import with validation and background processing | [import-export.md](modules/analytics/import-export.md) |

---

## Architecture

### Multi-Tenant Flow

```
User Request → Subdomain Detection → Tenant Lookup → Database Routing → API Response
     ↓
  JWT Token → User Authentication → Role-Based Access
```

### Layered Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        API Layer                               │
│  REST  │  GraphQL  │  WebSocket  │  gRPC                      │
├─────────────────────────────────────────────────────────────┤
│                      Event Bus                                 │
│  Outbox  │  Redis Streams  │  DLQ  │  Background Workers      │
├─────────────────────────────────────────────────────────────┤
│                    Authentication (Auth)                       │
├─────────────────────────────────────────────────────────────┤
│  Users  │  Tenants  │  Feature Flags  │  Configuration       │
├─────────┴───────────┴──────────────────┴────────────────────┤
│                      Core Modules                              │
│  Cari  │  Products  │  Stock  │  Invoices                   │
├─────────┴───────────┴─────────┴─────────────────────────────┤
│                   Business Modules                             │
│  Sales  │  Purchase  │  HR  │  Accounting  │  Assets        │
├─────────┴───────────┴──────┴──────────────┴───────────────┤
│                   Extended Modules                             │
│  Projects  │  Manufacturing  │  BOM  │  QC  │  Shop Floor   │
├───────────┴─────────────────┴───────┴──────┴───────────────┤
│                         CRM                                    │
│     Leads  │  Opportunities  │  Campaigns  │  Tickets        │
├───────────┴─────────────────┴─────────────┴──────────────────┤
│                   Finance & Billing                            │
│  Bank  │  Cost Center  │  Subscription  │  Workflow           │
├───────────┴─────────────────┴─────────────┴──────────────────┤
│                   Analytics & Reporting                        │
│  BI Dashboard  │  Import/Export  │  KPIs  │  Time-Series     │
├─────────────────────────────────────────────────────────────┤
│                      Domain Layer                              │
│  Services  │  Repositories  │  Validators  │  Domain Events   │
├─────────────────────────────────────────────────────────────┤
│                      Cache Layer                               │
│  Redis  │  TTL  │  Invalidation  │  Distributed Locks       │
├─────────────────────────────────────────────────────────────┤
│                     Database Layer                               │
│  PostgreSQL  │  CDC  │  LISTEN/NOTIFY  │  Triggers          │
├─────────────────────────────────────────────────────────────┤
│                  Infrastructure Layer                          │
│  File Storage (S3/MinIO)  │  Job Scheduler  │  Email/SMS   │
├─────────────────────────────────────────────────────────────┤
│                   Resilience Layer                             │
│  Circuit Breaker  │  Retry  │  Health Checks  │  Metrics      │
└─────────────────────────────────────────────────────────────┘

Data Flow:
  API → Event Bus → Domain Services → Cache → Database
                          ↓                ↑
                    File Storage       CDC (DB → Event Bus)
```

---

## Quick Links

- [API Documentation](http://localhost:8000/docs)
- [ReDoc](http://localhost:8000/redoc)
- [README](../README.md)
- [Configuration Guide](../docs/configuration.md)
