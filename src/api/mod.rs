mod errors;
mod parsing;
mod sql;

use crate::data_entries::{self, repo::DataEntriesRepoImpl, DataEntriesRepo};
use crate::log::APP_LOG;
use errors::*;
use parsing::{Entry, MgetByAddress, MgetEntries, SearchRequest};
use serde::{Serialize, Serializer};
use slog::{error, info};
use std::collections::HashMap;
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
    key_fragments: Vec<DataEntryFragment>,
    value_fragments: Vec<DataEntryValueFragment>,
}

impl Reply for DataEntry {
    fn into_response(self) -> Response {
        json(&self).into_response()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DataEntryFragment {
    String { value: String },
    Integer { value: i64 },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DataEntryValueFragment {
    String { value: String },
    Integer { value: i64 },
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

    let search = warp::path::path("search")
        .and(warp::path::end())
        .and(warp::post())
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
                    req.limit + 1,
                    req.offset,
                )
                .and_then::<DataEntriesResponse, _>(|data_entries| {
                    let has_next_page = data_entries.len() > req.limit as usize;
                    Ok(DataEntriesResponse {
                        entries: data_entries
                            .into_iter()
                            .take(req.limit as usize)
                            .map(|de| de.into())
                            .collect(),
                        has_next_page,
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

    let mget_entries = warp::path::path("entries")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json::<MgetEntries>())
        .and(with_data_entries_repo.clone())
        .and_then(
            |req: MgetEntries, repo: Arc<DataEntriesRepoImpl>| async move {
                let address_key_pairs = req.address_key_pairs.clone();
                repo.mget_data_entries(req)
                    .and_then(|data_entries| {
                        let mut data_entries_map = data_entries
                            .into_iter()
                            .map(|de| {
                                let key = (de.address.clone(), de.key.clone());
                                let de = de.into();
                                (key, de)
                            })
                            .collect::<HashMap<_, _>>();
                        let result = address_key_pairs
                            .into_iter()
                            .map(|entry| {
                                let k = &(entry.address, entry.key);
                                data_entries_map.remove(k)
                            })
                            .collect::<Vec<Option<DataEntry>>>();
                        Ok(MgetResponse(result))
                    })
                    .or_else::<Rejection, _>(|err| {
                        Err(warp::reject::custom::<AppError>(
                            AppError::DbError(err.to_string()).into(),
                        )
                        .into())
                    })
            },
        );

    let mget_by_address = warp::path!("entries" / String)
        .and(warp::path::end())
        .and(warp::get())
        .and(
            warp::filters::query::raw().and_then(|query: String| async move {
                serde_qs::from_str::<MgetByAddress>(query.as_str())
                    .map_err(|err| warp::reject::custom::<AppError>(AppError::from(err)))
            }),
        )
        .and(with_data_entries_repo.clone())
        .and_then(
            |address: String, query: MgetByAddress, repo: Arc<DataEntriesRepoImpl>| async move {
                let keys = query.keys.clone();
                let mget_entries = MgetEntries::from_query_by_address(address, query.keys);
                repo.mget_data_entries(mget_entries)
                    .and_then(|data_entries| {
                        let mut data_entries_map = data_entries
                            .into_iter()
                            .map(|de| {
                                let key = de.key.clone();
                                let de = de.into();
                                (key, de)
                            })
                            .collect::<HashMap<_, _>>();
                        let result = keys
                            .into_iter()
                            .map(|key| data_entries_map.remove(&key))
                            .collect::<Vec<Option<DataEntry>>>();
                        Ok(MgetResponse(result))
                    })
                    .or_else::<Rejection, _>(|err| {
                        Err(warp::reject::custom::<AppError>(
                            AppError::DbError(err.to_string()).into(),
                        )
                        .into())
                    })
            },
        );

    let get_by_address_key = warp::path!("entries" / String / String)
        .and(warp::path::end())
        .and(warp::get())
        .and(with_data_entries_repo.clone())
        .and_then(
            |address: String, key: String, repo: Arc<DataEntriesRepoImpl>| async move {
                let key = decode_uri_string(key)?;
                let entry = Entry {
                    address: address.clone(),
                    key: key.clone(),
                };
                let mget_entries = MgetEntries {
                    address_key_pairs: vec![entry],
                };
                repo.mget_data_entries(mget_entries)
                    .or_else::<Rejection, _>(|err| {
                        Err(warp::reject::custom::<AppError>(
                            AppError::DbError(err.to_string()).into(),
                        )
                        .into())
                    })
                    .and_then(|data_entries| {
                        if let Some(de) = data_entries.first() {
                            Ok(DataEntry::from(de.clone()))
                        } else {
                            Err(warp::reject::not_found())
                        }
                    })
            },
        );

    let log = warp::log::custom(access_log);

    info!(APP_LOG, "Starting web server at 0.0.0.0:{}", port);
    warp::serve(
        search
            .or(mget_entries)
            .or(mget_by_address)
            .or(get_by_address_key)
            .with(log)
            .recover(handle_rejection),
    )
    .run(([0, 0, 0, 0], port))
    .await
}

fn decode_uri_string(s: String) -> Result<String, Rejection> {
    percent_encoding::percent_decode(s.as_bytes())
        .decode_utf8()
        .map(|s| s.to_string())
        .map_err(|error| {
            warp::reject::custom::<AppError>(AppError::DecodePathError(error.to_string()))
        })
}

#[derive(Debug, serde::Serialize)]
struct MgetResponse(Vec<Option<DataEntry>>);

impl Reply for MgetResponse {
    fn into_response(self) -> Response {
        json(&self.0).into_response()
    }
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

impl From<data_entries::DataEntry> for DataEntry {
    fn from(v: data_entries::DataEntry) -> Self {
        let key_fragments = (&v).into();
        let value_fragments = (&v).into();
        let value;
        if let Some(v) = v.value_binary {
            value = DataEntryType::BinaryVal(v);
        } else if let Some(v) = v.value_bool {
            value = DataEntryType::BoolVal(v);
        } else if let Some(v) = v.value_integer {
            value = DataEntryType::IntVal(v);
        } else {
            // unwrap is safe because of data entry value is not null
            value = DataEntryType::StringVal(v.value_string.unwrap());
        }
        Self {
            address: v.address.clone(),
            key: v.key.clone(),
            height: v.height.clone(),
            value,
            key_fragments,
            value_fragments,
        }
    }
}

impl From<&data_entries::DataEntry> for Vec<DataEntryFragment> {
    fn from(v: &data_entries::DataEntry) -> Self {
        let fragments = vec![
            RawFragment(v.fragment_0_string.as_ref(), v.fragment_0_integer.as_ref()),
            RawFragment(v.fragment_1_string.as_ref(), v.fragment_1_integer.as_ref()),
            RawFragment(v.fragment_2_string.as_ref(), v.fragment_2_integer.as_ref()),
            RawFragment(v.fragment_3_string.as_ref(), v.fragment_3_integer.as_ref()),
            RawFragment(v.fragment_4_string.as_ref(), v.fragment_4_integer.as_ref()),
            RawFragment(v.fragment_5_string.as_ref(), v.fragment_5_integer.as_ref()),
            RawFragment(v.fragment_6_string.as_ref(), v.fragment_6_integer.as_ref()),
            RawFragment(v.fragment_7_string.as_ref(), v.fragment_7_integer.as_ref()),
            RawFragment(v.fragment_8_string.as_ref(), v.fragment_8_integer.as_ref()),
            RawFragment(v.fragment_9_string.as_ref(), v.fragment_9_integer.as_ref()),
            RawFragment(
                v.fragment_10_string.as_ref(),
                v.fragment_10_integer.as_ref(),
            ),
        ];
        fragments
            .into_iter()
            .map(Into::into)
            .take_while(|v: &Option<DataEntryFragment>| v.is_some())
            .filter_map(|v| v)
            .collect()
    }
}

impl From<&data_entries::DataEntry> for Vec<DataEntryValueFragment> {
    fn from(v: &data_entries::DataEntry) -> Self {
        let value_fragments = vec![
            RawFragment(
                v.value_fragment_0_string.as_ref(),
                v.value_fragment_0_integer.as_ref(),
            ),
            RawFragment(
                v.value_fragment_1_string.as_ref(),
                v.value_fragment_1_integer.as_ref(),
            ),
            RawFragment(
                v.value_fragment_2_string.as_ref(),
                v.value_fragment_2_integer.as_ref(),
            ),
            RawFragment(
                v.value_fragment_3_string.as_ref(),
                v.value_fragment_3_integer.as_ref(),
            ),
            RawFragment(
                v.value_fragment_4_string.as_ref(),
                v.value_fragment_4_integer.as_ref(),
            ),
            RawFragment(
                v.value_fragment_5_string.as_ref(),
                v.value_fragment_5_integer.as_ref(),
            ),
            RawFragment(
                v.value_fragment_6_string.as_ref(),
                v.value_fragment_6_integer.as_ref(),
            ),
            RawFragment(
                v.value_fragment_7_string.as_ref(),
                v.value_fragment_7_integer.as_ref(),
            ),
            RawFragment(
                v.value_fragment_8_string.as_ref(),
                v.value_fragment_8_integer.as_ref(),
            ),
            RawFragment(
                v.value_fragment_9_string.as_ref(),
                v.value_fragment_9_integer.as_ref(),
            ),
            RawFragment(
                v.value_fragment_10_string.as_ref(),
                v.value_fragment_10_integer.as_ref(),
            ),
        ];
        value_fragments
            .into_iter()
            .map(Into::into)
            .take_while(|v: &Option<DataEntryValueFragment>| v.is_some())
            .filter_map(|v| v)
            .collect()
    }
}

struct RawFragment<'a>(Option<&'a String>, Option<&'a i64>);

impl<'a> From<RawFragment<'a>> for Option<DataEntryFragment> {
    fn from(v: RawFragment) -> Self {
        match v {
            RawFragment(Some(string), _) => {
                let fragment = DataEntryFragment::String {
                    value: string.clone(),
                };
                Some(fragment)
            }
            RawFragment(_, Some(integer)) => {
                let fragment = DataEntryFragment::Integer { value: *integer };
                Some(fragment)
            }
            _ => None,
        }
    }
}

impl<'a> From<RawFragment<'a>> for Option<DataEntryValueFragment> {
    fn from(v: RawFragment) -> Self {
        match v {
            RawFragment(Some(string), _) => {
                let fragment = DataEntryValueFragment::String {
                    value: string.clone(),
                };
                Some(fragment)
            }
            RawFragment(_, Some(integer)) => {
                let fragment = DataEntryValueFragment::Integer { value: *integer };
                Some(fragment)
            }
            _ => None,
        }
    }
}
