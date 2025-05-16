use super::models::{Category, NewCategory};
use crate::utils::error::internal_error;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use axum_shop::schema;
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;

pub async fn create_category(
    State(pool): State<Pool>,
    Json(payload): Json<NewCategory>,
) -> Result<Json<Category>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;
    let res = conn
        .interact(|conn| {
            diesel::insert_into(schema::categories::table)
                .values(payload)
                .returning(Category::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn get_categories(
    State(pool): State<Pool>,
) -> Result<Json<Vec<Category>>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(|conn| {
            schema::categories::table
                .select(Category::as_select())
                .load(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}
