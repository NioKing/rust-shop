use super::models::{
    CreateProductWithCategories, NewProduct, Product, ProductCategory, ProductWithCategories,
    UpdateProduct,
};
use crate::category::models::Category;
use crate::utils::internal_error;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_shop::schema::{categories, product_categories, products};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;

pub async fn create_product(
    State(pool): State<Pool>,
    Json(payload): Json<NewProduct>,
) -> Result<Json<Product>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;
    let res = conn
        .interact(|conn| {
            diesel::insert_into(products::table)
                .values(payload)
                .returning(Product::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn create_product_with_categories(
    State(pool): State<Pool>,
    Json(payload): Json<CreateProductWithCategories>,
) -> Result<Json<Product>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            conn.transaction(|conn| {
                let product = diesel::insert_into(products::table)
                    .values(&payload.product)
                    .returning(Product::as_returning())
                    .get_result(conn)?;

                let categories = payload
                    .category_ids
                    .iter()
                    .map(|category_id| ProductCategory {
                        product_id: product.id,
                        category_id: *category_id,
                    })
                    .collect::<Vec<_>>();

                diesel::insert_into(product_categories::table)
                    .values(&categories)
                    .execute(conn)?;

                Ok(product)
            })
        })
        .await
        .map_err(|e| internal_error(e))?
        .map_err(|e: diesel::result::Error| internal_error(e))?;

    Ok(Json(res))
}

pub async fn get_products(
    State(pool): State<Pool>,
) -> Result<Json<Vec<ProductWithCategories>>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    // let res = conn
    //     .interact(|conn| products::table.select(Product::as_select()).load(conn))
    //     .await
    //     .map_err(internal_error)?
    //     .map_err(internal_error)?;

    let tuple = conn
        .interact(|conn| {
            products::table
                .inner_join(
                    product_categories::table.on(products::id.eq(product_categories::product_id)),
                )
                .inner_join(
                    categories::table.on(product_categories::category_id.eq(categories::id)),
                )
                .select((Product::as_select(), Category::as_select()))
                .load::<(Product, Category)>(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    let mut products_map = std::collections::HashMap::new();
    for (product, category) in tuple {
        let entry = products_map
            .entry(product.id)
            .or_insert_with(|| ProductWithCategories {
                product: product,
                categories: Vec::new(),
            });
        entry.categories.push(category);
    }
    let products = products_map.into_values().collect();

    Ok(Json(products))
}

pub async fn get_product_by_id(
    State(pool): State<Pool>,
    Path(id): Path<i32>,
) -> Result<Json<Product>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            products::table
                .find(id)
                .select(Product::as_select())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn remove_product(
    Path(id): Path<i32>,
    State(pool): State<Pool>,
) -> Result<Json<Product>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            diesel::delete(products::table.find(id))
                .returning(Product::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn update_product(
    State(pool): State<Pool>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateProduct>,
) -> Result<Json<Product>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            diesel::update(products::table.find(id))
                .set(&payload)
                .returning(Product::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}
