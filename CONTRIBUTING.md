# Contributing to Turerp ERP

Thank you for your interest in contributing. This document outlines the workflow and conventions used in this project.

## Git Workflow

### Branching Strategy

We follow a **feature-branch workflow**. All changes must go through a pull request.

| Rule | Description |
|------|-------------|
| **No direct pushes to `main`** | Pushing directly to the `main` branch is prohibited. All changes must be made on a feature branch and merged via pull request. |
| **Branch from `main`** | Always create your feature branch from the latest `main`. |
| **Branch naming** | Use the format `feature/<short-description>` or `fix/<short-description>`. Examples: `feature/multi-currency`, `fix/rate-limit-stats`. |

### Creating a Pull Request

1. **Create a branch:**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes** and commit them:
   ```bash
   git add .
   git commit -m "feat(scope): description"
   ```

3. **Push the branch:**
   ```bash
   git push -u origin feature/my-feature
   ```

4. **Open a pull request** on GitHub. Ensure:
   - The PR title follows [Conventional Commits](https://www.conventionalcommits.org/).
   - CI checks (tests, clippy, formatting) pass.
   - The PR description explains what changed and why.

5. **Merge only after approval.** The `main` branch is protected and requires pull request reviews.

### Commit Message Format

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

# Examples:
feat(currency): add exchange rate conversion endpoints
fix(auth): resolve MFA backup code invalidation bug
docs(readme): update contributing guidelines
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`

## Code Quality

### Pre-commit & Pre-push Hooks

This project uses [Lefthook](https://github.com/evilmartians/lefthook) to enforce quality checks automatically.

**Setup:**
```bash
cargo install lefthook
lefthook install
```

**Checks run on every commit:**
- `cargo fmt --check` — Code formatting
- `cargo clippy -- -D warnings` — Linting

**Checks run on every push:**
- `cargo test` — Full test suite
- `cargo audit` — Security audit

### Running Checks Manually

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Run tests
cargo test --features postgres

# Security audit
cargo audit
```

## Domain Module Structure

When adding a new domain module, follow the established pattern:

```
src/domain/<module>/
  mod.rs              — Module exports
  model.rs            — Domain models, DTOs, enums
  repository.rs       — Trait + in-memory implementation
  postgres_repository.rs — PostgreSQL implementation
  service.rs          — Business logic + unit tests
```

### Rules
- Keep files under **500 lines**.
- Use `async_trait` for repository traits.
- Implement both `InMemory` and `PostgreSQL` repositories.
- Add unit tests in the service file (`#[cfg(test)] mod tests`).
- Register the module in `src/domain/mod.rs` and wiring in `src/lib.rs`.

## Database Migrations

Add new migrations to `turerp/migrations/` with sequential numbering:

```
migrations/015_currency.sql
migrations/016_workflow.sql
```

Register the migration filename in `src/db/pool.rs` in the `MIGRATIONS` array.

## Testing

- **Unit tests:** In-service files using in-memory repositories.
- **Integration tests:** In `tests/` using Actix-web test server.
- **Coverage target:** All public service methods should have tests.

```bash
# Run all tests
cargo test --features postgres

# Run a specific test
cargo test test_create_invoice -- --nocapture
```

## Questions?

Open an issue on GitHub or reach out to the maintainers.
