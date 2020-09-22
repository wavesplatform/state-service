use super::errors::*;
use crate::data_entries::{
    AddressFilter, AddressSort, AndFilter, FragmentFilter, FragmentOperation, FragmentSort,
    FragmentType, InFilter, KeyFilter, KeySort, OrFilter, RequestFilter, RequestSort, Sort,
    SortDirection, ValueFilter, ValueSort, ValueSortType, ValueType,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::convert::{TryFrom, TryInto};

static INVALID_FRAGMENT_VALUE_TYPE_ERROR: Lazy<AppError> = Lazy::new(|| {
    AppError::new_validation_error(
        ValidationErrorCode::InvalidParamenterValue,
        ErrorDetails {
            parameter: "fragment.value".to_string(),
            reason: "Invalid value type, should be one of Array<u8>, boolean, number, String."
                .to_string(),
        },
    )
});

#[derive(Debug, Hash, Eq, PartialEq, Deserialize)]
enum QueryKey {
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

impl TryFrom<String> for QueryKey {
    type Error = AppError;

    fn try_from(value: String) -> Result<QueryKey, Self::Error> {
        match value.as_str() {
            "and" => Ok(QueryKey::AND),
            "or" => Ok(QueryKey::OR),
            "in" => Ok(QueryKey::IN),
            "fragment" => Ok(QueryKey::FRAGMENT),
            "key" => Ok(QueryKey::KEY),
            "value" => Ok(QueryKey::VALUE),
            "address" => Ok(QueryKey::ADDRESS),
            _ => Err(AppError::new_validation_error(ValidationErrorCode::InvalidParamenterValue, ErrorDetails {
                    parameter: "filter".to_string(),
                    reason: format!("{} is invalid filter key, should be one of: and, or, in, fragment, key, value, address.", value),
                },
            )),
        }
    }
}

impl TryFrom<&serde_json::Value> for FragmentType {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<FragmentType, Self::Error> {
        match value {
            serde_json::Value::String(op) => match op.as_str() {
                "string" => Ok(FragmentType::String),
                "integer" => Ok(FragmentType::Integer),
                _ => Err(AppError::new_validation_error(
                    ValidationErrorCode::InvalidParamenterValue,
                    ErrorDetails {
                        parameter: "type".to_string(),
                        reason: format!("{} is invalid fragment type.", op),
                    },
                )),
            },
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "fragment".to_string(),
                    reason: "Invalid fragment type value, shoud be an object.".to_string(),
                },
            )),
        }
    }
}

impl TryFrom<&serde_json::Value> for FragmentOperation {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<FragmentOperation, Self::Error> {
        match value {
            serde_json::Value::String(op) => match op.as_str() {
                "eq" => Ok(FragmentOperation::Eq),
                "gt" => Ok(FragmentOperation::Gt),
                "gte" => Ok(FragmentOperation::Gte),
                "lt" => Ok(FragmentOperation::Lt),
                "lte" => Ok(FragmentOperation::Lte),
                _ => Err(AppError::new_validation_error(
                    ValidationErrorCode::InvalidParamenterValue,
                    ErrorDetails {
                        parameter: "operation".to_string(),
                        reason: format!("{} is invalid fragment operation", op.to_string()),
                    },
                )),
            },
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "operation".to_string(),
                    reason: "Invalid fragment operation value, should be an object.".to_string(),
                },
            )),
        }
    }
}

struct FragmentKey(String);

impl TryFrom<&serde_json::Value> for FragmentKey {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Object(fr) => {
                let pos = fr.get("position").unwrap().as_u64().unwrap();
                let fragment_type: FragmentType = fr.get("type").unwrap().try_into()?;
                Ok(FragmentKey(format!("fragment_{}_{}", pos, fragment_type)))
            }
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "fragment".to_string(),
                    reason: "Invalid value, should be an object.".to_string(),
                },
            )),
        }
    }
}

impl ToString for FragmentKey {
    fn to_string(&self) -> String {
        self.0.to_owned()
    }
}

