use axum_shop::schema::{addresses, profiles};
use chrono::NaiveDate;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = profiles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Profile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone_number: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub language: String,
    pub currency: String,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = profiles)]
pub struct UpdateProfile {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone_number: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub language: Option<String>,
    pub currency: Option<String>,
}

#[derive(Debug, Serialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = addresses)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Address {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: Option<String>,
    pub address_line: String,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = addresses)]
pub struct NewAddress {
    pub label: Option<String>,
    pub address_line: String,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = addresses)]
pub struct UpdateAddress {
    pub label: Option<String>,
    pub address_line: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}
