use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::Utc;
use uuid::Uuid;
use crate::utils;
use crate::models::user::UserWithoutDates;
use crate::errors::AppError;

#[derive(Deserialize, Validate)]
pub struct UserProfileUpdate {
    #[validate(email)]
    email: Option<String>,
    #[validate(length(min = 4, max = 52))]
    name: Option<String>,
    #[validate(url)]
    user_image_uri: Option<String>,
    #[validate(length(min = 4, max = 52))]
    company_name: Option<String>,
    #[validate(url)]
    company_image_uri: Option<String>,
}

#[derive(Serialize)]
struct UserProfileResponse {
    email: String,
    name: Option<String>,
    user_image_uri: Option<String>,
    company_name: Option<String>,
    company_image_uri: Option<String>,
}

pub async fn get_user_profile(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    // Extract the JWT token from the Authorization header
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        // Decode the JWT token
        let claims = utils::jwt::validate_token(token)
            .map_err(|err| AppError::Unauthorized(err.to_string()))?;

        // Retrieve user by email
        let user = sqlx::query_as!(
            UserWithoutDates,
            "SELECT user_id, email, password, name, user_image_uri, company_name, company_image_uri FROM users WHERE email = $1",
            claims.sub
        )
        .fetch_one(&**pool)
        .await
        .map_err(|_| actix_web::error::ErrorNotFound("User not found"))?;

        Ok(HttpResponse::Ok().json(UserProfileResponse {
            email: user.email,
            name: user.name,
            user_image_uri: user.user_image_uri,
            company_name: user.company_name,
            company_image_uri: user.company_image_uri,
        }))
    } else {
        Err(actix_web::error::ErrorUnauthorized("Missing token"))?
    }
}

pub async fn update_user_profile(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    updates: web::Json<UserProfileUpdate>,
) -> Result<HttpResponse, actix_web::Error> {
    updates.validate()
        .map_err(|err| AppError::BadRequest(err.to_string()))?;

    // Extract the JWT token from the Authorization header
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        // Decode the JWT token
        let claims = utils::jwt::validate_token(token)
            .map_err(|err| AppError::Unauthorized(err.to_string()))?;

        // Convert claims.sub (String) to Uuid
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| actix_web::error::ErrorInternalServerError("Invalid user ID in token"))?;

        // Check if the new email is already in use (case-insensitive)
        if let Some(new_email) = &updates.email {
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(email) = LOWER($1) AND user_id != $2)",
                new_email,
                user_id
            )
            .fetch_one(&**pool)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Database error"))?;

            if exists.unwrap_or(false) {
                return Err(actix_web::error::ErrorConflict("Email already exists"));
            }
        }

        // Update the user's profile
        let mut query = "UPDATE users SET".to_string();
        let mut params: Vec<String> = Vec::new(); // Use Vec<String> for dynamic binding
        let mut set_clauses = Vec::new();

        if let Some(email) = &updates.email {
            set_clauses.push("email = $1".to_string());
            params.push(email.clone());
        }
        if let Some(name) = &updates.name {
            set_clauses.push("name = $2".to_string());
            params.push(name.clone());
        }
        if let Some(user_image_uri) = &updates.user_image_uri {
            set_clauses.push("user_image_uri = $3".to_string());
            params.push(user_image_uri.clone());
        }
        if let Some(company_name) = &updates.company_name {
            set_clauses.push("company_name = $4".to_string());
            params.push(company_name.clone());
        }
        if let Some(company_image_uri) = &updates.company_image_uri {
            set_clauses.push("company_image_uri = $5".to_string());
            params.push(company_image_uri.clone());
        }

        // Add updated_at timestamp
        let now = Utc::now();
        set_clauses.push("updated_at = $6".to_string());
        params.push(now.to_rfc3339()); // Convert DateTime to string

        query.push_str(&set_clauses.join(", "));
        query.push_str(" WHERE user_id = $7");
        params.push(user_id.to_string());

        sqlx::query(&query)
            .bind(&params[0])
            .bind(&params[1])
            .bind(&params[2])
            .bind(&params[3])
            .bind(&params[4])
            .bind(&params[5])
            .bind(&params[6])
            .execute(&**pool)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Update failed"))?;

        // Retrieve the updated user profile
        let user = sqlx::query_as!(
            UserWithoutDates,
            "SELECT user_id, email, password, name, user_image_uri, company_name, company_image_uri FROM users WHERE user_id = $1",
            user_id
        )
        .fetch_one(&**pool)
        .await
        .map_err(|_| actix_web::error::ErrorNotFound("User not found"))?;

        Ok(HttpResponse::Ok().json(UserProfileResponse {
            email: user.email,
            name: user.name,
            user_image_uri: user.user_image_uri,
            company_name: user.company_name,
            company_image_uri: user.company_image_uri,
        }))
    } else {
        Err(actix_web::error::ErrorUnauthorized("Missing token"))?
    }
}