impl TryFrom<&serde_json::Value> for FragmentFilter {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Object(fr) => {
                let pos = fr.get("position").unwrap().as_u64().unwrap();
                let fragment_type: FragmentType = fr.get("type").unwrap().try_into()?;
                let operation: FragmentOperation = fr.get("operation").unwrap().try_into()?;
                let v = fr.get("value").unwrap();
                let value = match v {
                    serde_json::Value::Array(vs) => {
                        let bytes: Vec<u8> = vs.into_iter().map(|v| {
                            let v_as_u64 = v.as_u64().ok_or(INVALID_FRAGMENT_VALUE_TYPE_ERROR.to_owned())?;
                            v_as_u64.try_into().or(Err(INVALID_FRAGMENT_VALUE_TYPE_ERROR.to_owned()))
                        }).collect::<Result<Vec<u8>, AppError>>()?;
                        Ok(ValueType::BinaryVal(bytes))
                    },
                    serde_json::Value::Bool(v) => Ok(ValueType::BoolVal(v.to_owned())),
                    serde_json::Value::Number(v) => Ok(ValueType::IntVal(v.as_i64().unwrap())),
                    serde_json::Value::String(v) => Ok(ValueType::StringVal(v.as_str().to_owned())),
                    _ => Err(AppError::new_validation_error(ValidationErrorCode::InvalidParamenterValue,ErrorDetails {
                            parameter: "fragment.value".to_string(),
                            reason: "Invalid value type, should be one of Array<u8>, boolean, number, String.".to_string(),
                        },
                    ))
                }?;
                Ok(FragmentFilter {
                    position: pos,
                    fragment_type: fragment_type,
                    operation: operation,
                    value: value,
                })
            }
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "fragment".to_string(),
                    reason: "Invalid value, should be an object.".to_string(),
                },
            )),
        }
    }
}

impl From<FragmentFilter> for RequestFilter {
    fn from(value: FragmentFilter) -> Self {
        RequestFilter::Fragment(value)
    }
}

impl TryFrom<&serde_json::Value> for InFilter {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Object(in_filter) => {
                let properties =
                    in_filter
                        .get("properties")
                        .ok_or(AppError::new_validation_error(
                            ValidationErrorCode::MissingRequiredParameter,
                            ErrorDetails {
                                parameter: "properties".to_string(),
                                reason: "Missing field.".to_string(),
                            },
                        ))?;
                let values = in_filter
                    .get("values")
                    .ok_or(AppError::new_validation_error(
                        ValidationErrorCode::MissingRequiredParameter,
                        ErrorDetails {
                            parameter: "values".to_string(),
                            reason: "Missing field.".to_string(),
                        },
                    ))?;

                let fields = match properties {
                serde_json::Value::Array(properties) => {
                    properties
                    .into_iter()
                    .map(|prop| match prop {
                        serde_json::Value::Object(fields) => {
                                fields.into_iter().last().map_or(Err(AppError::new_validation_error(ValidationErrorCode::InvalidParamenterValue,ErrorDetails {
                                        parameter: "properties".to_string(),
                                        reason: format!(
                                            "Property has invalid parameters value type, should be an object.",
                                        ),
                                    },
                                )), |field| {
                                    let query_key = field.0.to_lowercase().try_into()?;
                                    match query_key {
                                        QueryKey::FRAGMENT => Ok(FragmentKey::try_from(field.1)?.to_string()),
                                        QueryKey::KEY => Ok("key".to_string()),
                                        QueryKey::VALUE => Ok("value".to_string()),
                                        QueryKey::ADDRESS => Ok("address".to_string()),
                                        _ => Err(AppError::new_validation_error(ValidationErrorCode::InvalidParamenterValue,ErrorDetails {
                                                parameter: "properties".to_string(),
                                                reason: format!(
                                                    "{:?} is invalid parameter key.",
                                                    query_key
                                                ),
                                            },
                                        )),
                                    }
                                })
                        },
                        _ => Err(AppError::new_validation_error(ValidationErrorCode::InvalidParamenterValue,ErrorDetails {
                                parameter: "properties".to_string(),
                                reason: format!(
                                    "Property has invalid parameters value type,  should be an object.",
                                ),
                            },
                        )),
                    })
                    .collect()
                }
                _ => Err(AppError::new_validation_error(ValidationErrorCode::InvalidParamenterValue,ErrorDetails {
                        parameter: "properties".to_string(),
                        reason: "Invalid value type, should be an array.".to_string(),
                    },
                )),
            }?;

