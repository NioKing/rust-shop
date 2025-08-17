#![allow(dead_code)]

use std::usize;

use super::models::{
    CreateProductWithCategories, NewProduct, Pagination, Product, ProductCategory,
    ProductWithCategories, UpdateProduct,
};
use crate::category::models::Category;
use crate::utils::internal_error;
use crate::utils::types::Pool;
use axum::{
    extract::{Json, Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::{
    dsl::sql,
    prelude::*,
    sql_types::{Array, Json as JsonSql, Text},
};
use diesel_async::RunQueryDsl;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use validator::ValidateRequired;

pub async fn create_product(
    State(pool): State<Pool>,
    Json(payload): Json<NewProduct>,
) -> Result<Json<Product>, (StatusCode, String)> {
    use axum_shop::schema::products;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = diesel::insert_into(products::table)
        .values(&payload)
        .returning(Product::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn create_product_with_categories(
    State(pool): State<Pool>,
    Json(payload): Json<CreateProductWithCategories>,
) -> Result<Json<Product>, (StatusCode, String)> {
    use axum_shop::schema::{product_categories, products};

    let mut conn = pool.get().await.map_err(internal_error)?;

    let product = diesel::insert_into(products::table)
        .values(&payload.product)
        .returning(Product::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

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
        .execute(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(product))
}

pub async fn get_products(
    State(pool): State<Pool>,
    pagination: Query<Pagination>,
) -> Result<Json<Vec<ProductWithCategories>>, (StatusCode, String)> {
    use axum_shop::schema::{categories, product_categories, products};

    let mut conn = pool.get().await.map_err(internal_error)?;

    if pagination.limit.is_some() || pagination.offset.is_some() {
        println!("here");
    }

    let rows = products::table
        .left_join(product_categories::table.on(products::id.eq(product_categories::product_id)))
        .left_join(categories::table.on(product_categories::category_id.eq(categories::id)))
        .select((
            Product::as_select(),
            sql::<diesel::sql_types::Json>(
                "COALESCE(json_agg(categories.*) FILTER (WHERE categories.id IS NOT NULL), '[]')",
            ),
        ))
        .group_by(products::id)
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    let res = rows
        .into_iter()
        .map(|(product, cats_json)| {
            let categories = serde_json::from_value(cats_json).unwrap_or_default();
            ProductWithCategories {
                product,
                categories,
            }
        })
        .collect();

    Ok(Json(res))
}

pub async fn get_product_by_id(
    State(pool): State<Pool>,
    Path(id): Path<i32>,
) -> Result<Json<Product>, (StatusCode, String)> {
    use axum_shop::schema::products;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = products::table
        .find(id)
        .select(Product::as_select())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn delete_product(
    Path(id): Path<i32>,
    State(pool): State<Pool>,
) -> Result<Json<Product>, (StatusCode, String)> {
    use axum_shop::schema::products;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = diesel::delete(products::table.find(id))
        .returning(Product::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn update_product(
    State(pool): State<Pool>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateProduct>,
) -> Result<Json<Product>, (StatusCode, String)> {
    use axum_shop::schema::products;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = diesel::update(products::table.find(id))
        .set(&payload)
        .returning(Product::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn upload_image(
    State(pool): State<Pool>,
    Path(id): Path<i32>,
    mut multipart: Multipart,
) -> Result<(), (StatusCode, String)> {
    use axum_shop::schema::products;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let product = products::table
        .find(id)
        .select(Product::as_select())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    drop(product);

    let mut filename: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(internal_error)? {
        if field.name().is_none() || field.file_name().is_none() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to upload file without a name".to_owned(),
            ));
        }

        let content_type = field
            .content_type()
            .unwrap_or(mime::APPLICATION_OCTET_STREAM.as_ref());

        if content_type != mime::IMAGE_JPEG && content_type != mime::IMAGE_PNG {
            return Err((
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Only JPEG and PNG images are allowed".to_owned(),
            ));
        };

        let extension = if content_type == mime::IMAGE_JPEG {
            "jpg"
        } else {
            "png"
        };

        let date = chrono::Local::now().format("%Y-%m-%d_%H:%M:%S");

        let saved_file = format!("uploads/{}_{}.{}", Uuid::new_v4(), date, extension);

        filename = Some(format!("{}_{}.{}", Uuid::new_v4(), date, extension));

        let mut file = tokio::fs::File::create(&saved_file).await.map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to create a file".to_owned(),
            )
        })?;

        let mut field = field;

        while let Some(chunk) = field.chunk().await.map_err(internal_error)? {
            file.write_all(&chunk).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to write a file: {}", e),
                )
            })?;
        }
    }

    if let Some(image) = filename {
        diesel::update(products::table.find(id))
            .set(products::image.eq(image))
            .returning(Product::as_returning())
            .get_result(&mut conn)
            .await
            .map_err(internal_error)?;
    };
    Ok(())
}
