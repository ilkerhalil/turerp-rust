# Authentication Module

## Overview

The Authentication module provides user registration, login, and JWT token management. It uses the master database for user storage.

## Functionality

### User Registration
- Username and email validation
- Password hashing (bcrypt)
- Default role assignment (User)
- Email uniqueness check
- Tenant ID assignment

### User Login
- Username/password authentication
- JWT access token generation
- Refresh token support
- Token expiration

### Token Management
- Access tokens (1 hour default)
- Refresh tokens (7 days default)
- Token validation
- Token refresh

## API Endpoints

### Authentication
| Method | Endpoint | Description | OpenAPI Tag |
|--------|----------|-------------|-------------|
| POST | `/api/auth/register` | Register new user | Auth |
| POST | `/api/auth/login` | Login and get tokens | Auth |
| POST | `/api/auth/refresh` | Refresh access token | Auth |
| GET | `/api/auth/me` | Get current user | Auth |

### Users Management
| Method | Endpoint | Description | OpenAPI Tag |
|--------|----------|-------------|-------------|
| GET | `/api/users` | List all users | Users |
| POST | `/api/users` | Create new user | Users |
| GET | `/api/users/{id}` | Get user by ID | Users |
| PUT | `/api/users/{id}` | Update user | Users |
| DELETE | `/api/users/{id}` | Delete user | Users |

## Data Model

### User
| Field | Type | Description |
|-------|------|-------------|
| id | i64 | Primary key |
| username | String | Unique username |
| email | String | Unique email |
| full_name | String | Full name |
| hashed_password | String | Bcrypt hash |
| tenant_id | i64 | Tenant ID |
| role | Role | User role |
| is_active | bool | Active status |
| created_at | DateTime | Creation date |
| updated_at | Option<DateTime> | Update date |

### Roles
| Role | Description |
|------|-------------|
| Admin | Full access |
| User | Standard access |
| Viewer | Read-only access |

### LoginRequest
```rust
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}
```

### RegisterRequest
```rust
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub password: String,
    pub tenant_id: Option<i64>,
    pub role: Option<Role>,
}
```

### LoginResponse
```rust
pub struct LoginResponse {
    pub user: UserResponse,
    pub tokens: TokenPair,
}
```

### TokenPair
```rust
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}
```

## Swagger / OpenAPI

### Enable Swagger UI

Add to `src/main.rs`:

```rust
use utoipa_swagger_ui::SwaggerUi;

HttpServer::new(move || {
    App::new()
        .service(
            SwaggerUi::new("/swagger-ui/{_:.*}")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
})
```

### Access Swagger UI

- **Swagger UI**: `http://localhost:8080/swagger-ui/`
- **OpenAPI JSON**: `http://localhost:8080/api-docs/openapi.json`

## Example Usage

### Register
```bash
curl -X POST -H "Content-Type: application/json" \
  "http://localhost:8080/api/auth/register" \
  -d '{
    "username": "john",
    "email": "john@example.com",
    "full_name": "John Doe",
    "password": "password123",
    "tenant_id": 1
  }'
```

### Login
```bash
curl -X POST -H "Content-Type: application/json" \
  "http://localhost:8080/api/auth/login?tenant_id=1" \
  -d '{
    "username": "john",
    "password": "password123"
  }'
```

### Refresh Token
```bash
curl -X POST -H "Content-Type: application/json" \
  "http://localhost:8080/api/auth/refresh" \
  -d '{"refresh_token": "eyJhbGciOiJIUzI1NiJ9..."}'
```

### Response Example
```json
{
  "user": {
    "id": 1,
    "username": "john",
    "email": "john@example.com",
    "full_name": "John Doe",
    "tenant_id": 1,
    "role": "user"
  },
  "tokens": {
    "access_token": "eyJhbGciOiJIUzI1NiJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiJ9...",
    "token_type": "Bearer",
    "expires_in": 3600
  }
}
```

## Implementation Status

✅ **Complete** - All features implemented
- ✅ User registration with validation
- ✅ Password hashing with bcrypt
- ✅ JWT token generation
- ✅ Token refresh mechanism
- ✅ OpenAPI/Swagger documentation