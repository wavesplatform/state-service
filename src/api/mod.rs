mod errors;
pub mod historical;
pub mod parsing;
mod sql;

use serde::{Serialize, Serializer};
use std::collections::HashMap;
use tracing::{instrument, trace_span};
use warp::{
    reply::{json, Reply, Response},
    Filter, Rejection,
};
use wavesexchange_log::{error, info};
use wavesexchange_warp::error::{
    error_handler_with_serde_qs, handler, internal, timeout, validation,
};
use wavesexchange_warp::log::access;
use wavesexchange_warp::MetricsWarpBuilder;

use crate::data_entries;
use errors::*;
use historical::HistoricalRequestParams;
use itertools::Itertools;
use parsing::{Entry, MgetByAddress, MgetEntries, SearchRequest};

const ERROR_CODES_PREFIX: u16 = 95; // internal service

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
    fragments: Fragments,
}

#[derive(Clone, Debug, Serialize)]
pub struct Fragments {
    key: Vec<DataEntryFragment>,
    value: Vec<DataEntryValueFragment>,
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

pub async fn start(port: u16, metrics_port: u16, repo: data_entries::Repo) {
    let with_repo = warp::any().map(move || repo.clone());

    let request_tracing = warp::trace(|info| {
        let req_id = info
            .request_headers()
            .get("x-request-id")
            .map(|h| h.to_str().unwrap_or_default())
            .unwrap_or_default();
        trace_span!(
            "request",
            method = %info.method(),
            path = %info.path(),
            req_id = %req_id,
        )
    });

    let error_handler = handler(ERROR_CODES_PREFIX, |err| match err {
        AppError::ValidationError(_error_message, _error_code, error_details) => {
            validation::invalid_parameter(
                ERROR_CODES_PREFIX,
                error_details.to_owned().map(|details| details.into()),
            )
        }
        errors::AppError::DbError(error_message)
            if error_message == "canceling statement due to statement timeout" =>
        {
            error!("{:?}", err);
            timeout(ERROR_CODES_PREFIX)
        }
        _ => {
            error!("{:?}", err);
            internal(ERROR_CODES_PREFIX)
        }
    });

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
        .and(with_repo.clone())
        .and_then(search_handler);

    let mget_entries = warp::path::path("entries")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json::<MgetEntries>())
        .and(with_repo.clone())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(mget_handler);

    let post_by_address = warp::path!("entries" / String)
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json::<MgetByAddress>())
        .and(with_repo.clone())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(mget_by_address_handler);

    let mget_by_address = warp::path!("entries" / String)
        .and(warp::path::end())
        .and(warp::get())
        .and(serde_qs::warp::query::<MgetByAddress>(
            serde_qs::Config::new(5, false),
        ))
        .and(with_repo.clone())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(mget_by_address_handler);

    let get_by_address_key = warp::path!("entries" / String / String)
        .and(warp::path::end())
        .and(warp::get())
        .and(with_repo.clone())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(get_by_address_key_handler);

    let log = warp::log::custom(access);

    info!("Starting web server at 0.0.0.0:{}", port);

    let routes = search
        .or(mget_entries)
        .or(mget_by_address)
        .or(post_by_address)
        .or(get_by_address_key)
        .recover(move |rej| {
            error_handler_with_serde_qs(ERROR_CODES_PREFIX, error_handler.clone())(rej)
        })
        .with(request_tracing)
        .with(log);

    MetricsWarpBuilder::new()
        .with_main_routes(routes)
        .with_main_routes_port(port)
        .with_metrics_port(metrics_port)
        .run_async()
        .await;
}

fn decode_uri_string(s: String) -> Result<String, Rejection> {
    percent_encoding::percent_decode(s.as_bytes())
        .decode_utf8()
        .map(|s| s.to_string())
        .map_err(|error| {
            warp::reject::custom::<AppError>(AppError::DecodePathError(error.to_string()))
        })
}

#[derive(Debug, Serialize)]
struct MgetResponse {
    entries: Vec<Option<DataEntry>>,
}

impl Reply for MgetResponse {
    fn into_response(self) -> Response {
        json(&self).into_response()
    }
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
        let fragments = Fragments {
            key: key_fragments,
            value: value_fragments,
        };
        Self {
            address: v.address.clone(),
            key: v.key.clone(),
            height: v.height.clone(),
            value,
            fragments,
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

#[instrument(skip(req, repo))]
async fn search_handler(
    req: SearchRequest,
    repo: data_entries::Repo,
) -> Result<DataEntriesResponse, Rejection> {
    repo.search_data_entries(
        req.filter.clone(),
        req.sort.clone(),
        req.limit + 1,
        req.offset,
    )
    .await
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
        Err(warp::reject::custom::<AppError>(AppError::DbError(err.to_string()).into()).into())
    })
}

