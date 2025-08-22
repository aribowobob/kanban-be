use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use futures_util::TryStreamExt;
use sqlx::Row;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Serialize, Deserialize};

use crate::config::AppConfig;
use crate::Database;
use crate::models::auth::ApiResponse;
use crate::models::file::{AttachmentResponse, UploadResponse, UploadFileRequest};
use crate::utils::errors::ServiceError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user id)
    pub username: String,
    pub name: String,
    pub exp: usize, // Expiration time (Unix timestamp)
    pub iat: usize, // Issued at (Unix timestamp)
}

// Helper function to extract user ID from JWT token
async fn get_user_from_token(req: &HttpRequest, config: &AppConfig) -> Result<i32, ServiceError> {
    let auth_header = req.headers().get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    let token = auth_header.ok_or_else(|| {
        ServiceError::Unauthorized("Authentication required".to_string())
    })?;

    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| ServiceError::Unauthorized("Invalid token".to_string()))?;

    let user_id: i32 = claims.claims.sub.parse()
        .map_err(|_| ServiceError::Unauthorized("Invalid user ID in token".to_string()))?;

    Ok(user_id)
}

// Helper function to ensure upload directory exists
fn ensure_upload_dir() -> Result<PathBuf, ServiceError> {
    let upload_dir = Path::new("uploads");
    if !upload_dir.exists() {
        std::fs::create_dir_all(upload_dir)
            .map_err(|e| {
                log::error!("Failed to create upload directory: {}", e);
                ServiceError::InternalError("Failed to create upload directory".to_string())
            })?;
    }
    Ok(upload_dir.to_path_buf())
}

// Helper function to validate file type and size
fn validate_file(file_name: &str, file_size: usize) -> Result<String, ServiceError> {
    // Max file size: 10MB
    const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
    
    if file_size > MAX_FILE_SIZE {
        return Err(ServiceError::ValidationError(
            "File size exceeds 10MB limit".to_string()
        ));
    }

    // Allowed file extensions
    let allowed_extensions = [
        "jpg", "jpeg", "png", "gif", "pdf", "doc", "docx", 
        "txt", "zip", "rar", "json", "xml", "csv", "xlsx"
    ];
    
    let extension = Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default();

    if !allowed_extensions.contains(&extension.as_str()) {
        return Err(ServiceError::ValidationError(
            format!("File type '{}' not allowed", extension)
        ));
    }

    // Determine MIME type based on extension
    let mime_type = match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png", 
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "txt" => "text/plain",
        "zip" => "application/zip",
        "rar" => "application/x-rar-compressed",
        "json" => "application/json",
        "xml" => "application/xml",
        "csv" => "text/csv",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    };

    Ok(mime_type.to_string())
}

