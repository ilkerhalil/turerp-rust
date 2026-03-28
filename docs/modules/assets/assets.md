# Assets Module

## Overview

The Assets module provides fixed asset management with maintenance tracking, depreciation calculations, and QR code support.

## Functionality

### Asset Management
- Asset registration
- Asset categories
- Asset locations
- Asset tracking
- Asset photos/documents

### Depreciation
- Straight-line depreciation
- Declining balance depreciation
- Sum-of-years digits depreciation
- Depreciation schedules
- Depreciation reports

### Maintenance
- Preventive maintenance schedules
- Maintenance history
- Maintenance costs
- Downtime tracking
- Spare parts

### Asset Lifecycle
- Asset acquisition
- Asset transfer
- Asset disposal
- Asset revaluation

### QR Code Support
- QR code generation
- QR code scanning
- Mobile asset tracking

## API Endpoints

### Assets
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/assets` | List assets |
| GET | `/api/assets/{id}` | Get asset |
| POST | `/api/assets` | Create asset |
| PUT | `/api/assets/{id}` | Update asset |
| DELETE | `/api/assets/{id}` | Delete asset |
| POST | `/api/assets/{id}/transfer` | Transfer asset |
| POST | `/api/assets/{id}/dispose` | Dispose asset |

### Categories
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/assets/categories` | List categories |
| POST | `/api/assets/categories` | Create category |

### Maintenance
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/assets/{id}/maintenance` | Maintenance history |
| POST | `/api/assets/{id}/maintenance` | Create maintenance |
| GET | `/api/assets/maintenance/schedule` | Maintenance schedule |

### Depreciation
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/assets/{id}/depreciation` | Depreciation schedule |
| POST | `/api/assets/{id}/calculate-depreciation` | Calculate |

### QR Codes
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/assets/{id}/qr` | Generate QR code |

## Data Model

### Asset
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Asset code |
| name | String | Asset name |
| category_id | Integer | Category |
| purchase_date | Date | Acquisition date |
| purchase_price | Decimal | Cost |
| useful_life | Integer | Years |
| salvage_value | Decimal | Residual value |
| depreciation_method | Enum | straight_line, declining, sum_of_years |
| status | Enum | active, transferred, disposed |
| location_id | Integer | Location |

### AssetMaintenance
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| asset_id | Integer | Asset reference |
| type | Enum | preventive, corrective |
| description | String | Description |
| scheduled_date | Date | Scheduled date |
| completed_date | Date | Completed date |
| cost | Decimal | Cost |

## Example Usage

```bash
# Create asset
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/assets" \
  -d '{
    "code": "AST-001",
    "name": "Company Vehicle",
    "category_id": 1,
    "purchase_date": "2024-01-01",
    "purchase_price": 200000.00,
    "useful_life": 5,
    "salvage_value": 50000.00,
    "depreciation_method": "straight_line"
  }'

# Schedule maintenance
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/assets/1/maintenance" \
  -d '{
    "type": "preventive",
    "description": "Annual service",
    "scheduled_date": "2024-06-01"
  }'
```
