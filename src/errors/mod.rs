use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;
// use log::error;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Unauthorized(String),
    Conflict(String),
    InternalServerError(String),
    DatabaseError(String),
    AWSError(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::InternalServerError(msg) => write!(f, "Internal Server Error: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database Error: {}", msg),
            AppError::AWSError(msg) => write!(f, "AWS Error: {}", msg),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::NotFound(msg) => HttpResponse::NotFound().json(ErrorResponse { error: msg.clone() }),
            AppError::Unauthorized(msg) => HttpResponse::Unauthorized().json(ErrorResponse { error: msg.clone() }),
            AppError::Conflict(msg) => HttpResponse::Conflict().json(ErrorResponse { error: msg.clone() }),
            AppError::InternalServerError(msg) => HttpResponse::InternalServerError().json(ErrorResponse { error: msg.clone() }),
            AppError::DatabaseError(msg) => HttpResponse::InternalServerError().json(ErrorResponse { error: msg.clone() }),
            AppError::AWSError(msg) => HttpResponse::InternalServerError().json(ErrorResponse { error: msg.clone() }),
        }
    }
}