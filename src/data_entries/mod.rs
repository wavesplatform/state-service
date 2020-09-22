pub mod repo;
mod sql;

use crate::error::Error;
use crate::schema::data_entries;
use base64::encode;
use diesel::sql_types::Integer;
use std::fmt;

type SqlWhere = String;
type SqlSort = String;

#[derive(Clone, Debug)]
pub enum RequestFilter {
    And(AndFilter),
    Or(OrFilter),
    In(InFilter),
    Fragment(FragmentFilter),
    Key(KeyFilter),
    Value(ValueFilter),
    Address(AddressFilter),
}

#[derive(Clone, Debug)]
pub enum ValueType {
    BinaryVal(Vec<u8>),
    BoolVal(bool),
    IntVal(i64),
    StringVal(String),
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::BinaryVal(v) => write!(f, "{}", encode(v)),
            ValueType::BoolVal(v) => write!(f, "{}", v),
            ValueType::IntVal(v) => write!(f, "{}", v),
            ValueType::StringVal(v) => write!(f, "'{}'", v),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AndFilter {
    pub children: Vec<RequestFilter>,
}

#[derive(Clone, Debug)]
pub struct OrFilter {
    pub children: Vec<RequestFilter>,
}

#[derive(Clone, Debug)]
pub struct InFilter {
    pub properties: Vec<String>,
    pub values: Vec<Vec<ValueType>>,
}

#[derive(Clone, Debug)]
pub struct FragmentFilter {
    pub position: u64,
    pub fragment_type: FragmentType,
    pub operation: FragmentOperation,
    pub value: ValueType,
}

#[derive(Clone, Debug)]
pub struct KeyFilter {
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct ValueFilter {
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct AddressFilter {
    pub value: String,
}

#[derive(Clone, Debug)]
pub enum FragmentType {
    String,
    Integer,
}

impl fmt::Display for FragmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FragmentType::String => write!(f, "string"),
            FragmentType::Integer => write!(f, "integer"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum FragmentOperation {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl fmt::Display for FragmentOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FragmentOperation::Eq => write!(f, "="),
            FragmentOperation::Gt => write!(f, ">"),
            FragmentOperation::Gte => write!(f, ">="),
            FragmentOperation::Lt => write!(f, "<"),
            FragmentOperation::Lte => write!(f, "<="),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Sort {
    Fragment(FragmentSort),
    Key(KeySort),
    Value(ValueSort),
    Address(AddressSort),
}

#[derive(Clone, Debug)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Debug)]
pub struct RequestSort {
    pub children: Vec<Sort>,
}

#[derive(Clone, Debug)]
pub struct FragmentSort {
    pub position: u64,
    pub fragment_type: FragmentType,
    pub direction: SortDirection,
}

#[derive(Clone, Debug)]
pub struct KeySort {
    pub direction: SortDirection,
}

#[derive(Clone, Debug)]
pub enum ValueSortType {
    Binary,
    Bool,
    Integer,
    String,
}

#[derive(Clone, Debug)]
pub struct ValueSort {
    pub value_type: ValueSortType,
    pub direction: SortDirection,
}

#[derive(Clone, Debug)]
pub struct AddressSort {
    pub direction: SortDirection,
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
}

pub trait DataEntriesRepo {
    fn search_data_entries(
        &self,
        query_where: Option<RequestFilter>,
        query_sort: Option<RequestSort>,
        query_limit: u64,
        query_offset: u64,
    ) -> Result<Vec<DataEntry>, Error>;
}
