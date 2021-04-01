use super::{DataEntriesRepo, DataEntry, SqlSort, SqlWhere};
use crate::db::PgPool;
use crate::error::Error;
use diesel::prelude::*;

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
WHERE de.superseded_by = $1 AND (de.value_binary IS NOT NULL OR de.value_bool IS NOT NULL OR de.value_integer IS NOT NULL OR de.value_string IS NOT NULL)";

#[derive(Clone)]
pub struct DataEntriesRepoImpl {
    pool: PgPool,
}

impl DataEntriesRepoImpl {
    pub fn new(pg_pool: PgPool) -> DataEntriesRepoImpl {
        DataEntriesRepoImpl { pool: pg_pool }
    }
}

impl DataEntriesRepo for DataEntriesRepoImpl {
    fn search_data_entries(
        &self,
        filter: Option<impl Into<SqlWhere>>,
        sort: Option<impl Into<SqlSort>>,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<DataEntry>, Error> {
        let mut query_where_string: String = filter.map_or("".to_string(), |f| f.into());
        if query_where_string.len() > 0 {
            query_where_string = format!("AND {}", query_where_string);
        }

        let mut query_sort_string: String = sort.map_or("".to_string(), |s| s.into());

        if query_sort_string.len() > 0 {
            query_sort_string = format!("ORDER BY {}", query_sort_string);
        }

        diesel::sql_query(format!(
            "{} {} {} LIMIT {} OFFSET {}",
            BASE_QUERY, query_where_string, query_sort_string, limit, offset
        ))
        .bind::<diesel::sql_types::BigInt, _>(MAX_UID)
        .get_results::<DataEntry>(&self.pool.get()?)
        .map_err(|err| Error::DbError(err))
    }

    fn mget_data_entries(&self, filter: impl Into<SqlWhere>) -> Result<Vec<DataEntry>, Error> {
        let query_filter_string: String = filter.into();
        if query_filter_string.len() > 0 {
            diesel::sql_query(format!("{} AND {}", BASE_QUERY, query_filter_string))
                .bind::<diesel::sql_types::BigInt, _>(MAX_UID)
                .get_results::<DataEntry>(&self.pool.get()?)
                .map_err(|err| Error::DbError(err))
        } else {
            Ok(vec![])
        }
    }
}
