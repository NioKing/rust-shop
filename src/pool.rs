use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use std::env;

pub async fn get_pool() -> Result<bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>, String>
{
    let db_url =
        env::var("DATABASE_URL").map_err(|e| format!("Data base url must be set: {}", e))?;

    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(db_url);
    let pool = bb8::Pool::builder()
        .build(config)
        .await
        .map_err(|e| format!("Failed to create db pool: {}", e))?;

    Ok(pool)
}
