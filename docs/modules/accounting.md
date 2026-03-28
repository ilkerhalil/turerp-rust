# Accounting Module

## Overview

The Accounting module provides comprehensive financial management including chart of accounts, journal entries, general ledger, and financial reports.

## Functionality

### Chart of Accounts
- Hierarchical account structure
- Account types: Asset, Liability, Equity, Income, Expense
- Account groups and subgroups
- Active/inactive accounts

### Journal Entries
- Double-entry bookkeeping
- Manual journal entries
- Automated entries from transactions
- Entry templates
- Trial balance validation

### General Ledger
- Real-time ledger queries
- Account balance tracking
- Transaction history
- Date range filtering

### Cost Centers
- Cost center management
- Cost allocation
- Cost tracking by department/project

### Financial Reports
- Trial Balance
- Profit & Loss (Income Statement)
- Balance Sheet
- Account statements

## API Endpoints

### Accounts
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/accounting/accounts` | List accounts |
| GET | `/api/accounting/accounts/{id}` | Get account |
| POST | `/api/accounting/accounts` | Create account |
| PUT | `/api/accounting/accounts/{id}` | Update account |
| DELETE | `/api/accounting/accounts/{id}` | Delete account |
| GET | `/api/accounting/accounts/tree` | Account tree |

### Journal Entries
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/accounting/journal` | List journal entries |
| GET | `/api/accounting/journal/{id}` | Get entry |
| POST | `/api/accounting/journal` | Create entry |
| PUT | `/api/accounting/journal/{id}` | Update entry |
| DELETE | `/api/accounting/journal/{id}` | Delete entry |

### Cost Centers
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/accounting/cost-centers` | List cost centers |
| POST | `/api/accounting/cost-centers` | Create cost center |
| PUT | `/api/accounting/cost-centers/{id}` | Update |
| DELETE | `/api/accounting/cost-centers/{id}` | Delete |

### Reports
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/accounting/reports/trial-balance` | Trial Balance |
| GET | `/api/accounting/reports/profit-loss` | P&L Report |
| GET | `/api/accounting/reports/balance-sheet` | Balance Sheet |
| GET | `/api/accounting/reports/general-ledger` | General Ledger |

## Data Model

### Account
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Account code |
| name | String | Account name |
| type | Enum | asset, liability, equity, income, expense |
| parent_id | Integer | Parent account |
| level | Integer | Account level |
| is_active | Boolean | Active status |
| balance | Decimal | Current balance |

### JournalEntry
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| entry_number | String | Entry number |
| date | Date | Entry date |
| description | String | Description |
| status | Enum | draft, posted |
| total_debit | Decimal | Total debit |
| total_credit | Decimal | Total credit |

### CostCenter
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Cost center code |
| name | String | Name |
| type | Enum | department, project, product |

## Example Usage

```bash
# Create account
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/accounting/accounts" \
  -d '{
    "code": "120",
    "name": "Accounts Receivable",
    "type": "asset"
  }'

# Create journal entry
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/accounting/journal" \
  -d '{
    "date": "2024-01-15",
    "description": "Sales invoice #1001",
    "lines": [
      {"account_id": 120, "debit": 1180.00, "credit": 0},
      {"account_id": 600, "debit": 0, "credit": 1000.00},
      {"account_id": 391, "debit": 0, "credit": 180.00}
    ]
  }'
```
