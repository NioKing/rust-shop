use crate::utils::internal_error;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use deadpool_diesel::postgres::Pool;
use diesel::{insert_into, prelude::*};
use uuid::Uuid;

pub async fn create_cart(
    State(pool): State<Pool>,
    user_id: Uuid,
) -> Result<bool, (StatusCode, String)> {
    // use axum_shop::schema::carts;
    //
    // let conn = pool.get().await.map_err(internal_error)?;
    //
    // let res = conn
    //     .interact(move |conn| {
    //         insert_into(carts::table)
    //             .values(carts::user_id.eq(&user_id))
    //             .execute(conn)
    //     })
    //     .await
    //     .map_err(internal_error)?
    //     .map_err(internal_error)?;
    //
    // Ok(res > 0)
    todo!()
}
