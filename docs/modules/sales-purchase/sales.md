# Sales Module

## Overview

The Sales module manages sales orders and quotations. It provides complete sales workflow from quotation to delivery.

## Functionality

### Sales Orders
- Sales order creation with line items
- Order status workflow (draft → confirmed → shipped → delivered)
- Partial shipments
- Order pricing and discounts
- Customer-specific pricing

### Quotations
- Create quotations from sales orders
- Quotation validity period
- Quote to order conversion

### Order Management
- Order modifications
- Order cancellations
- Order fulfillment tracking

## API Endpoints

### Sales Orders
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/sales` | List sales orders |
| GET | `/api/sales/{id}` | Get sales order by ID |
| POST | `/api/sales` | Create sales order |
| PUT | `/api/sales/{id}` | Update sales order |
| DELETE | `/api/sales/{id}` | Delete sales order |
| POST | `/api/sales/{id}/confirm` | Confirm order |
| POST | `/api/sales/{id}/ship` | Mark as shipped |
| POST | `/api/sales/{id}/deliver` | Mark as delivered |
| POST | `/api/sales/{id}/cancel` | Cancel order |

## Data Model

### SalesOrder
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| order_number | String | Unique order number |
| cari_id | Integer | Customer reference |
| date | Date | Order date |
| delivery_date | Date | Expected delivery |
| status | Enum | draft, confirmed, shipped, delivered, cancelled |
| subtotal | Decimal | Subtotal |
| discount | Decimal | Discount amount |
| tax_amount | Decimal | Tax amount |
| total | Decimal | Total amount |
| notes | String | Notes |

### SalesOrderItem
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| order_id | Integer | Order reference |
| product_id | Integer | Product reference |
| quantity | Decimal | Quantity |
| unit_price | Decimal | Unit price |
| discount | Decimal | Discount % |
| tax_rate | Decimal | Tax rate |
| total | Decimal | Total |

## Example Usage

```bash
# Create sales order
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/sales" \
  -d '{
    "cari_id": 1,
    "date": "2024-01-15",
    "delivery_date": "2024-01-25",
    "items": [
      {
        "product_id": 1,
        "quantity": 10,
        "unit_price": 100.00,
        "tax_rate": 18
      }
    ]
  }'

# Confirm order
curl -X POST -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/sales/1/confirm"
```
