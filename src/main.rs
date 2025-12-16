#![allow(unused)]
mod auth;
mod cache;
mod cart;
mod category;
mod discount;
mod error;
mod health;
mod metrics;
mod notification;
mod pool;
mod product;
mod rmq;
mod user;
mod utils;

use anyhow::Context;
use axum::{
    Router,
    middleware::{self},
};
use listenfd::ListenFd;
use std::env;
use tokio::net::TcpListener;
use tokio_cron_scheduler::JobScheduler;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, timeout::TimeoutLayer, trace::TraceLayer};
use tracing_subscriber::{fmt::layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{error::AppError, pool::get_pool, rmq::client};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv::dotenv().ok();

    std::fs::create_dir_all("uploads").context("Failed to create directory")?;

    let pool = get_pool().await?;
    let redis = redis::Client::open(env::var("REDIS_URL").context("redis url must be set")?)?;
    let state = cache::AppState { redis };

    let mut scheduler = JobScheduler::new()
        .await
        .context("Failed to build a scheduler")?;

    tokio::spawn(async move {
        // scheduler.start().await.unwrap();
    });

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
        .merge(user::routes::get_routes())
        .merge(health::routes::get_routes())
        .merge(metrics::routes::get_routes())
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(utils::print_req_res))
                .layer((
                    TraceLayer::new_for_http(),
                    TimeoutLayer::new(std::time::Duration::from_secs(10)),
                ))
                .layer(middleware::from_fn(metrics::track_metrics))
                .layer(middleware::from_fn_with_state(
                    state,
                    cache::cache_middleware,
                )),
        )
        .with_state(pool.clone());

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
        None => TcpListener::bind("0.0.0.0:3000").await.unwrap(),
    };

    client::spawn_consumer(
        "notifications",
        "discount_consumer",
        pool.clone(),
        crate::notification::handlers::send_email,
    );

    client::spawn_consumer(
        "user",
        "user_consumer",
        pool.clone(),
        crate::notification::handlers::send_email,
    );

    println!("listening on {}", listener.local_addr().unwrap());

    tokio::spawn(async {
        metrics::start_metrics_server().await;
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(crate::utils::shutdown_signal())
        .await
        .unwrap();

    Ok(())
}
