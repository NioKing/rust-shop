use axum_shop::schema::users;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Queryable, Selectable, Debug, Serialize, Insertable)]
#[diesel(table_name=users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: uuid::Uuid,
    pub email: String,
    pub password_hash: String,
    pub hashed_rt: Option<String>,
}

#[derive(Insertable, Deserialize, Debug, Validate)]
#[diesel(table_name=users)]
pub struct NewUser {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 6, message = "Your password should be at least 6 symbols long"))]
    pub password_hash: String,
}

#[derive(Deserialize, Insertable, Validate)]
#[diesel(table_name=users)]
pub struct UserEmail {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Selectable, Queryable, Serialize)]
#[diesel(table_name=users)]
pub struct SafeUser {
    pub id: uuid::Uuid,
    pub email: String,
}

#[derive(Insertable, Deserialize, Debug, Validate)]
#[diesel(table_name=users)]
pub struct UpdateUser {
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 6, message = "Your password should be at least 6 symbols long"))]
    pub password_hash: Option<String>,
}
