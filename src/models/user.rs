use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug)]
pub struct User {
    pub user_id: Uuid,
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    pub user_image_uri: Option<String>,
    pub company_name: Option<String>,
    pub company_image_uri: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}