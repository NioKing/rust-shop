use axum_shop::schema::{product_categories, products};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(
    Queryable, Selectable, Debug, PartialEq, Identifiable, Serialize, QueryableByName, Deserialize,
)]
#[diesel(table_name=products)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Product {
    pub id: i32,
    pub title: String,
    pub price: f64,
    pub description: String,
    pub image: Option<String>,
}

#[derive(Insertable, Deserialize, Validate)]
#[diesel(table_name = products)]
pub struct NewProduct {
    #[validate(length(min = 6, message = "Product title must be at least 6 symbols long"))]
    pub title: String,
    pub price: f64,
    #[validate(length(
        min = 6,
        message = "Product description must be at least 6 symbols long"
    ))]
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

#[derive(Insertable, Deserialize, AsChangeset)]
#[diesel(table_name = products)]
pub struct UpdateProduct {
    pub title: Option<String>,
    pub price: Option<f64>,
    pub description: Option<String>,
    pub image: Option<String>,
    // pub category_id: Option<i32>,
}

#[derive(Deserialize)]
pub struct CreateProductWithCategories {
    #[serde(flatten)]
    pub product: NewProduct,
    pub category_ids: Vec<i32>,
}

#[derive(Serialize, Debug)]
pub struct ProductWithCategories {
    #[serde(flatten)]
    pub product: Product,
    pub categories: Vec<crate::category::models::Category>,
}

#[derive(Deserialize, Debug)]
pub struct QueryParams {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub category_id: Option<i32>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub sort_by: Option<SortByParams>,
    pub sort_ord: Option<OrderByParams>,
    pub search_title: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OrderByParams {
    Asc,
    Desc,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum SortByParams {
    Id,
    Title,
    Price,
}
