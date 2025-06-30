use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
pub type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;