/// Upload a file attachment to a task
#[utoipa::path(
    post,
    path = "/api/tasks/{task_id}/attachments",
    tag = "attachments",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("task_id" = i32, Path, description = "Task ID to attach file to")
    ),
    request_body(
        content = inline(UploadFileRequest),
        description = "File to upload as multipart/form-data",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 201, description = "File uploaded successfully", body = ApiResponse<UploadResponse>),
        (status = 400, description = "Validation error", body = crate::utils::errors::ServiceError),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError),
        (status = 404, description = "Task not found", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn upload_file(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
    path: web::Path<i32>,
    mut payload: Multipart,
) -> Result<HttpResponse, ServiceError> {
    let task_id = path.into_inner();
    log::info!("POST /api/tasks/{}/attachments - Uploading file", task_id);

    let user_id = get_user_from_token(&req, &config).await?;

    // Check if task exists
    let task_exists = sqlx::query("SELECT id FROM tasks WHERE id = $1")
        .bind(task_id)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| {
            log::error!("Database error checking task: {}", e);
            ServiceError::DatabaseError("Failed to check task".to_string())
        })?;

    if task_exists.is_none() {
        return Err(ServiceError::NotFound("Task not found".to_string()));
    }

    let upload_dir = ensure_upload_dir()?;
    
    // Process multipart upload
    while let Some(mut field) = payload.try_next().await.map_err(|e| {
        log::error!("Multipart error: {}", e);
        ServiceError::ValidationError("Invalid multipart data".to_string())
    })? {
        let content_disposition = field.content_disposition();
        
        if let Some(file_name) = content_disposition.and_then(|cd| cd.get_filename()) {
            log::info!("Processing file: {}", file_name);
            
            // Clone the filename to avoid borrowing issues
            let file_name = file_name.to_string();
            
            // Generate unique file name
            let file_id = Uuid::new_v4();
            let extension = Path::new(&file_name)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("bin");
            let stored_file_name = format!("{}_{}.{}", task_id, file_id, extension);
            let file_path = upload_dir.join(&stored_file_name);

            // Collect file data and validate size
            let mut file_data = Vec::new();
            while let Some(chunk) = field.try_next().await.map_err(|e| {
                log::error!("File chunk error: {}", e);
                ServiceError::ValidationError("Error reading file data".to_string())
            })? {
                file_data.extend_from_slice(&chunk);
                // Check size during upload to prevent memory issues
                if file_data.len() > 10 * 1024 * 1024 {
                    return Err(ServiceError::ValidationError(
                        "File size exceeds 10MB limit".to_string()
                    ));
                }
            }

            let file_size = file_data.len();
            let mime_type = validate_file(&file_name, file_size)?;

            // Write file to disk
            let mut file = std::fs::File::create(&file_path)
                .map_err(|e| {
                    log::error!("Failed to create file: {}", e);
                    ServiceError::InternalError("Failed to save file".to_string())
                })?;

            file.write_all(&file_data)
                .map_err(|e| {
                    log::error!("Failed to write file: {}", e);
                    ServiceError::InternalError("Failed to save file".to_string())
                })?;

            // Save file info to database
            let attachment_row = sqlx::query(
                "INSERT INTO task_attachments (task_id, file_name, original_name, file_path, file_size, mime_type, uploaded_by) 
                 VALUES ($1, $2, $3, $4, $5, $6, $7) 
                 RETURNING id, task_id, file_name, original_name, file_path, file_size, mime_type, uploaded_by, created_at"
            )
            .bind(task_id)
            .bind(&stored_file_name)
            .bind(&file_name)
            .bind(file_path.to_string_lossy().to_string())
            .bind(file_size as i64)
            .bind(&mime_type)
            .bind(user_id)
            .fetch_one(&db.pool)
            .await
            .map_err(|e| {
                log::error!("Database error saving attachment: {}", e);
                // Clean up file if database insert fails
                let _ = std::fs::remove_file(&file_path);
                ServiceError::DatabaseError("Failed to save attachment info".to_string())
            })?;

            let attachment_response = AttachmentResponse {
                id: attachment_row.get("id"),
                task_id: attachment_row.get("task_id"),
                file_name: attachment_row.get("file_name"),
                original_name: attachment_row.get("original_name"),
                file_size: attachment_row.get("file_size"),
                mime_type: attachment_row.get("mime_type"),
                uploaded_by: attachment_row.get("uploaded_by"),
                download_url: format!("/api/tasks/{}/attachments/{}/download", task_id, attachment_row.get::<i32, _>("id")),
                created_at: attachment_row.get("created_at"),
            };

            let upload_response = UploadResponse {
                attachment: attachment_response,
                message: "File uploaded successfully".to_string(),
            };

            log::info!("File uploaded successfully: {} ({})", &file_name, stored_file_name);
            return Ok(HttpResponse::Created().json(ApiResponse::success("File uploaded successfully", upload_response)));
        }
    }

    Err(ServiceError::ValidationError("No file found in request".to_string()))
}

/// Get all attachments for a task
#[utoipa::path(
    get,
    path = "/api/tasks/{task_id}/attachments",
    tag = "attachments",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("task_id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Attachments retrieved successfully", body = ApiResponse<Vec<AttachmentResponse>>),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError),
        (status = 404, description = "Task not found", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn get_task_attachments(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
    path: web::Path<i32>,
) -> Result<HttpResponse, ServiceError> {
    let task_id = path.into_inner();
    log::info!("GET /api/tasks/{}/attachments", task_id);

    let _user_id = get_user_from_token(&req, &config).await?;

    // Check if task exists
    let task_exists = sqlx::query("SELECT id FROM tasks WHERE id = $1")
        .bind(task_id)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| {
            log::error!("Database error checking task: {}", e);
            ServiceError::DatabaseError("Failed to check task".to_string())
        })?;

    if task_exists.is_none() {
        return Err(ServiceError::NotFound("Task not found".to_string()));
    }

    let attachment_rows = sqlx::query(
        "SELECT id, task_id, file_name, original_name, file_size, mime_type, uploaded_by, created_at 
         FROM task_attachments WHERE task_id = $1 ORDER BY created_at DESC"
    )
    .bind(task_id)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error fetching attachments: {}", e);
        ServiceError::DatabaseError("Failed to fetch attachments".to_string())
    })?;

    let attachments: Vec<AttachmentResponse> = attachment_rows.iter().map(|row| {
        AttachmentResponse {
            id: row.get("id"),
            task_id: row.get("task_id"),
            file_name: row.get("file_name"),
            original_name: row.get("original_name"),
            file_size: row.get("file_size"),
            mime_type: row.get("mime_type"),
            uploaded_by: row.get("uploaded_by"),
            download_url: format!("/api/tasks/{}/attachments/{}/download", task_id, row.get::<i32, _>("id")),
            created_at: row.get("created_at"),
        }
    }).collect();

    log::info!("Retrieved {} attachments for task {}", attachments.len(), task_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success("Attachments retrieved successfully", attachments)))
}

