use axum_shop::schema::users;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, PartialEq)]
#[diesel(table_name=users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: uuid::Uuid,
    pub email: String,
    pub password_hash: String,
    pub hashed_rt: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=users)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
}
