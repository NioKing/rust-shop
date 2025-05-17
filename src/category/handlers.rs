use super::models::{Category, NewCategory};
use crate::utils::internal_error;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_shop::schema;
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use schema::categories;

pub async fn create_category(
    State(pool): State<Pool>,
    Json(payload): Json<NewCategory>,
) -> Result<Json<Category>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;
    let res = conn
        .interact(|conn| {
            diesel::insert_into(categories::table)
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
        .interact(|conn| categories::table.select(Category::as_select()).load(conn))
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn update_category(
    State(pool): State<Pool>,
    Path(id): Path<i32>,
    Json(payload): Json<NewCategory>,
) -> Result<Json<Category>, (StatusCode, String)> {
    if payload.title.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Title cannot be empty".to_string()));
    }
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            diesel::update(categories::table.find(id))
                .set(categories::title.eq(payload.title))
                .returning(Category::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn get_category_by_id(
    State(pool): State<Pool>,
    Path(id): Path<i32>,
) -> Result<Json<Category>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            categories::table
                .find(id)
                .select(Category::as_select())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}
