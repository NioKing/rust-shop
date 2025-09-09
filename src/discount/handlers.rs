use super::models::{Discount, DiscountProduct, DiscountType, NewDiscount};
use crate::utils::internal_error;
use crate::utils::types::Pool;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

pub async fn get_all_discounts(
    State(pool): State<Pool>,
) -> Result<Json<Vec<Discount>>, (StatusCode, String)> {
    use axum_shop::schema::discounts;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = discounts::table
        .select(Discount::as_select())
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn create_discount(
    State(pool): State<Pool>,
    Json(payload): Json<NewDiscount>,
) -> Result<Json<Discount>, (StatusCode, String)> {
    use axum_shop::schema::discounts;

    let mut conn = pool.get().await.map_err(internal_error)?;

    if let Err(e) = payload.validate_dates() {
        return Err((StatusCode::BAD_REQUEST, format!("{}", e)));
    }

    // if payload.discount_type.to_lowercase() != "fixed"
    //     && payload.discount_type.to_lowercase() != "percentage"
    // {
    //     return Err((StatusCode::BAD_REQUEST, "Wrong discount_type".to_owned()));
    // }

    let discount_type = payload.discount_type.to_lowercase();

    if !matches!(discount_type.as_str(), "fixed" | "percentage") {
        return Err((StatusCode::BAD_REQUEST, "Wrong discount_type".to_owned()));
    }

    let res = diesel::insert_into(discounts::table)
        .values(&payload)
        .returning(Discount::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}
