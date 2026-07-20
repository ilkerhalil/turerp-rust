//! Turerp ERP - Main application entry point
//!
//! Run with: cargo run --package turerp
//! With PostgreSQL: cargo run --package turerp --features postgres

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use secrecy::ExposeSecret;
use std::sync::Arc;
use turerp::config::Config;
use turerp::middleware::{
    audit::spawn_audit_writer, AuditLoggingMiddleware, AuthUser, IdempotencyMiddleware,
    IpWhitelistMiddleware, JwtAuthMiddleware, MetricsMiddleware, RateLimitMiddleware,
    RequestIdMiddleware, SecurityHeadersMiddleware, TenantMiddleware, TracingMiddleware,
};

use tokio::sync::mpsc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use turerp::api::{
    v1_accounting_configure, v1_api_keys_configure, v1_archive_configure, v1_assets_configure,
    v1_audit_configure, v1_auth_configure, v1_bank_configure, v1_barcode_configure,
    v1_cari_configure, v1_chart_of_accounts_configure, v1_companies_configure,
    v1_cost_centers_configure, v1_crm_configure, v1_currency_configure, v1_custom_fields_configure,
    v1_customer_portal_configure, v1_dashboard_configure, v1_documents_configure,
    v1_earchive_configure, v1_edefter_blockchain_configure, v1_edefter_configure,
    v1_efatura_configure, v1_events_configure, v1_feature_flags_configure, v1_files_configure,
    v1_forecasting_configure, v1_goods_receipts_configure, v1_graphql_configure, v1_hr_configure,
    v1_import_configure, v1_invoice_configure, v1_ip_whitelist_configure, v1_jobs_configure,
    v1_ldap_configure, v1_manufacturing_configure, v1_mfa_configure, v1_notifications_configure,
    v1_observability_configure, v1_product_variants_configure, v1_project_configure,
    v1_purchase_orders_configure, v1_purchase_requests_configure, v1_push_notifications_configure,
    v1_rate_limits_configure, v1_reports_configure, v1_resilience_configure, v1_sales_configure,
    v1_search_configure, v1_settings_configure, v1_shifts_configure, v1_stock_configure,
    v1_subscriptions_configure, v1_tax_configure, v1_tenant_configure, v1_users_configure,
    v1_vendor_portal_configure, v1_webhooks_configure, v1_workflows_configure, ApiDoc, GlobalGate,
};
use turerp::middleware::audit::{AuditEvent, AUDIT_CHANNEL_CAPACITY};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use turerp::app::AppState;

/// Liveness probe - always returns 200 if the process is running
async fn health_live() -> actix_web::Result<actix_web::HttpResponse> {
    Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "turerp-erp",
        "version": env!("CARGO_PKG_VERSION")
    })))
}

/// Readiness probe - checks database and cache connectivity
/// with a per-probe timeout. A hung dependency would otherwise
/// exhaust the actix worker pool while we wait for it.
/// Redact an error string into a stable, non-leaking classification.
/// Returns a short hash of the error so operators can correlate with
/// server-side logs without exposing the raw sqlx error to anonymous
/// kubelets.
fn redact_error(err: impl std::fmt::Display) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let s = err.to_string();
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("error:{:x}", h.finish() & 0xFFFF_FFFF)
}

