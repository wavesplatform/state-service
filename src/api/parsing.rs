use super::errors::{AppError, ErrorDetails, ValidationErrorCode};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub filter: Option<RequestFilter>,
    pub sort: Option<RequestSort>,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default = "default_offset")]
    pub offset: u64,
}

impl SearchRequest {
    pub fn is_valid(&self) -> Result<(), AppError> {
        self.filter
            .as_ref()
            .map(|f| f.is_valid("filter.".to_string()))
            .unwrap_or(Ok(()))
    }
}

impl RequestFilter {
    fn is_valid(&self, context: String) -> Result<(), AppError> {
        match self {
            RequestFilter::And(filter) => filter.is_valid(context),
            RequestFilter::Or(filter) => filter.is_valid(context),
            RequestFilter::In(filter) => filter.is_valid(context),
            RequestFilter::Fragment(filter) => filter.is_valid(context),
            RequestFilter::ValueFragment(filter) => filter.is_valid(context),
            RequestFilter::Key(filter) => filter.is_valid(context),
            RequestFilter::Value(filter) => filter.is_valid(context),
            RequestFilter::Address(filter) => filter.is_valid(context),
        }
    }
}

impl AndFilter {
    fn is_valid(&self, context: String) -> Result<(), AppError> {
        self.0
            .iter()
            .enumerate()
            .try_for_each(|(idx, f)| f.is_valid(format!("{}and[{}].", context, idx)))
    }
}

impl OrFilter {
    fn is_valid(&self, context: String) -> Result<(), AppError> {
        self.0
            .iter()
            .enumerate()
            .try_for_each(|(idx, f)| f.is_valid(format!("{}or[{}].", context, idx)))
    }
}

impl InFilter {
    fn is_valid(&self, context: String) -> Result<(), AppError> {
        self.values.iter().try_fold(0, |idx, row| {
            if row.len() != self.properties.len() {
                let reason = format!("`values` row length at index {} is {}, while it should be equal to `properties` count ({}).", idx, row.len(), self.properties.len());
                Err(app_error(format!("{}in", context), reason))
            } else {
                Ok(idx + 1)
            }
        }).map(|_| ())
    }
}

impl FragmentFilter {
    fn is_valid(&self, context: String) -> Result<(), AppError> {
        let new_context = format!("{}fragment", context);
        match self {
            Self {
                value: FragmentValueType::IntVal(_),
                fragment_type: FragmentType::String,
                ..
            } => Err(app_error(
                new_context,
                "`string` fragment type requires `value` of string type, found integer.".into(),
            )),
            Self {
                value: FragmentValueType::StringVal(_),
                fragment_type: FragmentType::Integer,
                ..
            } => Err(app_error(
                new_context,
                "`integer` fragment type requires `value` of integer type, found string.".into(),
            )),
            Self {
                fragment_type: FragmentType::String,
                operation,
                ..
            } => {
                if *operation == FragmentOperation::Eq {
                    Ok(())
                } else {
                    Err(app_error(
                        new_context,
                        "String value type supports only `eq` operation.".into(),
                    ))
                }
            }
            _ => Ok(()),
        }
    }
}

impl ValueFragmentFilter {
    fn is_valid(&self, context: String) -> Result<(), AppError> {
        let new_context = format!("{}value_fragment", context);
        match self {
            Self {
                value: FragmentValueType::IntVal(_),
                fragment_type: FragmentType::String,
                ..
            } => Err(app_error(
                new_context,
                "`string` fragment type requires `value` of string type, found integer.".into(),
            )),
            Self {
                value: FragmentValueType::StringVal(_),
                fragment_type: FragmentType::Integer,
                ..
            } => Err(app_error(
                new_context,
                "`integer` fragment type requires `value` of integer type, found string.".into(),
            )),
            Self {
                fragment_type: FragmentType::String,
                operation,
                ..
            } => {
                if *operation == FragmentOperation::Eq {
                    Ok(())
                } else {
                    Err(app_error(
                        new_context,
                        "String value type supports only `eq` operation.".into(),
                    ))
                }
            }
            _ => Ok(()),
        }
    }
}

fn app_error(parameter: String, reason: String) -> AppError {
    AppError::new_validation_error(
        ValidationErrorCode::InvalidParamenterValue,
        ErrorDetails { parameter, reason },
    )
}

