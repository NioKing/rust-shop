mod auth;
mod cart;
mod category;
mod product;
mod utils;

use axum::{
    Router,
    middleware::{self},
    routing::{delete, get, patch, post},
};
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, bb8},
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use listenfd::ListenFd;
use std::env;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_url);
    let pool = bb8::Pool::builder().build(config).await.unwrap();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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
        .route(
            "/users",
            get(auth::handlers::get_all_users).post(auth::handlers::create_user),
        )
        .route(
            "/users/{id}",
            patch(auth::handlers::update_user_email_or_password)
                .get(auth::handlers::get_user_by_id),
        )
        .route("/carts", get(cart::handlers::get_all_cart))
        .layer(middleware::from_fn(utils::print_req_res))
        .with_state(pool);

    let app = Router::new().nest("/api", routes);
    let app = app.fallback(utils::handler_404);
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
