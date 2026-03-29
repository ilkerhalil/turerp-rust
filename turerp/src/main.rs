//! Turerp ERP - Main application entry point
//!
//! Run with: cargo run --package turerp
//! With PostgreSQL: cargo run --package turerp --features postgres

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use turerp::config::Config;
use turerp::middleware::{RateLimitMiddleware, RequestIdMiddleware};

use turerp::api::{auth_configure, users_configure, ApiDoc};
use turerp::setup_logging;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[cfg(feature = "postgres")]
use turerp::app::AppState;

/// Health check endpoint (in-memory mode)
#[cfg(not(feature = "postgres"))]
async fn health_check() -> actix_web::Result<actix_web::HttpResponse> {
    Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "turerp-erp",
        "storage": "in-memory"
    })))
}

/// Health check endpoint (PostgreSQL mode)
#[cfg(feature = "postgres")]
async fn health_check(
    app_state: web::Data<AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    // Get the pool from web::Data
    // web::Data<Arc<PgPool>> contains Arc<PgPool>, and we need &PgPool for sqlx
    let pool: &sqlx::PgPool = &*app_state.db_pool;

    // Test database connectivity
    let db_health = match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => "healthy",
        Err(e) => {
            tracing::error!("Database health check failed: {}", e);
            "unhealthy"
        }
    };

    let status = if db_health == "healthy" {
        "ok"
    } else {
        "degraded"
    };

    Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": status,
        "service": "turerp-erp",
        "storage": "postgresql",
        "database": db_health
    })))
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

    HttpServer::new(move || {
        #[cfg(feature = "postgres")]
        let app = App::new()
            // Security middlewares (ORDER MATTERS!)
            .wrap(RequestIdMiddleware) // 1. Request ID for tracing
            .wrap(RateLimitMiddleware::new()) // 2. Rate limiting (before auth)
            .wrap(middleware::Logger::default()) // 3. Logging
            .wrap(configure_cors(&config.cors)) // 4. CORS
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .app_data(app_state.db_pool.clone());

        #[cfg(not(feature = "postgres"))]
        let app = App::new()
            // Security middlewares (ORDER MATTERS!)
            .wrap(RequestIdMiddleware) // 1. Request ID for tracing
            .wrap(RateLimitMiddleware::new()) // 2. Rate limiting (before auth)
            .wrap(middleware::Logger::default()) // 3. Logging
            .wrap(configure_cors(&config.cors)) // 4. CORS
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone());

        app // Health check
            .route("/health", web::get().to(health_check))
            // API routes
            .service(
                web::scope("/api")
                    .configure(auth_configure)
                    .configure(users_configure),
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
