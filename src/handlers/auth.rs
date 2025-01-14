use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use time::{OffsetDateTime, Duration};
use argon2::{Argon2, password_hash::PasswordHasher, password_hash::SaltString, PasswordVerifier};
use jsonwebtoken::{encode, Header, EncodingKey};
use validator::{Validate, ValidationErrors};
use std::env;
use rand;
use crate::utils;

#[derive(Deserialize, Validate)]
pub struct AuthRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8, max = 32))]
    password: String,
    #[validate(custom = "validate_action")]
    action: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    email: String,
    token: String,
}

fn validate_action(action: &str) -> Result<(), validator::ValidationError> {
    if action != "create" && action != "login" {
        return Err(validator::ValidationError::new("Invalid action"));
    }
    Ok(())
}

fn map_sqlx_error(err: sqlx::Error) -> actix_web::Error {
    match err {
        sqlx::Error::RowNotFound => actix_web::error::ErrorNotFound("Resource not found"),
        _ => actix_web::error::InternalError::new(err, actix_web::http::StatusCode::INTERNAL_SERVER_ERROR).into(),
    }
}

fn map_validation_error(err: ValidationErrors) -> actix_web::Error {
    actix_web::error::ErrorBadRequest(err.to_string())
}

pub async fn auth_handler(
    req: web::Json<AuthRequest>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    req.0.validate().map_err(map_validation_error)?;

    match req.action.to_lowercase().as_str() {
        "create" => {
            if sqlx::query!("SELECT email FROM users WHERE LOWER(email) = LOWER($1)", &req.0.email)
                .fetch_optional(&**pool)
                .await
                .map_err(map_sqlx_error)?
                .is_some()
            {
                return Err(actix_web::error::ErrorConflict("Email already exists"));
            }

            let salt = SaltString::generate(&mut rand::thread_rng());
            let argon2 = Argon2::default();
            let password_hash = argon2.hash_password(req.0.password.as_bytes(), &salt)
                .map_err(|_| actix_web::error::ErrorInternalServerError("Hashing error"))?
                .to_string();

            let user_id = Uuid::new_v4();
            let now = Utc::now();
            sqlx::query!("INSERT INTO users (user_id, email, password, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
                         user_id, &req.0.email, &password_hash, now, now)
                .execute(&**pool)
                .await
                .map_err(map_sqlx_error)?;

            let claims = utils::jwt::Claims {
                sub: user_id.to_string(), // Use user_id instead of email
                exp: (OffsetDateTime::now_utc() + Duration::days(7)).unix_timestamp() as usize,
            };
            let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_ref()))
                .map_err(|_| actix_web::error::ErrorInternalServerError("Token generation error"))?;

            Ok(HttpResponse::Created().json(AuthResponse {
                email: req.0.email.clone(),
                token,
            }))
        },
        "login" => {
            let user = sqlx::query!("SELECT * FROM users WHERE LOWER(email) = LOWER($1)", &req.0.email)
                .fetch_one(&**pool)
                .await
                .map_err(map_sqlx_error)?;

            let parsed_hash = argon2::PasswordHash::new(&user.password)
                .map_err(|_| actix_web::error::ErrorInternalServerError("Invalid password hash"))?;
            Argon2::default()
                .verify_password(req.0.password.as_bytes(), &parsed_hash)
                .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid password"))?;

            let claims = utils::jwt::Claims {
                sub: user.user_id.to_string(), // Use user_id instead of email
                exp: (OffsetDateTime::now_utc() + Duration::days(7)).unix_timestamp() as usize,
            };
            let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_ref()))
                .map_err(|_| actix_web::error::ErrorInternalServerError("Token generation error"))?;

            Ok(HttpResponse::Ok().json(AuthResponse {
                email: user.email.clone(),
                token,
            }))
        },
        _ => Err(actix_web::error::ErrorBadRequest("Invalid action"))?,
    }
}