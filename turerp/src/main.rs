//! Turerp ERP - Main application entry point
//!
//! Run with: cargo run --package turerp

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Setup logging
    setup_logging();

    tracing::info!("Starting Turerp ERP server...");

    // Load configuration
    let config = turerp::Config::new().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config: {}, using defaults", e);
        turerp::Config::default()
    });

    let bind_addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Server will bind to: {}", bind_addr);

    // Create application state
    let app_state = create_app_state();

    HttpServer::new(move || {
        App::new()
            // Global middleware
            .wrap(middleware::Logger::default())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials(),
            )
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
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
