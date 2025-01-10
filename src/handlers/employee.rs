use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};
// use time::OffsetDateTime;
use validator::Validate;
use uuid::Uuid;
use chrono::Utc;
use crate::utils;
use crate::models::employee::Employee;
use serde_json::json;

#[derive(Deserialize, Validate)]
struct NewEmployee {
    #[validate(length(min = 5, max = 33))]
    identity_number: String,
    #[validate(length(min = 4, max = 33))]
    name: String,
    #[validate(url)]
    employee_image_uri: Option<String>,
    #[validate(custom = "validate_gender")]
    gender: String,
    #[validate(length(min = 36, max = 36))]
    department_id: String,
}

#[derive(Serialize)]
struct EmployeeResponse {
    identity_number: String,
    name: String,
    employee_image_uri: Option<String>,
    gender: String,
    department_id: String,
}

#[derive(Deserialize)]
struct EmployeeQueryParams {
    identity_number: Option<String>,
    name: Option<String>,
    gender: Option<String>,
    department_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Deserialize, Validate)]
struct EmployeeUpdate {
    #[validate(length(min = 5, max = 33))]
    identity_number: Option<String>,
    #[validate(length(min = 4, max = 33))]
    name: Option<String>,
    #[validate(url)]
    employee_image_uri: Option<String>,
    #[validate(custom = "validate_gender")]
    gender: Option<String>,
    #[validate(length(min = 36, max = 36))]
    department_id: Option<String>,
}

fn validate_gender(gender: &str) -> Result<(), validator::ValidationError> {
    if gender != "male" && gender != "female" {
        return Err(validator::ValidationError::new("Gender must be either 'male' or 'female'"));
    }
    Ok(())
}

pub async fn create_employee(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    new_employee: web::Json<NewEmployee>,
) -> Result<HttpResponse, actix_web::Error> {
    new_employee.validate()
        .map_err(|err| actix_web::error::ErrorBadRequest(err.to_string()))?;

    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        let _claims = utils::jwt::validate_token(token)
            .map_err(|err| actix_web::error::ErrorUnauthorized(err.to_string()))?;

        // Check if the identity_number already exists
        if sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM employees WHERE identity_number = $1)",
            &new_employee.identity_number
        )
        .fetch_one(&**pool)
        .await
        .map_err(|err| actix_web::error::ErrorInternalServerError(err.to_string()))?
        .unwrap_or(false)
        {
            return Err(actix_web::error::ErrorConflict("Identity number already exists"));
        }

        // Parse department_id into Uuid
        let department_id = Uuid::parse_str(&new_employee.department_id)
            .map_err(|_| actix_web::error::ErrorBadRequest("Invalid department ID"))?;

        // Convert chrono::DateTime<Utc> to OffsetDateTime
        let now = Utc::now();

        let employee_id = Uuid::new_v4();

        sqlx::query!(
            "INSERT INTO employees (employee_id, identity_number, name, employee_image_uri, gender, department_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            employee_id,
            &new_employee.identity_number,
            &new_employee.name,
            new_employee.employee_image_uri,
            &new_employee.gender,
            department_id, // Use parsed Uuid
            now,           // Use OffsetDateTime
            now            // Use OffsetDateTime
        )
        .execute(&**pool)
        .await
        .map_err(|err| actix_web::error::ErrorInternalServerError(err.to_string()))?;

        Ok(HttpResponse::Created().json(EmployeeResponse {
            identity_number: new_employee.identity_number.clone(),
            name: new_employee.name.clone(),
            employee_image_uri: new_employee.employee_image_uri.clone(),
            gender: new_employee.gender.clone(),
            department_id: new_employee.department_id.clone(),
        }))
    } else {
        Err(actix_web::error::ErrorUnauthorized("Missing token"))?
    }
}

