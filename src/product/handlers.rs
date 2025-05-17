use super::models::{NewProduct, Product};
use crate::utils::internal_error;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_shop::schema;
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use schema::products;

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

pub async fn get_products(
    State(pool): State<Pool>,
) -> Result<Json<Vec<Product>>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(|conn| products::table.select(Product::as_select()).load(conn))
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
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
    Json(payload): Json<NewProduct>,
) -> Result<Json<Product>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            diesel::update(products::table.find(id))
                .set(products::title.eq(payload.title))
                .returning(Product::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}
