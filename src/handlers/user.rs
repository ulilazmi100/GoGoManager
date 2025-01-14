use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;
use chrono::Utc;
use crate::utils;
use crate::models::user::{GetUserProfileResponse, UserWithoutDates};
use crate::errors::AppError;

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserProfileUpdate {
    #[validate(email)]
    email: String,
    #[validate(length(min = 4, max = 52))]
    name: String,
    #[validate(url)]
    user_image_uri: String,
    #[validate(length(min = 4, max = 52))]
    company_name: String,
    #[validate(url)]
    company_image_uri: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileResponse {
    email: String,
    name: String,
    user_image_uri: String,
    company_name: String,
    company_image_uri: String,
}

pub async fn get_user_profile(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1))
        .ok_or_else(|| AppError::Unauthorized("Missing token".to_string()))?;

    let claims = utils::jwt::validate_token(token)
        .map_err(|err| AppError::Unauthorized(err.to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()))?;

    let user = sqlx::query_as!(
        GetUserProfileResponse,
        r#"
        SELECT 
            email, 
            name, 
            user_image_uri, 
            company_name, 
            company_image_uri 
        FROM users 
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(&**pool)
    .await
    .map_err(|e| {
        log::error!("Database error during user retrieval: {:?}", e);
        AppError::InternalServerError("Database error".to_string())
    })?;

    if let Some(user) = user {
        Ok(HttpResponse::Ok().json(user))
    } else {
        Err(AppError::Unauthorized("User not found or unauthorized".to_string()).into())
    }
}

pub async fn update_user_profile(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    updates: web::Json<UserProfileUpdate>,
) -> Result<HttpResponse, actix_web::Error> {
    // Validate input fields
    updates.validate().map_err(|err| {
        let details = err.field_errors()
            .iter()
            .map(|(field, errs)| {
                let errors = errs.iter()
                    .map(|e| format!("{}: {}", e.code, e.message.as_deref().unwrap_or("")))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}: [{}]", field, errors)
            })
            .collect::<Vec<_>>()
            .join("; ");
        AppError::BadRequest(format!("Validation failed: {}", details))
    })?;

    // Extract token and validate
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1))
        .ok_or_else(|| AppError::Unauthorized("Missing token".to_string()))?;
    let claims = utils::jwt::validate_token(token)
        .map_err(|err| AppError::Unauthorized(err.to_string()))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()))?;

    // Check for duplicate email if provided
    let email_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(email) = LOWER($1) AND user_id != $2)",
        updates.email,
        user_id
    )
    .fetch_one(&**pool)
    .await
    .map_err(|e| {
        log::error!("DB error during email check: {:?}", e);
        AppError::InternalServerError("Database error".to_string())
    })?;

    if email_exists.unwrap_or(false) {
        return Err(AppError::Conflict("Email already exists".to_string()).into());
    }

    // Build the update query dynamically
    let mut query = sqlx::QueryBuilder::new("UPDATE users SET");
    let mut separated = query.separated(", ");

    separated.push("email = ");
    separated.push_bind(&updates.email);
    separated.push("name = ");
    separated.push_bind(&updates.name);
    separated.push("user_image_uri = ");
    separated.push_bind(&updates.user_image_uri);
    separated.push("company_name = ");
    separated.push_bind(&updates.company_name);
    separated.push("company_image_uri = ");
    separated.push_bind(&updates.company_image_uri);
    separated.push("updated_at = ");
    separated.push_bind(Utc::now());
    query.push(" WHERE user_id = ");
    query.push_bind(user_id);

    // Execute the query
    query.build()
        .execute(&**pool)
        .await
        .map_err(|e| {
            log::error!("DB error during update: {:?}", e);
            AppError::InternalServerError("Update failed".to_string())
        })?;

    // Fetch the updated user profile
    let user = sqlx::query_as!(
        UserWithoutDates,
        "SELECT user_id, email, name, password, user_image_uri, company_name, company_image_uri FROM users WHERE user_id = $1",
        user_id
    )
    .fetch_one(&**pool)
    .await
    .map_err(|e| {
        log::error!("DB error during fetch: {:?}", e);
        AppError::NotFound("User not found".to_string())
    })?;

    // Return updated response
    Ok(HttpResponse::Ok().json(UserProfileResponse {
        email: user.email,
        name: user.name.unwrap_or_default(),
        user_image_uri: user.user_image_uri.unwrap_or_default(),
        company_name: user.company_name.unwrap_or_default(),
        company_image_uri: user.company_image_uri.unwrap_or_default(),
    }))
}