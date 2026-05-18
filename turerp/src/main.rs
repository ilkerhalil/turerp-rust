//! Turerp ERP - Main application entry point
//!
//! Run with: cargo run --package turerp
//! With PostgreSQL: cargo run --package turerp --features postgres

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use secrecy::ExposeSecret;
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
    v1_vendor_portal_configure, v1_webhooks_configure, v1_workflows_configure, ApiDoc,
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
#[cfg(not(feature = "postgres"))]
async fn health_ready(
    app_state: web::Data<AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    let cache = app_state.infra.cache_service.get_ref();
    let cache_result = cache.health_check().await;

    match cache_result {
        Ok(_) => Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
            "status": "ok",
            "service": "turerp-erp",
            "version": env!("CARGO_PKG_VERSION"),
            "storage": "in-memory",
            "cache": "healthy"
        }))),
        Err(e) => {
            tracing::error!("Cache health check failed: {}", e);
            Ok(
                actix_web::HttpResponse::ServiceUnavailable().json(serde_json::json!({
                    "status": "unhealthy",
                    "service": "turerp-erp",
                    "version": env!("CARGO_PKG_VERSION"),
                    "storage": "in-memory",
                    "cache": "unhealthy"
                })),
            )
        }
    }
}

/// Readiness probe (PostgreSQL mode) - checks database and cache connectivity
#[cfg(feature = "postgres")]
async fn health_ready(
    app_state: web::Data<AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    let pool: &sqlx::PgPool = app_state.infra.db_pool.as_ref();

    let db_start = std::time::Instant::now();
    let db_result = sqlx::query("SELECT 1").execute(pool).await;
    let db_latency_ms = db_start.elapsed().as_millis();

    let cache = app_state.infra.cache_service.get_ref();
    let cache_start = std::time::Instant::now();
    let cache_result = cache.health_check().await;
    let cache_latency_ms = cache_start.elapsed().as_millis();

    let db_healthy = db_result.is_ok();
    let cache_healthy = cache_result.is_ok();

    if !db_healthy {
        tracing::error!("Database health check failed: {:?}", db_result.unwrap_err());
    }
    if !cache_healthy {
        tracing::error!("Cache health check failed: {:?}", cache_result.unwrap_err());
    }

    if db_healthy && cache_healthy {
        Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
            "status": "ok",
            "service": "turerp-erp",
            "version": env!("CARGO_PKG_VERSION"),
            "storage": "postgresql",
            "database": "healthy",
            "database_latency_ms": db_latency_ms,
            "cache": "healthy",
            "cache_latency_ms": cache_latency_ms
        })))
    } else {
        Ok(
            actix_web::HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "status": "unhealthy",
                "service": "turerp-erp",
                "version": env!("CARGO_PKG_VERSION"),
                "storage": "postgresql",
                "database": if db_healthy { "healthy" } else { "unhealthy" },
                "database_latency_ms": db_latency_ms,
                "cache": if cache_healthy { "healthy" } else { "unhealthy" },
                "cache_latency_ms": cache_latency_ms
            })),
        )
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

    for origin in &cors_config.allowed_origins {
        cors = cors.allowed_origin(origin);
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

    if cors_config.allow_credentials {
        cors = cors.supports_credentials();
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
    // Load configuration first (needed for OTLP settings before subscriber init)
    let mut config = Config::new().unwrap_or_else(|e| {
        eprintln!("Failed to load config from env: {}, using defaults", e);
        Config::default()
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

    // Create application state with config
    #[cfg(not(feature = "postgres"))]
    let app_state = {
        tracing::info!("Using in-memory storage (development mode)");
        turerp::app::create_app_state_in_memory(&config).unwrap_or_else(|e| {
            tracing::error!("Failed to create app state: {}", e);
            std::process::exit(1);
        })
    };

    #[cfg(feature = "postgres")]
    let app_state = {
        tracing::info!("Using PostgreSQL storage (production mode)");
        turerp::app::create_app_state(&config)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to create app state: {}", e);
                std::process::exit(1);
            })
    };

    // Start background job executor
    let job_executor = turerp::common::job_executor::JobExecutor::new(
        app_state.infra.job_scheduler.clone(),
        app_state.infra.import_service.clone(),
        app_state.document.file_storage.clone(),
    );
    job_executor.start().await;

    // Start background observability evaluator (alert rules + SLO + SLI collection)
    let (_obs_shutdown_tx, obs_shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
    {
        let observability_service = app_state.observability_service.get_ref().clone();
        let notification_service = app_state.infra.notification_service.clone().into_inner();
        let evaluator = turerp::common::background_evaluator::BackgroundEvaluator::new(
            observability_service,
            notification_service,
        );
        evaluator.start(obs_shutdown_rx);
        tracing::info!("Background observability evaluator started");
    }

    // Build rate-limit middleware with shared stats store so the dashboard can read them
    let rate_limit_middleware = {
        let stats_store = app_state.infra.rate_limit_stats.get_ref().clone();
        RateLimitMiddleware::with_config(&config.rate_limit).with_stats_store(stats_store)
    };

    // Set up audit log channel (bounded to prevent unbounded memory growth under load)
    let (audit_tx, audit_rx) = mpsc::channel::<AuditEvent>(AUDIT_CHANNEL_CAPACITY);
    let audit_sender: std::sync::Arc<mpsc::Sender<AuditEvent>> = std::sync::Arc::new(audit_tx);
    let audit_svc = app_state.analytics.audit_service.get_ref().clone();
    spawn_audit_writer(audit_rx, audit_svc);

    let is_production = config.is_production();
    let security_headers_config = config.security_headers.clone();

    // Macro to build the common Actix app (middleware + app_data).
    // Avoids duplicating ~100 lines for postgres vs in-memory feature flags.
    macro_rules! build_app_core {
        ($app:expr) => {{
            $app
                // Security middlewares (ORDER MATTERS!)
                // First wrap = outermost (touches request first, response last).
                // Last wrap = innermost (touches request last, response first).
                .wrap(middleware::Compress::default()) // Outermost: response compression
                .wrap(SecurityHeadersMiddleware::new(
                    &security_headers_config,
                    is_production,
                )) // Security headers
                .wrap(rate_limit_middleware.clone()) // Rate limiting: outer layer so unauthenticated requests count
                .wrap(configure_cors(&config.cors)) // CORS handling
                .wrap(middleware::Logger::default()) // Access logging
                .wrap(AuditLoggingMiddleware::with_sender(audit_sender.clone())) // Audit logging
                .wrap(JwtAuthMiddleware::new(
                    app_state.auth.jwt_service.get_ref().clone(),
                )) // JWT validation
                .wrap(IdempotencyMiddleware::in_memory()) // Idempotency key caching
                .wrap(MetricsMiddleware::new()) // Metrics collection
                .wrap(TenantMiddleware) // Tenant context extraction (after auth)
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
                ) // IP whitelist check (after tenant context)
                .wrap(RequestIdMiddleware) // Request ID for tracing
                .wrap(TracingMiddleware) // Innermost: structured request/response tracing
                .app_data(web::Data::new(app_state.clone())) // Full AppState for health probes
                .app_data(web::JsonConfig::default().limit(1024 * 1024)) // 1MB JSON limit
                .app_data(app_state.auth.auth_service.clone())
                .app_data(app_state.auth.user_service.clone())
                .app_data(app_state.auth.jwt_service.clone())
                .app_data(app_state.commerce.cari_service.clone())
                .app_data(app_state.commerce.stock_service.clone())
                .app_data(app_state.commerce.invoice_service.clone())
                .app_data(app_state.commerce.sales_service.clone())
                .app_data(app_state.commerce.barcode_service.clone())
                .app_data(app_state.hr.hr_service.clone())
                .app_data(app_state.hr.sgk_payroll_service.clone())
                .app_data(app_state.finance.accounting_service.clone())
                .app_data(app_state.project.project_service.clone())
                .app_data(app_state.project.manufacturing_service.clone())
                .app_data(app_state.project.qc_service.clone())
                .app_data(app_state.project.crm_service.clone())
                .app_data(app_state.chart_of_accounts_service.clone())
                .app_data(app_state.custom_field_service.clone())
                .app_data(app_state.admin.tenant_service.clone())
                .app_data(app_state.admin.tenant_config_service.clone())
                .app_data(app_state.assets_service.clone())
                .app_data(app_state.feature_service.clone())
                .app_data(app_state.commerce.product_service.clone())
                .app_data(app_state.commerce.purchase_service.clone())
                .app_data(app_state.analytics.audit_service.clone())
                .app_data(app_state.admin.settings_service.clone())
                .app_data(app_state.admin.api_key_service.clone())
                .app_data(app_state.admin.ip_whitelist_service.clone())
                .app_data(app_state.infra.job_scheduler.clone())
                .app_data(app_state.infra.event_bus.clone())
                .app_data(app_state.infra.notification_service.clone())
                .app_data(app_state.infra.report_engine.clone())
                .app_data(app_state.analytics.forecasting_service.clone())
                .app_data(app_state.analytics.subscription_service.clone())
                .app_data(app_state.observability_service.clone())
                .app_data(app_state.hr.shift_service.clone())
                .app_data(app_state.infra.tracing_service.clone())
                .app_data(app_state.infra.db_router.clone())
                .app_data(app_state.i18n.clone())
                .app_data(app_state.finance.tax_service.clone())
                .app_data(app_state.finance.bank_service.clone())
                .app_data(app_state.finance.cost_center_service.clone())
                .app_data(app_state.finance.currency_service.clone())
                .app_data(app_state.integration.efatura_service.clone())
                .app_data(app_state.integration.earchive_service.clone())
                .app_data(app_state.integration.edefter_service.clone())
                .app_data(app_state.integration.blockchain_ledger_service.clone())
                .app_data(app_state.integration.customer_portal_service.clone())
                .app_data(app_state.integration.vendor_portal_service.clone())
                .app_data(app_state.document.document_service.clone())
                .app_data(app_state.document.dashboard_service.clone())
                .app_data(app_state.document.file_storage.clone())
                .app_data(app_state.integration.webhook_service.clone())
                .app_data(app_state.integration.workflow_service.clone())
                .app_data(app_state.infra.cache_service.clone())
                .app_data(app_state.infra.search_service.clone())
                .app_data(app_state.infra.rate_limit_stats.clone())
                .app_data(app_state.analytics.archive_service.clone())
                .app_data(app_state.infra.import_service.clone())
                .app_data(app_state.commerce.inter_company_service.clone())
                .app_data(app_state.commerce.company_service.clone())
                .app_data(app_state.infra.circuit_breaker_registry.clone())
                .app_data(app_state.infra.retry_stats.clone())
                .app_data(app_state.auth.mfa_service.clone())
                .app_data(app_state.ldap_service.clone())
        }};
    }

    HttpServer::new(move || {
        #[cfg(feature = "postgres")]
        let app = build_app_core!(App::new()).app_data(app_state.infra.db_pool.clone());

        #[cfg(not(feature = "postgres"))]
        let app = build_app_core!(App::new());

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
                    .configure(v1_notifications_configure)
                    .configure(v1_push_notifications_configure)
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
    .shutdown_timeout(30) // Graceful shutdown: 30 seconds
    .run()
    .await
}
