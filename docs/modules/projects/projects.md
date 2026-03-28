# Projects Module

## Overview

The Projects module provides project management capabilities including client management, project tracking, and Work Breakdown Structure (WBS).

## Functionality

### Client Management
- Client database
- Client contact information
- Client-specific settings

### Project Management
- Project creation and tracking
- Project status workflow
- Budget tracking
- Timeline management
- Resource allocation

### Work Breakdown Structure (WBS)
- Hierarchical task structure
- Task assignments
- Progress tracking
- Dependencies

### Team Management
- Team member assignments
- Role assignments
- Time tracking

## API Endpoints

### Clients
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/projects/clients` | List clients |
| GET | `/api/projects/clients/{id}` | Get client |
| POST | `/api/projects/clients` | Create client |
| PUT | `/api/projects/clients/{id}` | Update client |
| DELETE | `/api/projects/clients/{id}` | Delete client |

### Projects
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/projects` | List projects |
| GET | `/api/projects/{id}` | Get project |
| POST | `/api/projects` | Create project |
| PUT | `/api/projects/{id}` | Update project |
| DELETE | `/api/projects/{id}` | Delete project |

### WBS
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/projects/{id}/wbs` | Get project WBS |
| POST | `/api/projects/{id}/wbs` | Add WBS item |
| PUT | `/api/projects/wbs/{id}` | Update WBS item |
| DELETE | `/api/projects/wbs/{id}` | Delete WBS item |

### Team
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/projects/{id}/team` | Get team members |
| POST | `/api/projects/{id}/team` | Add team member |
| DELETE | `/api/projects/{id}/team/{user_id}` | Remove member |

## Data Model

### Client
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| code | String | Client code |
| name | String | Company name |
| contact_name | String | Contact person |
| email | String | Email |
| phone | String | Phone |
| address | String | Address |

### Project
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| project_number | String | Project number |
| name | String | Project name |
| client_id | Integer | Client reference |
| status | Enum | planning, active, on_hold, completed, cancelled |
| start_date | Date | Start date |
| end_date | Date | End date |
| budget | Decimal | Budget |
| actual_cost | Decimal | Actual cost |

### WBSItem
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| project_id | Integer | Project reference |
| parent_id | Integer | Parent WBS item |
| name | String | Task name |
| code | String | WBS code |
| planned_hours | Decimal | Planned hours |
| actual_hours | Decimal | Actual hours |
| progress | Integer | Progress % |

## Example Usage

```bash
# Create client
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/projects/clients" \
  -d '{"code": "CLI001", "name": "ABC Corp", "contact_name": "John Smith"}'

# Create project
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/projects" \
  -d '{
    "name": "Website Redesign",
    "client_id": 1,
    "start_date": "2024-01-01",
    "end_date": "2024-06-30",
    "budget": 50000.00
  }'
```
