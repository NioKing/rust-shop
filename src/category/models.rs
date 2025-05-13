use axum_shop::schema::{categories, product_categories};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::product::models::Product;

#[derive(Queryable, Selectable, Debug, PartialEq, Identifiable, Serialize)]
#[diesel(table_name=categories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Category {
    pub id: i32,
    pub title: String,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = categories)]
pub struct NewCategory {
    pub title: String,
}

#[derive(Identifiable, Selectable, Queryable, Associations, Debug, Serialize, Deserialize)]
#[diesel(belongs_to(Product))]
#[diesel(belongs_to(Category))]
#[diesel(table_name = product_categories)]
#[diesel(primary_key(product_id, category_id))]
pub struct ProductCategories {
    pub product_id: i32,
    pub category_id: i32,
}
