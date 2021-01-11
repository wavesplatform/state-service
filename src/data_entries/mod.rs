pub mod repo;

use crate::error::Error;
use crate::schema::data_entries;
use diesel::sql_types::Integer;

pub type SqlWhere = String;
pub type SqlSort = String;

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
}

pub trait DataEntriesRepo {
    fn search_data_entries(
        &self,
        filter: Option<impl Into<SqlWhere>>,
        sort: Option<impl Into<SqlSort>>,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<DataEntry>, Error>;
}
