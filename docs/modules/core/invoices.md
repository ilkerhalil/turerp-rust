# Invoices Module

## Overview

The Invoices module manages customer invoices and payments. It supports both sales and purchase invoices with full accounting integration.

## Functionality

### Invoice Management
- Sales invoices (customer invoices)
- Purchase invoices (vendor invoices)
- Invoice line items
- Tax calculations
- Invoice status tracking
- Invoice printing/export

### Payment Processing
- Partial payments
- Full payments
- Payment methods (cash, credit card, bank transfer)
- Payment scheduling
- Payment reminders

### Invoice Workflow
- Draft → Posted → Paid
- Invoice cancellation
- Credit notes

## API Endpoints

### Invoices
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/invoices` | List invoices |
| GET | `/api/invoices/{id}` | Get invoice by ID |
| POST | `/api/invoices` | Create invoice |
| PUT | `/api/invoices/{id}` | Update invoice |
| DELETE | `/api/invoices/{id}` | Delete invoice |
| POST | `/api/invoices/{id}/post` | Post invoice |
| POST | `/api/invoices/{id}/cancel` | Cancel invoice |

### Payments
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/invoices/{id}/payments` | List invoice payments |
| POST | `/api/invoices/{id}/payments` | Record payment |
| GET | `/api/invoices/unpaid` | List unpaid invoices |

## Data Model

### Invoice
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| invoice_number | String | Unique invoice number |
| type | Enum | sales, purchase |
| cari_id | Integer | Cari account reference |
| date | Date | Invoice date |
| due_date | Date | Payment due date |
| status | Enum | draft, posted, paid, cancelled |
| subtotal | Decimal | Subtotal |
| tax_amount | Decimal | Tax amount |
| total | Decimal | Total amount |
| notes | String | Notes |

### InvoiceItem
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| invoice_id | Integer | Invoice reference |
| product_id | Integer | Product reference |
| description | String | Description |
| quantity | Decimal | Quantity |
| unit_price | Decimal | Unit price |
| tax_rate | Decimal | Tax rate |
| total | Decimal | Total |

### Payment
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| invoice_id | Integer | Invoice reference |
| date | Date | Payment date |
| amount | Decimal | Payment amount |
| method | Enum | cash, credit_card, bank_transfer |
| reference | String | Payment reference |

## Example Usage

```bash
# Create sales invoice
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/invoices" \
  -d '{
    "cari_id": 1,
    "date": "2024-01-15",
    "due_date": "2024-02-15",
    "items": [
      {
        "product_id": 1,
        "quantity": 5,
        "unit_price": 100.00,
        "tax_rate": 18
      }
    ]
  }'

# Record payment
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/invoices/1/payments" \
  -d '{
    "date": "2024-02-10",
    "amount": 590.00,
    "method": "bank_transfer",
    "reference": "TR123456789"
  }'
```
