# Shop Floor Module

## Overview

The Shop Floor module provides real-time shop floor integration with QR code support and data capture capabilities.

## Functionality

### QR Code Integration
- QR code generation for work orders
- Barcode support
- Mobile-friendly scanning
- Work order identification

### Real-Time Data Capture
- Progress updates
- Material consumption
- Operation completion
- Scrap recording
- Time tracking

### Material Issues
- Raw material issuance
- Material consumption tracking
- Return of unused materials

### Floor Data Collection
- Operation start/end times
- Operator assignments
- Machine logging
- Production counting

## API Endpoints

### QR Codes
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/shop-floor/qr/{work_order_id}` | Generate QR code |
| POST | `/api/shop-floor/scan` | Scan QR code |

### Progress
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/shop-floor/progress` | Update progress |
| GET | `/api/shop-floor/work-orders/active` | Active work orders |

### Material Issues
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/shop-floor/material-issue` | Issue materials |
| POST | `/api/shop-floor/material-return` | Return materials |

## Data Model

### ShopFloorData
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| work_order_id | Integer | Work order reference |
| operation_id | Integer | Operation reference |
| operator_id | Integer | Operator |
| start_time | DateTime | Operation start |
| end_time | DateTime | Operation end |
| quantity_produced | Decimal | Quantity produced |
| scrap_quantity | Decimal | Scrap quantity |

### MaterialIssue
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| work_order_id | Integer | Work order reference |
| material_id | Integer | Material reference |
| quantity | Decimal | Issued quantity |
| issued_by | Integer | Issuer |
| issue_date | DateTime | Issue date |

## Example Usage

```bash
# Generate QR code for work order
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/shop-floor/qr/1"

# Update progress
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/shop-floor/progress" \
  -d '{
    "work_order_id": 1,
    "operation_id": 1,
    "quantity_produced": 50,
    "scrap_quantity": 2
  }'

# Issue materials
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/shop-floor/material-issue" \
  -d '{
    "work_order_id": 1,
    "material_id": 1,
    "quantity": 100
  }'
```