pub async fn get_employees(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    query: web::Query<EmployeeQueryParams>,
) -> Result<HttpResponse, actix_web::Error> {
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        let _claims = utils::jwt::validate_token(token)
            .map_err(|err| actix_web::error::ErrorUnauthorized(err.to_string()))?;

        let mut query_builder: sqlx::QueryBuilder<'_, sqlx::Postgres> =
            sqlx::QueryBuilder::new("SELECT * FROM employees");

        let mut params: Vec<String> = Vec::new();

        if let Some(identity_number) = &query.identity_number {
            query_builder.push(" WHERE identity_number LIKE $1");
            params.push(format!("{}%", identity_number));
        }
        if let Some(name) = &query.name {
            if !params.is_empty() {
                query_builder.push(" AND name LIKE $2");
            } else {
                query_builder.push(" WHERE name LIKE $1");
            }
            params.push(format!("%{}%", name));
        }
        if let Some(gender) = &query.gender {
            if !params.is_empty() {
                query_builder.push(" AND gender = $");
                params.push(gender.clone());
            } else {
                query_builder.push(" WHERE gender = $1");
                params.push(gender.clone());
            }
        }
        if let Some(department_id) = &query.department_id {
            if !params.is_empty() {
                query_builder.push(" AND department_id = $");
                params.push(department_id.clone());
            } else {
                query_builder.push(" WHERE department_id = $1");
                params.push(department_id.clone());
            }
        }

        query_builder.push(" ORDER BY created_at DESC");

        if let Some(limit) = query.limit {
            query_builder.push(format!(" LIMIT {}", limit));
        }

        if let Some(offset) = query.offset {
            query_builder.push(format!(" OFFSET {}", offset));
        }

        let sql = query_builder.sql(); // Get the SQL query string

        let employees = sqlx::query_as::<_, Employee>(sql) // Pass the SQL query string
            .fetch_all(&**pool)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Query failed"))?;

        Ok(HttpResponse::Ok().json(employees))
    } else {
        Err(actix_web::error::ErrorUnauthorized("Missing token"))?
    }
}

pub async fn update_employee(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    identity_number: web::Path<String>,
    updates: web::Json<EmployeeUpdate>,
) -> Result<HttpResponse, actix_web::Error> {
    updates.validate()
        .map_err(|err| actix_web::error::ErrorBadRequest(err.to_string()))?;

    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        let _claims = utils::jwt::validate_token(token)
            .map_err(|err| actix_web::error::ErrorUnauthorized(err.to_string()))?;

        let identity_number = identity_number.into_inner();

        let employee = sqlx::query!("SELECT * FROM employees WHERE identity_number = $1", identity_number)
            .fetch_optional(&**pool)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Query failed"))?;

        if employee.is_none() {
            return Err(actix_web::error::ErrorNotFound("Employee not found"))?;
        }

        let mut query = "UPDATE employees SET".to_string();
        let mut params: Vec<String> = Vec::new();
        let mut set_clauses = Vec::new();

        if let Some(identity_number) = &updates.identity_number {
            set_clauses.push("identity_number = $1".to_string());
            params.push(identity_number.clone());
        }
        if let Some(name) = &updates.name {
            set_clauses.push("name = $2".to_string());
            params.push(name.clone());
        }
        if let Some(employee_image_uri) = &updates.employee_image_uri {
            set_clauses.push("employee_image_uri = $3".to_string());
            params.push(employee_image_uri.clone());
        }
        if let Some(gender) = &updates.gender {
            set_clauses.push("gender = $4".to_string());
            params.push(gender.clone());
        }
        if let Some(department_id) = &updates.department_id {
            set_clauses.push("department_id = $5".to_string());
            params.push(department_id.clone());
        }

        let now = Utc::now();
        set_clauses.push("updated_at = $6".to_string());
        params.push(now.to_string());

        query.push_str(&set_clauses.join(", "));
        query.push_str(" WHERE identity_number = $7");
        params.push(identity_number.clone());

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

        let updated_employee = sqlx::query_as!(
            Employee,
            "SELECT * FROM employees WHERE identity_number = $1",
            identity_number
        )
        .fetch_one(&**pool)
        .await
        .map_err(|_| actix_web::error::ErrorNotFound("Employee not found"))?;

        Ok(HttpResponse::Ok().json(updated_employee))
    } else {
        Err(actix_web::error::ErrorUnauthorized("Missing token"))?
    }
}

pub async fn delete_employee(
    req: HttpRequest,
    pool: web::Data<sqlx::PgPool>,
    identity_number: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let token = req.headers().get("Authorization")
        .and_then(|auth| auth.to_str().ok())
        .and_then(|auth| auth.split_whitespace().nth(1));

    if let Some(token) = token {
        let _claims = utils::jwt::validate_token(token)
            .map_err(|err| actix_web::error::ErrorUnauthorized(err.to_string()))?;

        let identity_number = identity_number.into_inner();

        let employee = sqlx::query!("SELECT * FROM employees WHERE identity_number = $1", identity_number)
            .fetch_optional(&**pool)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Query failed"))?;

        if employee.is_none() {
            return Err(actix_web::error::ErrorNotFound("Employee not found"))?;
        }

        sqlx::query!("DELETE FROM employees WHERE identity_number = $1", identity_number)
            .execute(&**pool)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("Delete failed"))?;

        Ok(HttpResponse::Ok().json(json!({
            "message": "Employee deleted successfully",
        })))
    } else {
        Err(actix_web::error::ErrorUnauthorized("Missing token"))?
    }
}