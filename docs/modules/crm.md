# CRM Module

## Overview

The CRM module provides customer relationship management including leads, contacts, opportunities, campaigns, and support tickets.

## Functionality

### Lead Management
- Lead capture and tracking
- Lead sources
- Lead status workflow
- Lead assignment
- Lead scoring

### Contact Management
- Contact database
- Contact information
- Activity history

### Opportunity Management
- Sales pipeline
- Stage tracking
- Probability and value
- Expected close date
- Win/loss analysis

### Campaign Management
- Campaign planning
- Campaign execution
- Target audience
- Campaign response tracking

### Support Tickets
- Ticket creation
- Ticket assignment
- Status tracking
- SLA management

## API Endpoints

### Leads
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/crm/leads` | List leads |
| GET | `/api/crm/leads/{id}` | Get lead |
| POST | `/api/crm/leads` | Create lead |
| PUT | `/api/crm/leads/{id}` | Update lead |
| POST | `/api/crm/leads/{id}/convert` | Convert to customer |

### Contacts
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/crm/contacts` | List contacts |
| POST | `/api/crm/contacts` | Create contact |

### Opportunities
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/crm/opportunities` | List opportunities |
| POST | `/api/crm/opportunities` | Create opportunity |
| PUT | `/api/crm/opportunities/{id}` | Update |
| POST | `/api/crm/opportunities/{id}/won` | Mark as won |
| POST | `/api/crm/opportunities/{id}/lost` | Mark as lost |

### Campaigns
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/crm/campaigns` | List campaigns |
| POST | `/api/crm/campaigns` | Create campaign |

### Tickets
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/crm/tickets` | List tickets |
| POST | `/api/crm/tickets` | Create ticket |
| PUT | `/api/crm/tickets/{id}` | Update |
| POST | `/api/crm/tickets/{id}/close` | Close ticket |

## Data Model

### Lead
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| first_name | String | First name |
| last_name | String | Last name |
| company | String | Company |
| email | String | Email |
| phone | String | Phone |
| source | String | Lead source |
| status | Enum | new, contacted, qualified, converted, lost |
| score | Integer | Lead score |

### CrmOpportunity
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| name | String | Opportunity name |
| lead_id | Integer | Lead reference |
| value | Decimal | Deal value |
| stage | Enum | prospect, proposal, negotiation, won, lost |
| probability | Integer | Win probability % |
| expected_close | Date | Expected close |

### SupportTicket
| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| ticket_number | String | Ticket number |
| subject | String | Subject |
| description | String | Description |
| priority | Enum | low, medium, high, urgent |
| status | Enum | open, in_progress, resolved, closed |
| assigned_to | Integer | Assigned user |

## Example Usage

```bash
# Create lead
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/crm/leads" \
  -d '{
    "first_name": "John",
    "last_name": "Doe",
    "company": "ABC Corp",
    "email": "john@abc.com",
    "source": "website"
  }'

# Create opportunity
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  "http://localhost:8000/api/crm/opportunities" \
  -d '{
    "name": "ABC Corp Deal",
    "lead_id": 1,
    "value": 50000.00,
    "stage": "proposal",
    "probability": 50
  }'
```
