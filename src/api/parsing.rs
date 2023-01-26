use super::errors::{AppError, ErrorDetails, ValidationErrorCode};
use serde::Deserialize;

const LIMIT_MAX: u64 = 5000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    pub filter: Option<RequestFilter>,
    pub sort: Option<RequestSort>,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default = "default_offset")]
    pub offset: u64,
}

#[derive(Debug, Deserialize)]
pub struct MgetEntries {
    pub address_key_pairs: Vec<Entry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Entry {
    pub address: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct MgetByAddress {
    pub keys: Vec<String>,
}

impl MgetEntries {
    pub fn from_query_by_address(address: String, keys: Vec<String>) -> Self {
        let address_key_pairs = keys
            .into_iter()
            .map(|key| Entry {
                address: address.clone(),
                key,
            })
            .collect();
        Self { address_key_pairs }
    }
}

impl SearchRequest {
    pub fn is_valid(&self) -> Result<(), AppError> {
        if self.limit > LIMIT_MAX {
            return Err(app_error(
                "limit".into(),
                format!("maximum value {} exceeded", LIMIT_MAX),
            ));
        }
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
                return Err(app_error(format!("{}in", context), reason));
            }
            for (index, key_value_pair) in self.properties.iter().zip(row.iter()).enumerate() {
                match key_value_pair {
                    (item @ InItemFilter::Fragment { fragment_type: FragmentType::Integer, .. }, InFilterValue::IntVal(_)) => {
                        let context = format!("{}in[{}][{}]", context, idx, index);
                        item.is_valid(context)?;
                    }
                    (item @ InItemFilter::Fragment { fragment_type: FragmentType::String, .. }, InFilterValue::StringVal(_)) => {
                        let context = format!("{}in[{}][{}]", context, idx, index);
                        item.is_valid(context)?;
                    }
                    (InItemFilter::Key {  }, InFilterValue::StringVal(_)) => {}
                    (InItemFilter::Address {  }, InFilterValue::StringVal(_)) => {}
                    (InItemFilter::Value { value_type: ValueType::Binary }, InFilterValue::BinaryVal(_)) => {}
                    (InItemFilter::Value { value_type: ValueType::Bool }, InFilterValue::BoolVal(_)) => {}
                    (InItemFilter::Value { value_type: ValueType::Integer }, InFilterValue::IntVal(_)) => {}
                    (InItemFilter::Value { value_type: ValueType::String }, InFilterValue::StringVal(_)) => {}
                    (filter, value) => {
                        return in_item_filter_error(filter, value, &context, idx, index);
                    }
                }
            };
            Ok(idx + 1)
        }).map(|_| ())
    }
}

fn in_item_filter_error(
    filter: &InItemFilter,
    value: &InFilterValue,
    context: &String,
    idx: i32,
    index: usize,
) -> Result<i32, AppError> {
    let base_type = filter.to_type();
    let name = filter.to_name();
    let current_type = value.to_type();
    let reason = match filter {
        InItemFilter::Address {} | InItemFilter::Key {} => {
            format!(
                "value of {} should be {}, found {}.",
                name, base_type, current_type
            )
        }
        _ => {
            format!(
                "`{}` {} type requires `value` of {} type, found {}.",
                base_type, name, base_type, current_type
            )
        }
    };
    let parameter = format!("{}in.values[{}][{}]", context, idx, index);
    Err(app_error(parameter, reason))
}

