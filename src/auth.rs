use axum::http::StatusCode;
use crate::state::AppState;

/// Check if request is authorized
pub fn check_auth(state: &AppState, query_token: Option<&String>) -> Result<(), StatusCode> {
    // If no auth token is configured, allow all requests
    let required_token = match &state.auth_token {
        Some(token) => token,
        None => return Ok(()),
    };
    
    match query_token {
        Some(token) if token == required_token => Ok(()),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
