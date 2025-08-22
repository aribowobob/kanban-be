use actix_web::{web, App, HttpResponse, HttpServer, Result, middleware::Logger};
use actix_cors::Cors;
use env_logger;

mod config;
mod database;

use config::AppConfig;
use database::Database;

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
async fn api_info(config: web::Data<AppConfig>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "name": "Kanban Backend API",
        "version": "0.1.0",
        "description": "REST API for Kanban board application",
        "environment": config.environment,
        "features": {
            "database": "PostgreSQL",
            "authentication": "JWT",
            "file_upload": if config.has_cloudinary_config() { "Cloudinary" } else { "Not configured" },
            "documentation": if config.is_development() { "Swagger UI available at /swagger-ui/" } else { "Available in development mode" }
        }
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Load and validate configuration
    let config = AppConfig::from_env()
        .expect("Failed to load configuration");
    
    config.validate()
        .expect("Configuration validation failed");

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
        println!("📖 Swagger UI will be available at: http://localhost:{}/swagger-ui/", config.port);
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
            .route("/", web::get().to(api_info))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
