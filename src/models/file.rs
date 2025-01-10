use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug)]
pub struct File {
    pub file_id: Uuid,
    pub user_id: Uuid,
    pub uri: String,
    pub created_at: chrono::DateTime<Utc>,
}