                let in_values: Vec<Vec<ValueType>> = match values {
                    serde_json::Value::Array(rows) => {
                        let mut result: Vec<Result<Vec<ValueType>, AppError>> = vec![];
                        rows.into_iter().for_each(|row| {
                            let res = match row {
                                serde_json::Value::Array(values) => {
                                    let mut vs = vec![];
                                    values.into_iter().for_each(|value| match value {
                                        serde_json::Value::Bool(v) => {
                                            vs.push(Ok(ValueType::BoolVal(v.to_owned())))
                                        }
                                        serde_json::Value::Number(v) => {
                                            vs.push(Ok(ValueType::IntVal(v.as_i64().unwrap())))
                                        }
                                        serde_json::Value::String(v) => {
                                            vs.push(Ok(ValueType::StringVal(v.to_owned())))
                                        }
                                        serde_json::Value::Array(v) => {
                                            match v
                                                .into_iter()
                                                .map(|v| {
                                                    let v_as_u64 = v.as_u64().ok_or(
                                                        INVALID_FRAGMENT_VALUE_TYPE_ERROR
                                                            .to_owned(),
                                                    )?;
                                                    v_as_u64
                                                        .try_into()
                                                        .or(Err(INVALID_FRAGMENT_VALUE_TYPE_ERROR
                                                            .to_owned()))
                                                })
                                                .collect::<Result<Vec<u8>, AppError>>()
                                            {
                                                Ok(bytes) => {
                                                    vs.push(Ok(ValueType::BinaryVal(bytes)))
                                                }
                                                Err(err) => vs.push(Err(err)),
                                            }
                                        }
                                        _ => vs.push(Err(AppError::new_validation_error(
                                            ValidationErrorCode::InvalidParamenterValue,
                                            ErrorDetails {
                                                parameter: "values".to_string(),
                                                reason: "Invalid value type, should be an array."
                                                    .to_string(),
                                            },
                                        ))),
                                    });
                                    // returns Result<Vec<ValueType>>
                                    vs.into_iter().collect()
                                }
                                _ => Err(AppError::new_validation_error(
                                    ValidationErrorCode::InvalidParamenterValue,
                                    ErrorDetails {
                                        parameter: "values".to_string(),
                                        reason: "Invalid value type, should be an array."
                                            .to_string(),
                                    },
                                )),
                            };
                            result.push(res);
                        });
                        result.into_iter().collect()
                    }
                    _ => Err(AppError::new_validation_error(
                        ValidationErrorCode::InvalidParamenterValue,
                        ErrorDetails {
                            parameter: "values".to_string(),
                            reason: "Invalid value type, should be an array.".to_string(),
                        },
                    )),
                }?;
                Ok(InFilter {
                    properties: fields,
                    values: in_values,
                })
            }
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "in".to_string(),
                    reason: "Invalid value type, should be an object.".to_string(),
                },
            )),
        }
    }
}

impl From<InFilter> for RequestFilter {
    fn from(value: InFilter) -> Self {
        RequestFilter::In(value)
    }
}

fn parse_string_part(param: &str, part: &str, raw: &serde_json::Value) -> Result<String, AppError> {
    match raw {
        serde_json::Value::Object(o) => match o.get(part) {
            Some(val) => Ok(val.as_str().unwrap().to_owned()),
            None => Err(AppError::new_validation_error(
                ValidationErrorCode::MissingRequiredParameter,
                ErrorDetails {
                    parameter: format!("{}.{}", param, part),
                    reason: "Missing value.".to_string(),
                },
            )),
        },
        _ => Err(AppError::new_validation_error(
            ValidationErrorCode::InvalidParamenterValue,
            ErrorDetails {
                parameter: param.to_owned(),
                reason: "Invalid value type, should be an object.".to_string(),
            },
        )),
    }
}

impl TryFrom<&serde_json::Value> for KeyFilter {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let key = parse_string_part("key", "value", &value)?;
        Ok(KeyFilter { value: key })
    }
}

impl From<KeyFilter> for RequestFilter {
    fn from(value: KeyFilter) -> Self {
        RequestFilter::Key(value)
    }
}

impl TryFrom<&serde_json::Value> for ValueFilter {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let value = parse_string_part("value", "value", &value)?;
        Ok(ValueFilter { value: value })
    }
}

impl From<ValueFilter> for RequestFilter {
    fn from(value: ValueFilter) -> Self {
        RequestFilter::Value(value)
    }
}

impl TryFrom<&serde_json::Value> for AddressFilter {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let addr = parse_string_part("address", "value", &value)?;
        Ok(AddressFilter { value: addr })
    }
}

