# Quality Control Module

## Overview

The Quality Control module provides quality management capabilities including non-conformance reporting, checklists, and inspections.

## Functionality

### Non-Conformance Reports (NCR)
- NCR creation and tracking
- Root cause analysis
- Corrective action planning
- NCR closure workflow
- Cost tracking

### Quality Checklists
- Checklist templates
- Checklist items
- Pass/fail criteria
- Checklist execution

### Quality Inspections
- Inspection templates
- Measurement recording
- Tolerances
- Inspection results
- Approval workflow

## API Endpoints

### Non-Conformance Reports
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/quality/ncr` | List NCRs |
| GET | `/api/quality/ncr/{id}` | Get NCR |
| POST | `/api/quality/ncr` | Create NCR |
| PUT | `/api/quality/ncr/{id}` | Update NCR |
| POST | `/api/quality/ncr/{id}/close` | Close NCR |

### Checklists
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/quality/checklists` | List checklists |
| POST | `/api/quality/checklists` | Create checklist |
| POST | `/api/quality/checklists/{id}/execute` | Execute |

### Inspections
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/quality/inspections` | List inspections |
| POST | `/api/quality/inspections` | Create inspection |
| POST | `/api/quality/inspections/{id}/approve` | Approve |

## Data Model

### NonConformanceReport
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| ncr_number | String | NCR number |
| type | Enum | supplier, internal, customer |
| description | String | Description |
| root_cause | String | Root cause |
| corrective_action | String | Corrective action |
| status | Enum | open, in_progress, closed |
| cost | Decimal | Related cost |

## Example Usage

```bash
# Create NCR
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/quality/ncr" \
  -d '{
    "type": "supplier",
    "description": "Defective raw materials received",
    "product_id": 1
  }'
```
