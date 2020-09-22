use super::errors::*;
use crate::data_entries::{
    AddressFilter, AddressSort, AndFilter, FragmentFilter, FragmentOperation, FragmentSort,
    FragmentType, InFilter, KeyFilter, KeySort, OrFilter, RequestFilter, RequestSort, Sort,
    SortDirection, ValueFilter, ValueSort, ValueSortType, ValueType,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::convert::TryInto;

static INVALID_FRAGMENT_VALUE_TYPE_ERROR: Lazy<AppError> = Lazy::new(|| {
    AppError::new_validation_error(ErrorDetails {
        parameter: "fragment.value".to_string(),
        reason: "Invalid value type, should be one of Array<u8>, boolean, number, String."
            .to_string(),
    })
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

impl TryInto<QueryKey> for String {
    type Error = AppError;

    fn try_into(self) -> Result<QueryKey, Self::Error> {
        match self.as_str() {
            "and" => Ok(QueryKey::AND),
            "or" => Ok(QueryKey::OR),
            "in" => Ok(QueryKey::IN),
            "fragment" => Ok(QueryKey::FRAGMENT),
            "key" => Ok(QueryKey::KEY),
            "value" => Ok(QueryKey::VALUE),
            "address" => Ok(QueryKey::ADDRESS),
            _ => Err(AppError::new_validation_error(
                
                ErrorDetails {
                    parameter: "filter".to_string(),
                    reason: format!("{} is invalid filter key, should be one of: and, or, in, fragment, key, value, address.", self),
                },
            )),
        }
    }
}

impl TryInto<FragmentType> for serde_json::Value {
    type Error = AppError;

    fn try_into(self) -> Result<FragmentType, Self::Error> {
        match self {
            serde_json::Value::String(op) => match op.as_str() {
                "string" => Ok(FragmentType::String),
                "integer" => Ok(FragmentType::Integer),
                _ => Err(AppError::new_validation_error(
                    
                    ErrorDetails {
                        parameter: "type".to_string(),
                        reason: format!("{} is invalid fragment type.", op.as_str()),
                    },
                )),
            },
            _ => Err(AppError::new_validation_error(
                
                ErrorDetails {
                    parameter: "fragment".to_string(),
                    reason: "Invalid fragment type value, shoud be an object.".to_string(),
                },
            )),
        }
    }
}

impl TryInto<FragmentOperation> for serde_json::Value {
    type Error = AppError;

    fn try_into(self) -> Result<FragmentOperation, Self::Error> {
        match self {
            serde_json::Value::String(op) => match op.as_str() {
                "eq" => Ok(FragmentOperation::Eq),
                "gt" => Ok(FragmentOperation::Gt),
                "gte" => Ok(FragmentOperation::Gte),
                "lt" => Ok(FragmentOperation::Lt),
                "lte" => Ok(FragmentOperation::Lte),
                _ => Err(AppError::new_validation_error(
                    
                    ErrorDetails {
                        parameter: "operation".to_string(),
                        reason: format!("{} is invalid fragment operation", op.to_string()),
                    },
                )),
            },
            _ => Err(AppError::new_validation_error(
                
                ErrorDetails {
                    parameter: "operation".to_string(),
                    reason: "Invalid fragment operation value, should be an object.".to_string(),
                },
            )),
        }
    }
}

// todo TryInto<FragmentKey>
fn parse_fragment_key(raw: &serde_json::Value) -> Result<String, AppError> {
    match raw.clone() {
        serde_json::Value::Object(fr) => {
            let pos = fr.get("position").unwrap().as_u64().unwrap();
            let fragment_type: FragmentType = fr.get("type").unwrap().to_owned().try_into()?;
            Ok(format!("fragment_{}_{}", pos, fragment_type))
        }
        _ => Err(AppError::new_validation_error(
            
            ErrorDetails {
                parameter: "fragment".to_string(),
                reason: "Invalid value, should be an object.".to_string(),
            },
        )),
    }
}

fn parse_fragment_with_operation(raw: &serde_json::Value) -> Result<RequestFilter, AppError> {
    match raw.clone() {
        serde_json::Value::Object(fr) => {
            let pos = fr.get("position").unwrap().as_u64().unwrap();
            let fragment_type: FragmentType = fr.get("type").unwrap().to_owned().try_into()?;
            let operation: FragmentOperation =
                fr.get("operation").unwrap().to_owned().try_into()?;
            let value;
            let v = fr.get("value").unwrap();
            value = match v {
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
                _ => Err(AppError::new_validation_error(
                    
                    ErrorDetails {
                        parameter: "fragment.value".to_string(),
                        reason: "Invalid value type, should be one of Array<u8>, boolean, number, String.".to_string(),
                    },
                ))
            }?;
            Ok(RequestFilter::Fragment(FragmentFilter {
                position: pos,
                fragment_type: fragment_type,
                operation: operation,
                value: value,
            }))
        }
        _ => Err(AppError::new_validation_error(
            
            ErrorDetails {
                parameter: "fragment".to_string(),
                reason: "Invalid value, should be an object.".to_string(),
            },
        )),
    }
}

fn parse_in(raw: &serde_json::Value) -> Result<RequestFilter, AppError> {
    match raw {
        serde_json::Value::Object(in_filter) => {
            let properties = in_filter
                .get("properties")
                .ok_or(AppError::new_validation_error(
                    
                    ErrorDetails {
                        parameter: "properties".to_string(),
                        reason: "Missing field.".to_string(),
                    },
                ))?;
            let values = in_filter.get("values").ok_or(AppError::new_validation_error(
                
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
                                fields.into_iter().last().map_or(Err(AppError::new_validation_error(
                                    
                                    ErrorDetails {
                                        parameter: "properties".to_string(),
                                        reason: format!(
                                            "Property has invalid parameters value type,  should be an object.",
                                        ),
                                    },
                                )), |field| {
                                    let query_key = field.0.to_lowercase().try_into()?;
                                    match query_key {
                                        QueryKey::FRAGMENT => {
                                            parse_fragment_key(&field.1.to_owned())
                                        }
                                        QueryKey::KEY => Ok("key".to_string()),
                                        QueryKey::VALUE => Ok("value".to_string()),
                                        QueryKey::ADDRESS => Ok("address".to_string()),
                                        _ => Err(AppError::new_validation_error(
                                            
                                            ErrorDetails {
                                                parameter: "properties".to_string(),
                                                reason: format!(
                                                    "{:?} is invalid parameters key.",
                                                    query_key
                                                ),
                                            },
                                        )),
                                    }
                                })
                        },
                        _ => Err(AppError::new_validation_error(
                            
                            ErrorDetails {
                                parameter: "properties".to_string(),
                                reason: format!(
                                    "Property has invalid parameters value type,  should be an object.",
                                ),
                            },
                        )),
                    })
                    .collect()
                }
                _ => Err(AppError::new_validation_error(
                    
                    ErrorDetails {
                        parameter: "properties".to_string(),
                        reason: "Invalid value, should be an array.".to_string(),
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
                                                    INVALID_FRAGMENT_VALUE_TYPE_ERROR.to_owned(),
                                                )?;
                                                v_as_u64
                                                    .try_into()
                                                    .or(Err(INVALID_FRAGMENT_VALUE_TYPE_ERROR
                                                        .to_owned()))
                                            })
                                            .collect::<Result<Vec<u8>, AppError>>()
                                        {
                                            Ok(bytes) => vs.push(Ok(ValueType::BinaryVal(bytes))),
                                            Err(err) => vs.push(Err(err)),
                                        }
                                    }
                                    _ => vs.push(Err(AppError::new_validation_error(
                                        
                                        ErrorDetails {
                                            parameter: "values".to_string(),
                                            reason: "Invalid value, should be an array."
                                                .to_string(),
                                        },
                                    ))),
                                });
                                // returns Result<Vec<ValueType>>
                                vs.into_iter().collect()
                            }
                            _ => Err(AppError::new_validation_error(
                                
                                ErrorDetails {
                                    parameter: "values".to_string(),
                                    reason: "Invalid value, should be an array.".to_string(),
                                },
                            )),
                        };
                        result.push(res);
                    });
                    result.into_iter().collect()
                }
                _ => Err(AppError::new_validation_error(
                    
                    ErrorDetails {
                        parameter: "values".to_string(),
                        reason: "Invalid value, should be an array.".to_string(),
                    },
                )),
            }?;
            Ok(RequestFilter::In(InFilter {
                properties: fields,
                values: in_values,
            }))
        }
        _ => Err(AppError::new_validation_error(
            
            ErrorDetails {
                parameter: "in".to_string(),
                reason: "Invalid value, should be an object.".to_string(),
            },
        )),
    }
}