impl From<AddressFilter> for RequestFilter {
    fn from(value: AddressFilter) -> Self {
        RequestFilter::Address(value)
    }
}

impl TryFrom<&serde_json::Value> for RequestFilter {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Object(o) => {
                let filter = o.keys().into_iter().fold(Ok(None), |acc, key| {
                    let query_key = key.to_lowercase().try_into()?;
                    let next_filter = match query_key {
                        QueryKey::AND => match value.get(key).unwrap() {
                            serde_json::Value::Array(values) => {
                                let values: Vec<RequestFilter> = values
                                    .into_iter()
                                    .map(|value| value.try_into())
                                    .collect::<Result<Vec<RequestFilter>, AppError>>()?;

                                Some(RequestFilter::And(AndFilter { children: values }))
                            }
                            _ => None,
                        },
                        QueryKey::OR => match value.get(key).unwrap() {
                            serde_json::Value::Array(values) => {
                                let values: Vec<RequestFilter> = values
                                    .into_iter()
                                    .map(|value| value.try_into())
                                    .collect::<Result<Vec<RequestFilter>, AppError>>()?;

                                Some(RequestFilter::Or(OrFilter {
                                    children: values.clone(),
                                }))
                            }
                            _ => None,
                        },
                        QueryKey::IN => Some(InFilter::try_from(o.get(key).unwrap())?.into()),
                        QueryKey::FRAGMENT => {
                            Some(FragmentFilter::try_from(o.get(key).unwrap())?.into())
                        }
                        QueryKey::KEY => Some(KeyFilter::try_from(o.get(key).unwrap())?.into()),
                        QueryKey::VALUE => Some(ValueFilter::try_from(o.get(key).unwrap())?.into()),
                        QueryKey::ADDRESS => {
                            Some(AddressFilter::try_from(o.get(key).unwrap())?.into())
                        }
                    };

                    match next_filter {
                        Some(next_filter) => acc.map(|acc| match acc {
                            Some(filter) => match filter {
                                RequestFilter::And(and) => {
                                    and.clone().children.push(next_filter);
                                    Some(RequestFilter::And(and))
                                }
                                RequestFilter::Or(or) => {
                                    or.clone().children.push(next_filter);
                                    Some(RequestFilter::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(RequestFilter::And(AndFilter {
                                children: vec![next_filter],
                            })),
                        }),
                        _ => acc,
                    }
                });

                filter.and_then(|op| match op {
                    Some(n) => Ok(n),
                    None => Err(AppError::new_validation_error(
                        ValidationErrorCode::MissingRequiredParameter,
                        ErrorDetails {
                            parameter: "filter".to_string(),
                            reason: "Filter is empty.".to_string(),
                        },
                    )),
                })
            }
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "filter".to_string(),
                    reason: "Invalid value type, should be an object.".to_string(),
                },
            )),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Deserialize)]
enum QuerySortKey {
    #[serde(alias = "fragment")]
    FRAGMENT,
    #[serde(alias = "key")]
    KEY,
    #[serde(alias = "value")]
    VALUE,
    #[serde(alias = "address")]
    ADDRESS,
}

impl TryFrom<String> for QuerySortKey {
    type Error = AppError;

    fn try_from(value: String) -> Result<QuerySortKey, Self::Error> {
        match value.as_str() {
            "fragment" => Ok(QuerySortKey::FRAGMENT),
            "key" => Ok(QuerySortKey::KEY),
            "value" => Ok(QuerySortKey::VALUE),
            "address" => Ok(QuerySortKey::ADDRESS),
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "sort".to_string(),
                    reason: format!("{} is invalid sort key.", value),
                },
            )),
        }
    }
}

impl TryFrom<&str> for SortDirection {
    type Error = AppError;

    fn try_from(value: &str) -> Result<SortDirection, Self::Error> {
        match value {
            "asc" => Ok(SortDirection::Asc),
            "desc" => Ok(SortDirection::Desc),
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "direction".to_string(),
                    reason: format!("{} is invalid sort direction value.", value),
                },
            )),
        }
    }
}

impl TryFrom<&str> for ValueSortType {
    type Error = AppError;

