use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::fmt;
use warp::http::StatusCode;
use warp::reject::Reject;
use warp::reply::{json, with_status, Reply, Response};

const VALIDATION_ERROR_TITLE: &str = "Validation Error";
const MISSING_FIELD_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"missing field `(\w+)`").unwrap());
const INVALID_VALUE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"invalid value: (\w+) `(.*)`, expected (\w+)").unwrap());

#[derive(Clone, Debug, Serialize, thiserror::Error)]
pub enum AppError {
    DbError(String),
    ValidationError(String, u32, Option<ErrorDetails>),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DbError(msg) => write!(f, "DbError: {}", msg),
            AppError::ValidationError(msg, code, details) => write!(
                f,
                "ValidationError: message={} code={} details={:?}",
                msg, code, details
            ),
        }
    }
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

pub enum ValidationErrorCode {
    MissingRequiredParameter = 950200,
    InvalidParamenterValue = 950201,
    UnknownError = 950299,
}

impl AppError {
    pub fn new_validation_error(code: ValidationErrorCode, details: ErrorDetails) -> AppError {
        AppError::ValidationError(
            VALIDATION_ERROR_TITLE.to_owned(),
            code as u32,
            Some(details),
        )
    }
}

impl From<serde_path_to_error::Error<serde_json::Error>> for AppError {
    fn from(e: serde_path_to_error::Error<serde_json::Error>) -> Self {
        let path = e.path().to_string();
        let err_message = e.into_inner().to_string();
        if err_message.starts_with("missing field") {
            AppError::new_validation_error(
                ValidationErrorCode::MissingRequiredParameter,
                ErrorDetails {
                    parameter: path,
                    reason: format!(
                        "Missing field `{}`.",
                        MISSING_FIELD_RE
                            .captures(&err_message)
                            .unwrap()
                            .get(1)
                            .map_or("", |v| v.as_str())
                    ),
                },
            )
        } else if err_message.starts_with("invalid value") {
            println!("{}", err_message);
            let caps = INVALID_VALUE_RE.captures(&err_message).unwrap();
            AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: path,
                    reason: format!(
                        "Invalid value: found `{}` of type {}, expected type {}.",
                        caps.get(2).map_or("", |v| v.as_str()),
                        caps.get(1).map_or("", |v| v.as_str()),
                        caps.get(3).map_or("", |v| v.as_str())
                    ),
                },
            )
        } else {
            AppError::new_validation_error(
                ValidationErrorCode::UnknownError,
                ErrorDetails {
                    parameter: path,
                    reason: err_message,
                },
            )
        }
    }
}