fn parse_string_part(param: &str, part: &str, raw: &serde_json::Value) -> Result<String, AppError> {
    match raw {
        serde_json::Value::Object(o) => match o.get(part) {
            Some(val) => Ok(val.as_str().unwrap().to_owned()),
            None => Err(AppError::new_validation_error(
                
                ErrorDetails {
                    parameter: format!("{}.{}", param, part),
                    reason: "Missing value.".to_string(),
                },
            )),
        },
        _ => Err(AppError::new_validation_error(
            
            ErrorDetails {
                parameter: param.to_owned(),
                reason: "Invalid value, should be an object.".to_string(),
            },
        )),
    }
}

// todo TryInto<KeyNode>
fn parse_key(raw: &serde_json::Value) -> Result<RequestFilter, AppError> {
    let key = parse_string_part("key", "value", &raw)?;
    Ok(RequestFilter::Key(KeyFilter { value: key }))
}

fn parse_value(raw: &serde_json::Value) -> Result<RequestFilter, AppError> {
    let value = parse_string_part("value", "value", &raw)?;
    Ok(RequestFilter::Value(ValueFilter { value: value }))
}

fn parse_address(raw: &serde_json::Value) -> Result<RequestFilter, AppError> {
    let addres = parse_string_part("address", "value", &raw)?;
    Ok(RequestFilter::Address(AddressFilter { value: addres }))
}

