use super::models::{
    Discount, DiscountProduct, DiscountType, DiscountWithProducts, DiscountWithProductsResponse,
    NewDiscount, ProductsForDiscount, UpdateDiscount,
};
use crate::utils::types::Pool;
use crate::{discount, utils::internal_error};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use diesel::{dsl::sql, prelude::*};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

const QUEUE_NAME: &str = "notifications";

pub async fn get_all_discounts(
    State(pool): State<Pool>,
) -> Result<Json<DiscountWithProductsResponse>, (StatusCode, String)> {
    use axum_shop::schema::{discount_products, discounts, products};

    let mut conn = pool.get().await.map_err(internal_error)?;

    let rows = discounts::table
        .left_join(discount_products::table.on(discounts::id.eq(discount_products::discount_id)))
        .left_join(products::table.on(discount_products::product_id.eq(products::id)))
        .select((
            Discount::as_select(),
            sql::<diesel::sql_types::Json>(
                "COALESCE(json_agg(products.* ORDER BY products.id), '[]')",
            ),
        ))
        .group_by(discounts::id)
        .load::<(Discount, serde_json::Value)>(&mut conn)
        .await
        .map_err(internal_error)?;

    let discounts_with_products: Vec<DiscountWithProducts> = rows
        .into_iter()
        .map(|(discount, prod_json)| {
            let products = serde_json::from_value(prod_json).unwrap_or_default();

            DiscountWithProducts { discount, products }
        })
        .collect();

    let res = DiscountWithProductsResponse {
        discounts: discounts_with_products,
    };

    Ok(Json(res))
}

pub async fn create_discount(
    State(pool): State<Pool>,
    Json(mut payload): Json<NewDiscount>,
) -> Result<Json<Discount>, (StatusCode, String)> {
    use axum_shop::schema::discounts;

    let mut conn = pool.get().await.map_err(internal_error)?;

    if let Err(e) = payload.validate_dates() {
        return Err((StatusCode::BAD_REQUEST, format!("{}", e)));
    }

    let discount_type = payload.discount_type.to_lowercase();

    if !matches!(discount_type.as_str(), "fixed" | "percentage") {
        return Err((StatusCode::BAD_REQUEST, "Wrong discount_type".to_owned()));
    }

    payload.discount_type = payload.discount_type.to_lowercase();

    let res = diesel::insert_into(discounts::table)
        .values(&payload)
        .returning(Discount::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    let event = serde_json::json!({
        "event": "discount_created",
        "id": res.id,
        "title": res.title,
        "amount": res.amount,
        "start_date": res.start_date,
        "end_date": res.end_date,
        "discount_type": res.discount_type
    });

    if let Err(er) = crate::rmq::client::publish_event(QUEUE_NAME, &event.to_string()).await {
        eprintln!("Failed to publish event: {:?}", er);
    }

    Ok(Json(res))
}

pub async fn add_discount_products(
    State(pool): State<Pool>,
    Path(id): Path<i32>,
    Json(payload): Json<ProductsForDiscount>,
) -> Result<Json<DiscountWithProducts>, (StatusCode, String)> {
    use axum_shop::schema::{discount_products, discounts, products};

    let mut conn = pool.get().await.map_err(internal_error)?;

    let prods: Vec<_> = payload
        .product_id
        .iter()
        .map(|prod_id| DiscountProduct {
            discount_id: id,
            product_id: *prod_id,
        })
        .collect();

    let res = conn
        .transaction::<DiscountWithProducts, diesel::result::Error, _>(move |mut conn| {
            Box::pin(async move {
                diesel::insert_into(discount_products::table)
                    .values(&prods)
                    .execute(&mut conn)
                    .await?;

                let (discount, products_json) = discounts::table
                    .find(&id)
                    .left_join(
                        discount_products::table
                            .on(discounts::id.eq(discount_products::discount_id)),
                    )
                    .left_join(products::table.on(discount_products::product_id.eq(products::id)))
                    .select((
                        Discount::as_select(),
                        sql::<diesel::sql_types::Json>(
                            "COALESCE(json_agg(products.* ORDER BY products.id), '[]')",
                        ),
                    ))
                    .group_by(discounts::id)
                    .get_result::<(Discount, serde_json::Value)>(&mut conn)
                    .await?;

                let res = DiscountWithProducts {
                    discount,
                    products: serde_json::from_value(products_json).unwrap_or_default(),
                };

                Ok(res)
            })
        })
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn remove_products_from_discount(
    State(pool): State<Pool>,
    Path(id): Path<i32>,
    Json(payload): Json<ProductsForDiscount>,
) -> Result<Json<DiscountWithProducts>, (StatusCode, String)> {
    use axum_shop::schema::{discount_products, discounts, products};

    let mut conn = pool.get().await.map_err(internal_error)?;

    if payload.product_id.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            "Products list cannot be empty".to_owned(),
        ));
    }

    let ids: Vec<&i32> = payload.product_id.iter().collect();

    let deleted_count = diesel::delete(
        discount_products::table
            .filter(discount_products::discount_id.eq(&id))
            .filter(discount_products::product_id.eq_any(&ids)),
    )
    .execute(&mut conn)
    .await
    .map_err(internal_error)?;

    if &deleted_count < &ids.len() {
        return Err((
            StatusCode::NOT_FOUND,
            "Failed to remove products from discount".to_owned(),
        ));
    }

    let res = get_discount_with_products(&id, &mut conn).await?;

    Ok(Json(res))
}

pub async fn update_discount(
    State(pool): State<Pool>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateDiscount>,
) -> Result<Json<DiscountWithProducts>, (StatusCode, String)> {
    use axum_shop::schema::discounts;

    let mut conn = pool.get().await.map_err(internal_error)?;

    diesel::update(discounts::table.find(&id))
        .set(&payload)
        .returning(Discount::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    let discount = get_discount_with_products(&id, &mut conn).await?;

    Ok(Json(discount))
}

async fn get_discount_with_products(
    discount_id: &i32,
    conn: &mut bb8::PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
) -> std::result::Result<DiscountWithProducts, (StatusCode, String)> {
    use axum_shop::schema::{discount_products, discounts, products};

    let (discount, products_json) = discounts::table
        .find(discount_id)
        .left_join(discount_products::table.on(discounts::id.eq(discount_products::discount_id)))
        .left_join(products::table.on(discount_products::product_id.eq(products::id)))
        .select((
            Discount::as_select(),
            sql::<diesel::sql_types::Json>(
                "COALESCE(json_agg(products.* ORDER BY products.id), '[]')",
            ),
        ))
        .group_by(discounts::id)
        .get_result::<(Discount, serde_json::Value)>(conn)
        .await
        .map_err(internal_error)?;

    let res = DiscountWithProducts {
        discount,
        products: serde_json::from_value(products_json).unwrap_or_default(),
    };

    Ok(res)
}
