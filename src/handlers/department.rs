use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;
use uuid::Uuid;
use chrono::Utc;
use jsonwebtoken::errors::Error as JwtError;
use validator::ValidationErrors;
use crate::utils;
use crate::models::department::Department;
use crate::errors::AppError;
use actix_web::error::{ErrorBadRequest, ErrorUnauthorized, ErrorConflict, ErrorNotFound};

#[derive(Deserialize, Validate)]
pub struct NewDepartment {
    #[validate(length(min = 4, max = 33))]
    name: String,
}

#[derive(Serialize)]
struct DepartmentResponse {
    #[serde(rename = "departmentId")]
    department_id: Uuid,
    name: String,
}

#[derive(Deserialize)]
pub struct DepartmentQueryParams {
    name: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Deserialize, Validate)]
pub struct DepartmentUpdate {
    #[validate(length(min = 4, max = 33))]
    name: String,
}

fn map_validation_error(err: ValidationErrors) -> actix_web::Error {
    ErrorBadRequest(json!({ "error": err.to_string() }))
}

fn map_jwt_error(_err: JwtError) -> actix_web::Error {
    ErrorUnauthorized(json!({ "error": "Invalid or expired token" }))
}

/// Extracts and validates the token from the request.
/// Returns `401 Unauthorized` if the token is missing or empty.
fn extract_and_validate_token(req: &HttpRequest) -> Result<(), actix_web::Error> {
    let token = req.headers()
        .get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1))
        .filter(|token| !token.is_empty())
        .ok_or_else(|| ErrorUnauthorized(json!({ "error": "Missing or empty token" })))?;

    // Validate the token
    utils::jwt::validate_token(token).map_err(map_jwt_error)?;
    Ok(())
}

pub async fn create_department(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    new_department: web::Json<NewDepartment>,
) -> Result<HttpResponse, actix_web::Error> {
    // Validate the token (FIRST STEP)
    extract_and_validate_token(&req)?;

    // Validate the input payload (SECOND STEP)
    new_department.validate().map_err(map_validation_error)?;

    // Check if the department name already exists
    if sqlx::query!("SELECT name FROM departments WHERE name = $1", &new_department.name)
        .fetch_optional(&**pool)
        .await
        .map_err(|err| AppError::DatabaseError(err.to_string()))?
        .is_some()
    {
        return Err(ErrorConflict(json!({ "error": "Department name already exists" })));
    }

    // Generate a new department ID and current timestamp
    let department_id = Uuid::new_v4();
    let now = Utc::now();

    // Insert the new department into the database
    sqlx::query!(
        "INSERT INTO departments (department_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4)",
        department_id,
        &new_department.name,
        now,
        now
    )
    .execute(&**pool)
    .await
    .map_err(|err| AppError::DatabaseError(err.to_string()))?;

    // Return the created department as a response
    Ok(HttpResponse::Created().json(DepartmentResponse {
        department_id,
        name: new_department.name.clone(),
    }))
}

pub async fn get_departments(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    query: web::Query<DepartmentQueryParams>,
) -> Result<HttpResponse, actix_web::Error> {
    // Validate the token (FIRST STEP)
    extract_and_validate_token(&req)?;

    // Build the SQL query dynamically based on query parameters
    let mut query_builder = sqlx::QueryBuilder::new("SELECT * FROM departments");

    if let Some(name) = &query.name {
        query_builder.push(" WHERE name ILIKE ");
        query_builder.push_bind(format!("%{}%", name));
    }

    query_builder.push(" ORDER BY created_at DESC");

    if let Some(limit) = query.limit {
        query_builder.push(" LIMIT ");
        query_builder.push_bind(limit);
    }

    if let Some(offset) = query.offset {
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);
    }

    // Execute the query and fetch departments
    let departments = query_builder
        .build_query_as::<Department>()
        .fetch_all(&**pool)
        .await
        .map_err(|err| AppError::DatabaseError(err.to_string()))?;

    // Map the response to camelCase keys
    let response = departments.into_iter().map(|dept| json!({
        "departmentId": dept.department_id,
        "name": dept.name,
        "createdAt": dept.created_at,
        "updatedAt": dept.updated_at,
    }))
    .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_department(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    department_id: web::Path<String>,
    updates: web::Json<DepartmentUpdate>,
) -> Result<HttpResponse, actix_web::Error> {
    // Validate the token (FIRST STEP)
    extract_and_validate_token(&req)?;

    // Validate the input payload (SECOND STEP)
    updates.validate().map_err(map_validation_error)?;

    // Parse the department ID
    let department_id = Uuid::parse_str(&department_id.into_inner())
        .map_err(|_| ErrorBadRequest(json!({ "error": "Invalid department ID" })))?;

    // Check if the department exists
    let department = sqlx::query!("SELECT * FROM departments WHERE department_id = $1", department_id)
        .fetch_optional(&**pool)
        .await
        .map_err(|err| AppError::DatabaseError(err.to_string()))?;

    if department.is_none() {
        return Err(ErrorNotFound(json!({ "error": "Department not found" })));
    }

    // Update the department
    let now = Utc::now();
    sqlx::query!(
        "UPDATE departments SET name = $1, updated_at = $2 WHERE department_id = $3",
        &updates.name,
        now,
        department_id
    )
    .execute(&**pool)
    .await
    .map_err(|err| AppError::DatabaseError(err.to_string()))?;

    // Return the updated department
    Ok(HttpResponse::Ok().json(json!({
        "departmentId": department_id,
        "name": updates.name,
    })))
}

pub async fn delete_department(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    department_id: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    // Validate the token (FIRST STEP)
    extract_and_validate_token(&req)?;

    // Parse the department ID
    let department_id = Uuid::parse_str(&department_id.into_inner())
        .map_err(|_| ErrorBadRequest(json!({ "error": "Invalid department ID" })))?;

    // Check if the department exists
    let department = sqlx::query!("SELECT * FROM departments WHERE department_id = $1", department_id)
        .fetch_optional(&**pool)
        .await
        .map_err(|err| AppError::DatabaseError(err.to_string()))?;

    if department.is_none() {
        return Err(ErrorNotFound(json!({ "error": "Department not found" })));
    }

    // Check if the department has employees
    let employees = sqlx::query!("SELECT * FROM employees WHERE department_id = $1", department_id)
        .fetch_all(&**pool)
        .await
        .map_err(|err| AppError::DatabaseError(err.to_string()))?;

    if !employees.is_empty() {
        return Err(ErrorConflict(json!({ "error": "Department still contains employees" })));
    }

    // Delete the department
    sqlx::query!("DELETE FROM departments WHERE department_id = $1", department_id)
        .execute(&**pool)
        .await
        .map_err(|err| AppError::DatabaseError(err.to_string()))?;

    Ok(HttpResponse::Ok().json(json!({
        "message": "Department deleted successfully",
    })))
}