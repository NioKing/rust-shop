use axum_shop::schema::carts;
use diesel::data_types::PgDate;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Selectable, Debug, PartialEq, Identifiable)]
#[diesel(table_name=carts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Cart {
    pub id: i32,
    pub user_id: Uuid,
    pub updated_at: PgDate,
}

#[derive(Insertable)]
#[diesel(table_name = carts)]
pub struct NewCart {
    pub user_id: Uuid,
    pub updated_at: PgDate,
}
