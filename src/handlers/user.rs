use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;
use chrono::Utc;
use crate::utils;

#[derive(Deserialize, Validate)]
struct UserProfileUpdate {
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
        let claims = utils::jwt::validate_token(token)?;

        // Retrieve user by email
        let user = sqlx::query_as!(
            models::user::User,
            "SELECT * FROM users WHERE email = $1",
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
    updates.validate()?;

    // Extract the JWT token from the Authorization header
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        // Decode the JWT token
        let claims = utils::jwt::validate_token(token)?;

        // Check if the new email is already in use (case-insensitive)
        if let Some(new_email) = &updates.email {
            if sqlx::query!("SELECT 1 FROM users WHERE LOWER(email) = LOWER($1) AND user_id != $2", new_email, claims.sub)
                .fetch_optional(&**pool)
                .await?
                .is_some()
            {
                return Err(actix_web::error::ErrorConflict("Email already exists"));
            }
        }

        // Update the user's profile
        let mut query = "UPDATE users SET".to_string();
        let mut params = Vec::new();
        let mut set_clauses = Vec::new();

        if let Some(email) = &updates.email {
            set_clauses.push("email = $1".to_string());
            params.push(email);
        }
        if let Some(name) = &updates.name {
            set_clauses.push("name = $2".to_string());
            params.push(name);
        }
        if let Some(user_image_uri) = &updates.user_image_uri {
            set_clauses.push("user_image_uri = $3".to_string());
            params.push(user_image_uri);
        }
        if let Some(company_name) = &updates.company_name {
            set_clauses.push("company_name = $4".to_string());
            params.push(company_name);
        }
        if let Some(company_image_uri) = &updates.company_image_uri {
            set_clauses.push("company_image_uri = $5".to_string());
            params.push(company_image_uri);
        }
        set_clauses.push("updated_at = $6".to_string());
        params.push(Utc::now());

        query.push_str(&set_clauses.join(", "));
        query.push_str(" WHERE email = $7");
        params.push(claims.sub);

        sqlx::query(&query)
            .bind(&params[0..params.len()])
            .execute(&**pool)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Update failed"))?;

        // Retrieve the updated user profile
        let user = sqlx::query_as!(
            models::user::User,
            "SELECT * FROM users WHERE email = $1",
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