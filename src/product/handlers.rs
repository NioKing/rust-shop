#![allow(dead_code)]

use super::models::{
    CreateProductWithCategories, NewProduct, OrderByParams, Product, ProductCategory,
    ProductWithCategories, QueryParams, SortByParams, UpdateProduct,
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
    sql_query,
    sql_types::{Array, Text},
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
    query_params: Query<QueryParams>,
) -> Result<Json<Vec<ProductWithCategories>>, (StatusCode, String)> {
    use axum_shop::schema::{categories, product_categories, products};
    use diesel_full_text_search::*;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let mut query = products::table
        .left_join(product_categories::table.on(products::id.eq(product_categories::product_id)))
        .left_join(categories::table.on(product_categories::category_id.eq(categories::id)))
        .select((
            Product::as_select(),
            sql::<diesel::sql_types::Json>(
                "COALESCE(json_agg(categories.*) FILTER (WHERE categories.id IS NOT NULL), '[]')",
            ),
        ))
        .group_by(products::id)
        .into_boxed();

    if let Some(offset) = query_params.offset {
        query = query.offset(offset);
    };

    if let Some(limit) = query_params.limit {
        query = query.limit(limit);
    }

    if let Some(cat_id) = query_params.category_id {
        query = query.filter(product_categories::category_id.eq(cat_id));
    }

    if let Some(min_price) = query_params.min_price {
        query = query.filter(
            products::price
                .gt(min_price)
                .or(products::price.eq(min_price)),
        );
    }

    if let Some(max_price) = query_params.max_price {
        query = query.filter(
            products::price
                .lt(max_price)
                .or(products::price.eq(max_price)),
        );
    }

    if let (Some(order), Some(sort_param)) = (&query_params.sort_ord, &query_params.sort_by) {
        match (sort_param.to_owned(), order) {
            (SortByParams::Id, OrderByParams::Asc) => query = query.order(products::id.asc()),
            (SortByParams::Id, OrderByParams::Desc) => query = query.order(products::id.desc()),
            (SortByParams::Title, OrderByParams::Asc) => query = query.order(products::title.asc()),
            (SortByParams::Title, OrderByParams::Desc) => {
                query = query.order(products::title.desc())
            }
            (SortByParams::Price, OrderByParams::Asc) => query = query.order(products::price.asc()),
            (SortByParams::Price, OrderByParams::Desc) => {
                query = query.order(products::price.desc())
            }

            _ => {
                return Err((StatusCode::BAD_REQUEST, "Invalid sort param".to_owned()));
            }
        }
    };

    if let Some(title) = &query_params.search_title {
        // query = query.filter(to_tsvector(products::title).matches(to_tsquery(title)));
        query = query.filter(products::title.ilike(format!("%{}%", title)));
    };
    // let rows = products::table
    //     .left_join(product_categories::table.on(products::id.eq(product_categories::product_id)))
    //     .left_join(categories::table.on(product_categories::category_id.eq(categories::id)))
    //     .select((
    //         Product::as_select(),
    //         sql::<diesel::sql_types::Json>(
    //             "COALESCE(json_agg(categories.*) FILTER (WHERE categories.id IS NOT NULL), '[]')",
    //         ),
    //     ))
    //     .order(products::id.asc())
    //     .limit(query_params.limit.unwrap_or(i64::MAX))
    //     .offset(query_params.offset.unwrap_or(0))
    //     .group_by(products::id)
    //     .load::<(Product, serde_json::Value)>(&mut conn)
    //     .await
    //     .map_err(internal_error)?;

    let rows = query
        .load::<(Product, serde_json::Value)>(&mut conn)
        .await
        .map_err(internal_error)?;

    let res = rows
        .into_iter()
        .map(|(product, categories_json)| {
            let categories = serde_json::from_value(categories_json).unwrap_or_default();
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
) -> Result<Json<ProductWithCategories>, (StatusCode, String)> {
    use axum_shop::schema::{categories, product_categories, products};

    let mut conn = pool.get().await.map_err(internal_error)?;

    let (product, categories_json) = products::table
        .find(id)
        .left_join(product_categories::table.on(products::id.eq(product_categories::product_id)))
        .left_join(categories::table.on(product_categories::category_id.eq(categories::id)))
        .select((
            Product::as_select(),
            sql::<diesel::sql_types::Json>(
                "COALESCE(json_agg(categories.*) FILTER (WHERE categories.id IS NOT NULL), '[]')",
            ),
        ))
        .group_by(products::id)
        .get_result::<(Product, serde_json::Value)>(&mut conn)
        .await
        .map_err(internal_error)?;

    let res = ProductWithCategories {
        product: product,
        categories: serde_json::from_value(categories_json).unwrap_or_default(),
    };

    // let res = products::table
    //     .find(id)
    //     .select(Product::as_select())
    //     .get_result(&mut conn)
    //     .await
    //     .map_err(internal_error)?;

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
