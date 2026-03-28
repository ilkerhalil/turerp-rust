# Purchase Module

## Overview

The Purchase module manages the complete procurement workflow from purchase requests to vendor invoices.

## Functionality

### Purchase Requests
- Create purchase requests from departments
- Request approval workflow
- Convert to purchase orders
- Request tracking

### Purchase Orders
- PO creation with line items
- Vendor selection
- Delivery scheduling
- Partial receipts
- Order status tracking

### Goods Receipt
- Receive goods against PO
- Quality inspection integration
- Warehouse routing
- Receipt to invoice matching

### Purchase Returns
- Return defective goods
- Return authorization
- Credit note processing

### Purchase Invoices
- Match receipt to invoice
- Invoice approval workflow
- Payment processing

## API Endpoints

### Purchase Requests
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/purchase/requests` | List purchase requests |
| GET | `/api/purchase/requests/{id}` | Get request by ID |
| POST | `/api/purchase/requests` | Create request |
| PUT | `/api/purchase/requests/{id}` | Update request |
| POST | `/api/purchase/requests/{id}/approve` | Approve request |
| POST | `/api/purchase/requests/{id}/convert` | Convert to PO |

### Purchase Orders
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/purchase/orders` | List purchase orders |
| GET | `/api/purchase/orders/{id}` | Get order by ID |
| POST | `/api/purchase/orders` | Create order |
| PUT | `/api/purchase/orders/{id}` | Update order |
| POST | `/api/purchase/orders/{id}/receive` | Receive goods |

### Receipts
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/purchase/receipts` | List receipts |
| POST | `/api/purchase/receipts` | Create receipt |

### Returns
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/purchase/returns` | List returns |
| POST | `/api/purchase/returns` | Create return |

## Data Model

### PurchaseRequest
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| request_number | String | Request number |
| requested_by | Integer | User ID |
| department_id | Integer | Department |
| status | Enum | draft, pending, approved, rejected |
| priority | Enum | low, normal, high, urgent |
| expected_date | Date | Expected delivery |
| notes | String | Notes |

### PurchaseOrder
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| order_number | String | PO number |
| vendor_id | Integer | Vendor cari account |
| status | Enum | draft, confirmed, partial, received, cancelled |
| expected_date | Date | Expected delivery |
| total | Decimal | Total amount |

## Example Usage

```bash
# Create purchase request
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/purchase/requests" \
  -d '{
    "department_id": 1,
    "priority": "normal",
    "expected_date": "2024-02-01",
    "items": [
      {"product_id": 1, "quantity": 50}
    ]
  }'

# Create purchase order
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/purchase/orders" \
  -d '{
    "vendor_id": 1,
    "expected_date": "2024-02-15",
    "items": [
      {"product_id": 1, "quantity": 50, "unit_price": 25.00}
    ]
  }'
```
