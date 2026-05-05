# Ruflo — Claude Code Configuration

## Rules

- Do what has been asked; nothing more, nothing less
- NEVER create files unless absolutely necessary — prefer editing existing files
- NEVER create documentation files unless explicitly requested
- NEVER save working files or tests to root — use `/src`, `/tests`, `/docs`, `/config`, `/scripts`
- ALWAYS read a file before editing it
- NEVER commit secrets, credentials, or .env files
- Keep files under 500 lines
- Validate input at system boundaries
- NEVER push directly to `main` — always use a feature branch and open a pull request
- **ALWAYS read `AGENTS.md` before starting a task** — it contains project-specific architecture, domain rules, and Rust best practices that agents must follow

## Agent Comms (SendMessage-First Coordination)

Named agents coordinate via `SendMessage`, not polling or shared state.

```
Lead (you) ←→ architect ←→ developer ←→ tester ←→ reviewer
              (named agents message each other directly)
```

### Spawning a Coordinated Team

```javascript
// ALL agents in ONE message, each knows WHO to message next
Agent({ prompt: "Research the codebase. SendMessage findings to 'architect'.",
  subagent_type: "researcher", name: "researcher", run_in_background: true })
Agent({ prompt: "Wait for 'researcher'. Design solution. SendMessage to 'coder'.",
  subagent_type: "system-architect", name: "architect", run_in_background: true })
Agent({ prompt: "Wait for 'architect'. Implement it. SendMessage to 'tester'.",
  subagent_type: "coder", name: "coder", run_in_background: true })
Agent({ prompt: "Wait for 'coder'. Write tests. SendMessage results to 'reviewer'.",
  subagent_type: "tester", name: "tester", run_in_background: true })
Agent({ prompt: "Wait for 'tester'. Review code quality and security.",
  subagent_type: "reviewer", name: "reviewer", run_in_background: true })

// Kick off the pipeline
SendMessage({ to: "researcher", summary: "Start", message: "[task context]" })
```

### Patterns

| Pattern | Flow | Use When |
|---------|------|----------|
| **Pipeline** | A → B → C → D | Sequential dependencies (feature dev) |
| **Fan-out** | Lead → A, B, C → Lead | Independent parallel work (research) |
| **Supervisor** | Lead ↔ workers | Ongoing coordination (complex refactor) |

### Rules

- ALWAYS name agents — `name: "role"` makes them addressable
- ALWAYS include comms instructions in prompts — who to message, what to send
- Spawn ALL agents in ONE message with `run_in_background: true`
- After spawning: STOP, tell user what's running, wait for results
- NEVER poll status — agents message back or complete automatically

## Swarm & Routing

### Config
- **Topology**: hierarchical-mesh (anti-drift)
- **Max Agents**: 15
- **Memory**: hybrid
- **HNSW**: Enabled
- **Neural**: Enabled

```bash
npx @claude-flow/cli@latest swarm init --topology hierarchical --max-agents 8 --strategy specialized
```

### Agent Routing

| Task | Agents | Topology |
|------|--------|----------|
| Bug Fix | researcher, coder, tester | hierarchical |
| Feature | architect, coder, tester, reviewer | hierarchical |
| Refactor | architect, coder, reviewer | hierarchical |
| Performance | perf-engineer, coder | hierarchical |
| Security | security-architect, auditor | hierarchical |

### When to Swarm
- **YES**: 3+ files, new features, cross-module refactoring, API changes, security, performance
- **NO**: single file edits, 1-2 line fixes, docs updates, config changes, questions

### 3-Tier Model Routing

| Tier | Handler | Use Cases |
|------|---------|-----------|
| 1 | Agent Booster (WASM) | Simple transforms — skip LLM, use Edit directly |
| 2 | Haiku | Simple tasks, low complexity |
| 3 | Sonnet/Opus | Architecture, security, complex reasoning |

## Memory & Learning

### Before Any Task
```bash
npx @claude-flow/cli@latest memory search --query "[task keywords]" --namespace patterns
npx @claude-flow/cli@latest hooks route --task "[task description]"
```

