use actix_web::{web, HttpResponse, HttpRequest, Error};
use aws_sdk_s3::Client as S3Client;
use uuid::Uuid;
use infer;
use crate::utils;
use std::env;
use serde_json::json;
use actix_multipart::Multipart;
use futures_util::StreamExt;
use log::{info, error};

pub async fn upload_file(
    req: HttpRequest,
    s3_client: web::Data<S3Client>,
    payload: web::Payload,
) -> Result<HttpResponse, Error> {
    // Extract and validate JWT token
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .ok_or_else(|| {
            error!("Missing or invalid token");
            actix_web::error::ErrorUnauthorized("Missing or invalid token")
        })?;

    info!("Token: {:?}", token);

    // Validate the token
    utils::jwt::validate_token(token)
        .map_err(|err| {
            error!("Invalid token: {:?}", err);
            actix_web::error::ErrorUnauthorized("Invalid token")
        })?;

    // Parse multipart form-data
    let mut multipart = Multipart::new(&req.headers(), payload);
    let mut file_data = Vec::new();
    let mut file_size = 0;
    let mut file_type = None;

    while let Some(item) = multipart.next().await {
        let mut field = item.map_err(|err| {
            error!("Invalid multipart field: {:?}", err);
            actix_web::error::ErrorBadRequest("Invalid multipart field")
        })?;
        if field.name() != "file" {
            continue;
        }

        // Process file chunks
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|err| {
                error!("Failed to read chunk: {:?}", err);
                actix_web::error::ErrorBadRequest("Failed to read chunk")
            })?;
            file_size += chunk.len();
            if file_size > 102400 {
                error!("File size exceeds 100KiB limit");
                return Err(actix_web::error::ErrorBadRequest("File size exceeds 100KiB limit"));
            }
            file_data.extend_from_slice(&chunk);
        }

        // Capture the file's content type
        file_type = field.content_type().map(|ct| ct.to_string());
    }

    if file_data.is_empty() {
        error!("File part is missing");
        return Err(actix_web::error::ErrorBadRequest("File part is missing"));
    }

    info!("File size: {}", file_size);
    info!("File type: {:?}", file_type);

    // Validate file type
    let file_type = file_type.ok_or_else(|| {
        error!("Invalid file type");
        actix_web::error::ErrorBadRequest("Invalid file type")
    })?;
    if !["image/jpeg", "image/png"].contains(&file_type.as_str()) {
        error!("Only JPEG and PNG files are allowed");
        return Err(actix_web::error::ErrorBadRequest("Only JPEG and PNG files are allowed"));
    }

    // Generate unique filename
    let file_id = Uuid::new_v4();
    let extension = match file_type.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        _ => "bin", // Fallback, though validation should prevent this
    };
    let file_name = format!("{}.{}", file_id, extension);

    info!("Uploading to S3: {}", file_name);

    // Upload to S3
    let bucket_name = env::var("AWS_S3_BUCKET")
        .map_err(|err| {
            error!("AWS_S3_BUCKET environment variable not set: {:?}", err);
            actix_web::error::ErrorInternalServerError("AWS_S3_BUCKET not set")
        })?;
    s3_client.put_object()
        .bucket(&bucket_name)
        .key(&file_name)
        .body(file_data.into())
        .send()
        .await
        .map_err(|err| {
            error!("Failed to upload file to S3: {:?}", err);
            actix_web::error::ErrorInternalServerError("Failed to upload file")
        })?;

    // Construct S3 URL
    let s3_url = format!("https://{}.s3.amazonaws.com/{}", bucket_name, file_name);

    // Return JSON response
    Ok(HttpResponse::Ok().json(json!({ "uri": s3_url })))
}