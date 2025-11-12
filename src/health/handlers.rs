use super::models::{ApplicationHealthResponse, DbHealth, RmqHealth, Status};
use crate::utils::{internal_error, types::Pool};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use diesel::{dsl::sql, prelude::*, sql_types::Integer};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use std::env;

pub async fn check_application_health(
    State(pool): State<Pool>,
) -> Result<Json<ApplicationHealthResponse>, (StatusCode, String)> {
    let res = ApplicationHealthResponse {
        database: db_check(&pool).await?,
        rabbitmq: rmq_check()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)))?,
    };

    Ok(Json(res))
}

async fn db_check(pool: &Pool) -> Result<DbHealth, (StatusCode, String)> {
    use axum_shop::schema::products;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let health_check = products::table
        .select(diesel::dsl::sql::<Integer>("1"))
        .first::<i32>(&mut conn)
        .await
        .map_err(internal_error)?;

    let status = if health_check == 1 {
        Status::Up
    } else {
        Status::Down
    };

    let res = DbHealth { status };

    Ok(res)
}

async fn rmq_check() -> Result<RmqHealth, Box<dyn std::error::Error>> {
    use lapin::{Connection, ConnectionProperties, ConnectionState};
    use tokio_executor_trait::Tokio as TokioExec;
    use tokio_reactor_trait::Tokio as TokioReactor;

    let url = env::var("RMQ_URL")?;

    let conn = Connection::connect(
        &url,
        ConnectionProperties::default()
            .with_executor(TokioExec::current())
            .with_reactor(TokioReactor::current()),
    )
    .await;

    let res = match conn {
        Ok(connection) => {
            let channel_health = connection.create_channel().await.is_ok();

            RmqHealth {
                status: if channel_health && connection.status().connected() {
                    Status::Up
                } else {
                    Status::Down
                },
            }
        }
        Err(_) => RmqHealth {
            status: Status::Down,
        },
    };

    Ok(res)
}
