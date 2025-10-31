use axum_shop::schema::{addresses, profiles, user_subscriptions};
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

#[derive(Debug, Serialize, Queryable, Selectable, Insertable, QueryableByName)]
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
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub is_default: bool,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = addresses)]
pub struct NewAddress {
    pub label: Option<String>,
    pub address_line: String,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = addresses)]
pub struct UpdateAddressPayload {
    pub label: Option<String>,
    pub address_line: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = addresses)]
pub struct UpdateAddress {
    pub label: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct GeocodeResponse {
    pub lat: String,
    pub lon: String,
    pub display_name: String,
    pub address: AddressExtras,
}

#[derive(Debug, Deserialize)]
pub struct AddressExtras {
    pub city: String,
    pub city_district: String,
    pub state: String,
    pub postcode: String,
    pub country: String,
}

#[derive(Debug, Serialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = user_subscriptions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserSubscriptions {
    pub channel: String,
    pub orders_notifications: Option<bool>,
    pub discount_notifications: Option<bool>,
    pub newsletter_notifications: Option<bool>,
}

#[derive(Debug, Serialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = user_subscriptions)]
pub struct NewUserSubscriptions {
    pub user_id: uuid::Uuid,
    pub channel: String,
    pub orders_notifications: bool,
    pub discount_notifications: bool,
    pub newsletter_notifications: bool,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = user_subscriptions)]
pub struct UpdateUserSubscriptions {
    pub channel: Option<String>,
    pub orders_notifications: Option<bool>,
    pub discount_notifications: Option<bool>,
    pub newsletter_notifications: Option<bool>,
}
