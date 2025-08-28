use super::models::{Cart, CartWithProducts};
use crate::utils::internal_error;
use crate::utils::types::Pool;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use diesel::{dsl::sql, prelude::*};
use diesel_async::RunQueryDsl;

pub async fn get_all_cart(
    State(pool): State<Pool>,
) -> Result<Json<Vec<CartWithProducts>>, (StatusCode, String)> {
    use axum_shop::schema::{cart_products, carts, products};

    let mut conn = pool.get().await.map_err(internal_error)?;

    let rows = carts::table
        .left_join(cart_products::table.on(carts::id.eq(cart_products::cart_id)))
        .left_join(products::table.on(cart_products::product_id.eq(products::id)))
        .select((
            Cart::as_select(),
            sql::<diesel::sql_types::Json>(
                "COALESCE(json_agg(products.*) FILTER (WHERE products.id IS NOT NULL), '[]')",
            ),
        ))
        .group_by(carts::id)
        .load::<(Cart, serde_json::Value)>(&mut conn)
        .await
        .map_err(internal_error)?;

    let res = rows
        .into_iter()
        .map(|(cart, products_json)| {
            let products = serde_json::from_value(products_json).unwrap_or_default();
            CartWithProducts { cart, products }
        })
        .collect();

    Ok(Json(res))
}
