# Quality Control Module

## Overview

The Quality Control module provides quality management capabilities including inspections, checklists, and non-conformance reporting.

## Functionality

### Non-Conformance Reports (NCR)
- NCR creation and tracking
- Severity levels (critical, major, minor)
- Root cause analysis
- Corrective actions
- NCR closure workflow

### Quality Checklists
- Checklist templates
- Checklist items
- Pass/fail criteria
- Checklist execution

### Quality Inspections
- Incoming inspection (IQC)
- In-process inspection (IPQC)
- Final inspection (FQC)
- Inspection sampling
- Measurement recording
- Results approval

## API Endpoints

### Non-Conformance Reports
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/quality-control/ncr` | List NCRs |
| GET | `/api/quality-control/ncr/{id}` | Get NCR |
| POST | `/api/quality-control/ncr` | Create NCR |
| PUT | `/api/quality-control/ncr/{id}` | Update NCR |
| POST | `/api/quality-control/ncr/{id}/close` | Close NCR |

### Checklists
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/quality-control/checklists` | List checklists |
| POST | `/api/quality-control/checklists` | Create |
| POST | `/api/quality-control/checklists/{id}/execute` | Execute |

### Inspections
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/quality-control/inspections` | List inspections |
| POST | `/api/quality-control/inspections` | Create inspection |
| POST | `/api/quality-control/inspections/{id}/approve` | Approve |

## Data Model

### NonConformanceReport
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| ncr_number | String | NCR number |
| type | Enum | supplier, internal, customer |
| severity | Enum | critical, major, minor |
| description | String | Description |
| status | Enum | open, in_progress, closed |
| root_cause | String | Root cause |
| corrective_action | String | Corrective action |

### QualityChecklist
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| name | String | Checklist name |
| type | Enum | incoming, in_process, final |
| is_active | Boolean | Active status |

### QualityInspection
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| inspection_number | String | Number |
| type | Enum | incoming, in_process, final |
| reference_type | String | Reference type |
| reference_id | Integer | Reference ID |
| status | Enum | pending, approved, rejected |
| inspector_id | Integer | Inspector |
| result | Enum | pass, fail, pending |

## Example Usage

```bash
# Create NCR
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/quality-control/ncr" \
  -d '{
    "type": "supplier",
    "severity": "major",
    "description": "Defective material received"
  }'

# Create inspection
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/quality-control/inspections" \
  -d '{
    "type": "incoming",
    "reference_type": "purchase_order",
    "reference_id": 101
  }'
```
