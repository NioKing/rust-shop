mod auth;
mod cart;
mod category;
mod product;
mod utils;

use axum::{
    Router,
    routing::{delete, get, patch},
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use listenfd::ListenFd;
use std::env;
// use std::net::SocketAddr;
use tokio::net::TcpListener;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");
#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = deadpool_diesel::postgres::Manager::new(db_url, deadpool_diesel::Runtime::Tokio1);
    let pool = deadpool_diesel::postgres::Pool::builder(manager)
        .build()
        .unwrap();
    //
    // {
    //     let conn = pool.get().await.unwrap();
    //     conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
    //         .await
    //         .unwrap()
    //         .unwrap();
    // }
    //
    let routes = Router::new()
        .route(
            "/products",
            get(product::handlers::get_products)
                .post(product::handlers::create_product_with_categories),
        )
        .route(
            "/products/{id}",
            delete(product::handlers::remove_product)
                .patch(product::handlers::update_product)
                .get(product::handlers::get_product_by_id),
        )
        .route(
            "/categories",
            get(category::handlers::get_categories).post(category::handlers::create_category),
        )
        .route(
            "/categories/{id}",
            patch(category::handlers::update_category).get(category::handlers::get_category_by_id),
        )
        .with_state(pool);
    let app = Router::new().nest("/api", routes);
    let app = app.fallback(utils::handler_404);
    // let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    // tracing::debug!("listening on {:?}", addr);
    // let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let mut listenfd = ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0).unwrap() {
        // if we are given a tcp listener on listen fd 0, we use that one
        Some(listener) => {
            listener.set_nonblocking(true).unwrap();
            TcpListener::from_std(listener).unwrap()
        }
        // otherwise fall back to local listening
        None => TcpListener::bind("127.0.0.1:3000").await.unwrap(),
    };
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
