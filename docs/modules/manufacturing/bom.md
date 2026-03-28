# Bill of Materials (BOM) Module

## Overview

The BOM module manages product recipes and material structures. It provides comprehensive Bill of Materials management for manufacturing.

## Functionality

### Material Management
- Raw materials
- Components
- Sub-assemblies
- Finished goods
- Material vendors

### BOM Management
- Multiple BOM versions
- BOM revisions with history
- Effective dates
- BOM status (draft, active, obsolete)

### BOM Structure
- Multi-level BOMs
- Component quantities
- Scrap percentages
- Optional components
- Phantom BOMs

### Cost Analysis
- Material cost rollup
- Labor cost calculation
- Overhead allocation
- Total cost analysis

### Import/Export
- BOM import from Excel/CSV
- BOM export

## API Endpoints

### Materials
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/bom/materials` | List materials |
| GET | `/api/bom/materials/{id}` | Get material |
| POST | `/api/bom/materials` | Create material |
| PUT | `/api/bom/materials/{id}` | Update material |
| DELETE | `/api/bom/materials/{id}` | Delete material |

### BOM Headers
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/bom` | List BOMs |
| GET | `/api/bom/{id}` | Get BOM |
| POST | `/api/bom` | Create BOM |
| PUT | `/api/bom/{id}` | Update BOM |
| DELETE | `/api/bom/{id}` | Delete BOM |
| POST | `/api/bom/{id}/activate` | Activate BOM |
| GET | `/api/bom/{id}/cost` | Get BOM cost |

### BOM Items
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/bom/{id}/items` | List BOM items |
| POST | `/api/bom/{id}/items` | Add item |
| PUT | `/api/bom/items/{id}` | Update item |
| DELETE | `/api/bom/items/{id}` | Delete item |

## Data Model

### Material
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Material code |
| name | String | Material name |
| type | Enum | raw_material, component, sub_assembly, finished_good |
| unit | String | Unit of measure |
| is_active | Boolean | Active status |

### BOMHeader
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| product_id | Integer | Finished product |
| version | String | BOM version |
| status | Enum | draft, active, obsolete |
| effective_date | Date | Effective from |
| is_active | Boolean | Active status |

### BOMItem
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| bom_id | Integer | BOM reference |
| material_id | Integer | Material reference |
| quantity | Decimal | Quantity needed |
| scrap_percent | Decimal | Scrap % |
| sequence | Integer | Line number |

## Example Usage

```bash
# Create material
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/bom/materials" \
  -d '{
    "code": "RAW-001",
    "name": "Steel Sheet",
    "type": "raw_material",
    "unit": "kg"
  }'

# Create BOM
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/bom" \
  -d '{
    "product_id": 1,
    "version": "1.0",
    "effective_date": "2024-01-01"
  }'

# Get BOM cost
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/bom/1/cost"
```
