use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskAttachment {
    pub id: i32,
    pub task_id: i32,
    pub file_name: String,
    pub original_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub uploaded_by: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttachmentResponse {
    pub id: i32,
    pub task_id: i32,
    pub file_name: String,
    pub original_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub uploaded_by: i32,
    pub download_url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UploadResponse {
    pub attachment: AttachmentResponse,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileUploadInfo {
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskAttachmentSimple {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UploadFileRequest {
    #[schema(format = "binary")]
    pub file: String,
}