async fn health_ready(
    app_state: web::Data<AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    // Per-probe timeout. Set deliberately lower than the actix
    // request timeout (30s by default) so a single slow probe
    // does not consume the worker for that whole window.
    let probe_timeout = std::time::Duration::from_secs(2);
    let cache = app_state.infra.cache_service.get_ref();

    let cache_start = std::time::Instant::now();
    let cache_result = tokio::time::timeout(probe_timeout, cache.health_check()).await;
    let cache_latency_ms = cache_start.elapsed().as_millis();
    if let Err(_) | Ok(Err(_)) = &cache_result {
        tracing::error!("Cache health check failed: {:?}", cache_result);
    }

    let mut deps = serde_json::Map::new();
    let mut healthy = true;

    match cache_result {
        Ok(Ok(())) => {
            deps.insert("cache".into(), serde_json::json!("ok"));
        }
        Ok(Err(e)) => {
            // Redact the error so the raw sqlx string is not leaked
            // to anonymous kubelets. Operators correlate via the
            // server-side log and the redacted hash.
            tracing::error!("Cache health check error: {}", e);
            deps.insert("cache".into(), serde_json::json!(redact_error(e)));
            healthy = false;
        }
        Err(_) => {
            deps.insert("cache".into(), serde_json::json!("timeout"));
            healthy = false;
        }
    }

    if let Some(ref pool) = app_state.infra.db_pool {
        let db_start = std::time::Instant::now();
        let db_result = tokio::time::timeout(
            probe_timeout,
            sqlx::query("SELECT 1").execute(&**pool.get_ref()),
        )
        .await;
        let db_latency_ms = db_start.elapsed().as_millis();
        match db_result {
            Ok(Ok(_)) => {
                deps.insert("db".into(), serde_json::json!("ok"));
            }
            Ok(Err(e)) => {
                tracing::error!("DB health check error: {}", e);
                deps.insert("db".into(), serde_json::json!(redact_error(e)));
                healthy = false;
            }
            Err(_) => {
                deps.insert("db".into(), serde_json::json!("timeout"));
                healthy = false;
            }
        }
        deps.insert(
            "db_latency_ms".into(),
            serde_json::json!(db_latency_ms as u64),
        );
    } else {
        deps.insert(
            "db".into(),
            serde_json::json!("not configured (in-memory mode)"),
        );
    }

    // Scheduler probe: the previous code never invoked this, so
    // /health/ready could claim "ok" even when the JobScheduler
    // (PostgresJobScheduler in production) was wedged. The probe
    // must be cheap and bounded by probe_timeout; the in-memory
    // scheduler's default impl returns Ok(()) immediately, and
    // PostgresJobScheduler::health_check runs a SELECT 1.
    {
        let scheduler = app_state.infra.job_scheduler.get_ref();
        let scheduler_start = std::time::Instant::now();
        let scheduler_result = tokio::time::timeout(probe_timeout, scheduler.health_check()).await;
        let scheduler_latency_ms = scheduler_start.elapsed().as_millis();
        match scheduler_result {
            Ok(Ok(())) => {
                deps.insert("scheduler".into(), serde_json::json!("ok"));
            }
            Ok(Err(e)) => {
                tracing::error!("JobScheduler health check error: {}", e);
                deps.insert("scheduler".into(), serde_json::json!(redact_error(e)));
                healthy = false;
            }
            Err(_) => {
                deps.insert("scheduler".into(), serde_json::json!("timeout"));
                healthy = false;
            }
        }
        deps.insert(
            "scheduler_latency_ms".into(),
            serde_json::json!(scheduler_latency_ms as u64),
        );
    }

    deps.insert(
        "cache_latency_ms".into(),
        serde_json::json!(cache_latency_ms as u64),
    );

    let body = serde_json::json!({
        "status": if healthy { "ok" } else { "degraded" },
        "service": "turerp-erp",
        "version": env!("CARGO_PKG_VERSION"),
        "deps": deps,
    });

    if healthy {
        Ok(actix_web::HttpResponse::Ok().json(body))
    } else {
        Ok(actix_web::HttpResponse::ServiceUnavailable().json(body))
    }
}

/// Backwards-compatible health check endpoint (aliases to readiness)
async fn health_check(
    app_state: web::Data<AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    health_ready(app_state).await
}

/// Metrics endpoint - exposes Prometheus-format metrics
///
/// Requires authentication. Prometheus scraper must be configured
/// with a valid Bearer token.
async fn metrics_endpoint(_auth_user: AuthUser) -> actix_web::Result<actix_web::HttpResponse> {
    let body = turerp::middleware::render_metrics();
    Ok(actix_web::HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(body))
}

