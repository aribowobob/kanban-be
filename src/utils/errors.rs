use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;
use utoipa::ToSchema;
use crate::models::auth::ErrorResponse;

#[derive(Debug, Serialize, ToSchema)]
pub enum ServiceError {
    Unauthorized(String),
    NotFound(String),
    InternalError(String),
    DatabaseError(String),
    ValidationError(String),
    AuthenticationError(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ServiceError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            ServiceError::InternalError(msg) => write!(f, "Internal Error: {}", msg),
            ServiceError::DatabaseError(msg) => write!(f, "Database Error: {}", msg),
            ServiceError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            ServiceError::AuthenticationError(msg) => write!(f, "Authentication Error: {}", msg),
        }
    }
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::Unauthorized(msg) => {
                log::error!("Unauthorized: {}", msg);
                HttpResponse::Unauthorized().json(ErrorResponse {
                    status: "error".to_string(),
                    message: msg.clone(),
                })
            }
            ServiceError::NotFound(msg) => {
                log::error!("Not Found: {}", msg);
                HttpResponse::NotFound().json(ErrorResponse {
                    status: "error".to_string(),
                    message: msg.clone(),
                })
            }
            ServiceError::InternalError(msg) => {
                log::error!("Internal Error: {}", msg);
                HttpResponse::InternalServerError().json(ErrorResponse {
                    status: "error".to_string(),
                    message: "Something went wrong".to_string(), // Don't expose internal details
                })
            }
            ServiceError::DatabaseError(msg) => {
                log::error!("Database Error: {}", msg);
                HttpResponse::InternalServerError().json(ErrorResponse {
                    status: "error".to_string(),
                    message: "Database operation failed".to_string(), // Don't expose database details
                })
            }
            ServiceError::ValidationError(msg) => {
                log::error!("Validation Error: {}", msg);
                HttpResponse::BadRequest().json(ErrorResponse {
                    status: "error".to_string(),
                    message: msg.clone(),
                })
            }
            ServiceError::AuthenticationError(msg) => {
                log::error!("Authentication Error: {}", msg);
                HttpResponse::Unauthorized().json(ErrorResponse {
                    status: "error".to_string(),
                    message: msg.clone(),
                })
            }
        }
    }
}

// Convert sqlx errors to ServiceError
impl From<sqlx::Error> for ServiceError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ServiceError::NotFound("Record not found".to_string()),
            _ => ServiceError::DatabaseError(err.to_string()),
        }
    }
}

// Convert bcrypt errors to ServiceError  
impl From<bcrypt::BcryptError> for ServiceError {
    fn from(err: bcrypt::BcryptError) -> Self {
        ServiceError::InternalError(format!("Password hashing error: {}", err))
    }
}

// Convert JWT errors to ServiceError
impl From<jsonwebtoken::errors::Error> for ServiceError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        ServiceError::AuthenticationError(format!("JWT error: {}", err))
    }
}
