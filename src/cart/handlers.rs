use super::models::{Cart, CartWithProducts, ProductCarts, ProductsToCart};
use crate::auth::models::User;
use crate::error::{AppError, AppErrorKind};
use crate::utils::types::Pool;
use crate::{
    auth::models::AccessTokenClaims,
    utils::{internal_error, parse_user_id},
};
use anyhow::Context;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use diesel::row;
use diesel::{dsl::sql, prelude::*};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

pub async fn get_all_cart(
    State(pool): State<Pool>,
) -> Result<Json<Vec<CartWithProducts>>, AppError> {
    use axum_shop::schema::{cart_products, carts, products};

    let mut conn = pool.get().await.context(AppError::pool_context())?;

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
        .context("Cart query failed")?;

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
) -> Result<Json<CartWithProducts>, AppError> {
    use axum_shop::schema::{cart_products, carts, products, users};

    let mut conn = pool.get().await.context(AppError::pool_context())?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = conn
        .transaction::<CartWithProducts, diesel::result::Error, _>(move |mut conn| {
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
                    let product = ProductCarts {
                        cart_id: cart.id,
                        product_id: item.product_id,
                        quantity: item.quantity,
                    };

                    if !prods_qty.contains_key(&item.product_id) {
                        diesel::insert_into(cart_products::table)
                            .values(&product)
                            .execute(&mut conn)
                            .await?;
                    } else {
                        let cur_qty = prods_qty.get(&item.product_id).unwrap();

                        diesel::update(
                            cart_products::table.filter(
                                cart_products::cart_id
                                    .eq(&cart.id)
                                    .and(cart_products::product_id.eq(&item.product_id)),
                            ),
                        )
                        .set(cart_products::quantity.eq(cur_qty + item.quantity))
                        .returning(ProductCarts::as_returning())
                        .get_result(&mut conn)
                        .await?;
                    }
                }

                let updated_at = chrono::Local::now().date_naive();

                diesel::update(carts::table.find(&cart.id))
                    .set(carts::updated_at.eq(&updated_at))
                    .returning(Cart::as_returning())
                    .get_result(&mut conn)
                    .await?;

                let updated_cart = get_cart_with_products(&cart.id, &mut conn).await?;

                Ok(updated_cart)
            })
        })
        .await
        .context("Failed to products to cart")?;

    Ok(Json(res))
}

pub async fn remove_product_from_cart(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
    Json(payload): Json<ProductsToCart>,
) -> Result<Json<CartWithProducts>, AppError> {
    use axum_shop::schema::{cart_products, carts, products, users};

    let mut conn = pool.get().await.context(AppError::pool_context())?;

    if payload.items.is_empty() {
        return Err(AppError::validation("Product ids cannot be empty"));
    }

    let user_id = parse_user_id(&claims.sub)?;

    let res = conn
        .transaction::<CartWithProducts, diesel::result::Error, _>(move |mut conn| {
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

                diesel::update(carts::table.find(&cart.id))
                    .set(carts::updated_at.eq(&updated_at))
                    .returning(Cart::as_returning())
                    .get_result(&mut conn)
                    .await?;

                let updated_cart = get_cart_with_products(&cart.id, &mut conn).await?;

                Ok(updated_cart)
            })
        })
        .await
        .context("Failed to remove products from cart")?;

    Ok(Json(res))
}

async fn get_cart_with_products(
    cart_id: &i32,
    conn: &mut bb8::PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
) -> Result<CartWithProducts, diesel::result::Error> {
    use axum_shop::schema::{cart_products, carts, products, users};

    let (cart, products) = carts::table
        .find(cart_id)
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
                        ORDER BY products.id
                    ) FILTER (WHERE products.id IS NOT NULL),
                        '[]'
                        )",
            ),
        ))
        .group_by(carts::id)
        .get_result::<(Cart, serde_json::Value)>(conn)
        .await?;

    let updated_cart = CartWithProducts {
        cart,
        products: serde_json::from_value(products).unwrap_or_default(),
    };

    Ok(updated_cart)
}
