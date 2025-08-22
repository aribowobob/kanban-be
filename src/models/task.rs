use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use utoipa::ToSchema;
use crate::models::file::TaskAttachmentSimple;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Task {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub external_link: Option<String>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaskResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub external_link: Option<String>,
    pub created_by: i32,
    pub teams: Vec<String>,
    pub attachments: Vec<TaskAttachmentSimple>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTaskRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub external_link: Option<String>,
    pub teams: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub external_link: Option<String>,
    pub teams: Option<Vec<String>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Team {
    pub id: i32,
    pub name: String,
    pub created_at: DateTime<Utc>,
}
