use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

pub mod schema;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("DATABASE URL MUST BE SET");
    PgConnection::establish(&db_url)
        .unwrap_or_else(|_| panic!("failed to connect to db url {}", db_url))
}
