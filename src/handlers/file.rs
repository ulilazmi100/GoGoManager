use actix_web::{web, HttpResponse, HttpRequest};
use aws_sdk_s3::Client as S3Client;
use serde::Serialize;
use uuid::Uuid;
use infer;
use crate::utils;
use crate::errors::AppError;
// use time::OffsetDateTime;
use chrono::Utc;

#[derive(Serialize)]
struct FileUploadResponse {
    uri: String,
}

pub async fn upload_file(
    req: HttpRequest,
    s3_client: web::Data<S3Client>,
    pool: web::Data<sqlx::PgPool>,
    file: web::Bytes,
) -> Result<HttpResponse, actix_web::Error> {
    // Extract the JWT token from the Authorization header
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        // Decode the JWT token
        let claims = utils::jwt::validate_token(token)
            .map_err(|err| AppError::JwtError(err.to_string()))?;

        // Validate file size (max 100KiB)
        if file.len() > 102400 {
            return Err(actix_web::error::ErrorBadRequest("File size exceeds 100KiB limit"));
        }

        // Validate file type
        let file_type = infer::get(&file).ok_or_else(|| actix_web::error::ErrorBadRequest("Invalid file type"))?;
        if !matches!(file_type.mime_type(), "image/jpeg" | "image/png") {
            return Err(actix_web::error::ErrorBadRequest("Only JPEG and PNG files are allowed"));
        }

        // Generate a unique file name
        let file_id = Uuid::new_v4();
        let file_name = format!("{}.{}", file_id, file_type.extension());

        // Upload the file to S3
        let bucket_name = std::env::var("AWS_S3_BUCKET").expect("AWS_S3_BUCKET must be set");
        s3_client
            .put_object()
            .bucket(&bucket_name)
            .key(&file_name)
            .body(file.to_vec().into())
            .send()
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to upload file"))?;

        // Parse the user ID from the token
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()))?;

        // Convert chrono::DateTime<Utc> to time::OffsetDateTime
        let now = Utc::now();

        // Store the file URI in the database
        let file_uri = format!("https://{}.s3.amazonaws.com/{}", bucket_name, file_name);
        sqlx::query!(
            "INSERT INTO files (file_id, user_id, uri, created_at) VALUES ($1, $2, $3, $4)",
            file_id,
            user_id,
            file_uri,
            now
        )
        .execute(&**pool)
        .await
        .map_err(|err| AppError::DatabaseError(err.to_string()))?;

        Ok(HttpResponse::Ok().json(FileUploadResponse { uri: file_uri }))
    } else {
        Err(actix_web::error::ErrorUnauthorized("Missing token"))
    }
}