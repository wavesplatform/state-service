use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use warp::reject::Reject;

const VALIDATION_ERROR_TITLE: &str = "Validation Error";
const MISSING_FIELD_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"missing field `(\w+)`").unwrap());
const INVALID_VALUE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"invalid value: (\w+) `(.*)`, expected (\w+)").unwrap());

#[derive(Clone, Debug, Serialize, thiserror::Error)]
pub enum AppError {
    DbError(String),
    ValidationError(String, u32, Option<ErrorDetails>),
    DecodePathError(String),
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
            AppError::DecodePathError(msg) => write!(f, "DecodePathError: {}", msg),
        }
    }
}

impl Reject for AppError {}

#[derive(Clone, Debug, Serialize)]
pub struct ErrorDetails {
    pub parameter: String,
    pub reason: String,
}

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

impl From<serde_qs::Error> for AppError {
    fn from(e: serde_qs::Error) -> Self {
        let reason = e.to_string();
        Self::new_validation_error(
            ValidationErrorCode::InvalidParamenterValue,
            ErrorDetails {
                parameter: "query".into(),
                reason,
            },
        )
    }
}

impl From<ErrorDetails> for HashMap<String, String> {
    fn from(v: ErrorDetails) -> Self {
        let mut hm = HashMap::with_capacity(2);
        hm.insert("parameter".to_owned(), v.parameter);
        hm.insert("reason".to_owned(), v.reason);
        hm
    }
}