/// Configure CORS from config
fn configure_cors(cors_config: &turerp::config::CorsConfig) -> Cors {
    use actix_web::http::{header, Method};

    let mut cors = Cors::default();

    // actix-cors forbids mixing wildcard with explicit origins or credentials.
    // The new API uses `send_wildcard()` to allow any origin (no credentials),
    // or iterate explicit origins.
    let has_wildcard = cors_config.allowed_origins.iter().any(|o| o == "*");
    if has_wildcard {
        cors = cors.send_wildcard();
    } else {
        for origin in &cors_config.allowed_origins {
            cors = cors.allowed_origin(origin);
        }
    }

    // Convert method strings to Method
    let methods: Vec<Method> = cors_config
        .allowed_methods
        .iter()
        .filter_map(|m| m.parse().ok())
        .collect();
    cors = cors.allowed_methods(methods);

    // Convert header strings to HeaderName
    let headers: Vec<header::HeaderName> = cors_config
        .allowed_headers
        .iter()
        .filter_map(|h| h.parse().ok())
        .collect();
    cors = cors.allowed_headers(headers);

    if cors_config.allow_credentials && !has_wildcard {
        cors = cors.supports_credentials();
    } else if cors_config.allow_credentials && has_wildcard {
        tracing::warn!(
            "CORS allow_credentials is ignored when wildcard origin is configured; \
             credentials cannot be combined with wildcard origins."
        );
    }

    if let Some(max_age) = cors_config.max_age {
        cors = cors.max_age(max_age as usize);
    }

    cors
}

