use serde::Serialize;
use warp::http::StatusCode;
use warp::reject::Reject;
use warp::reply::{json, with_status, Reply, Response};

#[derive(Clone, Debug, Serialize)]
pub enum AppError {
    DbError(String),
    ValidationError(String, u32, ErrorDetails),
}

impl Reject for AppError {}

#[derive(Clone, Debug, Serialize)]
pub struct ErrorDetails {
    pub parameter: String,
    pub reason: String,
}

#[derive(Serialize, Debug, Clone)]
struct Error {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<ErrorDetails>,
}

#[derive(Serialize, Debug, Clone)]
struct ErrorList {
    errors: Vec<Error>,
}

#[derive(Debug, Clone)]
pub struct ErrorListResponse {
    list: ErrorList,
    status: StatusCode,
}

impl ErrorListResponse {
    pub fn singleton(msg: String, status: StatusCode) -> Self {
        Self::new(msg, status, None, None)
    }

    pub fn new(
        msg: String,
        status: StatusCode,
        details: Option<ErrorDetails>,
        code: Option<u32>,
    ) -> Self {
        Self {
            list: ErrorList {
                errors: vec![Error {
                    message: msg,
                    details: details,
                    code: code,
                }],
            },
            status,
        }
    }
}

impl Reply for ErrorListResponse {
    fn into_response(self) -> Response {
        with_status(json(&self.list), self.status).into_response()
    }
}

impl Reject for ErrorListResponse {}
