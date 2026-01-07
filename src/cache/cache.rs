use crate::utils::types::AppState;
use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use redis::{AsyncCommands, Client};
use reqwest::StatusCode;

use crate::error::AppError;

pub async fn cache_middleware(State(pool): State<AppState>, req: Request, next: Next) -> Response {
    if req.method() != axum::http::Method::GET {
        return next.run(req).await;
    }

    let key = req.uri().to_string();

    let mut conn = match pool.redis.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(_) => return next.run(req).await,
    };

    if let Ok(cached) = conn.get::<_, String>(&key).await {
        println!("middleware: {:?}", cached);

        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .header("X-Cache", "HIT")
            .body(Body::from(cached))
            .unwrap();
    };

    let response = next.run(req).await;

    if response.status() != StatusCode::OK {
        return response;
    }

    let (parts, body) = response.into_parts();
    let bytes = match to_bytes(body, 1_000_000).await {
        Ok(b) => b,
        Err(_) => return Response::from_parts(parts, Body::empty()),
    };

    let body_str = match String::from_utf8(bytes.clone().to_vec()) {
        Ok(s) => s,
        Err(_) => return Response::from_parts(parts, Body::from(bytes)),
    };

    let _: Result<(), _> = conn.set_ex(key, body_str.clone(), 30).await;

    let mut response = Response::from_parts(parts, Body::from(bytes));
    response
        .headers_mut()
        .insert("X-Cache", "MISS".parse().unwrap());

    response
}
