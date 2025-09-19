#![allow(unused)]
mod auth;
mod cart;
mod category;
mod discount;
mod notification;
mod pool;
mod product;
mod rmq;
mod utils;

use axum::{
    Router,
    http::StatusCode,
    middleware::{self},
    routing::{delete, get, patch, post},
};
use axum_shop::schema::products;
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, bb8},
};
use listenfd::ListenFd;
use std::env;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{pool::get_pool, utils::internal_error};

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenv::dotenv().ok();

    std::fs::create_dir_all("uploads").expect("Failed to create uploads directory");

    let pool = get_pool().await?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let routes = Router::new()
        .nest_service("/uploads", ServeDir::new("uploads"))
        .merge(product::routes::get_routes())
        .merge(category::routes::get_routes())
        .merge(auth::routes::get_routes())
        .merge(cart::routes::get_routes())
        .merge(discount::routes::get_routes())
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

    tokio::spawn(async move {
        if let Err(er) = rmq::client::consume(
            "notifications",
            "discount_consumer",
            notification::handlers::send_email,
        )
        .await
        {
            eprintln!("Error: {:?}", er);
        }
    });

    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
