//! Turerp ERP - Main application entry point
//!
//! Run with: cargo run --package turerp
//! With PostgreSQL: cargo run --package turerp --features postgres

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use turerp::config::Config;
use turerp::middleware::{
    audit::spawn_audit_writer, AuditLoggingMiddleware, JwtAuthMiddleware, MetricsMiddleware,
    RateLimitMiddleware, RequestIdMiddleware, TenantMiddleware,
};

use tokio::sync::mpsc;
use turerp::api::{
    v1_accounting_configure, v1_assets_configure, v1_audit_configure, v1_auth_configure,
    v1_cari_configure, v1_crm_configure, v1_feature_flags_configure, v1_hr_configure,
    v1_invoice_configure, v1_manufacturing_configure, v1_product_variants_configure,
    v1_project_configure, v1_purchase_requests_configure, v1_sales_configure, v1_stock_configure,
    v1_tenant_configure, v1_users_configure, ApiDoc,
};
use turerp::middleware::audit::{AuditEvent, AUDIT_CHANNEL_CAPACITY};
use turerp::setup_logging;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[cfg(feature = "postgres")]
use turerp::app::AppState;

/// Liveness probe - always returns 200 if the process is running
async fn health_live() -> actix_web::Result<actix_web::HttpResponse> {
    Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "turerp-erp",
        "version": env!("CARGO_PKG_VERSION")
    })))
}

/// Readiness probe (in-memory mode) - always ready
#[cfg(not(feature = "postgres"))]
async fn health_ready() -> actix_web::Result<actix_web::HttpResponse> {
    Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "turerp-erp",
        "version": env!("CARGO_PKG_VERSION"),
        "storage": "in-memory"
    })))
}

/// Readiness probe (PostgreSQL mode) - checks database connectivity
#[cfg(feature = "postgres")]
async fn health_ready(
    app_state: web::Data<AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    let pool: &sqlx::PgPool = &*app_state.db_pool;

    let start = std::time::Instant::now();
    let db_result = sqlx::query("SELECT 1").execute(pool).await;
    let latency_ms = start.elapsed().as_millis();

    match db_result {
        Ok(_) => Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
            "status": "ok",
            "service": "turerp-erp",
            "version": env!("CARGO_PKG_VERSION"),
            "storage": "postgresql",
            "database": "healthy",
            "latency_ms": latency_ms
        }))),
        Err(e) => {
            tracing::error!("Database health check failed: {}", e);
            Ok(
                actix_web::HttpResponse::ServiceUnavailable().json(serde_json::json!({
                    "status": "unhealthy",
                    "service": "turerp-erp",
                    "version": env!("CARGO_PKG_VERSION"),
                    "storage": "postgresql",
                    "database": "unhealthy",
                    "latency_ms": latency_ms
                })),
            )
        }
    }
}

/// Backwards-compatible health check endpoint (aliases to readiness)
#[cfg(not(feature = "postgres"))]
async fn health_check() -> actix_web::Result<actix_web::HttpResponse> {
    health_ready().await
}

/// Backwards-compatible health check endpoint (aliases to readiness)
#[cfg(feature = "postgres")]
async fn health_check(
    app_state: web::Data<AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    health_ready(app_state).await
}

