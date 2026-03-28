# Tenant Module

## Overview

The Tenant module provides multi-tenant support for the ERP system. Each tenant represents a separate organization with isolated data.

## Functionality

### Tenant Management
- Create new tenants
- Update tenant details
- List all tenants
- Get tenant by ID
- Get tenant by subdomain
- Delete tenant

### Tenant Isolation
- Database-per-tenant architecture
- Subdomain-based tenant identification
- Tenant-scoped data access

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/tenants` | List all tenants |
| POST | `/api/tenants` | Create new tenant |
| GET | `/api/tenants/{id}` | Get tenant by ID |
| PUT | `/api/tenants/{id}` | Update tenant |
| DELETE | `/api/tenants/{id}` | Delete tenant |
| GET | `/api/tenants/subdomain/{subdomain}` | Get tenant by subdomain |

## Data Model

### Tenant
| Field | Type | Description |
|-------|------|-------------|
| id | i64 | Primary key |
| name | String | Company name |
| subdomain | String | Unique subdomain (e.g., "acme") |
| db_name | String | Database name (e.g., "turerp_acme") |
| is_active | bool | Active status |
| created_at | DateTime | Creation date |

### CreateTenant
```rust
pub struct CreateTenant {
    pub name: String,
    pub subdomain: String,
}
```

### UpdateTenant
```rust
pub struct UpdateTenant {
    pub name: Option<String>,
    pub subdomain: Option<String>,
    pub is_active: Option<bool>,
}
```

## Subdomain Validation

### Rules
- Must be lowercase alphanumeric
- Can contain hyphens (-)
- Must be 1-63 characters
- Must be unique

### Examples
```
Valid:    "acme", "company-name", "test123"
Invalid:  "ACME", "Company", "company name"
```

### Database Name Generation

```rust
fn generate_db_name(subdomain: &str) -> String {
    format!("turerp_{}", subdomain.to_lowercase().replace('-', "_"))
}

// Examples:
// "acme" -> "turerp_acme"
// "company-name" -> "turerp_company_name"
// "Test123" -> "turerp_test123"
```

## Example Usage

### Create Tenant
```bash
curl -X POST -H "Content-Type: application/json" \
  "http://localhost:8080/api/tenants" \
  -d '{
    "name": "Acme Corporation",
    "subdomain": "acme"
  }'
```

### Response
```json
{
  "id": 1,
  "name": "Acme Corporation",
  "subdomain": "acme",
  "db_name": "turerp_acme",
  "is_active": true,
  "created_at": "2024-01-15T10:30:00Z"
}
```

### Get Tenant by Subdomain
```bash
curl "http://localhost:8080/api/tenants/subdomain/acme"
```

### Update Tenant
```bash
curl -X PUT -H "Content-Type: application/json" \
  "http://localhost:8080/api/tenants/1" \
  -d '{
    "name": "Acme Inc.",
    "is_active": true
  }'
```

### Delete Tenant
```bash
curl -X DELETE "http://localhost:8080/api/tenants/1"
```

## Implementation Status

✅ **Complete** - Core features implemented
- ✅ Tenant CRUD operations
- ✅ Subdomain validation
- ✅ Database name generation
- ✅ Duplicate subdomain prevention
- ⚠️ Database-per-tenant routing (planned)