#[instrument(skip(req, repo))]
async fn mget_handler(
    req: MgetEntries,
    repo: data_entries::Repo,
    get_params: HashMap<String, String>,
) -> Result<MgetResponse, Rejection> {
    let address_key_pairs = req.address_key_pairs.clone();

    let hp = HistoricalRequestParams::from_hashmap(&get_params)?;

    let mget_entries = MgetEntries {
        address_key_pairs: address_key_pairs.clone(),
    };

    let e_uids = repo
        .find_entities_uids(&hp, &mget_entries)
        .await
        .or_else::<Rejection, _>(|err| {
            Err(warp::reject::custom::<AppError>(AppError::DbError(err.to_string()).into()).into())
        })?;

    reject_if_empty_uids(&hp, &e_uids)?;

    repo.mget_data_entries(req, build_historical_sql(&e_uids))
        .await
        .and_then(|data_entries| {
            let mut data_entries_map = data_entries
                .into_iter()
                .map(|de| {
                    let key = (de.address.clone(), de.key.clone());
                    let de = de.into();
                    (key, de)
                })
                .collect::<HashMap<_, _>>();
            let entries = address_key_pairs
                .into_iter()
                .map(|entry| {
                    let k = &(entry.address, entry.key);
                    data_entries_map.remove(k)
                })
                .collect::<Vec<Option<DataEntry>>>();
            Ok(MgetResponse { entries })
        })
        .or_else::<Rejection, _>(|err| {
            Err(warp::reject::custom::<AppError>(AppError::DbError(err.to_string()).into()).into())
        })
}

#[instrument(skip(query, repo))]
async fn mget_by_address_handler(
    address: String,
    query: MgetByAddress,
    repo: data_entries::Repo,
    get_params: HashMap<String, String>,
) -> Result<MgetResponse, Rejection> {
    let keys = query.keys.clone();
    let mget_entries = MgetEntries::from_query_by_address(address, query.keys);

    let hp = HistoricalRequestParams::from_hashmap(&get_params)?;

    let e_uids = repo
        .find_entities_uids(&hp, &mget_entries)
        .await
        .or_else::<Rejection, _>(|err| {
            Err(warp::reject::custom::<AppError>(AppError::DbError(err.to_string()).into()).into())
        })?;

    reject_if_empty_uids(&hp, &e_uids)?;

    repo.mget_data_entries(mget_entries, build_historical_sql(&e_uids))
        .await
        .and_then(|data_entries| {
            let mut data_entries_map = data_entries
                .into_iter()
                .map(|de| {
                    let key = de.key.clone();
                    let de = de.into();
                    (key, de)
                })
                .collect::<HashMap<_, _>>();
            let entries = keys
                .into_iter()
                .map(|key| data_entries_map.remove(&key))
                .collect::<Vec<Option<DataEntry>>>();
            Ok(MgetResponse { entries })
        })
        .or_else::<Rejection, _>(|err| {
            Err(warp::reject::custom::<AppError>(AppError::DbError(err.to_string()).into()).into())
        })
}

#[instrument(skip(repo))]
async fn get_by_address_key_handler(
    address: String,
    key: String,
    repo: data_entries::Repo,
    get_params: HashMap<String, String>,
) -> Result<DataEntry, Rejection> {
    let hp = HistoricalRequestParams::from_hashmap(&get_params)?;

    let key = decode_uri_string(key)?;
    let entry = Entry {
        address: address.clone(),
        key: key.clone(),
    };

    let mget_entries = MgetEntries {
        address_key_pairs: vec![entry],
    };

    let e_uids = repo
        .find_entities_uids(&hp, &mget_entries)
        .await
        .or_else::<Rejection, _>(|err| {
            Err(warp::reject::custom::<AppError>(AppError::DbError(err.to_string()).into()).into())
        })?;

    reject_if_empty_uids(&hp, &e_uids)?;

    repo.mget_data_entries(mget_entries, build_historical_sql(&e_uids))
        .await
        .or_else::<Rejection, _>(|err| {
            Err(warp::reject::custom::<AppError>(AppError::DbError(err.to_string()).into()).into())
        })
        .and_then(|data_entries| {
            if let Some(de) = data_entries.first() {
                Ok(DataEntry::from(de.clone()))
            } else {
                Err(warp::reject::not_found())
            }
        })
}

fn reject_if_empty_uids(hp: &HistoricalRequestParams, uids: &Vec<i64>) -> Result<(), Rejection> {
    if hp.is_empty() {
        return Ok(());
    }

    if uids.is_empty() {
        return Err(warp::reject::not_found());
    }

    Ok(())
}

fn build_historical_sql(uids: &Vec<i64>) -> String {
    if uids.is_empty() {
        " AND de.superseded_by = $1".to_string()
    } else {
        format!(" AND de.uid in ({}) AND $1 = $1", uids.iter().join(","))
    }
}
