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
use diesel_async::{AsyncConnection, RunQueryDsl};
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
                "COALESCE(
                json_agg(
                json_build_object(
                    'id', products.id,
                    'title', products.title,
                    'price', products.price,
                    'description', products.description,
                    'image', products.image,
                    'quantity', cart_products.quantity
                )
            ) FILTER (WHERE products.id IS NOT NULL),
            '[]'
        )",
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

    let res = conn
        .transaction::<Cart, diesel::result::Error, _>(move |mut conn| {
            Box::pin(async move {
                let cart = carts::table
                    .filter(carts::user_id.eq(&user_id))
                    .select(Cart::as_select())
                    .get_result(&mut conn)
                    .await?;

                let products = payload
                    .items
                    .iter()
                    .map(|item| ProductCarts {
                        cart_id: cart.id,
                        product_id: item.product_id,
                        quantity: item.quantity,
                    })
                    .collect::<Vec<_>>();

                diesel::insert_into(cart_products::table)
                    .values(&products)
                    .execute(&mut conn)
                    .await?;

                let updated_at = chrono::Local::now().date_naive();

                let updated_cart = diesel::update(carts::table.find(&cart.id))
                    .set(carts::updated_at.eq(&updated_at))
                    .returning(Cart::as_returning())
                    .get_result(&mut conn)
                    .await?;

                Ok(updated_cart)
            })
        })
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn remove_product_from_cart(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
    Json(payload): Json<ProductsToCart>,
) -> Result<Json<Cart>, (StatusCode, String)> {
    use axum_shop::schema::{cart_products, carts, products, users};

    let mut conn = pool.get().await.map_err(internal_error)?;

    if payload.items.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Product ids cannot be empty!".to_owned(),
        ));
    }

    let user_id = Uuid::parse_str(&claims.sub).unwrap();

    let res = conn
        .transaction::<Cart, diesel::result::Error, _>(move |mut conn| {
            Box::pin(async move {
                let cart = carts::table
                    .filter(carts::user_id.eq(&user_id))
                    .select(Cart::as_select())
                    .get_result(&mut conn)
                    .await?;

                let ids: Vec<i32> = payload.items.iter().map(|item| item.product_id).collect();

                let prods = cart_products::table
                    .filter(cart_products::cart_id.eq(&cart.id))
                    .filter(cart_products::product_id.eq_any(&ids))
                    .select(ProductCarts::as_select())
                    .load(&mut conn)
                    .await?;

                let mut prods_qty: std::collections::HashMap<i32, i32> =
                    std::collections::HashMap::new();

                for prod in prods.iter() {
                    prods_qty.insert(prod.product_id, prod.quantity);
                }

                for item in payload.items.iter() {
                    if !prods_qty.contains_key(&item.product_id) {
                        return Err(diesel::result::Error::RollbackTransaction);
                    }

                    let cur_qty = prods_qty.get(&item.product_id).unwrap();

                    if &item.quantity > cur_qty {
                        return Err(diesel::result::Error::RollbackTransaction);
                    } else if &item.quantity == cur_qty {
                        diesel::delete(
                            cart_products::table.filter(
                                cart_products::cart_id
                                    .eq(&cart.id)
                                    .and(cart_products::product_id.eq(&item.product_id)),
                            ),
                        )
                        .execute(&mut conn)
                        .await?;
                    } else if &item.quantity < cur_qty {
                        diesel::update(
                            cart_products::table.filter(
                                cart_products::cart_id
                                    .eq(&cart.id)
                                    .and(cart_products::product_id.eq(&item.product_id)),
                            ),
                        )
                        .set(cart_products::quantity.eq(cur_qty - item.quantity))
                        .returning(ProductCarts::as_returning())
                        .get_result(&mut conn)
                        .await?;
                    }
                }

                let updated_at = chrono::Local::now().date_naive();

                let updated_cart = diesel::update(carts::table.find(&cart.id))
                    .set(carts::updated_at.eq(&updated_at))
                    .returning(Cart::as_returning())
                    .get_result(&mut conn)
                    .await?;

                Ok(updated_cart)
            })
        })
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}