impl KeyFragmentFilter {
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
                if *operation == Operation::Eq {
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
                if *operation == Operation::Eq {
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
    fn is_valid(&self, context: String) -> Result<(), AppError> {
        let new_context = format!("{}value", context);
        self.valid_type(&new_context)?;
        self.valid_operation(&new_context)?;
        Ok(())
    }

    fn valid_type(&self, context: &String) -> Result<(), AppError> {
        match self {
            Self {
                value_type: ValueType::String,
                value: ValueData::String(_),
                ..
            } => {}
            Self {
                value_type: ValueType::Integer,
                value: ValueData::Integer(_),
                ..
            } => {}
            Self {
                value_type: ValueType::Binary,
                value: ValueData::Binary(_),
                ..
            } => {}
            Self {
                value_type: ValueType::Bool,
                value: ValueData::Bool(_),
                ..
            } => {}
            Self {
                value_type, value, ..
            } => {
                let base_type = value_type.to_type();
                let current_type = value.to_type();
                let reason = format!(
                    "`{}` value type requires `value` of {} type, found {}",
                    base_type, base_type, current_type
                );
                return Err(app_error(context.to_owned(), reason));
            }
        }
        Ok(())
    }

    fn valid_operation(&self, context: &String) -> Result<(), AppError> {
        match self {
            Self {
                operation: Operation::Eq,
                ..
            } => {}
            Self {
                value_type: ValueType::Integer,
                ..
            } => {}
            Self {
                value_type,
                operation,
                ..
            } => {
                let base_type = value_type.to_type();
                let op_type = operation.to_type();
                let reason = format!(
                    "`{}` value type support only `eq` operation, found {}",
                    base_type, op_type
                );
                return Err(app_error(context.to_owned(), reason));
            }
        }
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
pub enum InFilterValue {
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
pub enum Operation {
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
    Fragment(KeyFragmentFilter),
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
pub struct KeyFragmentFilter {
    #[serde(rename = "type")]
    pub fragment_type: FragmentType,
    pub position: u64,
    pub operation: Operation,
    pub value: FragmentValueType,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ValueFragmentFilter {
    #[serde(rename = "type")]
    pub fragment_type: FragmentType,
    pub position: u64,
    pub operation: Operation,
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
    Value {
        #[serde(rename = "type")]
        value_type: ValueType,
    },
    #[serde(rename = "address")]
    Address {},
}

impl InItemFilter {
    fn is_valid(&self, context: String) -> Result<(), AppError> {
        match self {
            InItemFilter::Fragment { position, .. } => {
                if *position > 10 {
                    let reason = "`position` out of range, should be less or equal than 10.".into();
                    return Err(app_error(context, reason));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn to_name(&self) -> String {
        match self {
            InItemFilter::Fragment { .. } => "fragment".to_string(),
            InItemFilter::Key {} => "key".to_string(),
            InItemFilter::Value { .. } => "value".to_string(),
            InItemFilter::Address {} => "address".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    String,
    Integer,
    Binary,
    Bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct InFilter {
    pub properties: Vec<InItemFilter>,
    pub values: Vec<Vec<InFilterValue>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeyFilter {
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ValueFilter {
    #[serde(rename = "type")]
    pub value_type: ValueType,
    pub operation: Operation,
    pub value: ValueData,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum ValueData {
    String(String),
    Binary(Vec<u8>),
    Bool(bool),
    Integer(i64),
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
    //default order by data_entries.uid
    #[serde(rename = "base")]
    Base { direction: SortItemDirection },
    #[serde(rename = "value_fragment")]
    ValueFragment {
        position: u64,
        #[serde(rename = "type")]
        fragment_type: FragmentType,
        direction: SortItemDirection,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct RequestSort(pub Vec<SortItem>);

pub trait ToType {
    fn to_type(&self) -> String;
}

impl ToType for InItemFilter {
    fn to_type(&self) -> String {
        match self {
            InItemFilter::Fragment { fragment_type, .. } => fragment_type.to_type(),
            InItemFilter::Key {} => "string".to_string(),
            InItemFilter::Value { value_type } => value_type.to_type(),
            InItemFilter::Address {} => "string".to_string(),
        }
    }
}

impl ToType for FragmentType {
    fn to_type(&self) -> String {
        match self {
            FragmentType::String => "string".to_string(),
            FragmentType::Integer => "integer".to_string(),
        }
    }
}

impl ToType for ValueType {
    fn to_type(&self) -> String {
        match self {
            ValueType::String => "string".to_string(),
            ValueType::Integer => "integer".to_string(),
            ValueType::Binary => "binary".to_string(),
            ValueType::Bool => "bool".to_string(),
        }
    }
}

impl ToType for InFilterValue {
    fn to_type(&self) -> String {
        match self {
            InFilterValue::BinaryVal(_) => "binary".to_string(),
            InFilterValue::BoolVal(_) => "bool".to_string(),
            InFilterValue::IntVal(_) => "integer".to_string(),
            InFilterValue::StringVal(_) => "string".to_string(),
        }
    }
}

impl ToType for ValueData {
    fn to_type(&self) -> String {
        match self {
            ValueData::String(_) => "string".to_string(),
            ValueData::Binary(_) => "binary".to_string(),
            ValueData::Bool(_) => "bool".to_string(),
            ValueData::Integer(_) => "integer".to_string(),
        }
    }
}

impl ToType for Operation {
    fn to_type(&self) -> String {
        match self {
            Operation::Eq => "eq".to_string(),
            Operation::Gt => "gt".to_string(),
            Operation::Gte => "gte".to_string(),
            Operation::Lt => "lt".to_string(),
            Operation::Lte => "lte".to_string(),
        }
    }
}
