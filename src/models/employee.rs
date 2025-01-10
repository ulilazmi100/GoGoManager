use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug)]
pub struct Employee {
    pub employee_id: Uuid,
    pub identity_number: String,
    pub name: String,
    pub employee_image_uri: Option<String>,
    pub gender: String,
    pub department_id: Uuid,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}