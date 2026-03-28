# Project Finance Module

## Overview

The Project Finance module provides financial management for projects including cost tracking, budgeting, and profitability analysis.

## Functionality

### Cost Tracking
- Project cost recording
- Cost categories
- Labor costs
- Material costs
- Overhead allocation

### Budget Management
- Initial budget setting
- Budget revisions
- Budget vs actual comparison
- Variance analysis

### Profitability Analysis
- Revenue tracking
- Cost analysis
- Profit margin calculation
- Earned Value Analysis (EVA)

### Cash Flow
- Project cash flow
- Payment tracking
- Invoice management

## API Endpoints

### Costs
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/project-finance/costs` | List costs |
| POST | `/api/project-finance/costs` | Record cost |

### Budget
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/project-finance/budgets` | List budgets |
| POST | `/api/project-finance/budgets` | Create budget |
| POST | `/api/project-finance/budgets/{id}/revise` | Revise budget |

### Reports
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/project-finance/profitability` | Profitability report |
| GET | `/api/project-finance/cashflow` | Cash flow report |
| GET | `/api/project-finance/earned-value` | EVA report |

## Data Model

### ProjectCost
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| project_id | Integer | Project reference |
| date | Date | Cost date |
| type | Enum | labor, material, other |
| description | String | Description |
| amount | Decimal | Amount |

### ProjectBudget
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| project_id | Integer | Project reference |
| version | Integer | Budget version |
| total_budget | Decimal | Total budget |
| status | Enum | draft, approved |

## Example Usage

```bash
# Record cost
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/project-finance/costs" \
  -d '{
    "project_id": 1,
    "date": "2024-01-15",
    "type": "material",
    "description": "Steel purchase",
    "amount": 5000.00
  }'

# Get profitability report
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/project-finance/profitability?project_id=1"
```
