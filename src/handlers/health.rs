use actix_web::{web, HttpResponse, Result};
use serde_json::json;

use crate::models::common::ApiResponse;

pub async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        json!({
            "status": "healthy",
            "service": "kanban-api",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
        "Service is healthy"
    )))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health_check));
}
