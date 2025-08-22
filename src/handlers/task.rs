use actix_web::{web, HttpRequest, HttpResponse, Result};
use sqlx::Row;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Serialize, Deserialize};

use crate::config::AppConfig;
use crate::Database;
use crate::models::auth::ApiResponse;
use crate::models::task::{TaskResponse, CreateTaskRequest, UpdateTaskRequest, Team};
use crate::models::file::TaskAttachmentSimple;
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

// Helper function to get team IDs from team names
async fn get_team_ids_from_names(db: &Database, team_names: &[String]) -> Result<Vec<i32>, ServiceError> {
    let mut team_ids = Vec::new();
    
    for team_name in team_names {
        let team_row = sqlx::query(
            "SELECT id FROM teams WHERE name = $1"
        )
        .bind(team_name)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| {
            log::error!("Database error getting team: {}", e);
            ServiceError::DatabaseError("Failed to query team".to_string())
        })?;

        if let Some(row) = team_row {
            team_ids.push(row.get("id"));
        } else {
            return Err(ServiceError::ValidationError(format!("Team '{}' not found", team_name)));
        }
    }

    Ok(team_ids)
}

// Helper function to get teams for a task
async fn get_task_teams(db: &Database, task_id: i32) -> Result<Vec<String>, ServiceError> {
    let team_rows = sqlx::query(
        "SELECT t.name FROM teams t 
         JOIN task_teams tt ON t.id = tt.team_id 
         WHERE tt.task_id = $1"
    )
    .bind(task_id)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error getting task teams: {}", e);
        ServiceError::DatabaseError("Failed to query task teams".to_string())
    })?;

    Ok(team_rows.iter().map(|row| row.get("name")).collect())
}

// Helper function to get attachments for a task
async fn get_task_attachments(db: &Database, task_id: i32) -> Result<Vec<TaskAttachmentSimple>, ServiceError> {
    let attachment_rows = sqlx::query(
        "SELECT file_name, cloudinary_secure_url FROM task_attachments WHERE task_id = $1"
    )
    .bind(task_id)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error getting task attachments: {}", e);
        ServiceError::DatabaseError("Failed to query task attachments".to_string())
    })?;

    let mut attachments = Vec::new();
    for row in attachment_rows {
        let file_name: String = row.get("file_name");
        let cloudinary_url: String = row.get("cloudinary_secure_url");
        
        attachments.push(TaskAttachmentSimple {
            name: file_name,
            url: cloudinary_url,
        });
    }

    Ok(attachments)
}

/// Create a new task
#[utoipa::path(
    post,
    path = "/api/tasks",
    tag = "tasks",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateTaskRequest,
    responses(
        (status = 201, description = "Task created successfully", body = ApiResponse<TaskResponse>),
        (status = 400, description = "Validation error", body = crate::utils::errors::ServiceError),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn create_task(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
    task_req: web::Json<CreateTaskRequest>,
) -> Result<HttpResponse, ServiceError> {
    log::info!("POST /api/tasks - Creating new task: {}", task_req.name);

    let user_id = get_user_from_token(&req, &config).await?;

    // Validate input
    if task_req.name.trim().is_empty() {
        return Err(ServiceError::ValidationError("Task name is required".to_string()));
    }

    // Validate status
    let valid_statuses = ["TO_DO", "DOING", "DONE"];
    if !valid_statuses.contains(&task_req.status.as_str()) {
        return Err(ServiceError::ValidationError("Invalid task status".to_string()));
    }

    // Begin transaction
    let mut tx = db.pool.begin().await
        .map_err(|e| {
            log::error!("Failed to begin transaction: {}", e);
            ServiceError::DatabaseError("Transaction failed".to_string())
        })?;

    // Create task
    let task_row = sqlx::query(
        "INSERT INTO tasks (name, description, status, external_link, created_by) 
         VALUES ($1, $2, $3, $4, $5) 
         RETURNING id, name, description, status, external_link, created_by, created_at, updated_at"
    )
    .bind(&task_req.name)
    .bind(&task_req.description)
    .bind(&task_req.status)
    .bind(&task_req.external_link)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        log::error!("Database error creating task: {}", e);
        ServiceError::DatabaseError("Failed to create task".to_string())
    })?;

    let task_id: i32 = task_row.get("id");

    // Assign teams if provided
    let mut teams = Vec::new();
    if let Some(ref team_names) = task_req.teams {
        if !team_names.is_empty() {
            let team_ids = get_team_ids_from_names(&db, team_names).await?;
            
            for team_id in team_ids {
                sqlx::query(
                    "INSERT INTO task_teams (task_id, team_id) VALUES ($1, $2)"
                )
                .bind(task_id)
                .bind(team_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    log::error!("Database error assigning team: {}", e);
                    ServiceError::DatabaseError("Failed to assign team".to_string())
                })?;
            }
            teams = team_names.clone();
        }
    }

    // Commit transaction
    tx.commit().await
        .map_err(|e| {
            log::error!("Failed to commit transaction: {}", e);
            ServiceError::DatabaseError("Transaction failed".to_string())
        })?;

    let task_response = TaskResponse {
        id: task_id,
        name: task_row.get("name"),
        description: task_row.get("description"),
        status: task_row.get("status"),
        external_link: task_row.get("external_link"),
        created_by: task_row.get("created_by"),
        teams,
        attachments: Vec::new(), // New task has no attachments
        created_at: task_row.get("created_at"),
        updated_at: task_row.get("updated_at"),
    };

    log::info!("Task created successfully with ID: {}", task_id);
    Ok(HttpResponse::Created().json(ApiResponse::success("Task created successfully", task_response)))
}