### After Success
```bash
npx @claude-flow/cli@latest memory store --namespace patterns --key "[name]" --value "[what worked]"
npx @claude-flow/cli@latest hooks post-task --task-id "[id]" --success true --store-results true
```

### MCP Tools (use `ToolSearch("keyword")` to discover)

| Category | Key Tools |
|----------|-----------|
| **Memory** | `memory_store`, `memory_search`, `memory_search_unified` |
| **Bridge** | `memory_import_claude`, `memory_bridge_status` |
| **Swarm** | `swarm_init`, `swarm_status`, `swarm_health` |
| **Agents** | `agent_spawn`, `agent_list`, `agent_status` |
| **Hooks** | `hooks_route`, `hooks_post-task`, `hooks_worker-dispatch` |
| **Security** | `aidefence_scan`, `aidefence_is_safe`, `aidefence_has_pii` |
| **Hive-Mind** | `hive-mind_init`, `hive-mind_consensus`, `hive-mind_spawn` |

### Background Workers

| Worker | When |
|--------|------|
| `audit` | After security changes |
| `optimize` | After performance work |
| `testgaps` | After adding features |
| `map` | Every 5+ file changes |
| `document` | After API changes |

```bash
npx @claude-flow/cli@latest hooks worker dispatch --trigger audit
```

## Agents

**Core**: `coder`, `reviewer`, `tester`, `planner`, `researcher`
**Architecture**: `system-architect`, `backend-dev`, `mobile-dev`
**Security**: `security-architect`, `security-auditor`
**Performance**: `performance-engineer`, `perf-analyzer`
**Coordination**: `hierarchical-coordinator`, `mesh-coordinator`, `adaptive-coordinator`
**GitHub**: `pr-manager`, `code-review-swarm`, `issue-tracker`, `release-manager`

Any string works as a custom agent type.

## Rust Best Practices (from AGENTS.md)

All agents MUST follow these patterns when editing Rust code.

### 1. Error Handling

**Use `thiserror` for custom error types:**

```rust
use thiserror::Error;
use actix_web::{ResponseError, HttpResponse};
use serde::Serialize;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::NotFound(msg) => HttpResponse::NotFound().json(ErrorResponse { error: msg.clone() }),
            ApiError::Unauthorized(msg) => HttpResponse::Unauthorized().json(ErrorResponse { error: msg.clone() }),
            ApiError::BadRequest(msg) => HttpResponse::BadRequest().json(ErrorResponse { error: msg.clone() }),
            ApiError::Validation(msg) => HttpResponse::BadRequest().json(ErrorResponse { error: msg.clone() }),
            ApiError::InvalidCredentials => HttpResponse::Unauthorized().json(ErrorResponse { error: "Invalid credentials".to_string() }),
            ApiError::TokenExpired => HttpResponse::Unauthorized().json(ErrorResponse { error: "Token expired".to_string() }),
            ApiError::InvalidToken(msg) => HttpResponse::Unauthorized().json(ErrorResponse { error: msg.clone() }),
            ApiError::Conflict(msg) => HttpResponse::Conflict().json(ErrorResponse { error: msg.clone() }),
            ApiError::Database(msg) => {
                tracing::error!("Database error: {}", msg);
                HttpResponse::InternalServerError().json(ErrorResponse { error: "Internal database error".to_string() })
            }
            ApiError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                HttpResponse::InternalServerError().json(ErrorResponse { error: "Internal error".to_string() })
            }
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
```

### 2. Repository Pattern

**Define repository traits for testability:**

```rust
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: CreateUser, hashed_password: String) -> Result<User, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<User>, ApiError>;
    async fn find_all(&self, tenant_id: i64) -> Result<Vec<User>, ApiError>;
    async fn update(&self, id: i64, tenant_id: i64, user: UpdateUser) -> Result<User, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

pub type BoxUserRepository = Arc<dyn UserRepository>;
```

### 3. PostgreSQL Repository Implementation

**Use `FromRow` for type safety and `map_sqlx_error` helper:**

