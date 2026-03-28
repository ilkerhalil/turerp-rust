# Stock Management Module

## Overview

The Stock module manages warehouse inventory and stock movements. It provides real-time inventory tracking with support for multiple warehouses.

## Functionality

### Warehouse Management
- Multiple warehouse support
- Warehouse CRUD operations
- Warehouse address and contact info

### Stock Operations
- Stock in (receipts from vendors)
- Stock out (issues to production/sales)
- Stock transfer between warehouses
- Stock adjustment (count corrections)
- Serial number/lot tracking

### Stock Reporting
- Current stock levels
- Stock movement history
- Stock valuation
- Low stock alerts

## API Endpoints

### Warehouses
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/stock/warehouses` | List warehouses |
| GET | `/api/stock/warehouses/{id}` | Get warehouse by ID |
| POST | `/api/stock/warehouses` | Create warehouse |
| PUT | `/api/stock/warehouses/{id}` | Update warehouse |
| DELETE | `/api/stock/warehouses/{id}` | Delete warehouse |

### Stock Movements
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/stock/movements` | List stock movements |
| GET | `/api/stock/movements/{id}` | Get movement by ID |
| POST | `/api/stock/movements` | Create stock movement |
| GET | `/api/stock/summary` | Get stock summary |

### Stock Balance
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/stock/balance` | Get stock balance |
| GET | `/api/stock/balance/{product_id}` | Get balance for product |

## Data Model

### Warehouse
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Unique warehouse code |
| name | String | Warehouse name |
| address | String | Address |
| is_active | Boolean | Active status |

### StockMovement
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| product_id | Integer | Product reference |
| warehouse_id | Integer | Warehouse reference |
| type | Enum | in, out, transfer, adjustment |
| quantity | Decimal | Movement quantity |
| reference_type | String | Source document type |
| reference_id | Integer | Source document ID |
| notes | String | Notes |

## Example Usage

```bash
# Create warehouse
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/stock/warehouses" \
  -d '{
    "code": "WH-001",
    "name": "Main Warehouse",
    "address": "Istanbul, Turkey"
  }'

# Stock in (receipt)
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/stock/movements" \
  -d '{
    "product_id": 1,
    "warehouse_id": 1,
    "type": "in",
    "quantity": 100,
    "reference_type": "purchase_order",
    "reference_id": 101
  }'

# Get stock summary
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/stock/summary"
```
