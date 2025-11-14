use super::models::{ApplicationHealthResponse, DbHealth, RmqHealth, Status};
use crate::utils::{internal_error, types::Pool};
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use diesel::{dsl::sql, prelude::*, sql_types::Integer};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use std::env;

pub async fn check_application_health(State(pool): State<Pool>) -> Json<ApplicationHealthResponse> {
    let res = ApplicationHealthResponse {
        database: db_check(&pool).await,
        rabbitmq: rmq_check().await,
    };

    Json(res)
}

async fn db_check(pool: &Pool) -> DbHealth {
    use axum_shop::schema::products;

    let mut conn = match pool.get().await {
        Ok(p) => p,
        Err(_) => return DbHealth::default(),
    };

    let now = std::time::Instant::now();

    let health_check = products::table
        .select(sql::<Integer>("1"))
        .first::<i32>(&mut conn)
        .await;

    let response_time_ms = now.elapsed().as_millis();

    if health_check.is_ok() {
        DbHealth {
            status: Status::Up,
            response_time_ms,
        }
    } else {
        DbHealth::default()
    }
}

async fn rmq_check() -> RmqHealth {
    use lapin::{Connection, ConnectionProperties, ConnectionState};
    use tokio_executor_trait::Tokio as TokioExec;
    use tokio_reactor_trait::Tokio as TokioReactor;

    let url = match env::var("RMQ_URL") {
        Ok(url) => url,
        Err(_) => return RmqHealth::default(),
    };

    let now = std::time::Instant::now();

    let conn = Connection::connect(
        &url,
        ConnectionProperties::default()
            .with_executor(TokioExec::current())
            .with_reactor(TokioReactor::current()),
    )
    .await;

    let response_time_ms = now.elapsed().as_millis();

    let res = match conn {
        Ok(connection) => {
            let channel_health = connection.create_channel().await.is_ok();

            RmqHealth {
                status: if channel_health && connection.status().connected() {
                    Status::Up
                } else {
                    Status::Down
                },
                response_time_ms,
            }
        }
        Err(_) => RmqHealth::default(),
    };

    res
}
