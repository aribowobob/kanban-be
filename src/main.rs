use actix_web::{web, App, HttpResponse, HttpServer, Result};
use actix_cors::Cors;
use dotenv::dotenv;
use env_logger;
use std::env;

// Basic health check endpoint
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "message": "Kanban Backend API is running"
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize logger
    env_logger::init();
    
    // Get configuration from environment
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");
    
    let frontend_urls = env::var("FRONTEND_URLS")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    
    println!("🚀 Starting Kanban Backend API on port {}", port);
    println!("📋 Allowed frontend URLs: {}", frontend_urls);
    
    // Parse allowed origins for CORS
    let allowed_origins: Vec<String> = frontend_urls
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    
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
        for origin in &allowed_origins {
            cors = cors.allowed_origin(origin);
        }
        
        App::new()
            .wrap(cors)
            .wrap(actix_web::middleware::Logger::default())
            .route("/health", web::get().to(health_check))
            .route("/", web::get().to(|| async {
                HttpResponse::Ok().json(serde_json::json!({
                    "name": "Kanban Backend API",
                    "version": "0.1.0",
                    "description": "REST API for Kanban board application"
                }))
            }))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
