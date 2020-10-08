mod errors;
mod parsing;
mod sql;

use crate::data_entries::{repo::DataEntriesRepoImpl, DataEntriesRepo};
use crate::log::APP_LOG;
use errors::*;
use parsing::SearchRequest;
use serde::{Serialize, Serializer};
use slog::{error, info};
use std::convert::Infallible;
use std::sync::Arc;
use warp::http::StatusCode;
use warp::{
    reply::{json, Reply, Response},
    Filter, Rejection,
};

const NOT_FOUND_ERROR_MESSAGE: &str = "Not Found";
const METHOD_NOT_ALLOWED_ERROR_MESSAGE: &str = "Method Not Allowed";
const INTERNAL_SERVER_ERROR_MESSAGE: &str = "Internal Server Error";
const BODY_DESERIALIZATION_ERROR_MESSAGE: &str = "Body Deserialization Error";

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

pub async fn start(port: u16, repo: DataEntriesRepoImpl) {
    let data_entries_repo = Arc::new(repo);
    let with_data_entries_repo = warp::any().map(move || data_entries_repo.clone());

    let filtering = warp::post()
        .and(
            warp::body::json().and_then(|req: serde_json::Value| async move {
                let req_string = req.to_string();
                let jd = &mut serde_json::Deserializer::from_str(&req_string);
                serde_path_to_error::deserialize(jd)
                    .map_err(|err| warp::reject::custom(AppError::from(err)))
                    .and_then(|req: SearchRequest| match req.is_valid() {
                        Ok(_) => Ok(req),
                        Err(err) => Err(warp::reject::custom(err)),
                    })
            }),
        )
        .and(with_data_entries_repo.clone())
        .and_then(
            |req: SearchRequest, repo: Arc<DataEntriesRepoImpl>| async move {
                repo.search_data_entries(
                    req.filter.clone(),
                    req.sort.clone(),
                    req.limit,
                    req.offset,
                )
                .and_then::<DataEntriesResponse, _>(|data_entries| {
                    Ok(DataEntriesResponse {
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
                    })
                })
                .or_else::<Rejection, _>(|err| {
                    Err(
                        warp::reject::custom::<AppError>(AppError::DbError(err.to_string()).into())
                            .into(),
                    )
                })
            },
        );

    let log = warp::log::custom(access_log);

    let search_filtering = warp::path::path("search")
        .and(warp::path::end())
        .and(filtering.clone());
    let root_filtering = warp::path::end().and(filtering.clone());

    info!(APP_LOG, "Starting web server at 0.0.0.0:{}", port);
    warp::serve(
        search_filtering
            .or(root_filtering)
            .with(log)
            .recover(handle_rejection),
    )
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
        error = ErrorListResponse::singleton(
            NOT_FOUND_ERROR_MESSAGE.to_string(),
            StatusCode::NOT_FOUND,
        );
    } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
        error = ErrorListResponse::singleton(
            BODY_DESERIALIZATION_ERROR_MESSAGE.to_string(),
            StatusCode::BAD_REQUEST,
        );
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        error = ErrorListResponse::singleton(
            METHOD_NOT_ALLOWED_ERROR_MESSAGE.to_string(),
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
            error_details.to_owned(),
            Some(error_code.to_owned()),
        );
    } else if let Some(AppError::DbError(_)) = err.find() {
        error!(APP_LOG, "DbError: {:?}", err);
        error = ErrorListResponse::singleton(
            INTERNAL_SERVER_ERROR_MESSAGE.to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    } else {
        // We should have expected this... Just log and say its a 500
        error!(APP_LOG, "Unhandled rejection: {:?}", err);
        error = ErrorListResponse::singleton(
            INTERNAL_SERVER_ERROR_MESSAGE.to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    }

    Ok(error.into_response())
}
