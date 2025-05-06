use axum_shop::schema::products;
use diesel::data_types::PgMoney;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, PartialEq, Identifiable)]
#[diesel(table_name=products)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Product {
    pub id: i32,
    pub title: String,
    pub price: PgMoney,
    pub description: String,
    pub image: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = products)]
pub struct NewProduct {
    pub title: String,
    pub price: PgMoney,
    pub description: String,
    pub image: Option<String>,
}
