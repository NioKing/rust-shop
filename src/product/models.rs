use axum_shop::schema::{product_categories, products};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Debug, PartialEq, Identifiable, Serialize)]
#[diesel(table_name=products)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Product {
    pub id: i32,
    pub title: String,
    pub price: f64,
    pub description: String,
    pub image: Option<String>,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = products)]
pub struct NewProduct {
    pub title: String,
    pub price: f64,
    pub description: String,
    pub image: Option<String>,
}

#[derive(Insertable, Debug, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(belongs_to(Product))]
#[diesel(belongs_to(Category))]
#[diesel(table_name = product_categories)]
#[diesel(primary_key(product_id, category_id))]
pub struct ProductCategory {
    pub product_id: i32,
    pub category_id: i32,
}
