use reqwest::StatusCode;

pub(super) fn is_auth_error(err: &anyhow::Error) -> bool {
    err.downcast_ref::<super::HttpStatusError>()
        .map(|e| e.0 == StatusCode::UNAUTHORIZED || e.0 == StatusCode::FORBIDDEN)
        .unwrap_or(false)
}
