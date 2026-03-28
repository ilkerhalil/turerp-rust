# System Administration Modules

This document covers the system administration modules for multi-tenant ERP management.

## Overview

The system includes several administrative modules for managing tenants, users, feature flags, and system configuration.

---

## Tenants Module

### Overview

The Tenants module manages multi-tenant architecture - each tenant gets isolated database and subdomain routing.

### Functionality

- Tenant CRUD operations
- Subdomain management
- Database name mapping
- Tenant activation/deactivation

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/admin/tenants` | List all tenants |
| GET | `/api/admin/tenants/{id}` | Get tenant details |
| POST | `/api/admin/tenants` | Create new tenant |
| PUT | `/api/admin/tenants/{id}` | Update tenant |
| DELETE | `/api/admin/tenants/{id}` | Delete tenant |

### Data Model

| Field | Type | Description |
|-------|------|-------------|
| id | Integer | Primary key |
| subdomain | String | Subdomain (e.g., "demo") |
| name | String | Tenant name |
| db_name | String | Database name |
| is_active | Boolean | Active status |

---

## Users Module

### Overview

The Users module manages user accounts within each tenant.

### Functionality

- User CRUD operations
- Role assignment
- Current user profile

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/users` | List users (admin) |
| GET | `/api/users/me` | Current user profile |
| GET | `/api/users/{id}` | Get user by ID |
| POST | `/api/users` | Create user (admin) |
| PUT | `/api/users/{id}` | Update user |
| DELETE | `/api/users/{id}` | Delete user (admin) |

---

## Feature Flags Module

### Overview

The Feature Flags module provides per-tenant feature toggles for granular control.

### Functionality

- Enable/disable features per tenant
- Feature flag management

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/feature-flags` | List feature flags |
| GET | `/api/feature-flags/{name}` | Get flag by name |
| POST | `/api/feature-flags` | Create flag |
| PUT | `/api/feature-flags/{name}` | Update flag |
| DELETE | `/api/feature-flags/{name}` | Delete flag |

### Available Flags

| Flag | Description |
|------|-------------|
| PURCHASE_MODULE | Enable purchase module |
| SALES_MODULE | Enable sales module |
| CRM_MODULE | Enable CRM module |
| ASSET_MODULE | Enable asset management |

---

## Configuration Module

### Overview

The Configuration module manages global and tenant-specific settings with encrypted storage for sensitive data.

### Functionality

- Configuration categories
- Global configurations (master DB)
- Tenant configurations
- Encrypted values

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/config/categories` | List categories |
| POST | `/api/config/categories` | Create category |
| GET | `/api/config/global` | List global configs |
| POST | `/api/config/global` | Create global config |
| GET | `/api/config/global/{key}` | Get global config |
| PUT | `/api/config/global/{key}` | Update global config |
| DELETE | `/api/config/global/{key}` | Delete global config |
| GET | `/api/config/tenant` | List tenant configs |
| POST | `/api/config/tenant` | Create tenant config |
| GET | `/api/config/tenant/{key}` | Get tenant config |
| PUT | `/api/config/tenant/{key}` | Update tenant config |
| DELETE | `/api/config/tenant/{key}` | Delete tenant config |

---

## Authentication Module

### Overview

The Auth module handles user registration, login, and JWT token management.

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/auth/register` | Register new user |
| POST | `/api/auth/login` | Login & get token |
| POST | `/api/auth/refresh` | Refresh token |
| POST | `/api/auth/logout` | Logout |

### Example Usage

```bash
# Register
curl -X POST -H "Content-Type: application/json" \
  "http://localhost:8000/api/auth/register" \
  -d '{"username": "john", "email": "john@example.com", "full_name": "John Doe", "password": "secret123"}'

# Login
curl -X POST -H "Content-Type: application/json" \
  "http://localhost:8000/api/auth/login" \
  -d '{"username": "john", "password": "secret123"}'
```