/// Metrics endpoint - exposes Prometheus-format metrics
async fn metrics_endpoint() -> actix_web::Result<actix_web::HttpResponse> {
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Setup logging
    setup_logging();

    tracing::info!("Starting Turerp ERP server...");

    // Load configuration
    let config = Config::new().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config from env: {}, using defaults", e);
        Config::default()
    });

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
        turerp::app::create_app_state_in_memory(&config)
    };

    #[cfg(feature = "postgres")]
    let app_state = {
        tracing::info!("Using PostgreSQL storage (production mode)");
        turerp::app::create_app_state(&config).await
    };

    // Set up audit log channel (bounded to prevent unbounded memory growth under load)
    let (audit_tx, audit_rx) = mpsc::channel::<AuditEvent>(AUDIT_CHANNEL_CAPACITY);
    let audit_sender: std::sync::Arc<mpsc::Sender<AuditEvent>> = std::sync::Arc::new(audit_tx);
    let audit_svc = app_state.audit_service.get_ref().clone();
    spawn_audit_writer(audit_rx, audit_svc);

    HttpServer::new(move || {
        #[cfg(feature = "postgres")]
        let app = App::new()
            // Security middlewares (ORDER MATTERS!)
            // First wrap = outermost (touches request first, response last).
            // Last wrap = innermost (touches request last, response first).
            .wrap(middleware::Compress::default()) // Outermost: response compression
            .wrap(configure_cors(&config.cors)) // CORS handling
            .wrap(middleware::Logger::default()) // Access logging
            .wrap(AuditLoggingMiddleware::with_sender(audit_sender.clone())) // Audit logging
            .wrap(JwtAuthMiddleware::new(
                app_state.jwt_service.get_ref().clone(),
            )) // JWT validation
            .wrap(RateLimitMiddleware::with_config(&config.rate_limit)) // Rate limiting
            .wrap(MetricsMiddleware::new()) // Metrics collection
            .wrap(TenantMiddleware) // Tenant context extraction (after auth)
            .wrap(RequestIdMiddleware) // Innermost: request ID for tracing
            .app_data(web::JsonConfig::default().limit(1024 * 1024)) // 1MB JSON limit
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .app_data(app_state.cari_service.clone())
            .app_data(app_state.stock_service.clone())
            .app_data(app_state.invoice_service.clone())
            .app_data(app_state.sales_service.clone())
            .app_data(app_state.hr_service.clone())
            .app_data(app_state.accounting_service.clone())
            .app_data(app_state.project_service.clone())
            .app_data(app_state.manufacturing_service.clone())
            .app_data(app_state.qc_service.clone())
            .app_data(app_state.crm_service.clone())
            .app_data(app_state.tenant_service.clone())
            .app_data(app_state.tenant_config_service.clone())
            .app_data(app_state.assets_service.clone())
            .app_data(app_state.feature_service.clone())
            .app_data(app_state.product_service.clone())
            .app_data(app_state.purchase_service.clone())
            .app_data(app_state.audit_service.clone())
            .app_data(app_state.db_pool.clone());

        #[cfg(not(feature = "postgres"))]
        let app = App::new()
            // Security middlewares (ORDER MATTERS!)
            // First wrap = outermost (touches request first, response last).
            // Last wrap = innermost (touches request last, response first).
            .wrap(middleware::Compress::default()) // Outermost: response compression
            .wrap(configure_cors(&config.cors)) // CORS handling
            .wrap(middleware::Logger::default()) // Access logging
            .wrap(AuditLoggingMiddleware::with_sender(audit_sender.clone())) // Audit logging
            .wrap(JwtAuthMiddleware::new(
                app_state.jwt_service.get_ref().clone(),
            )) // JWT validation
            .wrap(RateLimitMiddleware::with_config(&config.rate_limit)) // Rate limiting
            .wrap(MetricsMiddleware::new()) // Metrics collection
            .wrap(TenantMiddleware) // Tenant context extraction (after auth)
            .wrap(RequestIdMiddleware) // Innermost: request ID for tracing
            .app_data(web::JsonConfig::default().limit(1024 * 1024)) // 1MB JSON limit
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .app_data(app_state.cari_service.clone())
            .app_data(app_state.stock_service.clone())
            .app_data(app_state.invoice_service.clone())
            .app_data(app_state.sales_service.clone())
            .app_data(app_state.hr_service.clone())
            .app_data(app_state.accounting_service.clone())
            .app_data(app_state.project_service.clone())
            .app_data(app_state.manufacturing_service.clone())
            .app_data(app_state.qc_service.clone())
            .app_data(app_state.crm_service.clone())
            .app_data(app_state.tenant_service.clone())
            .app_data(app_state.tenant_config_service.clone())
            .app_data(app_state.assets_service.clone())
            .app_data(app_state.feature_service.clone())
            .app_data(app_state.product_service.clone())
            .app_data(app_state.purchase_service.clone())
            .app_data(app_state.audit_service.clone());

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
                    .configure(v1_feature_flags_configure)
                    .configure(v1_product_variants_configure)
                    .configure(v1_purchase_requests_configure)
                    .configure(v1_cari_configure)
                    .configure(v1_stock_configure)
                    .configure(v1_invoice_configure)
                    .configure(v1_sales_configure)
                    .configure(v1_hr_configure)
                    .configure(v1_accounting_configure)
                    .configure(v1_project_configure)
                    .configure(v1_manufacturing_configure)
                    .configure(v1_crm_configure)
                    .configure(v1_tenant_configure)
                    .configure(v1_assets_configure)
                    .configure(v1_audit_configure),
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
