use diesel::prelude::*;
use diesel::sql_types::Integer;
use tokio::task::block_in_place;
use tracing::{info_span, instrument};

use crate::db::PgPool;
use crate::error::Error;
use crate::schema::data_entries;
use crate::schema::blocks_microblocks;
use crate::api::historical::HistoricalRequestParams;
use crate::api::parsing::{MgetEntries};
use crate::text_utils::pg_escape;

pub type SqlWhere = String;
pub type SqlSort = String;

#[derive(Clone, Debug, QueryableByName)]
#[table_name = "blocks_microblocks"]
struct BlockMicroblock {
    uid: i64
}

#[derive(Clone, Debug, QueryableByName)]
#[table_name = "data_entries"]
pub struct DataEntry {
    pub address: String,
    pub key: String,
    #[sql_type = "Integer"]
    pub height: i32,
    pub value_binary: Option<Vec<u8>>,
    pub value_bool: Option<bool>,
    pub value_integer: Option<i64>,
    pub value_string: Option<String>,
    pub fragment_0_string: Option<String>,
    pub fragment_0_integer: Option<i64>,
    pub fragment_1_string: Option<String>,
    pub fragment_1_integer: Option<i64>,
    pub fragment_2_string: Option<String>,
    pub fragment_2_integer: Option<i64>,
    pub fragment_3_string: Option<String>,
    pub fragment_3_integer: Option<i64>,
    pub fragment_4_string: Option<String>,
    pub fragment_4_integer: Option<i64>,
    pub fragment_5_string: Option<String>,
    pub fragment_5_integer: Option<i64>,
    pub fragment_6_string: Option<String>,
    pub fragment_6_integer: Option<i64>,
    pub fragment_7_string: Option<String>,
    pub fragment_7_integer: Option<i64>,
    pub fragment_8_string: Option<String>,
    pub fragment_8_integer: Option<i64>,
    pub fragment_9_string: Option<String>,
    pub fragment_9_integer: Option<i64>,
    pub fragment_10_string: Option<String>,
    pub fragment_10_integer: Option<i64>,
    pub value_fragment_0_string: Option<String>,
    pub value_fragment_0_integer: Option<i64>,
    pub value_fragment_1_string: Option<String>,
    pub value_fragment_1_integer: Option<i64>,
    pub value_fragment_2_string: Option<String>,
    pub value_fragment_2_integer: Option<i64>,
    pub value_fragment_3_string: Option<String>,
    pub value_fragment_3_integer: Option<i64>,
    pub value_fragment_4_string: Option<String>,
    pub value_fragment_4_integer: Option<i64>,
    pub value_fragment_5_string: Option<String>,
    pub value_fragment_5_integer: Option<i64>,
    pub value_fragment_6_string: Option<String>,
    pub value_fragment_6_integer: Option<i64>,
    pub value_fragment_7_string: Option<String>,
    pub value_fragment_7_integer: Option<i64>,
    pub value_fragment_8_string: Option<String>,
    pub value_fragment_8_integer: Option<i64>,
    pub value_fragment_9_string: Option<String>,
    pub value_fragment_9_integer: Option<i64>,
    pub value_fragment_10_string: Option<String>,
    pub value_fragment_10_integer: Option<i64>,
}

const MAX_UID: i64 = std::i64::MAX - 1;

const BASE_QUERY: &str = "SELECT de.address, de.key, bm.height, de.value_binary, de.value_bool, de.value_integer, de.value_string, \
de.fragment_0_string, de.fragment_0_integer, de.fragment_1_string, de.fragment_1_integer, \
de.fragment_2_string, de.fragment_2_integer, de.fragment_3_string, de.fragment_3_integer, \
de.fragment_4_string, de.fragment_4_integer, de.fragment_5_string, de.fragment_5_integer, \
de.fragment_6_string, de.fragment_6_integer, de.fragment_7_string, de.fragment_7_integer, \
de.fragment_8_string, de.fragment_8_integer, de.fragment_9_string, de.fragment_9_integer, \
de.fragment_10_string, de.fragment_10_integer, \
de.value_fragment_0_string, de.value_fragment_0_integer, de.value_fragment_1_string, de.value_fragment_1_integer, \
de.value_fragment_2_string, de.value_fragment_2_integer, de.value_fragment_3_string, de.value_fragment_3_integer, \
de.value_fragment_4_string, de.value_fragment_4_integer, de.value_fragment_5_string, de.value_fragment_5_integer, \
de.value_fragment_6_string, de.value_fragment_6_integer, de.value_fragment_7_string, de.value_fragment_7_integer, \
de.value_fragment_8_string, de.value_fragment_8_integer, de.value_fragment_9_string, de.value_fragment_9_integer, \
de.value_fragment_10_string, de.value_fragment_10_integer \
FROM data_entries de \
LEFT JOIN blocks_microblocks bm ON bm.uid = de.block_uid \
WHERE (de.value_binary IS NOT NULL OR de.value_bool IS NOT NULL OR de.value_integer IS NOT NULL OR de.value_string IS NOT NULL) ";

