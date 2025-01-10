use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug)]
pub struct Employee {
    pub employee_id: Uuid,
    pub identity_number: String,
    pub name: String,
    pub employee_image_uri: Option<String>,
    pub gender: String,
    pub department_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}