    fn try_from(value: &str) -> Result<ValueSortType, Self::Error> {
        match value {
            "binary" => Ok(ValueSortType::Binary),
            "bool" => Ok(ValueSortType::Bool),
            "integer" => Ok(ValueSortType::Integer),
            "string" => Ok(ValueSortType::String),
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "type".to_string(),
                    reason: format!("{} is invalid value type.", value),
                },
            )),
        }
    }
}

impl TryFrom<&serde_json::Value> for SortDirection {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Object(o) => {
                o.get("direction").unwrap().as_str().unwrap().try_into()
            }
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "direction".to_string(),
                    reason: "Invalid value, should be asc or desc.".to_string(),
                },
            )),
        }
    }
}

impl TryFrom<&serde_json::Value> for FragmentSort {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Object(fr) => {
                let pos = fr.get("position").unwrap().as_u64().unwrap();
                let fragment_type = fr.get("type").unwrap().try_into()?;
                let direction = value.try_into()?;
                Ok(FragmentSort {
                    position: pos,
                    fragment_type: fragment_type,
                    direction: direction,
                })
            }
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "sort.fragment".to_string(),
                    reason: "Invalid value, should be an object.".to_string(),
                },
            )),
        }
    }
}

impl From<FragmentSort> for Sort {
    fn from(value: FragmentSort) -> Self {
        Sort::Fragment(value)
    }
}

impl TryFrom<&serde_json::Value> for KeySort {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let direction = value.try_into()?;
        Ok(KeySort {
            direction: direction,
        })
    }
}

impl From<KeySort> for Sort {
    fn from(value: KeySort) -> Self {
        Sort::Key(value)
    }
}

impl TryFrom<&serde_json::Value> for ValueSort {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let value_type = parse_string_part("value", "type", value)?
            .as_str()
            .try_into()?;
        let direction = value.try_into()?;
        Ok(ValueSort {
            value_type: value_type,
            direction: direction,
        })
    }
}

impl From<ValueSort> for Sort {
    fn from(value: ValueSort) -> Self {
        Sort::Value(value)
    }
}

impl TryFrom<&serde_json::Value> for AddressSort {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let direction = value.try_into()?;
        Ok(AddressSort {
            direction: direction,
        })
    }
}

impl From<AddressSort> for Sort {
    fn from(value: AddressSort) -> Self {
        Sort::Address(value)
    }
}

fn parse_sort_item(raw: &serde_json::Value) -> Result<Option<Sort>, AppError> {
    match raw {
        serde_json::Value::Object(o) => o
            .keys()
            .last()
            .map(|key| {
                let query_sort_key = key.to_lowercase().try_into()?;
                let next_sort = match query_sort_key {
                    QuerySortKey::FRAGMENT => FragmentSort::try_from(o.get(key).unwrap())?.into(),
                    QuerySortKey::KEY => KeySort::try_from(o.get(key).unwrap())?.into(),
                    QuerySortKey::VALUE => ValueSort::try_from(o.get(key).unwrap())?.into(),
                    QuerySortKey::ADDRESS => AddressSort::try_from(o.get(key).unwrap())?.into(),
                };
                Ok(Some(next_sort))
            })
            .unwrap_or(Err(AppError::new_validation_error(
                ValidationErrorCode::MissingRequiredParameter,
                ErrorDetails {
                    parameter: "sort".to_string(),
                    reason: "Missing sort key.".to_string(),
                },
            ))),
        _ => Err(AppError::new_validation_error(
            ValidationErrorCode::InvalidParamenterValue,
            ErrorDetails {
                parameter: "sort".to_string(),
                reason: "Invalid value type, should be an object.".to_string(),
            },
        )),
    }
}

impl TryFrom<&serde_json::Value> for RequestSort {
    type Error = AppError;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Array(vs) => {
                let values = vs
                    .iter()
                    .map(|value| parse_sort_item(value))
                    .collect::<Result<Option<Vec<Sort>>, AppError>>()?;
                values
                    .map(|vs| RequestSort { children: vs })
                    .ok_or(AppError::new_validation_error(
                        ValidationErrorCode::InvalidParamenterValue,
                        ErrorDetails {
                            parameter: "sort".to_string(),
                            reason: "Cannot extract sort.".to_string(),
                        },
                    ))
            }
            _ => Err(AppError::new_validation_error(
                ValidationErrorCode::InvalidParamenterValue,
                ErrorDetails {
                    parameter: "sort".to_string(),
                    reason: "Invalid value type, should be an object.".to_string(),
                },
            )),
        }
    }
}
