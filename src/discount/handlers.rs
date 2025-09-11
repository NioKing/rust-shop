use super::models::{
    Discount, DiscountProduct, DiscountType, DiscountWithProducts, DiscountWithProductsResponse,
    NewDiscount, ProductsForDiscount,
};
use crate::utils::types::Pool;
use crate::{discount, utils::internal_error};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_shop::schema::cart_products::product_id;
use diesel::{dsl::sql, prelude::*};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

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