impl KeyFilter {
    fn is_valid(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
}

impl ValueFilter {
    fn is_valid(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
}

impl AddressFilter {
    fn is_valid(&self, _: String) -> Result<(), AppError> {
        Ok(())
    }
}

fn default_limit() -> u64 {
    100u64
}

fn default_offset() -> u64 {
    0u64
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum FragmentValueType {
    IntVal(i64),
    StringVal(String),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum ValueType {
    BinaryVal(Vec<u8>),
    BoolVal(bool),
    IntVal(i64),
    StringVal(String),
}

#[derive(Clone, Debug, Deserialize)]
pub enum FragmentType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "integer")]
    Integer,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub enum FragmentOperation {
    #[serde(rename = "eq")]
    Eq,
    #[serde(rename = "gt")]
    Gt,
    #[serde(rename = "gte")]
    Gte,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "lte")]
    Lte,
}

#[derive(Clone, Debug, Deserialize)]
pub enum RequestFilter {
    #[serde(rename = "and")]
    And(AndFilter),
    #[serde(rename = "or")]
    Or(OrFilter),
    #[serde(rename = "in")]
    In(InFilter),
    #[serde(rename = "fragment")]
    Fragment(FragmentFilter),
    #[serde(rename = "value_fragment")]
    ValueFragment(ValueFragmentFilter),
    #[serde(rename = "key")]
    Key(KeyFilter),
    #[serde(rename = "value")]
    Value(ValueFilter),
    #[serde(rename = "address")]
    Address(AddressFilter),
}

#[derive(Clone, Debug, Deserialize)]
pub struct AndFilter(pub Vec<RequestFilter>);

#[derive(Clone, Debug, Deserialize)]
pub struct OrFilter(pub Vec<RequestFilter>);

#[derive(Clone, Debug, Deserialize)]
pub struct FragmentFilter {
    #[serde(rename = "type")]
    pub fragment_type: FragmentType,
    pub position: u64,
    pub operation: FragmentOperation,
    pub value: FragmentValueType,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ValueFragmentFilter {
    #[serde(rename = "type")]
    pub fragment_type: FragmentType,
    pub position: u64,
    pub operation: FragmentOperation,
    pub value: FragmentValueType,
}

#[derive(Clone, Debug, Deserialize)]
pub enum InItemFilter {
    #[serde(rename = "fragment")]
    Fragment {
        #[serde(rename = "type")]
        fragment_type: FragmentType,
        position: u64,
    },
    #[serde(rename = "key")]
    Key {},
    #[serde(rename = "value")]
    Value {},
    #[serde(rename = "address")]
    Address {},
}

#[derive(Clone, Debug, Deserialize)]
pub struct InFilter {
    pub properties: Vec<InItemFilter>,
    pub values: Vec<Vec<ValueType>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeyFilter {
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ValueFilter {
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AddressFilter {
    pub value: String,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub enum QueryKey {
    #[serde(alias = "and")]
    AND,
    #[serde(alias = "or")]
    OR,
    #[serde(alias = "in")]
    IN,
    #[serde(alias = "fragment")]
    FRAGMENT,
    #[serde(alias = "key")]
    KEY,
    #[serde(alias = "value")]
    VALUE,
    #[serde(alias = "address")]
    ADDRESS,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub enum QuerySortKey {
    #[serde(rename = "fragment")]
    FRAGMENT,
    #[serde(rename = "key")]
    KEY,
    #[serde(rename = "value")]
    VALUE,
    #[serde(rename = "address")]
    ADDRESS,
}

#[derive(Clone, Debug, Deserialize)]
pub enum SortItemDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

#[derive(Clone, Debug, Deserialize)]
pub enum SortItem {
    #[serde(rename = "fragment")]
    Fragment {
        position: u64,
        #[serde(rename = "type")]
        fragment_type: FragmentType,
        direction: SortItemDirection,
    },
    #[serde(rename = "key")]
    Key { direction: SortItemDirection },
    #[serde(rename = "value")]
    Value { direction: SortItemDirection },
    #[serde(rename = "address")]
    Address { direction: SortItemDirection },
}

#[derive(Clone, Debug, Deserialize)]
pub struct RequestSort(pub Vec<SortItem>);