/// Get all tasks
#[utoipa::path(
    get,
    path = "/api/tasks",
    tag = "tasks",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Tasks retrieved successfully", body = ApiResponse<Vec<TaskResponse>>),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn get_tasks(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
) -> Result<HttpResponse, ServiceError> {
    log::info!("GET /api/tasks");

    let _user_id = get_user_from_token(&req, &config).await?;

    let task_rows = sqlx::query(
        "SELECT id, name, description, status, external_link, created_by, created_at, updated_at 
         FROM tasks ORDER BY created_at DESC"
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error fetching tasks: {}", e);
        ServiceError::DatabaseError("Failed to fetch tasks".to_string())
    })?;

    let mut tasks = Vec::new();
    for row in task_rows {
        let task_id: i32 = row.get("id");
        let teams = get_task_teams(&db, task_id).await?;
        let attachments = get_task_attachments(&db, task_id).await?;

        tasks.push(TaskResponse {
            id: task_id,
            name: row.get("name"),
            description: row.get("description"),
            status: row.get("status"),
            external_link: row.get("external_link"),
            created_by: row.get("created_by"),
            teams,
            attachments,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        });
    }

    log::info!("Retrieved {} tasks", tasks.len());
    Ok(HttpResponse::Ok().json(ApiResponse::success("Tasks retrieved successfully", tasks)))
}

/// Get a specific task by ID
#[utoipa::path(
    get,
    path = "/api/tasks/{id}",
    tag = "tasks",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task retrieved successfully", body = ApiResponse<TaskResponse>),
        (status = 404, description = "Task not found", body = crate::utils::errors::ServiceError),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn get_task(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
    path: web::Path<i32>,
) -> Result<HttpResponse, ServiceError> {
    let task_id = path.into_inner();
    log::info!("GET /api/tasks/{}", task_id);

    let _user_id = get_user_from_token(&req, &config).await?;

    let task_row = sqlx::query(
        "SELECT id, name, description, status, external_link, created_by, created_at, updated_at 
         FROM tasks WHERE id = $1"
    )
    .bind(task_id)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error fetching task: {}", e);
        ServiceError::DatabaseError("Failed to fetch task".to_string())
    })?;

    let task_row = match task_row {
        Some(row) => row,
        None => {
            log::warn!("Task not found: {}", task_id);
            return Ok(HttpResponse::Ok().json(ApiResponse::success("Task not found", None::<TaskResponse>)));
        }
    };

    let teams = get_task_teams(&db, task_id).await?;
    let attachments = get_task_attachments(&db, task_id).await?;

    let task_response = TaskResponse {
        id: task_row.get("id"),
        name: task_row.get("name"),
        description: task_row.get("description"),
        status: task_row.get("status"),
        external_link: task_row.get("external_link"),
        created_by: task_row.get("created_by"),
        teams,
        attachments,
        created_at: task_row.get("created_at"),
        updated_at: task_row.get("updated_at"),
    };

    log::info!("Task retrieved: {}", task_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success("Task retrieved successfully", task_response)))
}

