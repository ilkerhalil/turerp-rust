# Cari (Customer/Vendor) Accounts Module

## Overview

The Cari module manages customer and vendor accounts (Cari Hesaplar) - a fundamental concept in Turkish ERP systems. It provides CRUD operations for managing business relationships with customers and vendors.

## Functionality

### Account Management
- Create, read, update, delete cari accounts
- Filter by account type (customer/vendor)
- Filter by active status
- Search by account code or name

### Account Types
- **Customer** (`customer`): Sales customers
- **Vendor** (`vendor`): Suppliers/vendors
- **Both** (`both`): Customers who are also vendors

### Sales Order Integration
- Link sales orders to cari accounts
- Track customer orders

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/cari` | List all cari accounts |
| GET | `/api/cari/{id}` | Get cari account by ID |
| GET | `/api/cari/code/{code}` | Get cari account by code |
| POST | `/api/cari` | Create new cari account |
| PUT | `/api/cari/{id}` | Update cari account |
| DELETE | `/api/cari/{id}` | Delete cari account |
| GET | `/api/cari/{id}/orders` | Get customer's sales orders |

## Data Model

### CariAccount
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Unique account code |
| name | String | Account name |
| type | Enum | customer, vendor, both |
| tax_number | String | Tax identification number |
| tax_office | String | Tax office |
| address | String | Address |
| phone | String | Phone number |
| email | String | Email address |
| is_active | Boolean | Active status |
| credit_limit | Decimal | Credit limit |
| balance | Decimal | Current balance |

## Example Usage

```bash
# List customers
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/cari?type=customer"

# Create vendor
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/cari" \
  -d '{
    "code": "V001",
    "name": "ABC Suppliers",
    "type": "vendor",
    "tax_number": "1234567890",
    "email": "sales@abc-suppliers.com"
  }'
```
