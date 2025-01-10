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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

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
            .service(
                web::resource("/auth")
                    .route(web::post().to(handlers::auth::auth_handler)),
            )
            .service(
                web::resource("/user")
                    .route(web::get().to(handlers::user::get_user_profile))
                    .route(web::patch().to(handlers::user::update_user_profile)),
            )
            .service(
                web::resource("/file")
                    .route(web::post().to(handlers::file::upload_file)),
            )
            .service(
                web::resource("/employee")
                    .route(web::post().to(handlers::employee::create_employee))
                    .route(web::get().to(handlers::employee::get_employees))
                    .route(web::patch().to(handlers::employee::update_employee))
                    .route(web::delete().to(handlers::employee::delete_employee)),
            )
            .service(
                web::resource("/department")
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