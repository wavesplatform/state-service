pub mod repo;
mod sql;

use crate::error::Error;
use crate::schema::data_entries;
use diesel::QueryableByName;
use diesel::sql_types::Integer;
use sql::{ToSqlSortString, ToSqlWhereString};

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
}

pub trait DataEntriesRepo {
    fn search_data_entries<W: ToSqlWhereString, S: ToSqlSortString>(
        &self,
        query_where: Option<W>,
        query_sort: Option<S>,
        query_limit: u64,
        query_offset: u64,
    ) -> Result<Vec<DataEntry>, Error>;
}
