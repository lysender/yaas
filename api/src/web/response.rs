use axum::Json;
use axum::response::IntoResponse;
use axum::{body::Body, http::StatusCode, response::Response};
use serde::Serialize;

pub fn json_response<T: Serialize>(status_code: StatusCode, payload: T) -> Response<Body> {
    (status_code, Json(payload)).into_response()
}

pub fn empty_response(status_code: StatusCode) -> Response<Body> {
    status_code.into_response()
}
