use reqwest::StatusCode;
use uuid::Uuid;

pub fn parse_user_id(id: &str) -> Result<Uuid, (StatusCode, String)> {
    let user_id = Uuid::parse_str(id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Failed to parse user id".to_owned(),
        )
    })?;

    Ok(user_id)
}
