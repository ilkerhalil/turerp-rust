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

---

## Architecture

### Multi-Tenant Flow

```
User Request → Subdomain Detection → Tenant Lookup → Database Routing → API Response
     ↓
  JWT Token → User Authentication → Role-Based Access
```

### Module Dependencies

```
┌─────────────────────────────────────────────────────────────┐
│                    Authentication (Auth)                       │
├─────────────────────────────────────────────────────────────┤
│  Users  │  Tenants  │  Feature Flags  │  Configuration     │
├─────────┴───────────┴──────────────────┴──────────────────┤
│                      Core Modules                             │
│  Cari  │  Products  │  Stock  │  Invoices                  │
├─────────┴───────────┴─────────┴───────────────────────────┤
│                   Business Modules                            │
│  Sales  │  Purchase  │  HR  │  Accounting  │  Assets       │
├─────────┴───────────┴──────┴──────────────┴──────────────┤
│                   Extended Modules                             │
│  Projects  │  Manufacturing  │  BOM  │  QC  │  Shop Floor  │
├───────────┴─────────────────┴───────┴──────┴──────────────┤
│                         CRM                                   │
│     Leads  │  Opportunities  │  Campaigns  │  Tickets       │
└─────────────────────────────────────────────────────────────┘
```

---

## Quick Links

- [API Documentation](http://localhost:8000/docs)
- [ReDoc](http://localhost:8000/redoc)
- [README](../README.md)
- [Configuration Guide](../docs/configuration.md)