```rust
use sqlx::FromRow;

#[derive(Debug, FromRow)]
struct UserRow {
    id: i64,
    username: String,
    email: String,
    role: String,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            role: row.role.parse().unwrap_or_else(|e| {
                tracing::warn!("Invalid role in database: {}", e);
                Role::default()
            }),
            // ...
        }
    }
}

fn map_sqlx_error(e: sqlx::Error, entity: &str) -> ApiError {
    match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("{} not found", entity)),
        _ => {
            let msg = e.to_string();
            if msg.contains("duplicate key") || msg.contains("unique constraint") {
                ApiError::Conflict(format!("{} already exists", entity))
            } else {
                ApiError::Database(format!("Failed to operate on {}: {}", entity, e))
            }
        }
    }
}
```

### 4. Use `parking_lot::Mutex` (never `std::sync::Mutex`)

```rust
// ❌ Bad: Can panic on poisoned mutex
use std::sync::Mutex;
let guard = self.users.lock().unwrap();

// ✅ Good: parking_lot::Mutex never poisons
use parking_lot::Mutex;
let guard = self.users.lock(); // Returns MutexGuard directly
```

### 5. Lazy Static for Regex

```rust
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref UPPERCASE_REGEX: Regex = Regex::new(r"[A-Z]").unwrap();
    static ref LOWERCASE_REGEX: Regex = Regex::new(r"[a-z]").unwrap();
    static ref DIGIT_REGEX: Regex = Regex::new(r"[0-9]").unwrap();
    static ref SPECIAL_REGEX: Regex = Regex::new(r"[^A-Za-z0-9]").unwrap();
}
```

### 6. Code Conventions

**Naming:**
- `snake_case` for variables, functions, modules
- `CamelCase` for types, enums, traits
- `UPPER_SNAKE_CASE` for constants

**Imports Order:**
```rust
// 1. Standard library
use std::sync::Arc;

// 2. External crates
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};

// 3. Internal modules
use crate::config::Config;
use crate::domain::cari::model::Cari;
use crate::error::ApiError;
```

**Error Handling:**
```rust
// ✅ Good: Use map_err for conversions
repo.find_by_id(id, tenant_id).await?
    .ok_or(ApiError::NotFound(format!("User {} not found", id)))?;

// ✅ Good: Use helper for sqlx errors
.fetch_one(&*self.pool).await
    .map_err(|e| map_sqlx_error(e, "User"))?;

// ❌ Bad: Silent unwrap
let user = repo.find_by_id(id).await.unwrap();
```

### 7. Common Pitfalls

1. **Don't use `std::sync::Mutex`** — Use `parking_lot::Mutex`
2. **Don't compile regex every call** — Use `lazy_static!`
3. **Don't skip password validation** — Always validate with `create.validate_password()?`
4. **Don't forget tenant isolation** — Always filter by `tenant_id`
5. **Don't use `.unwrap()` in production** — Handle errors properly
6. **Don't block async runtime** — Use `tokio::fs` instead of `std::fs`

## Build & Test

- ALWAYS run tests after code changes
- ALWAYS verify build succeeds before committing

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## CLI Quick Reference

```bash
npx @claude-flow/cli@latest init --wizard           # Setup
npx @claude-flow/cli@latest swarm init --v3-mode     # Start swarm
npx @claude-flow/cli@latest memory search --query "" # Vector search
npx @claude-flow/cli@latest hooks route --task ""    # Route to agent
npx @claude-flow/cli@latest doctor --fix             # Diagnostics
npx @claude-flow/cli@latest security scan            # Security scan
npx @claude-flow/cli@latest performance benchmark    # Benchmarks
```

26 commands, 140+ subcommands. Use `--help` on any command for details.

## Setup

```bash
claude mcp add claude-flow -- npx -y @claude-flow/cli@latest
npx @claude-flow/cli@latest daemon start
npx @claude-flow/cli@latest doctor --fix
```

**Agent tool** handles execution (agents, files, code, git). **MCP tools** handle coordination (swarm, memory, hooks). **CLI** is the same via Bash.
