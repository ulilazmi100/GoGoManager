mod handlers;
mod models;
mod utils;
mod db;
mod errors;

use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use sqlx::PgPool;
use std::env;
use log::info;
use crate::utils::s3::create_s3_client;
use env_logger::Env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Initialize S3 client
    let s3_client = create_s3_client().await;

    // Validate JWT secret
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    if jwt_secret.is_empty() {
        panic!("JWT_SECRET cannot be empty");
    }

    // Initialize the database pool
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await.expect("Failed to connect to the database");

    info!("Starting server at 127.0.0.1:8080");

    // Start the HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(s3_client.clone())) // Add S3 client to app data
            .service(
                web::resource("/v1/auth")
                    .route(web::post().to(handlers::auth::auth_handler)),
            )
            .service(
                web::resource("/v1/user")
                    .route(web::get().to(handlers::user::get_user_profile))
                    .route(web::patch().to(handlers::user::update_user_profile)),
            )
            .service(
                web::resource("/v1/file")
                    .route(web::post().to(handlers::file::upload_file)),
            )
            .service(
                web::resource("/v1/employee")
                    .route(web::post().to(handlers::employee::create_employee))
                    .route(web::get().to(handlers::employee::get_employees))
                    .route(web::patch().to(handlers::employee::update_employee))
                    .route(web::delete().to(handlers::employee::delete_employee)),
            )
            .service(
                web::resource("/v1/department")
                    .route(web::post().to(handlers::department::create_department))
                    .route(web::get().to(handlers::department::get_departments))
                    .route(web::patch().to(handlers::department::update_department))
                    .route(web::delete().to(handlers::department::delete_department)),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}