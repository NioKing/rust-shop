use super::models::Cart;
use crate::utils::internal_error;
use crate::utils::types::Pool;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

pub async fn get_all_cart(
    State(pool): State<Pool>,
) -> Result<Json<Vec<Cart>>, (StatusCode, String)> {
    use axum_shop::schema::carts;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = carts::table
        .select(Cart::as_select())
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}
