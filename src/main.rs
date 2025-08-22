use actix_web::{web, App, HttpResponse, HttpServer, Result, middleware::Logger};
use actix_cors::Cors;
use env_logger;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use utoipa::{Modify, openapi::security::{SecurityScheme, HttpAuthScheme, Http}};

mod config;
mod database;
mod models;
mod services;
mod handlers;
mod middleware;
mod utils;

use config::AppConfig;
use database::Database;
use handlers::auth_config;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer))
            )
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::auth::login,
        handlers::auth::logout,
        handlers::auth::get_me,
    ),
    components(
        schemas(
            models::auth::LoginRequest,
            models::auth::LoginResponseData,
            models::auth::UserResponse,
            models::auth::ApiResponse<models::auth::LoginResponseData>,
            models::auth::ApiResponse<models::auth::UserResponse>,
            models::auth::ApiResponse<bool>,
            models::auth::ErrorResponse,
            utils::errors::ServiceError
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication endpoints")
    ),
    info(
        title = "Kanban Backend API",
        version = "0.1.0",
        description = "REST API for Kanban board application with JWT authentication",
        contact(
            name = "API Support",
            email = "admin@kanban.com"
        )
    )
)]
struct ApiDoc;

// Basic health check endpoint
async fn health_check(db: web::Data<Database>) -> Result<HttpResponse> {
    match db.health_check().await {
        Ok(_) => {
            let stats = db.get_stats().await.unwrap_or_else(|_| database::DatabaseStats {
                users: 0,
                teams: 0,
                tasks: 0,
                attachments: 0,
            });

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "ok",
                "message": "Kanban Backend API is running",
                "database": "connected",
                "stats": {
                    "users": stats.users,
                    "teams": stats.teams,
                    "tasks": stats.tasks,
                    "attachments": stats.attachments
                }
            })))
        }
        Err(e) => {
            log::error!("Database health check failed: {}", e);
            Ok(HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "status": "error",
                "message": "Database connection failed",
                "error": e.to_string()
            })))
        }
    }
}

// API info endpoint
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Load and validate configuration
    let config = AppConfig::from_env()
        .expect("Failed to load configuration");

    // Create database connection
    let database = Database::new(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Run database checks
    if let Err(e) = database.health_check().await {
        log::error!("Database health check failed: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = database.check_tables().await {
        log::error!("Database table check failed: {}", e);
        std::process::exit(1);
    }

    // Log database stats
    if let Ok(stats) = database.get_stats().await {
        stats.log_stats();
    }

    println!("🚀 Starting Kanban Backend API on port {}", config.port);
    println!("📋 Allowed frontend URLs: {:?}", config.frontend_urls);
    println!("🔧 Environment: {}", config.environment);
    
    if config.is_development() {
        println!("📖 Swagger UI available at: http://localhost:{}/swagger-ui/", config.port);
    }

    let port = config.port;
    let server_config = web::Data::new(config.clone());
    let db_data = web::Data::new(database);

    HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                "Authorization",
                "Content-Type",
                "Accept",
                "Origin",
                "X-Requested-With",
            ])
            .supports_credentials();
        
        // Add allowed origins
        for origin in &config.frontend_urls {
            cors = cors.allowed_origin(origin);
        }
        
        App::new()
            .app_data(server_config.clone())
            .app_data(db_data.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .route("/health", web::get().to(health_check))
            .configure(auth_config)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi())
            )
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
