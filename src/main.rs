mod auth;
mod cart;
mod category;
mod product;

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::env;
use std::net::SocketAddr;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");
#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = deadpool_diesel::postgres::Manager::new(db_url, deadpool_diesel::Runtime::Tokio1);
    let pool = deadpool_diesel::postgres::Pool::builder(manager)
        .build()
        .unwrap();
    // {
    //     let conn = pool.get().await.unwrap();
    //     conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
    //         .await
    //         .unwrap()
    //         .unwrap();
    // }
    //
    //
    let routes = Router::new()
        .route(
            "/products",
            get(product::handlers::get_products).post(product::handlers::create_product),
        )
        .with_state(pool);
    let app = Router::new().nest("/api", routes);
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {:?}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
