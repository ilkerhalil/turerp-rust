# Assets Module

## Overview

The Assets module provides fixed asset management with maintenance tracking, depreciation calculations, and disposal handling.

## Functionality

### Asset Management
- Asset categories
- Asset CRUD operations
- Asset location tracking
- Asset documents and images

### Depreciation
- Straight-line depreciation
- Declining balance
- Sum-of-years digits
- Depreciation schedules
- Asset revaluation

### Maintenance
- Preventive maintenance schedules
- Corrective maintenance
- Maintenance history
- Cost tracking

### Asset Lifecycle
- Asset acquisition
- Asset transfer
- Asset disposal
- Asset retirement

## API Endpoints

### Asset Categories
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/assets/categories` | List categories |
| POST | `/api/assets/categories` | Create |
| PUT | `/api/assets/categories/{id}` | Update |

### Assets
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/assets` | List assets |
| GET | `/api/assets/{id}` | Get asset |
| POST | `/api/assets` | Create asset |
| PUT | `/api/assets/{id}` | Update |
| DELETE | `/api/assets/{id}` | Delete |
| GET | `/api/assets/{id}/depreciation` | Depreciation schedule |

### Maintenance
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/assets/{id}/maintenance` | Maintenance history |
| POST | `/api/assets/maintenance` | Create maintenance |
| POST | `/api/assets/{id}/transfer` | Transfer asset |

### Disposal
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/assets/{id}/dispose` | Dispose asset |
| POST | `/api/assets/{id}/revalue` | Revalue asset |

## Data Model

### Asset
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| asset_number | String | Asset number |
| name | String | Asset name |
| category_id | Integer | Category |
| purchase_date | Date | Acquisition date |
| purchase_price | Decimal | Purchase price |
| salvage_value | Decimal | Salvage value |
| useful_life | Integer | Years |
| depreciation_method | Enum | straight_line, declining, sum_of_years |
| status | Enum | active, disposed, transferred |
| location_id | Integer | Location |

### AssetCategory
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Category code |
| name | String | Name |
| default_life | Integer | Default useful life |

### AssetMaintenance
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| asset_id | Integer | Asset reference |
| type | Enum | preventive, corrective |
| description | String | Description |
| scheduled_date | Date | Scheduled date |
| cost | Decimal | Cost |
| status | Enum | scheduled, completed |

## Example Usage

```bash
# Create asset category
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/assets/categories" \
  -d '{"code": "VEH", "name": "Vehicles", "default_life": 5}'

# Create asset
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/assets" \
  -d '{
    "asset_number": "AST001",
    "name": "Company Car",
    "category_id": 1,
    "purchase_date": "2024-01-01",
    "purchase_price": 300000.00,
    "salvage_value": 50000.00,
    "useful_life": 5,
    "depreciation_method": "straight_line"
  }'

# Get depreciation
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/assets/1/depreciation"
```