/// Update a task
#[utoipa::path(
    put,
    path = "/api/tasks/{id}",
    tag = "tasks",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    request_body = UpdateTaskRequest,
    responses(
        (status = 200, description = "Task updated successfully", body = ApiResponse<TaskResponse>),
        (status = 404, description = "Task not found", body = crate::utils::errors::ServiceError),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn update_task(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
    path: web::Path<i32>,
    update_req: web::Json<UpdateTaskRequest>,
) -> Result<HttpResponse, ServiceError> {
    let task_id = path.into_inner();
    log::info!("PUT /api/tasks/{}", task_id);

    let _user_id = get_user_from_token(&req, &config).await?;

    // Check if task exists
    let existing_task = sqlx::query(
        "SELECT id FROM tasks WHERE id = $1"
    )
    .bind(task_id)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error checking task: {}", e);
        ServiceError::DatabaseError("Failed to check task".to_string())
    })?;

    if existing_task.is_none() {
        return Err(ServiceError::NotFound("Task not found".to_string()));
    }

    // Validate status if provided
    if let Some(ref status) = update_req.status {
        let valid_statuses = ["TO_DO", "DOING", "DONE"];
        if !valid_statuses.contains(&status.as_str()) {
            return Err(ServiceError::ValidationError("Invalid task status".to_string()));
        }
    }

    // Begin transaction
    let mut tx = db.pool.begin().await
        .map_err(|e| {
            log::error!("Failed to begin transaction: {}", e);
            ServiceError::DatabaseError("Transaction failed".to_string())
        })?;

    // Build dynamic update query
    let mut has_updates = false;
    let mut query = "UPDATE tasks SET updated_at = NOW()".to_string();
    let mut bind_index = 1;

    if update_req.name.is_some() {
        query.push_str(&format!(", name = ${}", bind_index));
        bind_index += 1;
        has_updates = true;
    }

    if update_req.description.is_some() {
        query.push_str(&format!(", description = ${}", bind_index));
        bind_index += 1;
        has_updates = true;
    }

    if update_req.status.is_some() {
        query.push_str(&format!(", status = ${}", bind_index));
        bind_index += 1;
        has_updates = true;
    }

    if update_req.external_link.is_some() {
        query.push_str(&format!(", external_link = ${}", bind_index));
        bind_index += 1;
        has_updates = true;
    }

    query.push_str(&format!(" WHERE id = ${} RETURNING id, name, description, status, external_link, created_by, created_at, updated_at", bind_index));

    // Execute the update query using QueryBuilder for better type safety
    let updated_task = if has_updates {
        let mut query_builder = sqlx::QueryBuilder::new("UPDATE tasks SET updated_at = NOW()");
        
        if let Some(ref name) = update_req.name {
            query_builder.push(", name = ").push_bind(name);
        }
        if let Some(ref description) = update_req.description {
            query_builder.push(", description = ").push_bind(description);
        }
        if let Some(ref status) = update_req.status {
            query_builder.push(", status = ").push_bind(status);
        }
        if let Some(ref external_link) = update_req.external_link {
            query_builder.push(", external_link = ").push_bind(external_link);
        }
        
        query_builder.push(" WHERE id = ").push_bind(task_id);
        query_builder.push(" RETURNING id, name, description, status, external_link, created_by, created_at, updated_at");

        query_builder.build()
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                log::error!("Database error updating task: {}", e);
                ServiceError::DatabaseError("Failed to update task".to_string())
            })?
    } else {
        // No task fields to update, just get current task
        sqlx::query(
            "SELECT id, name, description, status, external_link, created_by, created_at, updated_at 
             FROM tasks WHERE id = $1"
        )
        .bind(task_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            log::error!("Database error fetching task: {}", e);
            ServiceError::DatabaseError("Failed to fetch task".to_string())
        })?
    };

    // Update teams if provided
    let teams = if let Some(ref team_names) = update_req.teams {
        // Remove existing team assignments
        sqlx::query("DELETE FROM task_teams WHERE task_id = $1")
            .bind(task_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                log::error!("Database error removing team assignments: {}", e);
                ServiceError::DatabaseError("Failed to remove team assignments".to_string())
            })?;

        // Add new team assignments
        if !team_names.is_empty() {
            let team_ids = get_team_ids_from_names(&db, team_names).await?;
            
            for team_id in team_ids {
                sqlx::query(
                    "INSERT INTO task_teams (task_id, team_id) VALUES ($1, $2)"
                )
                .bind(task_id)
                .bind(team_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    log::error!("Database error assigning team: {}", e);
                    ServiceError::DatabaseError("Failed to assign team".to_string())
                })?;
            }
        }

        team_names.clone()
    } else {
        // Keep existing teams
        get_task_teams(&db, task_id).await?
    };

    // Commit transaction
    tx.commit().await
        .map_err(|e| {
            log::error!("Failed to commit transaction: {}", e);
            ServiceError::DatabaseError("Transaction failed".to_string())
        })?;

    let task_response = TaskResponse {
        id: updated_task.get("id"),
        name: updated_task.get("name"),
        description: updated_task.get("description"),
        status: updated_task.get("status"),
        external_link: updated_task.get("external_link"),
        created_by: updated_task.get("created_by"),
        teams,
        attachments: get_task_attachments(&db, task_id).await?,
        created_at: updated_task.get("created_at"),
        updated_at: updated_task.get("updated_at"),
    };

    log::info!("Task updated successfully: {}", task_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success("Task updated successfully", task_response)))
}

