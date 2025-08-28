use super::models::{Cart, CartWithProducts, ProductCarts, ProductsToCart};
use crate::auth::models::User;
use crate::utils::types::Pool;
use crate::{auth::models::AccessTokenClaims, utils::internal_error};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use diesel::{dsl::sql, prelude::*};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

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

pub async fn add_products_to_cart(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
    Json(payload): Json<ProductsToCart>,
) -> Result<Json<Cart>, (StatusCode, String)> {
    use axum_shop::schema::{cart_products, carts, products, users};

    let mut conn = pool.get().await.map_err(internal_error)?;

    let user_id = Uuid::parse_str(&claims.sub).unwrap();

    let cart = carts::table
        .filter(carts::user_id.eq(&user_id))
        .select(Cart::as_select())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    let products = payload
        .product_ids
        .iter()
        .map(|prod_id| ProductCarts {
            cart_id: cart.id,
            product_id: *prod_id,
        })
        .collect::<Vec<_>>();

    //TODO cart updated at timestamp

    diesel::insert_into(cart_products::table)
        .values(&products)
        .execute(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(cart))
}