/// Parse Prometheus text-format metrics into a simple key-value map.
/// Extracts availability, error_rate, latency_p95, and throughput estimates.

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration first (needed for OTLP settings before subscriber init).
    // Fail hard on config errors — falling back to Config::default() would silently
    // downgrade to development mode with an empty JWT secret and in-memory storage,
    // bypassing all production validation (see issue #322).
    let mut config = Config::new().unwrap_or_else(|e| {
        eprintln!("Failed to load configuration from environment: {e}");
        eprintln!(
            "Set required environment variables (TURERP_JWT_SECRET, etc.) or fix the error above."
        );
        std::process::exit(1);
    });

    // Build subscriber with optional OTLP layers.
    // OTel trace layer must be added directly to Registry (it implements Layer<Registry>),
    // then EnvFilter, fmt layer, and log bridge can follow.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "turerp=debug,actix_web=info".into());

    if config.metrics.otlp_enabled {
        let trace_result =
            turerp::common::otlp::create_otlp_trace_layer(&config.metrics.otlp_endpoint);
        let log_result = turerp::common::otlp::create_otlp_log_layer(&config.metrics.otlp_endpoint);

        match (trace_result, log_result) {
            (Ok(t), Ok(l)) => {
                tracing_subscriber::registry()
                    .with(t)
                    .with(env_filter)
                    .with(tracing_subscriber::fmt::layer())
                    .with(l)
                    .init();
            }
            (Ok(t), Err(e)) => {
                eprintln!("Failed to create OTLP log layer: {}", e);
                tracing_subscriber::registry()
                    .with(t)
                    .with(env_filter)
                    .with(tracing_subscriber::fmt::layer())
                    .init();
            }
            (Err(e), Ok(l)) => {
                eprintln!("Failed to create OTLP trace layer: {}", e);
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(tracing_subscriber::fmt::layer())
                    .with(l)
                    .init();
            }
            (Err(e1), Err(e2)) => {
                eprintln!("Failed to create OTLP trace layer: {}", e1);
                eprintln!("Failed to create OTLP log layer: {}", e2);
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(tracing_subscriber::fmt::layer())
                    .init();
            }
        }
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    tracing::info!("Starting Turerp ERP server...");

    // Install OTLP metric exporter when enabled
    if config.metrics.otlp_enabled {
        if let Err(e) = turerp::common::otlp::install_otlp_metrics(&config.metrics.otlp_endpoint) {
            tracing::warn!("Failed to install OTLP metrics exporter: {}", e);
        }
    }

    // Write OpenAPI spec to file so it stays in sync with the code
    if let Ok(json) = ApiDoc::openapi().to_pretty_json() {
        let spec_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("openapi.json");
        if let Err(e) = std::fs::write(&spec_path, &json) {
            tracing::warn!("Failed to write openapi.json: {}", e);
        } else {
            tracing::info!("OpenAPI spec written to {}", spec_path.display());
        }
    }

    // Resolve secrets from Vault if enabled
    if config.secrets.vault_enabled {
        match turerp::common::secrets::VaultSecretsService::new(
            &config.secrets.vault_addr,
            config.secrets.vault_token.expose_secret(),
            &config.secrets.vault_mount,
        )
        .await
        {
            Ok(vault) => {
                if let Err(e) = config.resolve_secrets(&vault).await {
                    tracing::warn!("Failed to resolve secrets from Vault: {}", e);
                } else {
                    tracing::info!(
                        "Secrets resolved from Vault at {}",
                        config.secrets.vault_addr
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to connect to Vault: {}", e);
            }
        }
    }

    // Validate configuration (logs warnings for production issues)
    if let Err(e) = config.validate() {
        tracing::error!("Invalid configuration: {}", e);
        std::process::exit(1);
    }

    // Install Prometheus metrics exporter
    if config.metrics.enabled {
        if let Err(e) = turerp::middleware::install_metrics_exporter() {
            tracing::warn!("Failed to install metrics exporter: {}", e);
        }
    }

    // Log CORS warning in production
    if config.is_production() && config.cors.is_wildcard() {
        tracing::warn!(
            "CORS allows all origins (*) in production mode. \
             Set TURERP_CORS_ORIGINS environment variable for better security."
        );
    }

    let bind_addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Server will bind to: {}", bind_addr);
    tracing::info!("Environment: {}", config.environment);

    // MIGRATIONS_DOWN=1: replay down.sql in reverse and exit. This
    // is the GA-cut rollback path (see RUNBOOK.md § 4.3 and the
    // design spec § 4.3). The check is BEFORE `create_app_state`
    // because the typical use case is "the previous boot applied
    // 037_pg_audit_dlq and we want to undo that without losing
    // earlier migrations". We exit after the down-replay so the
    // app does not start writing with a partial schema.
    //
    // Operator command (non-production):
    //   docker compose run --rm -e MIGRATIONS_DOWN=1 turerp
    //
    // Operator command (production) — requires an explicit second
    // confirmation env (see the guard below):
    //   docker compose run --rm \
    //     -e MIGRATIONS_DOWN=1 \
    //     -e MIGRATIONS_DOWN_CONFIRM=I_UNDERSTAND_THIS_IS_DESTRUCTIVE \
    //     turerp
    if std::env::var("MIGRATIONS_DOWN").as_deref() == Ok("1") {
        // Production down-replay safety guard. The down migrations are
        // destructive (they DROP tenant_id columns/indexes/FKs that the
        // #162 cross-tenant leak-audit added). An accidental
        // MIGRATIONS_DOWN=1 leaking into a helm values file or a k8s
        // CronJob/Job env would replay ALL downs and re-open the leak
        // class the audit closed. In production we therefore refuse to
        // run any down migration unless the operator also sets
        // MIGRATIONS_DOWN_CONFIRM to the exact acknowledgment phrase
        // below. The phrase is deliberately NOT `1` so that a
        // copy-paste of `MIGRATIONS_DOWN=1` does not silently carry a
        // matching `=1` confirm. Safe default = refuse + exit before
        // any down SQL runs. Non-production (development) keeps the
        // original single-env behavior so local rollback is unaffected.
        if config.is_production() {
            let confirm = std::env::var("MIGRATIONS_DOWN_CONFIRM").unwrap_or_default();
            if confirm != "I_UNDERSTAND_THIS_IS_DESTRUCTIVE" {
                tracing::error!(
                    "MIGRATIONS_DOWN=1 refused in production: down-replay is \
                     destructive (drops tenant_id columns and re-opens the \
                     cross-tenant leak class). To proceed, set \
                     MIGRATIONS_DOWN_CONFIRM=I_UNDERSTAND_THIS_IS_DESTRUCTIVE. \
                     Aborting before any down migration runs."
                );
                std::process::exit(1);
            }
            tracing::warn!(
                "MIGRATIONS_DOWN=1 confirmed in production — running \
                 down-replay then exiting"
            );
        } else {
            tracing::warn!("MIGRATIONS_DOWN=1 — running down-replay then exiting");
        }
        let down_pool = turerp::db::create_pool(&config.database)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to create pool for down-replay: {}", e);
                std::process::exit(1);
            });
        match turerp::db::run_migrations_down(&down_pool).await {
            Ok(n) => {
                tracing::warn!("MIGRATIONS_DOWN=1 — replayed {} migrations, exiting", n);
                std::process::exit(0);
            }
            Err(e) => {
                tracing::error!("MIGRATIONS_DOWN=1 — down-replay failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Create application state with config
    let app_state = turerp::app::create_app_state_unified(&config)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Failed to create app state: {}", e);
            std::process::exit(1);
        });

    // Start background job executor
    let (job_handle, job_shutdown_tx) = {
        let executor = turerp::common::job_executor::JobExecutor::new(
            app_state.infra.job_scheduler.clone(),
            app_state.infra.import_service.clone(),
            app_state.document.file_storage.clone(),
        );
        executor.start()
    };
    tracing::info!("Job executor started");

    // Start background observability evaluator (alert rules + SLO + SLI collection)
    let (obs_shutdown_tx, obs_shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
    let obs_handle = {
        let observability_service = app_state.observability_service.get_ref().clone();
        let notification_service = app_state.infra.notification_service.clone().into_inner();
        let evaluator = turerp::common::background_evaluator::BackgroundEvaluator::new(
            observability_service,
            notification_service,
        );
        evaluator.start(obs_shutdown_rx)
    };
    tracing::info!("Background observability evaluator started");

    // Start job service background tasks: 60s cron evaluator + 300s
    // stalled-job resetter. This was previously never invoked from
    // main, so scheduled jobs in the database silently never fired —
    // the JobExecutor polls next_pending() but the cron schedule
    // never enqueued anything. We construct a JobService with the
    // appropriate repository (Postgres in production, in-memory in
    // dev) and capture the JoinHandle for the drain sequence.
    let (job_cron_handle, job_cron_shutdown_tx) = {
        use turerp::domain::job::repository::JobRepository;
        let job_repo: std::sync::Arc<dyn JobRepository> =
            if let Some(pool) = app_state.infra.db_pool.as_ref() {
                std::sync::Arc::new(
                    turerp::domain::job::postgres_repository::PostgresJobRepository::new(
                        pool.get_ref().as_ref().clone(),
                    ),
                )
            } else {
                std::sync::Arc::new(turerp::domain::job::repository::InMemoryJobRepository::new())
            };
        let svc = turerp::domain::job::service::JobService::new(job_repo);
        svc.start_background_tasks()
    };
    tracing::info!("Job service background tasks started (60s cron + 300s heartbeat)");

    // Build rate-limit middleware with shared stats store so the dashboard can read them.
    // A background eviction task removes idle client keys from the governor's
    // internal DashMap to prevent unbounded memory growth (issue #345).
    let rate_limit_middleware = {
        let stats_store = app_state.infra.rate_limit_stats.get_ref().clone();
        RateLimitMiddleware::with_config(&config.rate_limit)
            .with_stats_store(stats_store)
            .spawn_idle_eviction(std::time::Duration::from_secs(300))
    };

    // Set up audit log channel (bounded to prevent unbounded memory growth under load)
    let (audit_tx, audit_rx) = mpsc::channel::<AuditEvent>(AUDIT_CHANNEL_CAPACITY);
    let audit_sender: std::sync::Arc<mpsc::Sender<AuditEvent>> = std::sync::Arc::new(audit_tx);
    let audit_svc = app_state.analytics.audit_service.get_ref().clone();
    // The audit writer also spools failed batches to the persistent
    // `pg_audit_dlq` table (see `domain::audit::dlq`). The pool is
    // optional because the in-memory audit service is testable
    // without a DB; in production it is always set.
    let dlq_pool: Option<std::sync::Arc<sqlx::PgPool>> = app_state
        .infra
        .db_pool
        .as_ref()
        .map(|p| p.get_ref().clone());
    let audit_handle = spawn_audit_writer(audit_rx, audit_svc, dlq_pool);

    let is_production = config.is_production();
    let security_headers_config = config.security_headers.clone();

    // Build idempotency middleware: Redis in production if enabled, otherwise in-memory.
    // When Redis is explicitly enabled, connection failure is a hard startup error —
    // falling back to in-memory silently would break deduplication in multi-instance
    // deployments (issue #344).
    let idempotency_middleware = if config.redis.enabled {
        match turerp::middleware::RedisIdempotencyStore::new(&config.redis.url).await {
            Ok(store) => {
                tracing::info!("Redis idempotency store initialized");
                IdempotencyMiddleware::new(Arc::new(store))
            }
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!(
                        "Redis idempotency store is enabled (TURERP_REDIS_ENABLED=true) \
                         but connection to {} failed: {}. \
                         Refusing to start with in-memory fallback — in multi-instance \
                         deployments this would silently break idempotency deduplication.",
                        config.redis.url, e
                    ),
                ));
            }
        }
    } else {
        IdempotencyMiddleware::in_memory()
    };

    // Macro to build the common Actix app (middleware + app_data).
    // Avoids duplicating ~100 lines for postgres vs in-memory feature flags.
    macro_rules! build_app_core {
        ($app:expr) => {{
            $app
                .configure(|cfg| app_state.register_services(cfg))
                // Security middlewares (ORDER MATTERS!)
                // In actix-web 4 the FIRST `.wrap()` call is the INNERMOST
                // middleware (runs last on request, first on response).
                // The LAST `.wrap()` call is the OUTERMOST middleware (runs
                // first on request, last on response). See
                // https://actix.rs/docs/middleware/ — "if you use `wrap()` or
                // `wrap_fn()` multiple times, the last occurrence will be
                // executed first".
                //
                // Request path flows outermost → innermost:
                //   RequestId → RateLimit → SecurityHeaders → CORS →
                //   JwtAuth → IpWhitelist → Tenant → Audit → Metrics →
                //   Idempotency → GlobalGate → Compress → Tracing
                //
                // Key ordering constraints (issue #323):
                //   - JwtAuth must run BEFORE IpWhitelist, Tenant, Audit,
                //     and GlobalGate so that `AuthClaims` is in request
                //     extensions when those middlewares run. Previously
                //     JwtAuth was at wrap #6 (inner) and IpWhitelist was at
                //     wrap #11 (outer), so IpWhitelist ran before JwtAuth on
                //     the request path — the IP allowlist was dead code.
                //   - TracingMiddleware is innermost so it sees ALL
                //     extensions (request_id, AuthClaims) set by upstream
                //     middlewares when creating spans. Previously it was at
                //     wrap #12 (nearly outermost), running before JwtAuth,
                //     so spans lacked user/tenant context.
                //   - RequestIdMiddleware is outermost so every downstream
                //     middleware and log line has a request_id.
                //   - RateLimit is just inside RequestId so all requests
                //     (including rejected ones) are counted.
                .wrap(TracingMiddleware) // 1. Innermost: sees all extensions (request_id, AuthClaims)
                .wrap(middleware::Compress::default()) // 2. Response compression
                .wrap(GlobalGate::new(
                    // Per-request gate rules. Each rule is (path_prefix, flag_name).
                    // Longest prefix wins. Add new gated routes here.
                    // NOTE: prefixes must include the /api scope that the App
                    // registers (actix-web 4 returns the full request path,
                    // not the post-scope path).
                    //
                    // Order groups:
                    //   - tier2.*  : well-known gated modules (off by default)
                    //   - core.*   : PR-2 broken-endpoint gates — routes are
                    //                currently 500/404-broken, gate is in place
                    //                so when PR 2 fixes the handler the gate
                    //                is already there and operators can flip
                    //                the flag on consistently.
                    vec![
                        // tier2.* — well-known gated modules
                        ("/api/v1/files".to_string(),         "tier2.file_upload".to_string()),
                        ("/api/v1/shifts".to_string(),        "tier2.shifts".to_string()),
                        ("/api/v1/graphql".to_string(),       "tier2.graphql".to_string()),
                        ("/api/v1/projects".to_string(),      "tier2.projects".to_string()),
                        ("/api/v1/manufacturing".to_string(), "tier2.manufacturing".to_string()),
                        // core.* — 7 broken-endpoint gates (issue #152 +
                        // /api/v1/hr/leave-types). These routes are
                        // currently 500/404-broken. The gate is in place
                        // today so PR 2's handler fix is the only thing
                        // needed to make the route operator-enable-able.
                        ("/api/v1/categories".to_string(),    "core.categories".to_string()),
                        ("/api/v1/units".to_string(),         "core.units".to_string()),
                        ("/api/v1/currencies".to_string(),    "core.currencies".to_string()),
                        ("/api/v1/settings".to_string(),      "core.settings".to_string()),
                        ("/api/v1/stock/warehouses".to_string(), "core.stock.warehouses".to_string()),
                        // /api/v1/hr/leave-types is more specific than
                        // /api/v1/hr/employees (the 11_hr_employees hurl
                        // target), and the gate uses segment-aware prefix
                        // matching, so this rule does NOT match
                        // /api/v1/hr/employees. HR coarse-gate
                        // (tier2.payroll) is intentionally deferred — see
                        // PR body "Concerns / NOT in this PR".
                        ("/api/v1/hr/leave-types".to_string(), "core.hr.leave_types".to_string()),
                    ],
                    app_state.feature_service.clone(),
                )) // 3. Feature-flag gate (needs AuthClaims — inner of JwtAuth)
                .wrap(idempotency_middleware.clone()) // 4. Idempotency key caching
                .wrap(MetricsMiddleware::new()) // 5. Metrics collection
                .wrap(AuditLoggingMiddleware::with_sender(audit_sender.clone())) // 6. Audit logging (needs AuthClaims)
                .wrap(TenantMiddleware) // 7. Tenant context extraction (needs AuthClaims)
                .wrap(
                    IpWhitelistMiddleware::new(app_state.admin.ip_whitelist_service.get_ref().clone())
                        .with_trusted_proxies(
                            config
                                .rate_limit
                                .trusted_proxies
                                .iter()
                                .filter_map(|s| s.parse().ok())
                                .collect(),
                        ),
                ) // 8. IP whitelist check (needs AuthClaims for tenant_id)
                .wrap(JwtAuthMiddleware::new(
                    app_state.auth.jwt_service.get_ref().clone(),
                )) // 9. JWT validation (sets AuthClaims — inner middlewares above can read it)
                .wrap(configure_cors(&config.cors)) // 10. CORS handling
                .wrap(SecurityHeadersMiddleware::new(
                    &security_headers_config,
                    is_production,
                )) // 11. Security headers
                .wrap(rate_limit_middleware.clone()) // 12. Rate limiting (counts all requests including rejected)
                .wrap(RequestIdMiddleware) // 13. Outermost: generates request_id first for all downstream logs
        }};
    }

    // Build a future that runs the HTTP server and triggers a clean worker
    // shutdown on first signal. Actix's `run()` already handles graceful
    // shutdown of in-flight HTTP requests; we add worker drain on top.
    let server = HttpServer::new(move || {
        let mut app = build_app_core!(App::new());
        if let Some(ref pool) = app_state.infra.db_pool {
            app = app.app_data(pool.clone());
        }

        app // Health check
            .route("/health", web::get().to(health_check))
            .route("/health/live", web::get().to(health_live))
            .route("/health/ready", web::get().to(health_ready))
            .route("/metrics", web::get().to(metrics_endpoint))
            // V1 API routes
            .service(
                web::scope("/api")
                    .configure(v1_auth_configure)
                    .configure(v1_users_configure)
                    .configure(v1_bank_configure)
                    .configure(v1_barcode_configure)
                    .configure(v1_cost_centers_configure)
                    .configure(v1_currency_configure)
                    .configure(v1_dashboard_configure)
                    .configure(v1_documents_configure)
                    .configure(v1_feature_flags_configure)
                    .configure(v1_files_configure)
                    .configure(v1_import_configure)
                    .configure(v1_ip_whitelist_configure)
                    .configure(v1_mfa_configure)
                    .configure(v1_product_variants_configure)
                    .configure(v1_purchase_requests_configure)
                    .configure(v1_rate_limits_configure)
                    .configure(v1_purchase_orders_configure)
                    .configure(v1_resilience_configure)
                    .configure(v1_shifts_configure)
                    .configure(v1_subscriptions_configure)
                    .configure(v1_forecasting_configure)
                    .configure(v1_workflows_configure)
                    .configure(v1_goods_receipts_configure)
                    .configure(v1_graphql_configure)
                    .configure(v1_cari_configure)
                    .configure(v1_companies_configure)
                    .configure(v1_stock_configure)
                    .configure(v1_invoice_configure)
                    .configure(v1_sales_configure)
                    .configure(v1_hr_configure)
                    .configure(v1_accounting_configure)
                    .configure(v1_project_configure)
                    .configure(v1_manufacturing_configure)
                    .configure(v1_crm_configure)
                    .configure(v1_chart_of_accounts_configure)
                    .configure(v1_custom_fields_configure)
                    .configure(v1_tenant_configure)
                    .configure(v1_assets_configure)
                    .configure(v1_audit_configure)
                    .configure(v1_settings_configure)
                    .configure(v1_api_keys_configure)
                    .configure(v1_jobs_configure)
                    .configure(v1_ldap_configure)
                    .configure(v1_push_notifications_configure)
                    .configure(v1_notifications_configure)
                    .configure(v1_observability_configure)
                    .configure(v1_reports_configure)
                    .configure(v1_events_configure)
                    .configure(v1_search_configure)
                    .configure(v1_tax_configure)
                    .configure(v1_efatura_configure)
                    .configure(v1_earchive_configure)
                    .configure(v1_edefter_configure)
                    .configure(v1_edefter_blockchain_configure)
                    .configure(v1_customer_portal_configure)
                    .configure(v1_vendor_portal_configure)
                    .configure(v1_webhooks_configure)
                    .configure(v1_archive_configure),
            )
            // Swagger UI
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(&bind_addr)?
    .shutdown_timeout(30) // Graceful shutdown: 30 seconds for in-flight HTTP requests
    .run();

    // Race the HTTP server against SIGTERM/SIGINT. On signal, signal the
    // background workers to stop, then wait for them with a 5s budget
    // (anything past that risks a hung restart loop in the orchestrator).
    tokio::select! {
        result = server => {
            result?;
        }
        _ = shutdown_signal() => {
            tracing::info!("Shutdown signal received, draining background workers");
        }
    }

    // Drain background workers within a hard 5s budget. The channels are
    // bounded at capacity 1 so the sends cannot block; the joins are
    // bounded by tokio::time::timeout so a stuck worker cannot hang us.
    // The audit writer drains naturally: the server future above has been
    // awaited/cancelled, so the audit_sender Arc clones inside the
    // middleware have been dropped, the mpsc channel closes, and the
    // writer's recv() returns None — which then flushes its buffer and
    // exits.
    let drain = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let _ = job_shutdown_tx.send(()).await;
        let _ = obs_shutdown_tx.send(()).await;
        let _ = job_cron_shutdown_tx.send(()).await;
        let _ = job_handle.await;
        let _ = obs_handle.await;
        let _ = job_cron_handle.await;
        let _ = audit_handle.await;
    })
    .await;

    match drain {
        Ok(()) => tracing::info!("All background workers stopped cleanly"),
        Err(_) => {
            tracing::error!("Background workers did not stop within 5 seconds; exiting anyway")
        }
    }

    Ok(())
}

/// Wait for SIGTERM (production) or Ctrl-C (dev) to coordinate shutdown.
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl-C"),
        _ = terminate => tracing::info!("Received SIGTERM"),
    }
}
