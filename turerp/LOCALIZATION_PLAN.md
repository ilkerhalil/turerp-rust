# Localization (i18n) Implementation Plan

## Status: ✅ Completed

### Overview
Turerp ERP already had a lightweight `i18n` module (`src/i18n/`), `Locale` extractor, and JSON translation bundles (`locales/`). However, it was **not wired into production**: `AppState` never initialized `I18n`, handlers did not extract `Locale`, and error/success responses were hardcoded in English.

This plan brings localization to production-readiness. **Implementation is complete.**

---

## Goals
1. ✅ Initialize `I18n` in all `AppState` factory variants (bug fix)
2. ✅ Expand translation keys for **all business domains** (auth, user, tenant, cari, stock, invoice, sales, purchase, hr, accounting, assets, project, manufacturing, crm, audit)
3. ✅ Add `ApiError::localized()` so user-facing errors can be translated
4. ✅ Inject `Locale` extractor into v1 handlers and localize **success messages**
5. ✅ Localize common **user-facing error paths** (invalid credentials, not found, conflicts, validation)
6. ✅ Ensure `cargo test` and `cargo clippy` pass

---

## Phase 1: Infrastructure Fixes

### 1.1 AppState Initialization
- **File**: `src/lib.rs`
- **Problem**: `AppState` has `i18n: web::Data<I18n>` but none of `create_app_state`, `create_app_state_in_memory`, or `#[cfg(feature = "postgres")] create_app_state` populate this field.
- **Fix**: Add `i18n: web::Data::new(I18n::init())` to every factory function.

### 1.2 Locale Extractor
- **File**: `src/i18n/extractor.rs` (already exists)
- **Actions**: None required — it already parses `Accept-Language`.

---

## Phase 2: Translation Catalog Expansion

### 2.1 Translation Key Schema
```
generic.{action}
errors.{type}
errors.{type}_detail
auth.{action}.{result}
user.{action}.{result}
tenant.{action}.{result}
invoice.{action}.{result}
...
```

### 2.2 Files
- `locales/en.json` — expand from ~30 keys to 120+ keys covering all domains
- `locales/tr.json` — Turkish equivalents

### 2.3 Content Checklist
- [x] Generic greetings & labels
- [x] Auth (register, login, logout, token refresh)
- [x] Users (CRUD, role changes, password)
- [x] Tenant (CRUD, config)
- [x] Cari (accounts, types)
- [x] Stock (warehouses, movements)
- [x] Invoice (create, pay, cancel)
- [x] Sales (orders, quotations)
- [x] Purchase (orders, requests, receipts)
- [x] HR (employees, attendance, leave, payroll)
- [x] Accounting (accounts, journal entries)
- [x] Assets (fixed assets, categories, maintenance)
- [x] Project (projects, WBS, costs)
- [x] Manufacturing (BOM, work orders, routing, QC)
- [x] CRM (leads, opportunities, tickets, campaigns)
- [x] Product (products, categories, variants, units)
- [x] Audit (logs, trail)

---

## Phase 3: Error Localization

### 3.1 ApiError Translation
- **File**: `src/error.rs`
- **Approach**: Add `localized(&self, i18n: &I18n, locale: &str) -> String` to `ApiError`.
- **Mapping**:
  - `InvalidCredentials` → `errors.invalid_credentials`
  - `TokenExpired` → `errors.token_expired`
  - `Database` / `Internal` → generic internal keys (no detail leak)
  - `NotFound(msg)` → extract subject from `"… not found"` pattern, fall back to full msg; key = `errors.not_found`
  - `Unauthorized(msg)` → `errors.unauthorized` with `{detail}`
  - `Forbidden(msg)` → `errors.forbidden` with `{detail}`
  - `BadRequest(msg)` → `errors.bad_request` with `{detail}`
  - `Conflict(msg)` → `errors.conflict` with `{detail}`
  - `Validation(msg)` → `errors.validation_error` with `{detail}`
  - `InvalidToken(msg)` → `errors.invalid_token` with `{detail}`

### 3.2 Handler Integration
- Add `locale: Locale` extractor to handler signatures.
- On success paths, return localized strings via `i18n.t()` / `i18n.t_args()`.
- On error paths, use `ApiError::to_localized_response()` for user-facing JSON.

---

## Phase 4: Handler Coverage (v1)

### 4.1 Priority Modules (reference implementation)
| Module | File | Changes |
|--------|------|---------|
| Auth | `src/api/v1/auth.rs` | Add `locale`, localize success/error |
| Users | `src/api/v1/users.rs` | Add `locale`, localize success/error |
| Tenant | `src/api/v1/tenant.rs` | Add `locale`, localize success/error |

### 4.2 Remaining Modules
Follow the same pattern established in Phase 4.1. Each handler receives `locale: Locale` and uses `i18n` from AppState for success/error localization.

---

## Phase 5: Tests & Quality

- [ ] `cargo check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (in-memory)
- [ ] Existing `i18n` unit tests still pass
- [ ] Add integration test: request with `Accept-Language: tr` returns Turkish error/success messages

---

## Open Questions
1. Should `DbError` / `map_sqlx_error` return translation keys instead of raw strings? → **Deferred** (internal errors should stay generic).
2. Should middleware log messages also be localized? → **No** (logs remain English for operator consistency).
3. Date / number / currency formatting? → **Out of scope** for this plan; revisit when i18n formatting crate is needed.

---

## Acceptance Criteria
- [ ] `curl -H "Accept-Language: tr" …` returns a Turkish success/error message for auth endpoints.
- [ ] `curl -H "Accept-Language: en" …` (or no header) returns English.
- [ ] Unsupported languages fall back to English.
- [ ] `AppState.i18n` is initialized in all build modes.
- [ ] No compiler warnings.
