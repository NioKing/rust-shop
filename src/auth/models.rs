use axum_shop::schema::users;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Debug, Serialize, Insertable)]
#[diesel(table_name=users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: uuid::Uuid,
    pub email: String,
    pub password_hash: String,
    pub hashed_rt: Option<String>,
}

#[derive(Insertable, Deserialize, Debug)]
#[diesel(table_name=users)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name=users)]
pub struct UserEmail {
    pub email: String,
}

#[derive(Debug, Selectable, Queryable, Serialize)]
#[diesel(table_name=users)]
pub struct SafeUser {
    pub id: uuid::Uuid,
    pub email: String,
}

#[derive(Insertable, Deserialize, Debug)]
#[diesel(table_name=users)]
pub struct UpdateUser {
    pub email: Option<String>,
    pub password_hash: Option<String>,
}
