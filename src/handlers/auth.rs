use actix_web::{web, HttpRequest, HttpResponse, Result};
use sqlx::Row;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};
use bcrypt::verify;
use serde::{Serialize, Deserialize};

use crate::config::AppConfig;
use crate::Database;
use crate::models::auth::{LoginRequest, LoginResponseData, UserResponse, ApiResponse};
use crate::utils::errors::ServiceError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user id)
    pub username: String,
    pub name: String,
    pub exp: usize, // Expiration time (Unix timestamp)
    pub iat: usize, // Issued at (Unix timestamp)
}

/// User login endpoint
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = ApiResponse<LoginResponseData>),
        (status = 401, description = "Invalid credentials", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn login(
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
    login_req: web::Json<LoginRequest>,
) -> Result<HttpResponse, ServiceError> {
    log::info!("POST /api/auth/login - Login attempt for: {}", login_req.username);

    // Validate input
    if login_req.username.trim().is_empty() {
        return Err(ServiceError::ValidationError("Username is required".to_string()));
    }
    
    if login_req.password.trim().is_empty() {
        return Err(ServiceError::ValidationError("Password is required".to_string()));
    }

    // Query user from database
    let user_row = sqlx::query(
        "SELECT id, username, name, password_hash, created_at, updated_at FROM users WHERE username = $1"
    )
    .bind(&login_req.username)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error during login: {}", e);
        ServiceError::DatabaseError("Failed to query user".to_string())
    })?;

    let user_row = match user_row {
        Some(row) => row,
        None => {
            log::warn!("Login failed: User not found - {}", login_req.username);
            return Err(ServiceError::Unauthorized("Invalid credentials".to_string()));
        }
    };

    // Verify password
    let stored_hash: String = user_row.get("password_hash");
    let password_valid = verify(&login_req.password, &stored_hash)
        .map_err(|e| {
            log::error!("Password verification error: {}", e);
            ServiceError::AuthenticationError("Password verification failed".to_string())
        })?;

    if !password_valid {
        log::warn!("Login failed: Invalid password for user - {}", login_req.username);
        return Err(ServiceError::Unauthorized("Invalid credentials".to_string()));
    }

    // Create JWT token
    let user_id: i32 = user_row.get("id");
    let now = Utc::now();
    let exp = now
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;
    let iat = now.timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        username: login_req.username.clone(),
        name: user_row.get("name"),
        exp,
        iat,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_ref()),
    )
    .map_err(|e| {
        log::error!("JWT encoding error: {}", e);
        ServiceError::AuthenticationError("Failed to generate token".to_string())
    })?;

    let response_data = LoginResponseData {
        token,
        user: UserResponse {
            id: user_id,
            username: user_row.get("username"),
            name: user_row.get("name"),
            created_at: user_row.get("created_at"),
            updated_at: user_row.get("updated_at"),
        },
    };

    log::info!("Login successful for user: {}", login_req.username);
    Ok(HttpResponse::Ok().json(ApiResponse::success("Login successful", response_data)))
}

/// User logout endpoint
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Logout successful", body = ApiResponse<bool>),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn logout(req: HttpRequest) -> Result<HttpResponse, ServiceError> {
    log::info!("POST /api/auth/logout");

    // Extract token from Authorization header
    let auth_header = req.headers().get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    if auth_header.is_none() {
        log::warn!("Logout attempt without valid authentication");
        return Err(ServiceError::Unauthorized("Authentication required".to_string()));
    }

    // For logout, we just return success since we're stateless
    // In a real app, you might want to maintain a blacklist of tokens
    log::info!("User logout successful");
    Ok(HttpResponse::Ok().json(ApiResponse::success("Successfully logout from the system", true)))
}

/// Get current user information
#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "User information retrieved", body = ApiResponse<UserResponse>),
        (status = 401, description = "Unauthorized", body = crate::utils::errors::ServiceError)
    )
)]
pub async fn get_me(
    req: HttpRequest,
    db: web::Data<Database>,
    config: web::Data<AppConfig>,
) -> Result<HttpResponse, ServiceError> {
    log::info!("GET /api/auth/me");

    // Extract token from Authorization header
    let auth_header = req.headers().get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    let token = auth_header.ok_or_else(|| {
        log::warn!("Get me attempt without valid authentication");
        ServiceError::Unauthorized("Authentication required".to_string())
    })?;

    // Validate the token
    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|e| {
        log::warn!("JWT validation error: {}", e);
        ServiceError::Unauthorized("Invalid token".to_string())
    })?;

    // Query user from database
    let user_id: i32 = claims.claims.sub.parse()
        .map_err(|_| ServiceError::Unauthorized("Invalid user ID in token".to_string()))?;
    
    let user_row = sqlx::query(
        "SELECT id, username, name, created_at, updated_at FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| {
        log::error!("Database error during get_me: {}", e);
        ServiceError::DatabaseError("Failed to query user".to_string())
    })?;

    let user_row = match user_row {
        Some(row) => row,
        None => {
            log::warn!("User not found for ID: {}", user_id);
            return Err(ServiceError::Unauthorized("User not found".to_string()));
        }
    };

    let user_response = UserResponse {
        id: user_row.get("id"),
        username: user_row.get("username"),
        name: user_row.get("name"),
        created_at: user_row.get("created_at"),
        updated_at: user_row.get("updated_at"),
    };

    log::info!("User information retrieved for: {}", user_response.username);
    Ok(HttpResponse::Ok().json(ApiResponse::success("Successfully retrieved user data", user_response)))
}

pub fn auth_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/auth")
            .route("/login", web::post().to(login))
            .route("/logout", web::post().to(logout))
            .route("/me", web::get().to(get_me))
    );
}