pub fn parse_filter(raw: &serde_json::Value) -> Result<RequestFilter, AppError> {
    match raw.clone() {
        serde_json::Value::Object(o) => {
            let filter = o.keys().into_iter().fold(Ok(None), |acc, key| {
                let query_key = key.to_owned().to_lowercase().try_into()?;
                match query_key {
                    QueryKey::AND => match raw.get(key).unwrap() {
                        serde_json::Value::Array(values) => {
                            let values: Vec<RequestFilter> = values
                                .into_iter()
                                .map(|value| parse_filter(value))
                                .collect::<Result<Vec<RequestFilter>, AppError>>()?;

                            let values_filter = RequestFilter::And(AndFilter { children: values });

                            acc.map(|acc| {
                                acc.map_or(Some(values_filter.clone()), |filter| match filter {
                                    RequestFilter::And(and) => {
                                        and.clone().children.push(values_filter);
                                        Some(RequestFilter::And(and.to_owned()))
                                    }
                                    RequestFilter::Or(or) => {
                                        or.clone().children.push(values_filter);
                                        Some(RequestFilter::Or(or.to_owned()))
                                    }
                                    _ => None,
                                })
                            })
                        }
                        _ => acc,
                    },
                    QueryKey::OR => match raw.get(key).unwrap() {
                        serde_json::Value::Array(values) => {
                            let values: Vec<RequestFilter> = values
                                .into_iter()
                                .map(|value| parse_filter(value))
                                .collect::<Result<Vec<RequestFilter>, AppError>>()?;

                            let values_filter = RequestFilter::Or(OrFilter {
                                children: values.clone(),
                            });

                            acc.map(|acc| {
                                acc.map_or(Some(values_filter.clone()), |filter| match filter {
                                    RequestFilter::And(and) => {
                                        and.clone().children.push(values_filter);
                                        Some(RequestFilter::And(and.to_owned()))
                                    }
                                    RequestFilter::Or(or) => {
                                        or.clone().children.push(values_filter);
                                        Some(RequestFilter::Or(or.to_owned()))
                                    }
                                    _ => None,
                                })
                            })
                        }
                        _ => acc,
                    },
                    QueryKey::IN => {
                        let in_filter = parse_in(o.get(key).unwrap())?;
                        acc.map(|acc| match acc {
                            Some(filter) => match filter {
                                RequestFilter::And(and) => {
                                    and.clone().children.push(in_filter);
                                    Some(RequestFilter::And(and))
                                }
                                RequestFilter::Or(or) => {
                                    or.clone().children.push(in_filter);
                                    Some(RequestFilter::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(RequestFilter::And(AndFilter {
                                children: vec![in_filter],
                            })),
                        })
                    }
                    QueryKey::FRAGMENT => {
                        let fr = parse_fragment_with_operation(o.get(key).unwrap())?;
                        acc.map(|acc| match acc {
                            Some(filter) => match filter {
                                RequestFilter::And(and) => {
                                    and.clone().children.push(fr);
                                    Some(RequestFilter::And(and))
                                }
                                RequestFilter::Or(or) => {
                                    or.clone().children.push(fr);
                                    Some(RequestFilter::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(RequestFilter::And(AndFilter { children: vec![fr] })),
                        })
                    }
                    QueryKey::KEY => {
                        let key = parse_key(o.get(key).unwrap())?;
                        acc.map(|acc| match acc {
                            Some(filter) => match filter {
                                RequestFilter::And(and) => {
                                    and.clone().children.push(key);
                                    Some(RequestFilter::And(and))
                                }
                                RequestFilter::Or(or) => {
                                    or.clone().children.push(key);
                                    Some(RequestFilter::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(RequestFilter::And(AndFilter {
                                children: vec![key],
                            })),
                        })
                    }
                    QueryKey::VALUE => {
                        let value = parse_value(o.get(key).unwrap())?;
                        acc.map(|acc| match acc {
                            Some(filter) => match filter {
                                RequestFilter::And(and) => {
                                    and.clone().children.push(value);
                                    Some(RequestFilter::And(and))
                                }
                                RequestFilter::Or(or) => {
                                    or.clone().children.push(value);
                                    Some(RequestFilter::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(RequestFilter::And(AndFilter {
                                children: vec![value],
                            })),
                        })
                    }
                    QueryKey::ADDRESS => {
                        let addr = parse_address(o.get(key).unwrap())?;
                        acc.map(|acc| match acc {
                            Some(filter) => match filter {
                                RequestFilter::And(and) => {
                                    and.clone().children.push(addr);
                                    Some(RequestFilter::And(and))
                                }
                                RequestFilter::Or(or) => {
                                    or.clone().children.push(addr);
                                    Some(RequestFilter::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(RequestFilter::And(AndFilter {
                                children: vec![addr],
                            })),
                        })
                    }
                }
            });

            filter.and_then(|op| match op {
                Some(n) => Ok(n),
                None => Err(AppError::new_validation_error(
                    
                    ErrorDetails {
                        parameter: "filter".to_string(),
                        reason: "Filter is empty.".to_string(),
                    },
                )),
            })
        }
        _ => Err(AppError::new_validation_error(
            
            ErrorDetails {
                parameter: "filter".to_string(),
                reason: "Invalid value type, should be an object.".to_string(),
            },
        )),
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

impl TryInto<QuerySortKey> for String {
    type Error = AppError;

    fn try_into(self) -> Result<QuerySortKey, Self::Error> {
        match self.as_str() {
            "fragment" => Ok(QuerySortKey::FRAGMENT),
            "key" => Ok(QuerySortKey::KEY),
            "value" => Ok(QuerySortKey::VALUE),
            "address" => Ok(QuerySortKey::ADDRESS),
            _ => Err(AppError::new_validation_error(
                
                ErrorDetails {
                    parameter: "sort".to_string(),
                    reason: format!("{} is invalid sort key.", self),
                },
            )),
        }
    }
}

impl TryInto<SortDirection> for String {
    type Error = AppError;

    fn try_into(self) -> Result<SortDirection, Self::Error> {
        match self.as_str() {
            "asc" => Ok(SortDirection::Asc),
            "desc" => Ok(SortDirection::Desc),
            _ => Err(AppError::new_validation_error(
                
                ErrorDetails {
                    parameter: "direction".to_string(),
                    reason: format!("{} is invalid sort direction value.", self),
                },
            )),
        }
    }
}

impl TryInto<ValueSortType> for String {
    type Error = AppError;

    fn try_into(self) -> Result<ValueSortType, Self::Error> {
        match self.as_str() {
            "binary" => Ok(ValueSortType::Binary),
            "bool" => Ok(ValueSortType::Bool),
            "integer" => Ok(ValueSortType::Integer),
            "string" => Ok(ValueSortType::String),
            _ => Err(AppError::new_validation_error(
                
                ErrorDetails {
                    parameter: "type".to_string(),
                    reason: format!("{} is invalid value type.", self),
                },
            )),
        }
    }
}

fn parse_sort_direction(raw: &serde_json::Value) -> Result<SortDirection, AppError> {
    match raw {
        serde_json::Value::Object(o) => o
            .get("direction")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
            .try_into(),
        _ => Err(AppError::new_validation_error(
            
            ErrorDetails {
                parameter: "direction".to_string(),
                reason: "Invalid value, should be asc or desc.".to_string(),
            },
        )),
    }
}

fn parse_sort_fragment(raw: &serde_json::Value) -> Result<Sort, AppError> {
    match raw {
        serde_json::Value::Object(fr) => {
            let pos = fr.get("position").unwrap().as_u64().unwrap();
            let fragment_type: FragmentType = fr.get("type").unwrap().to_owned().try_into()?;
            let direction = parse_sort_direction(raw)?;
            Ok(Sort::Fragment(FragmentSort {
                position: pos,
                fragment_type: fragment_type,
                direction: direction,
            }))
        }
        _ => Err(AppError::new_validation_error(
            
            ErrorDetails {
                parameter: "sort.fragment".to_string(),
                reason: "Invalid value, should be an object.".to_string(),
            },
        )),
    }
}

fn parse_sort_key(raw: &serde_json::Value) -> Result<Sort, AppError> {
    let direction = parse_sort_direction(raw)?;
    Ok(Sort::Key(KeySort {
        direction: direction,
    }))
}

fn parse_sort_value(raw: &serde_json::Value) -> Result<Sort, AppError> {
    let value_type = parse_string_part("value", "type", raw)?.try_into()?;
    let direction = parse_sort_direction(raw)?;
    Ok(Sort::Value(ValueSort {
        value_type: value_type,
        direction: direction,
    }))
}

fn parse_sort_address(raw: &serde_json::Value) -> Result<Sort, AppError> {
    let direction = parse_sort_direction(raw)?;
    Ok(Sort::Address(AddressSort {
        direction: direction,
    }))
}

fn parse_sort_item(raw: &serde_json::Value) -> Result<Option<Sort>, AppError> {
    match raw {
        serde_json::Value::Object(o) => o
            .keys()
            .last()
            .map(|key| {
                let query_sort_key = key.to_owned().to_lowercase().try_into()?;
                match query_sort_key {
                    QuerySortKey::FRAGMENT => {
                        let fr = parse_sort_fragment(o.get(key).unwrap())?;
                        Ok(Some(fr))
                    }
                    QuerySortKey::KEY => {
                        let key = parse_sort_key(o.get(key).unwrap())?;
                        Ok(Some(key))
                    }
                    QuerySortKey::VALUE => {
                        let value = parse_sort_value(o.get(key).unwrap())?;
                        Ok(Some(value))
                    }
                    QuerySortKey::ADDRESS => {
                        let addr = parse_sort_address(o.get(key).unwrap())?;
                        Ok(Some(addr))
                    }
                }
            })
            .unwrap_or(Err(AppError::new_validation_error(
                
                ErrorDetails {
                    parameter: "sort".to_string(),
                    reason: "Missing sort key.".to_string(),
                },
            ))),
        _ => Err(AppError::new_validation_error(
            
            ErrorDetails {
                parameter: "sort".to_string(),
                reason: "Invalid value type, should be an object.".to_string(),
            },
        )),
    }
}

pub fn parse_sort(raw: &serde_json::Value) -> Result<RequestSort, AppError> {
    match raw {
        serde_json::Value::Array(vs) => {
            let values = vs
                .iter()
                .map(|value| parse_sort_item(value))
                .collect::<Result<Option<Vec<Sort>>, AppError>>()?;
            values
                .map(|vs| RequestSort { children: vs })
                .ok_or(AppError::new_validation_error(
                    
                    ErrorDetails {
                        parameter: "sort".to_string(),
                        reason: "Cannot extract sort.".to_string(),
                    },
                ))
        }
        _ => Err(AppError::new_validation_error(
            
            ErrorDetails {
                parameter: "sort".to_string(),
                reason: "Invalid value type, should be an object.".to_string(),
            },
        )),
    }
}