/// Delete a task
#[utoipa::path(
    delete,
    path = "/api/tasks/{id}",
    tag = "tasks",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task deleted successfully", body = ApiResponse<bool>),
        (status = 404, description = "Task not found", body = crate::utils::errors::ServiceError),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn delete_task(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
    path: web::Path<i32>,
) -> Result<HttpResponse, ServiceError> {
    let task_id = path.into_inner();
    log::info!("DELETE /api/tasks/{}", task_id);

    let _user_id = get_user_from_token(&req, &config).await?;

    let result = sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(task_id)
        .execute(&db.pool)
        .await
        .map_err(|e| {
            log::error!("Database error deleting task: {}", e);
            ServiceError::DatabaseError("Failed to delete task".to_string())
        })?;

    if result.rows_affected() == 0 {
        return Err(ServiceError::NotFound("Task not found".to_string()));
    }

    log::info!("Task deleted successfully: {}", task_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success("Task deleted successfully", true)))
}

/// Get all teams
#[utoipa::path(
    get,
    path = "/api/teams",
    tag = "teams",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Teams retrieved successfully", body = ApiResponse<Vec<Team>>),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn get_teams(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
) -> Result<HttpResponse, ServiceError> {
    log::info!("GET /api/teams");

    let _user_id = get_user_from_token(&req, &config).await?;

    let team_rows = sqlx::query(
        "SELECT id, name, created_at FROM teams ORDER BY name"
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error fetching teams: {}", e);
        ServiceError::DatabaseError("Failed to fetch teams".to_string())
    })?;

    let teams: Vec<Team> = team_rows.iter().map(|row| Team {
        id: row.get("id"),
        name: row.get("name"),
        created_at: row.get("created_at"),
    }).collect();

    log::info!("Retrieved {} teams", teams.len());
    Ok(HttpResponse::Ok().json(ApiResponse::success("Teams retrieved successfully", teams)))
}

pub fn task_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(
                web::scope("/tasks")
                    .route("", web::post().to(create_task))
                    .route("", web::get().to(get_tasks))
                    .route("/{id}", web::get().to(get_task))
                    .route("/{id}", web::put().to(update_task))
                    .route("/{id}", web::delete().to(delete_task))
            )
            .service(
                web::scope("/teams")
                    .route("", web::get().to(get_teams))
            )
    );
}
