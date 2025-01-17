use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;
use chrono::Utc;
use url::Url;
use crate::utils;
use crate::models::user::{GetUserProfileResponse, UserWithoutDates};
use crate::errors::AppError;
use log::{info, error};

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserProfileUpdate {
    #[validate(email)]
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[validate(length(min = 4, max = 52))]
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    user_image_uri: Option<String>,
    #[validate(length(min = 4, max = 52))]
    #[serde(skip_serializing_if = "Option::is_none")]
    company_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    company_image_uri: Option<String>,
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
    // Check token first
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1))
        .ok_or_else(|| AppError::Unauthorized("Missing token".to_string()))?;

    let claims = utils::jwt::validate_token(token)
        .map_err(|err| AppError::Unauthorized(err.to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()))?;

    // Check if the request contains at least one non-null field
    if updates.email.is_none()
        && updates.name.is_none()
        && updates.user_image_uri.is_none()
        && updates.company_name.is_none()
        && updates.company_image_uri.is_none()
    {
        return Err(AppError::BadRequest("No update fields provided".to_string()).into());
    }

    // Check if any field is explicitly set to null
    if updates.email.is_none()
        || updates.name.is_none()
        || updates.user_image_uri.is_none()
        || updates.company_name.is_none()
        || updates.company_image_uri.is_none()
    {
        return Err(AppError::BadRequest("Null values are not allowed".to_string()).into());
    }
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


    // Validate URLs if provided
    if let Some(uri) = &updates.user_image_uri {
        info!("Validating user_image_uri: {}", uri);
        match Url::parse(uri) {
            Ok(url) => {
                // Additional validation for domain structure
                if let Some(host) = url.host() {
                    match host {
                        url::Host::Domain(domain) => {
                            // Ensure the domain has at least one dot (.) to be valid
                            if !domain.contains('.') {
                                error!("Invalid domain in user_image_uri: {}", uri);
                                return Err(AppError::BadRequest("Invalid domain in 'user_image_uri'".to_string()).into());
                            }
                        }
                        url::Host::Ipv4(_) | url::Host::Ipv6(_) => {
                            // IP addresses are valid, so no additional checks are needed
                        }
                    }
                } else {
                    error!("Missing host in user_image_uri: {}", uri);
                    return Err(AppError::BadRequest("Missing host in 'user_image_uri'".to_string()).into());
                }
                info!("user_image_uri is valid: {}", uri);
            }
            Err(err) => {
                error!("Invalid user_image_uri: {}, error: {}", uri, err);
                return Err(AppError::BadRequest("Invalid URL format in 'user_image_uri'".to_string()).into());
            }
        };
    }

    if let Some(uri) = &updates.company_image_uri {
        info!("Validating company_image_uri: {}", uri);
        match Url::parse(uri) {
            Ok(url) => {
                // Additional validation for domain structure
                if let Some(host) = url.host() {
                    match host {
                        url::Host::Domain(domain) => {
                            // Ensure the domain has at least one dot (.) to be valid
                            if !domain.contains('.') {
                                error!("Invalid domain in company_image_uri: {}", uri);
                                return Err(AppError::BadRequest("Invalid domain in 'company_image_uri'".to_string()).into());
                            }
                        }
                        url::Host::Ipv4(_) | url::Host::Ipv6(_) => {
                            // IP addresses are valid, so no additional checks are needed
                        }
                    }
                } else {
                    error!("Missing host in company_image_uri: {}", uri);
                    return Err(AppError::BadRequest("Missing host in 'company_image_uri'".to_string()).into());
                }
                info!("company_image_uri is valid: {}", uri);
            }
            Err(err) => {
                error!("Invalid company_image_uri: {}, error: {}", uri, err);
                return Err(AppError::BadRequest("Invalid URL format in 'company_image_uri'".to_string()).into());
            }
        };
    }

    // Check for duplicate email if provided
    if let Some(email) = &updates.email {
        let email_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(email) = LOWER($1) AND user_id != $2)",
            email,
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
    }

    // Build the update query dynamically
    let mut query = sqlx::QueryBuilder::new("UPDATE users SET");
    let mut separated: sqlx::query_builder::Separated<'_, '_, sqlx::Postgres, &str> = query.separated(", ");

    if let Some(email) = &updates.email {
        separated.push("email = ");
        separated.push_bind(email);
    }
    if let Some(name) = &updates.name {
        separated.push("name = ");
        separated.push_bind(name);
    }
    if let Some(user_image_uri) = &updates.user_image_uri {
        separated.push("user_image_uri = ");
        separated.push_bind(user_image_uri);
    }
    if let Some(company_name) = &updates.company_name {
        separated.push("company_name = ");
        separated.push_bind(company_name);
    }
    if let Some(company_image_uri) = &updates.company_image_uri {
        separated.push("company_image_uri = ");
        separated.push_bind(company_image_uri);
    }
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
