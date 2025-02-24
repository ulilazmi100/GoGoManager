use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID (UUID)
    pub exp: usize,  // Expiration timestamp
}

pub fn generate_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .expect("Invalid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(), // Use user_id instead of email
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&env::var("JWT_SECRET").unwrap().as_ref()),
    )
}

pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(&env::var("JWT_SECRET").unwrap().as_ref()),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )
    .map(|data| data.claims)
}