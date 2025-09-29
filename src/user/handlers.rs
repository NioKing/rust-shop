use super::models::{Address, Profile};

use crate::utils::{internal_error, types::Pool};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use diesel::{dsl::sql, prelude::*};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

pub async fn get_user_profile_by_id(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Profile>>, (StatusCode, String)> {
    use axum_shop::schema::{profiles, users};

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = profiles::table
        .filter(profiles::user_id.eq(&id))
        .select(Profile::as_select())
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}
