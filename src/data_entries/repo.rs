use super::{DataEntriesRepo, DataEntry};
use super::{RequestFilter, RequestSort};
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
        query_where: Option<RequestFilter>,
        query_sort: Option<RequestSort>,
        query_limit: u64,
        query_offset: u64,
    ) -> Result<Vec<DataEntry>, Error> {
        let query_where_string =
            query_where.map_or("".to_string(), |query_where| query_where.into());
        let mut query_sort_string =
            query_sort.map_or("".to_string(), |query_sort| query_sort.into());
        if query_sort_string.len() > 0 {
            query_sort_string = format!("ORDER BY {}", query_sort_string);
        }

        diesel::sql_query(format!(
            "SELECT de.address, de.key, bm.height, de.value_binary, de.value_bool, de.value_integer, de.value_string FROM data_entries de LEFT JOIN blocks_microblocks bm ON bm.uid = de.block_uid WHERE de.superseded_by = $1 AND {} {} LIMIT {} OFFSET {}",
            query_where_string,
            query_sort_string,
            query_limit,
            query_offset
        )).bind::<diesel::sql_types::BigInt, _>(MAX_UID).get_results::<DataEntry>(&self.pool.get()?).map_err(|err| Error::DbError(err))
    }
}
