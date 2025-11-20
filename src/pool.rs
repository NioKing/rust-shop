use crate::error::AppError;
use anyhow::Context;
use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use std::env;

pub async fn get_pool()
-> Result<bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>, AppError> {
    let db_url = env::var("DATABASE_URL").context("Db url must be set")?;

    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_url);
    let pool = bb8::Pool::builder()
        .build(config)
        .await
        .context("Failed to create a db pool")?;

    Ok(pool)
}
