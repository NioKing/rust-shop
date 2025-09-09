use crate::product::models::Product;
use axum_shop::schema::{discount_products, discounts};
use diesel::deserialize::FromSqlRow;
use diesel::sql_types::Text;
use diesel::{expression::AsExpression, prelude::*};
use serde::{Deserialize, Serialize};
use validator::ValidationError;

#[derive(Debug, Serialize, Identifiable, Queryable, Selectable)]
#[diesel(table_name=discounts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
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

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name=discounts)]
pub struct NewDiscount {
    pub title: String,
    pub discount_type: String,
    pub amount: bigdecimal::BigDecimal,
    pub start_date: chrono::NaiveDateTime,
    pub end_date: chrono::NaiveDateTime,
    pub is_active: bool,
    pub applies_to_all: bool,
}

#[derive(Deserialize, Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
#[serde(rename_all = "lowercase")]
pub enum DiscountType {
    Percentage,
    Fixed,
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

impl NewDiscount {
    pub fn validate_dates(&self) -> Result<(), String> {
        if self.end_date <= self.start_date {
            return Err("End date must be after start date".to_string());
        }
        Ok(())
    }
}
