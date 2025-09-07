use crate::product::models::Product;
use axum_shop::schema::{discount_products, discounts};
use diesel::prelude::*;
use serde::Serialize;

#[derive(Debug, Serialize, Identifiable, Queryable)]
#[diesel(table_name=discounts)]
pub struct Discount {
    pub id: i32,
    pub title: String,
    pub discount_type: String,
    pub amount: bigdecimal::BigDecimal,
    pub start_date: chrono::NaiveDateTime,
    pub end_date: chrono::NaiveDateTime,
    pub is_active: bool,
    pub applies_to_all: bool,
}

#[derive(Debug, Queryable, Associations, Identifiable)]
#[diesel(table_name=discount_products)]
#[diesel(belongs_to(Discount))]
#[diesel(belongs_to(Product))]
#[diesel(primary_key(discount_id, product_id))]
pub struct DiscountProduct {
    pub discount_id: i32,
    pub product_id: i32,
}
