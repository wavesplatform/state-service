use super::{DataEntriesRepo, DataEntry, SqlSort, SqlWhere};
use crate::db::PgPool;
use crate::error::Error;
use diesel::prelude::*;

const MAX_UID: i64 = std::i64::MAX - 1;

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
        let query_where_string: String = filter.map_or("".to_string(), |f| f.into());

        let mut query_sort_string: String = sort.map_or("".to_string(), |s| s.into());

        if query_sort_string.len() > 0 {
            query_sort_string = format!("ORDER BY {}", query_sort_string);
        }

        diesel::sql_query(format!(
            "SELECT de.address, de.key, bm.height, de.value_binary, de.value_bool, de.value_integer, de.value_string 
            FROM data_entries de 
            LEFT JOIN blocks_microblocks bm ON bm.uid = de.block_uid 
            WHERE de.superseded_by = $1 AND (de.value_binary IS NOT NULL OR de.value_bool IS NOT NULL OR de.value_integer IS NOT NULL OR de.value_string IS NOT NULL) AND {} {} LIMIT {} OFFSET {}",
            query_where_string,
            query_sort_string,
            limit,
            offset
        )).bind::<diesel::sql_types::BigInt, _>(MAX_UID).get_results::<DataEntry>(&self.pool.get()?).map_err(|err| Error::DbError(err))
    }
}
