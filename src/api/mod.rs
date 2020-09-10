mod errors;
pub mod parsing;

use crate::data_entries::{repo::DataEntriesRepoImpl, DataEntriesRepo};
use crate::log::APP_LOG;
use errors::*;
use parsing::{parse_filter, parse_sort, Node, Sort};
use serde::Serialize;
use serde::Serializer;
use slog::{error, info};
use std::convert::Infallible;
use std::sync::Arc;
use warp::http::StatusCode;
use warp::{
    reply::{json, Reply, Response},
    Filter, Rejection,
};

const DEFAULT_LIMIT: u64 = 100;

#[derive(Clone, Debug)]
enum DataEntryType {
    BinaryVal(Vec<u8>),
    BoolVal(bool),
    IntVal(i64),
    StringVal(String),
}

impl Serialize for DataEntryType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DataEntryType::BinaryVal(v) => serializer.serialize_bytes(v),
            DataEntryType::BoolVal(v) => serializer.serialize_bool(v.to_owned()),
            DataEntryType::IntVal(v) => serializer.serialize_i64(v.to_owned()),
            DataEntryType::StringVal(v) => serializer.serialize_str(v),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DataEntry {
    address: String,
    key: String,
    height: i32,
    value: DataEntryType,
}

#[derive(Serialize, Debug, Clone)]
pub struct DataEntriesResponse {
    entries: Vec<DataEntry>,
    has_next_page: bool,
}

impl Reply for DataEntriesResponse {
    fn into_response(self) -> Response {
        json(&self).into_response()
    }
}

#[derive(Debug)]
struct SearchRequest {
    filter: Option<Node>,
    sort: Option<Sort>,
    limit: u64,
    offset: u64,
}

pub async fn start(port: u16, repo: DataEntriesRepoImpl) {
    info!(APP_LOG, "Starting server"; "port" => &port);

    let data_entries_repo = Arc::new(repo);
    let with_data_entries_repo = warp::any().map(move || data_entries_repo.clone());

    let filtering = warp::path!("filtering")
        .and(warp::post())
        .and(
            warp::body::json().and_then(|req: serde_json::Value| async move {
                if let serde_json::Value::Object(o) = req {
                    let filter = o
                        .get("filter")
                        .map(|req_filter| match parse_filter(&req_filter) {
                            Ok(query_filter) => Ok(query_filter),
                            Err(err) => Err(warp::reject::custom(err)),
                        })
                        .transpose()?;

                    let sort = o
                        .get("sort")
                        .map(|req_sort| match parse_sort(&req_sort) {
                            Ok(query_sort) => Ok(query_sort),
                            Err(err) => Err(warp::reject::custom(err)),
                        })
                        .transpose()?;

                    let limit: u64 = o.get("limit").map_or(Ok(DEFAULT_LIMIT), |l| {
                        l.as_u64()
                            .ok_or(warp::reject::custom(AppError::ValidationError(
                                "Validation Error".to_string(),
                                950201,
                                ErrorDetails {
                                    parameter: "limit".to_string(),
                                    reason: "Invalid value type, should be an integer.".to_string(),
                                },
                            )))
                    })?;
                    
                    let offset: u64 = o.get("offset").map_or(Ok(0 as u64), |o| {
                        o.as_u64()
                            .ok_or(warp::reject::custom(AppError::ValidationError(
                                "Validation Error".to_string(),
                                950201,
                                ErrorDetails {
                                    parameter: "offset".to_string(),
                                    reason: "Invalid value type, should be an integer.".to_string(),
                                },
                            )))
                    })?;

                    Ok(SearchRequest {
                        filter: filter,
                        sort: sort,
                        limit: limit,
                        offset: offset,
                    })
                } else {
                    Err(warp::reject::custom(AppError::ValidationError(
                        "Validation Error".to_string(),
                        950201,
                        ErrorDetails {
                            parameter: "body".to_string(),
                            reason: "Invalid type, should be an object.".to_string(),
                        },
                    )))
                }
            }),
        )
        .and(with_data_entries_repo.clone())
        .map(|req: SearchRequest, repo: Arc<DataEntriesRepoImpl>| {
            match repo.search_data_entries(req.filter, req.sort, req.limit + 1, req.offset) {
                Ok(data_entries) => DataEntriesResponse {
                    entries: data_entries
                        .clone()
                        .into_iter()
                        .take(req.limit as usize)
                        .map(|de| {
                            let value;
                            if let Some(v) = de.value_binary {
                                value = DataEntryType::BinaryVal(v);
                            } else if let Some(v) = de.value_bool {
                                value = DataEntryType::BoolVal(v);
                            } else if let Some(v) = de.value_integer {
                                value = DataEntryType::IntVal(v);
                            } else {
                                value = DataEntryType::StringVal(de.value_string.unwrap());
                            }
                            DataEntry {
                                address: de.address.clone(),
                                key: de.key.clone(),
                                height: de.height.clone(),
                                value: value,
                            }
                        })
                        .collect(),
                    has_next_page: data_entries.len() > req.limit as usize,
                }
                .into_response(),
                Err(err) => {
                    error!(APP_LOG, "couldn't query db"; "error" => format!("{:?}", err));

                    ErrorListResponse::singleton(err.into(), StatusCode::INTERNAL_SERVER_ERROR)
                        .into_response()
                }
            }
        });

    let log = warp::log::custom(access_log);

    // todo handle errors
    warp::serve(filtering.with(log).recover(handle_rejection))
        .run(([0, 0, 0, 0], port))
        .await
}

fn access_log(info: warp::log::Info) {
    let req_id = info
        .request_headers()
        .get("x-request-id")
        .map(|h| h.to_str().unwrap_or(&""));

    info!(
        APP_LOG, "access log";
        "path" => info.path(),
        "method" => info.method().to_string(),
        "status" => info.status().as_u16(),
        "ua" => info.user_agent(),
        "latency" => info.elapsed().as_millis(),
        "req_id" => req_id,
        "ip" => info.remote_addr().map(|a| format!("{}", a.ip())),
        "protocol" => format!("{:?}", info.version())
    );
}

// This function receives a `Rejection` and tries to return a custom
// value, otherwise simply passes the rejection along.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let error: ErrorListResponse;

    if err.is_not_found() {
        error = ErrorListResponse::singleton("Not Found".to_string(), StatusCode::NOT_FOUND);
    } else if let Some(_e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        error = ErrorListResponse::singleton(
            "Body Deserialization Error".to_string(),
            StatusCode::BAD_REQUEST,
        );
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        error = ErrorListResponse::singleton(
            "Method Not Allowed".to_string(),
            StatusCode::METHOD_NOT_ALLOWED,
        );
    } else if let Some(err) = err.find::<warp::reject::InvalidQuery>() {
        error =
            ErrorListResponse::singleton(format!("{}.", err.to_string()), StatusCode::BAD_REQUEST);
    } else if let Some(AppError::ValidationError(error_message, error_code, error_details)) =
        err.find()
    {
        error = ErrorListResponse::new(
            error_message.to_owned(),
            StatusCode::BAD_REQUEST,
            Some(error_details.to_owned()),
            Some(error_code.to_owned()),
        );
    } else if let Some(AppError::DbError(_)) = err.find() {
        error!(APP_LOG, "DbError: {:?}", err);
        error = ErrorListResponse::singleton(
            "Internal Server Error".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    } else {
        // We should have expected this... Just log and say its a 500
        error!(APP_LOG, "unhandled rejection: {:?}", err);
        error = ErrorListResponse::singleton(
            "Internal Server Error".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    }

    Ok(error.into_response())
}
