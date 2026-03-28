# Manufacturing Module

## Overview

The Manufacturing module provides production management capabilities including work orders, routing, quality inspections, and capacity planning.

## Functionality

### Work Orders
- Work order creation from sales orders or standalone
- Bill of Materials (BOM) integration
- Production routing (operation sequences)
- Status tracking (draft, released, in_progress, completed)
- Scrap and yield tracking

### Routing/Operations
- Operation definitions
- Work center assignment
- Standard times (setup, run)
- Sequence management
- Parallel/sequential operations

### Quality Inspections
- Incoming inspection (raw materials)
- In-process inspection
- Final inspection
- Measurement recording
- Pass/fail criteria
- Non-conformance reporting

### Work Centers
- Work center capacity
- Efficiency tracking
- Availability calendar
- Shift scheduling

### Production Scheduling
- Forward/backward scheduling
- Capacity load analysis
- Gantt chart support

## API Endpoints

### Work Orders
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/manufacturing/work-orders` | List work orders |
| GET | `/api/manufacturing/work-orders/{id}` | Get work order |
| POST | `/api/manufacturing/work-orders` | Create work order |
| PUT | `/api/manufacturing/work-orders/{id}` | Update |
| POST | `/api/manufacturing/work-orders/{id}/release` | Release |
| POST | `/api/manufacturing/work-orders/{id}/start` | Start |
| POST | `/api/manufacturing/work-orders/{id}/complete` | Complete |

### Operations
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/manufacturing/work-orders/{id}/operations` | List operations |
| POST | `/api/manufacturing/work-orders/{id}/operations` | Add operation |

### Work Centers
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/manufacturing/work-centers` | List work centers |
| POST | `/api/manufacturing/work-centers` | Create |
| PUT | `/api/manufacturing/work-centers/{id}` | Update |

### Inspections
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/manufacturing/inspections` | List inspections |
| POST | `/api/manufacturing/inspections` | Create inspection |
| POST | `/api/manufacturing/inspections/{id}/approve` | Approve |

## Data Model

### WorkOrder
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| order_number | String | Work order number |
| product_id | Integer | Product to produce |
| quantity | Decimal | Production quantity |
| bom_id | Integer | BOM reference |
| status | Enum | draft, released, in_progress, completed, cancelled |
| start_date | Date | Planned start |
| end_date | Date | Planned end |
| actual_start | Date | Actual start |
| actual_end | Date | Actual end |

### WorkOrderOperation
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| work_order_id | Integer | Work order reference |
| sequence | Integer | Operation sequence |
| work_center_id | Integer | Work center |
| operation_name | String | Operation name |
| setup_time | Decimal | Setup time (min) |
| run_time | Decimal | Run time per unit |

### WorkCenter
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Work center code |
| name | String | Name |
| capacity | Decimal | Daily capacity |
| efficiency | Decimal | Efficiency % |

## Example Usage

```bash
# Create work order
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/manufacturing/work-orders" \
  -d '{
    "product_id": 1,
    "quantity": 100,
    "bom_id": 1,
    "start_date": "2024-02-01",
    "end_date": "2024-02-15"
  }'

# Release work order
curl -X POST -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8000/api/manufacturing/work-orders/1/release"
```
