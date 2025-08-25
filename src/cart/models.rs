use axum_shop::schema::carts;
use chrono::NaiveDate;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Queryable, Selectable, Identifiable, Serialize, Debug, Deserialize, Default, Clone)]
#[diesel(table_name=carts)]
#[diesel(belongs_to(User))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Cart {
    pub id: i32,
    pub user_id: Uuid,
    pub updated_at: NaiveDate,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = carts)]
pub struct NewCart {
    pub user_id: Uuid,
    pub updated_at: NaiveDate,
}

#[derive(Debug, Selectable, Queryable, Serialize)]
#[diesel(table_name=carts)]
pub struct SafeCart {
    pub id: i32,
    pub updated_at: NaiveDate,
}