#[derive(Clone)]
pub struct Repo {
    pg_pool: PgPool,
}

impl Repo {
    pub fn new(pg_pool: PgPool) -> Self {
        Self { pg_pool }
    }

    #[instrument(level = "trace", skip(self, filter, sort, limit, offset))]
    pub async fn search_data_entries(
        &self,
        filter: Option<impl Into<SqlWhere>>,
        sort: Option<impl Into<SqlSort>>,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<DataEntry>, Error> {
        block_in_place(|| {
            let mut query_where_string: String = filter.map_or("".to_string(), |f| f.into());
            if query_where_string.len() > 0 {
                query_where_string = format!("AND {}", query_where_string);
            }

            let mut query_sort_string: String = sort.map_or("".to_string(), |s| s.into());

            if query_sort_string.len() > 0 {
                query_sort_string = format!("ORDER BY {}", query_sort_string);
            }

            let _g0 = info_span!("db_conn").entered();
            let conn = &self.pg_pool.get()?;
            let _g1 = info_span!("db_query").entered();

            let sql = format!(
                "{} AND de.superseded_by = $1 {} {} LIMIT {} OFFSET {}",
                BASE_QUERY, query_where_string, query_sort_string, limit, offset
            );

           //println!("{}", sql);

            diesel::sql_query(&sql)
            .bind::<diesel::sql_types::BigInt, _>(MAX_UID)
            .get_results::<DataEntry>(conn)
            .map_err(|err| Error::DbError(err))
        })
    }

    #[instrument(level = "trace", skip(self, filter, historical_filter))]
    pub async fn mget_data_entries(
        &self,
        filter: impl Into<SqlWhere>,
        historical_filter: String,
    ) -> Result<Vec<DataEntry>, Error> {
        block_in_place(|| {
            let query_filter_string: String = filter.into();

            if query_filter_string.len() > 0 {
                let _g0 = info_span!("db_conn").entered();
                let conn = &self.pg_pool.get()?;
                let _g1 = info_span!("db_query").entered();
                
                let sql = format!("{} AND ({}) {}", BASE_QUERY, query_filter_string, historical_filter);
                
                //println!("sql:{}; $1={}", sql, MAX_UID);

                diesel::sql_query(&sql)
                    .bind::<diesel::sql_types::BigInt, _>(MAX_UID)
                    .get_results::<DataEntry>(conn)
                    .map_err(|err| Error::DbError(err))
            } else {
                Ok(vec![])
            }
        })
    }

    pub async fn find_entities_uids(&self, hp: &HistoricalRequestParams, entries: &MgetEntries) -> Result<Vec<i64>, Error> {
        if hp.is_empty() {
            return Ok(vec![]);
        }

        block_in_place(|| {
            let mut uids = vec![];
            let mut sqls: Vec<String> = vec![];
            
            entries.address_key_pairs.iter().map(|e,| {
                    if hp.height.is_some() {
                        sqls.push(
                            format!(
                                "(select data_entry_uid as uid from data_entries_history_keys where address = '{}' and \"key\" = '{}' and height <= $1 order by height desc, data_entry_uid desc limit 1)",
                                pg_escape(e.address.as_str()),
                                pg_escape(e.key.as_str()),
                            )
                        );
                    }

                    if hp.block_timestamp.is_some() {
                        sqls.push(
                            format!(
                                "(select data_entry_uid as uid from data_entries_history_keys where address = '{}' and \"key\" = '{}' and block_timestamp <= to_timestamp($1) order by block_timestamp desc, data_entry_uid desc limit 1)",
                                pg_escape(e.address.as_str()),
                                pg_escape(e.key.as_str()),
                            )
                        );
                    }

            }).count();
            
            if ! sqls.is_empty() {
                //println!("history sqls: {:#?}", sqls);

                let _g0 = info_span!("db_conn").entered();
                let conn = &self.pg_pool.get()?;
                let _g1 = info_span!("db_query").entered();
                

                let v = match hp.height {
                    Some(h) => diesel::sql_query(sqls.join(" union ")).bind::<diesel::sql_types::BigInt, _>(h),
                    None => {
                        let t = hp.block_timestamp.unwrap().timestamp();
                        diesel::sql_query(sqls.join(" union ")).bind::<diesel::sql_types::BigInt, _>(t)
                    }
                };

                let r = v.get_results::<BlockMicroblock>(conn)
                .map_err(|err| Error::DbError(err))?;

                uids = r.iter().map(|e|{e.uid}).collect();
                
            }

            Ok(uids)
        })
    }
}
