use actix_web::{web, HttpResponse, Result};
use serde_json::json;

use crate::models::auth::ApiResponse;
use crate::database::{Database, DatabaseStats};

pub async fn health_check(db: web::Data<Database>) -> Result<HttpResponse> {
    match db.health_check().await {
        Ok(_) => {
            let stats = db.get_stats().await.unwrap_or_else(|_| DatabaseStats {
                users: 0,
                teams: 0,
                tasks: 0,
                attachments: 0,
            });

            Ok(HttpResponse::Ok().json(ApiResponse::success(
                "Kanban Backend API is running",
                json!({
                    "status": "ok",
                    "database": "connected",
                    "stats": {
                        "users": stats.users,
                        "teams": stats.teams,
                        "tasks": stats.tasks,
                        "attachments": stats.attachments
                    }
                })
            )))
        }
        Err(e) => {
            log::error!("Database health check failed: {}", e);
            Ok(HttpResponse::ServiceUnavailable().json(json!({
                "status": "error",
                "message": "Database connection failed",
                "error": e.to_string()
            })))
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health_check));
}