/// Download a file attachment
#[utoipa::path(
    get,
    path = "/api/tasks/{task_id}/attachments/{attachment_id}/download",
    tag = "attachments",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("task_id" = i32, Path, description = "Task ID"),
        ("attachment_id" = i32, Path, description = "Attachment ID")
    ),
    responses(
        (status = 200, description = "File download", content_type = "application/octet-stream"),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError),
        (status = 404, description = "File not found", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn download_file(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
    path: web::Path<(i32, i32)>,
) -> Result<HttpResponse, ServiceError> {
    let (task_id, attachment_id) = path.into_inner();
    log::info!("GET /api/tasks/{}/attachments/{}/download", task_id, attachment_id);

    let _user_id = get_user_from_token(&req, &config).await?;

    // Get attachment info
    let attachment_row = sqlx::query(
        "SELECT file_path, original_name, mime_type 
         FROM task_attachments 
         WHERE id = $1 AND task_id = $2"
    )
    .bind(attachment_id)
    .bind(task_id)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error fetching attachment: {}", e);
        ServiceError::DatabaseError("Failed to fetch attachment".to_string())
    })?;

    let attachment_row = match attachment_row {
        Some(row) => row,
        None => {
            log::warn!("Attachment not found: {} for task {}", attachment_id, task_id);
            return Err(ServiceError::NotFound("Attachment not found".to_string()));
        }
    };

    let file_path: String = attachment_row.get("file_path");
    let original_name: String = attachment_row.get("original_name");
    let mime_type: String = attachment_row.get("mime_type");

    // Check if file exists on disk
    if !Path::new(&file_path).exists() {
        log::error!("File not found on disk: {}", file_path);
        return Err(ServiceError::NotFound("File not found on disk".to_string()));
    }

    // Read file
    let file_data = std::fs::read(&file_path)
        .map_err(|e| {
            log::error!("Failed to read file {}: {}", file_path, e);
            ServiceError::InternalError("Failed to read file".to_string())
        })?;

    log::info!("File downloaded: {} ({} bytes)", original_name, file_data.len());

    Ok(HttpResponse::Ok()
        .content_type(mime_type.as_str())
        .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", original_name)))
        .body(file_data))
}

/// Delete a file attachment
#[utoipa::path(
    delete,
    path = "/api/tasks/{task_id}/attachments/{attachment_id}",
    tag = "attachments",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("task_id" = i32, Path, description = "Task ID"),
        ("attachment_id" = i32, Path, description = "Attachment ID")
    ),
    responses(
        (status = 200, description = "Attachment deleted successfully", body = ApiResponse<bool>),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError),
        (status = 404, description = "Attachment not found", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn delete_attachment(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
    path: web::Path<(i32, i32)>,
) -> Result<HttpResponse, ServiceError> {
    let (task_id, attachment_id) = path.into_inner();
    log::info!("DELETE /api/tasks/{}/attachments/{}", task_id, attachment_id);

    let _user_id = get_user_from_token(&req, &config).await?;

    // Get attachment info before deletion (to clean up file)
    let attachment_row = sqlx::query(
        "SELECT file_path FROM task_attachments WHERE id = $1 AND task_id = $2"
    )
    .bind(attachment_id)
    .bind(task_id)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error fetching attachment: {}", e);
        ServiceError::DatabaseError("Failed to fetch attachment".to_string())
    })?;

    let file_path = match attachment_row {
        Some(row) => row.get::<String, _>("file_path"),
        None => {
            return Err(ServiceError::NotFound("Attachment not found".to_string()));
        }
    };

    // Delete from database
    let result = sqlx::query("DELETE FROM task_attachments WHERE id = $1 AND task_id = $2")
        .bind(attachment_id)
        .bind(task_id)
        .execute(&db.pool)
        .await
        .map_err(|e| {
            log::error!("Database error deleting attachment: {}", e);
            ServiceError::DatabaseError("Failed to delete attachment".to_string())
        })?;

    if result.rows_affected() == 0 {
        return Err(ServiceError::NotFound("Attachment not found".to_string()));
    }

    // Clean up file from disk
    if Path::new(&file_path).exists() {
        if let Err(e) = std::fs::remove_file(&file_path) {
            log::warn!("Failed to delete file {}: {}", file_path, e);
            // Don't fail the request if file cleanup fails
        }
    }

    log::info!("Attachment deleted successfully: {}", attachment_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success("Attachment deleted successfully", true)))
}

pub fn file_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(
                web::scope("/tasks/{task_id}/attachments")
                    .route("", web::post().to(upload_file))
                    .route("", web::get().to(get_task_attachments))
                    .route("/{attachment_id}/download", web::get().to(download_file))
                    .route("/{attachment_id}", web::delete().to(delete_attachment))
            )
    );
}
