use actix_web::{web, App, HttpServer, middleware::Logger};
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
use handlers::{auth_config, task_config, file_config, health};

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
        handlers::task::create_task,
        handlers::task::get_tasks,
        handlers::task::get_task,
        handlers::task::update_task,
        handlers::task::delete_task,
        handlers::task::get_teams,
        handlers::file::upload_file,
        handlers::file::get_task_attachments,
        handlers::file::download_file,
        handlers::file::delete_attachment,
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
            models::task::Task,
            models::task::TaskResponse,
            models::task::CreateTaskRequest,
            models::task::UpdateTaskRequest,
            models::task::Team,
            models::auth::ApiResponse<models::task::TaskResponse>,
            models::auth::ApiResponse<Vec<models::task::TaskResponse>>,
            models::auth::ApiResponse<Vec<models::task::Team>>,
            models::file::TaskAttachment,
            models::file::AttachmentResponse,
            models::file::UploadResponse,
            models::file::FileUploadInfo,
            models::file::TaskAttachmentSimple,
            models::file::UploadFileRequest,
            models::auth::ApiResponse<models::file::UploadResponse>,
            models::auth::ApiResponse<Vec<models::file::AttachmentResponse>>,
            utils::errors::ServiceError
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "tasks", description = "Task management endpoints"),
        (name = "teams", description = "Team management endpoints"),
        (name = "attachments", description = "File attachment endpoints")
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
            .configure(health::configure)
            .configure(auth_config)
            .configure(task_config)
            .configure(file_config)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi())
            )
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
