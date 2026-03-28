# Products Module

## Overview

The Products module manages product catalogs and categories. It provides comprehensive product management including inventory tracking, pricing, and categorization.

## Functionality

### Category Management
- Hierarchical category structure (parent-child)
- Category CRUD operations
- Active/inactive status

### Product Management
- Product CRUD operations
- Multiple product types (goods, service, composite)
- Barcode support
- Unit of measure management
- Price lists (purchase/sales prices)
- Stock tracking integration
- Image support

## API Endpoints

### Categories
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/products/categories` | List all categories |
| GET | `/api/products/categories/{id}` | Get category by ID |
| POST | `/api/products/categories` | Create category |
| PUT | `/api/products/categories/{id}` | Update category |
| DELETE | `/api/products/categories/{id}` | Delete category |

### Products
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/products` | List all products |
| GET | `/api/products/{id}` | Get product by ID |
| GET | `/api/products/barcode/{barcode}` | Get product by barcode |
| POST | `/api/products` | Create product |
| PUT | `/api/products/{id}` | Update product |
| DELETE | `/api/products/{id}` | Delete product |

## Data Model

### Category
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Unique category code |
| name | String | Category name |
| parent_id | Integer | Parent category ID |
| description | String | Description |
| is_active | Boolean | Active status |

### Product
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Unique product code |
| name | String | Product name |
| category_id | Integer | Category reference |
| type | Enum | goods, service, composite |
| barcode | String | Barcode |
| unit | String | Unit of measure |
| purchase_price | Decimal | Purchase price |
| sales_price | Decimal | Sales price |
| min_stock | Decimal | Minimum stock level |
| max_stock | Decimal | Maximum stock level |
| is_active | Boolean | Active status |

## Example Usage

```bash
# Create category
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/products/categories" \
  -d '{
    "code": "ELECTRONICS",
    "name": "Electronics"
  }'

# Create product
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/products" \
  -d '{
    "code": "PROD-001",
    "name": "Laptop Computer",
    "category_id": 1,
    "type": "goods",
    "purchase_price": 1000.00,
    "sales_price": 1500.00,
    "unit": "pc"
  }'
```
