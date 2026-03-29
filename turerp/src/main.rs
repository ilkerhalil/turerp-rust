//! Turerp ERP - Main application entry point
//!
//! Run with: cargo run --package turerp

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use turerp::config::Config;

use turerp::api::{auth_configure, users_configure, ApiDoc};
use turerp::app::create_app_state;
use turerp::setup_logging;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Health check endpoint
async fn health_check() -> actix_web::Result<actix_web::HttpResponse> {
    Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "turerp-erp"
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
    let app_state = create_app_state(&config);

    HttpServer::new(move || {
        App::new()
            // Global middleware
            .wrap(middleware::Logger::default())
            .wrap(configure_cors(&config.cors))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            // Health check
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
    .run()
    .await
}
