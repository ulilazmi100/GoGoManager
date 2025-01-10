use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;
use uuid::Uuid;
// use time::OffsetDateTime;
use crate::utils;
use crate::models::department::Department;
use actix_web::error::{ErrorBadRequest, ErrorUnauthorized, ErrorConflict, ErrorNotFound};
use jsonwebtoken::errors::Error as JwtError;
use validator::ValidationErrors;
use chrono::Utc;
use crate::errors::AppError;

#[derive(Deserialize, Validate)]
struct NewDepartment {
    #[validate(length(min = 4, max = 33))]
    name: String,
}

#[derive(Serialize)]
struct DepartmentResponse {
    department_id: Uuid,
    name: String,
}

#[derive(Deserialize)]
struct DepartmentQueryParams {
    name: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Deserialize, Validate)]
struct DepartmentUpdate {
    #[validate(length(min = 4, max = 33))]
    name: String,
}

fn map_validation_error(err: ValidationErrors) -> actix_web::Error {
    ErrorBadRequest(err.to_string())
}

fn map_jwt_error(_err: JwtError) -> actix_web::Error {
    ErrorUnauthorized("Invalid token")
}

pub async fn create_department(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    new_department: web::Json<NewDepartment>,
) -> Result<HttpResponse, actix_web::Error> {
    new_department.validate().map_err(map_validation_error)?;

    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        let _claims = utils::jwt::validate_token(token).map_err(map_jwt_error)?;

        if sqlx::query!("SELECT name FROM departments WHERE name = $1", &new_department.name.clone())
            .fetch_optional(&**pool)
            .await
            .map_err(|err| AppError::DatabaseError(err.to_string()))?
            .is_some()
        {
            return Err(ErrorConflict("Department name already exists"));
        }

        let department_id = Uuid::new_v4();
        let now = Utc::now();

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

        Ok(HttpResponse::Created().json(DepartmentResponse {
            department_id,
            name: new_department.name.clone(),
        }))
    } else {
        Err(ErrorUnauthorized("Missing token"))
    }
}

pub async fn get_departments(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    query: web::Query<DepartmentQueryParams>,
) -> Result<HttpResponse, actix_web::Error> {
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        let _claims = utils::jwt::validate_token(token).map_err(map_jwt_error)?;

        let mut query_builder: sqlx::QueryBuilder<'_, sqlx::Postgres> = sqlx::QueryBuilder::new("SELECT * FROM departments");
        let mut params: Vec<String> = Vec::new();

        if let Some(name) = &query.name {
            query_builder.push(" WHERE name LIKE $1");
            params.push(format!("%{}%", name));
        }

        query_builder.push(" ORDER BY created_at DESC");

        if let Some(limit) = query.limit {
            query_builder.push(format!(" LIMIT {}", limit));
        }

        if let Some(offset) = query.offset {
            query_builder.push(format!(" OFFSET {}", offset));
        }

        let sql = query_builder.sql(); // Extract the SQL query as a string
        let departments = sqlx::query_as::<_, Department>(sql) // Pass the SQL query string
            .fetch_all(&**pool)
            .await
            .map_err(|err| AppError::DatabaseError(err.to_string()))?;

        Ok(HttpResponse::Ok().json(departments))
    } else {
        Err(ErrorUnauthorized("Missing token"))
    }
}

pub async fn update_department(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    department_id: web::Path<String>,
    updates: web::Json<DepartmentUpdate>,
) -> Result<HttpResponse, actix_web::Error> {
    updates.validate().map_err(map_validation_error)?;

    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        let _claims = utils::jwt::validate_token(token).map_err(map_jwt_error)?;

        let department_id = Uuid::parse_str(&department_id.into_inner())
            .map_err(|_| ErrorBadRequest("Invalid department ID"))?;

        let department = sqlx::query!("SELECT * FROM departments WHERE department_id = $1", department_id)
            .fetch_optional(&**pool)
            .await
            .map_err(|err| AppError::DatabaseError(err.to_string()))?;

        if department.is_none() {
            return Err(ErrorNotFound("Department not found"));
        }

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

        Ok(HttpResponse::Ok().json(json!({
            "departmentId": department_id,
            "name": updates.name,
        })))
    } else {
        Err(ErrorUnauthorized("Missing token"))
    }
}

pub async fn delete_department(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    department_id: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        let _claims = utils::jwt::validate_token(token).map_err(map_jwt_error)?;

        let department_id = Uuid::parse_str(&department_id.into_inner())
            .map_err(|_| ErrorBadRequest("Invalid department ID"))?;

        let department = sqlx::query!("SELECT * FROM departments WHERE department_id = $1", department_id)
            .fetch_optional(&**pool)
            .await
            .map_err(|err| AppError::DatabaseError(err.to_string()))?;

        if department.is_none() {
            return Err(ErrorNotFound("Department not found"));
        }

        let employees = sqlx::query!("SELECT * FROM employees WHERE department_id = $1", department_id)
            .fetch_all(&**pool)
            .await
            .map_err(|err| AppError::DatabaseError(err.to_string()))?;

        if !employees.is_empty() {
            return Err(ErrorConflict("Department still contains employees"));
        }

        sqlx::query!("DELETE FROM departments WHERE department_id = $1", department_id)
            .execute(&**pool)
            .await
            .map_err(|err| AppError::DatabaseError(err.to_string()))?;

        Ok(HttpResponse::Ok().json(json!({
            "message": "Department deleted successfully",
        })))
    } else {
        Err(ErrorUnauthorized("Missing token"))
    }
}