use super::errors::*;
use base64::encode;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::convert::TryInto;
use std::fmt;

static INVALID_FRAGMENT_VALUE_TYPE_ERROR: Lazy<AppError> = Lazy::new(|| {
    AppError::ValidationError(
        "Validation Error".to_string(),
        950201,
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
            _ => Err(AppError::ValidationError(
                "Validation Error".to_string(),
                950201,
                ErrorDetails {
                    parameter: "filter".to_string(),
                    reason: format!("{} is invalid filter key, should be one of: and, or, in, fragment, key, value, address.", self),
                },
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Node {
    And(AndNode),
    Or(OrNode),
    In(InNode),
    Fragment(FragmentNode),
    Key(KeyNode),
    Value(ValueNode),
    Address(AddressNode),
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
pub struct AndNode {
    pub children: Vec<Node>,
}

#[derive(Clone, Debug)]
pub struct OrNode {
    pub children: Vec<Node>,
}

#[derive(Clone, Debug)]
pub struct InNode {
    pub properties: Vec<String>,
    pub values: Vec<Vec<ValueType>>,
}

#[derive(Clone, Debug)]
pub struct FragmentNode {
    pub position: u64,
    pub fragment_type: FragmentType,
    pub operation: FragmentOperation,
    pub value: ValueType,
}

#[derive(Clone, Debug)]
pub struct KeyNode {
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct ValueNode {
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct AddressNode {
    pub value: String,
}

#[derive(Clone, Debug)]
pub enum FragmentType {
    String,
    Integer,
}

impl TryInto<FragmentType> for serde_json::Value {
    type Error = AppError;

    fn try_into(self) -> Result<FragmentType, Self::Error> {
        match self {
            serde_json::Value::String(op) => match op.as_str() {
                "string" => Ok(FragmentType::String),
                "integer" => Ok(FragmentType::Integer),
                _ => Err(AppError::ValidationError(
                    "Validation Error".to_string(),
                    950201,
                    ErrorDetails {
                        parameter: "type".to_string(),
                        reason: format!("{} is invalid fragment type.", op.as_str()),
                    },
                )),
            },
            _ => Err(AppError::ValidationError(
                "Validation Error".to_string(),
                950201,
                ErrorDetails {
                    parameter: "fragment".to_string(),
                    reason: "Invalid fragment type value, shoud be an object.".to_string(),
                },
            )),
        }
    }
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
                _ => Err(AppError::ValidationError(
                    "Validation Error".to_string(),
                    950201,
                    ErrorDetails {
                        parameter: "operation".to_string(),
                        reason: format!("{} is invalid fragment operation", op.to_string()),
                    },
                )),
            },
            _ => Err(AppError::ValidationError(
                "Validation Error".to_string(),
                950201,
                ErrorDetails {
                    parameter: "operation".to_string(),
                    reason: "Invalid fragment operation value, should be an object.".to_string(),
                },
            )),
        }
    }
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

fn parse_fragment_key(raw: &serde_json::Value) -> Result<String, AppError> {
    match raw.clone() {
        serde_json::Value::Object(fr) => {
            let pos = fr.get("position").unwrap().as_u64().unwrap();
            let fragment_type: FragmentType = fr.get("type").unwrap().to_owned().try_into()?;
            Ok(format!("fragment_{}_{}", pos, fragment_type))
        }
        _ => Err(AppError::ValidationError(
            "Validation Error".to_string(),
            950201,
            ErrorDetails {
                parameter: "fragment".to_string(),
                reason: "Invalid value, should be an object.".to_string(),
            },
        )),
    }
}

fn parse_fragment_with_operation(raw: &serde_json::Value) -> Result<Node, AppError> {
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
                _ => Err(AppError::ValidationError(
                    "Validation Error".to_string(),
                    950201,
                    ErrorDetails {
                        parameter: "fragment.value".to_string(),
                        reason: "Invalid value type, should be one of Array<u8>, boolean, number, String.".to_string(),
                    },
                ))
            }?;
            Ok(Node::Fragment(FragmentNode {
                position: pos,
                fragment_type: fragment_type,
                operation: operation,
                value: value,
            }))
        }
        _ => Err(AppError::ValidationError(
            "Validation Error".to_string(),
            950201,
            ErrorDetails {
                parameter: "fragment".to_string(),
                reason: "Invalid value, should be an object.".to_string(),
            },
        )),
    }
}

fn parse_in(raw: &serde_json::Value) -> Result<Node, AppError> {
    match raw {
        serde_json::Value::Object(in_filter) => {
            let properties = in_filter
                .get("properties")
                .ok_or(AppError::ValidationError(
                    "Validation Error".to_string(),
                    950201,
                    ErrorDetails {
                        parameter: "properties".to_string(),
                        reason: "Missing field.".to_string(),
                    },
                ))?;
            let values = in_filter.get("values").ok_or(AppError::ValidationError(
                "Validation Error".to_string(),
                950201,
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
                                fields.into_iter().last().map_or(Err(AppError::ValidationError(
                                    "ValidationError".to_string(),
                                    950201,
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
                                        _ => Err(AppError::ValidationError(
                                            "ValidationError".to_string(),
                                            950201,
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
                        _ => Err(AppError::ValidationError(
                            "ValidationError".to_string(),
                            950201,
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
                _ => Err(AppError::ValidationError(
                    "Validation Error".to_string(),
                    950201,
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
                                    _ => vs.push(Err(AppError::ValidationError(
                                        "Validation Error".to_string(),
                                        950201,
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
                            _ => Err(AppError::ValidationError(
                                "Validation Error".to_string(),
                                950201,
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
                _ => Err(AppError::ValidationError(
                    "Validation Error".to_string(),
                    950201,
                    ErrorDetails {
                        parameter: "values".to_string(),
                        reason: "Invalid value, should be an array.".to_string(),
                    },
                )),
            }?;
            Ok(Node::In(InNode {
                properties: fields,
                values: in_values,
            }))
        }
        _ => Err(AppError::ValidationError(
            "Validation Error".to_string(),
            950201,
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
            None => Err(AppError::ValidationError(
                "Validation Error".to_string(),
                950201,
                ErrorDetails {
                    parameter: format!("{}.{}", param, part),
                    reason: "Missing value.".to_string(),
                },
            )),
        },
        _ => Err(AppError::ValidationError(
            "Validation Error".to_string(),
            950201,
            ErrorDetails {
                parameter: param.to_owned(),
                reason: "Invalid value, should be an object.".to_string(),
            },
        )),
    }
}

fn parse_key(raw: &serde_json::Value) -> Result<Node, AppError> {
    let key = parse_string_part("key", "value", &raw)?;
    Ok(Node::Key(KeyNode { value: key }))
}

fn parse_value(raw: &serde_json::Value) -> Result<Node, AppError> {
    let value = parse_string_part("value", "value", &raw)?;
    Ok(Node::Value(ValueNode { value: value }))
}

fn parse_address(raw: &serde_json::Value) -> Result<Node, AppError> {
    let addres = parse_string_part("address", "value", &raw)?;
    Ok(Node::Address(AddressNode { value: addres }))
}

pub fn parse_filter(raw: &serde_json::Value) -> Result<Node, AppError> {
    match raw.clone() {
        serde_json::Value::Object(o) => {
            let filter = o.keys().into_iter().fold(Ok(None), |acc, key| {
                let query_key = key.to_owned().to_lowercase().try_into()?;
                match query_key {
                    QueryKey::AND => match raw.get(key).unwrap() {
                        serde_json::Value::Array(values) => {
                            let values: Vec<Node> = values
                                .into_iter()
                                .map(|value| parse_filter(value))
                                .collect::<Result<Vec<Node>, AppError>>()?;

                            let values_node = Node::And(AndNode { children: values });

                            acc.map(|acc| {
                                acc.map_or(Some(values_node.clone()), |node| match node {
                                    Node::And(n) => {
                                        n.clone().children.push(values_node);
                                        Some(Node::And(n.to_owned()))
                                    }
                                    Node::Or(n) => {
                                        n.clone().children.push(values_node);
                                        Some(Node::Or(n.to_owned()))
                                    }
                                    _ => None,
                                })
                            })
                        }
                        _ => acc,
                    },
                    QueryKey::OR => match raw.get(key).unwrap() {
                        serde_json::Value::Array(values) => {
                            let values: Vec<Node> = values
                                .into_iter()
                                .map(|value| parse_filter(value))
                                .collect::<Result<Vec<Node>, AppError>>()?;

                            let values_node = Node::Or(OrNode {
                                children: values.clone(),
                            });

                            acc.map(|acc| {
                                acc.map_or(Some(values_node.clone()), |node| match node {
                                    Node::And(n) => {
                                        n.clone().children.push(values_node);
                                        Some(Node::And(n.to_owned()))
                                    }
                                    Node::Or(n) => {
                                        n.clone().children.push(values_node);
                                        Some(Node::Or(n.to_owned()))
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
                            Some(n) => match n {
                                Node::And(and) => {
                                    and.clone().children.push(in_filter);
                                    Some(Node::And(and))
                                }
                                Node::Or(or) => {
                                    or.clone().children.push(in_filter);
                                    Some(Node::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(Node::And(AndNode {
                                children: vec![in_filter],
                            })),
                        })
                    }
                    QueryKey::FRAGMENT => {
                        let fr = parse_fragment_with_operation(o.get(key).unwrap())?;
                        acc.map(|acc| match acc {
                            Some(n) => match n {
                                Node::And(and) => {
                                    and.clone().children.push(fr);
                                    Some(Node::And(and))
                                }
                                Node::Or(or) => {
                                    or.clone().children.push(fr);
                                    Some(Node::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(Node::And(AndNode { children: vec![fr] })),
                        })
                    }
                    QueryKey::KEY => {
                        let key = parse_key(o.get(key).unwrap())?;
                        acc.map(|acc| match acc {
                            Some(n) => match n {
                                Node::And(and) => {
                                    and.clone().children.push(key);
                                    Some(Node::And(and))
                                }
                                Node::Or(or) => {
                                    or.clone().children.push(key);
                                    Some(Node::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(Node::And(AndNode {
                                children: vec![key],
                            })),
                        })
                    }
                    QueryKey::VALUE => {
                        let value = parse_value(o.get(key).unwrap())?;
                        acc.map(|acc| match acc {
                            Some(n) => match n {
                                Node::And(and) => {
                                    and.clone().children.push(value);
                                    Some(Node::And(and))
                                }
                                Node::Or(or) => {
                                    or.clone().children.push(value);
                                    Some(Node::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(Node::And(AndNode {
                                children: vec![value],
                            })),
                        })
                    }
                    QueryKey::ADDRESS => {
                        let addr = parse_address(o.get(key).unwrap())?;
                        acc.map(|acc| match acc {
                            Some(n) => match n {
                                Node::And(and) => {
                                    and.clone().children.push(addr);
                                    Some(Node::And(and))
                                }
                                Node::Or(or) => {
                                    or.clone().children.push(addr);
                                    Some(Node::Or(or))
                                }
                                _ => None,
                            },
                            None => Some(Node::And(AndNode {
                                children: vec![addr],
                            })),
                        })
                    }
                }
            });

            filter.and_then(|op| match op {
                Some(n) => Ok(n),
                None => Err(AppError::ValidationError(
                    "Validation Error".to_string(),
                    950201,
                    ErrorDetails {
                        parameter: "filter".to_string(),
                        reason: "Filter is empty.".to_string(),
                    },
                )),
            })
        }
        _ => Err(AppError::ValidationError(
            "Validation Error".to_string(),
            950201,
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
            _ => Err(AppError::ValidationError(
                "Validation Error".to_string(),
                950201,
                ErrorDetails {
                    parameter: "sort".to_string(),
                    reason: format!("{} is invalid sort key.", self),
                },
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub enum SortNode {
    Fragment(FragmentSortNode),
    Key(KeySortNode),
    Value(ValueSortNode),
    Address(AddressSortNode),
}

#[derive(Clone, Debug)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl TryInto<SortDirection> for String {
    type Error = AppError;

    fn try_into(self) -> Result<SortDirection, Self::Error> {
        match self.as_str() {
            "asc" => Ok(SortDirection::Asc),
            "desc" => Ok(SortDirection::Desc),
            _ => Err(AppError::ValidationError(
                "Validation Error".to_string(),
                950201,
                ErrorDetails {
                    parameter: "direction".to_string(),
                    reason: format!("{} is invalid sort direction value.", self),
                },
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Sort {
    pub children: Vec<SortNode>,
}

#[derive(Clone, Debug)]
pub struct FragmentSortNode {
    pub position: u64,
    pub fragment_type: FragmentType,
    pub direction: SortDirection,
}

#[derive(Clone, Debug)]
pub struct KeySortNode {
    pub direction: SortDirection,
}

#[derive(Clone, Debug)]
pub enum ValueSortType {
    Binary,
    Bool,
    Integer,
    String,
}

impl TryInto<ValueSortType> for String {
    type Error = AppError;

    fn try_into(self) -> Result<ValueSortType, Self::Error> {
        match self.as_str() {
            "binary" => Ok(ValueSortType::Binary),
            "bool" => Ok(ValueSortType::Bool),
            "integer" => Ok(ValueSortType::Integer),
            "string" => Ok(ValueSortType::String),
            _ => Err(AppError::ValidationError(
                "Validation Error".to_string(),
                950201,
                ErrorDetails {
                    parameter: "type".to_string(),
                    reason: format!("{} is invalid value type.", self),
                },
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ValueSortNode {
    pub value_type: ValueSortType,
    pub direction: SortDirection,
}

#[derive(Clone, Debug)]
pub struct AddressSortNode {
    pub direction: SortDirection,
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
        _ => Err(AppError::ValidationError(
            "Validation Error".to_string(),
            950201,
            ErrorDetails {
                parameter: "direction".to_string(),
                reason: "Invalid value, should be asc or desc.".to_string(),
            },
        )),
    }
}

fn parse_sort_fragment(raw: &serde_json::Value) -> Result<SortNode, AppError> {
    match raw {
        serde_json::Value::Object(fr) => {
            let pos = fr.get("position").unwrap().as_u64().unwrap();
            let fragment_type: FragmentType = fr.get("type").unwrap().to_owned().try_into()?;
            let direction = parse_sort_direction(raw)?;
            Ok(SortNode::Fragment(FragmentSortNode {
                position: pos,
                fragment_type: fragment_type,
                direction: direction,
            }))
        }
        _ => Err(AppError::ValidationError(
            "Validation Error".to_string(),
            950201,
            ErrorDetails {
                parameter: "sort.fragment".to_string(),
                reason: "Invalid value, should be an object.".to_string(),
            },
        )),
    }
}

fn parse_sort_key(raw: &serde_json::Value) -> Result<SortNode, AppError> {
    let direction = parse_sort_direction(raw)?;
    Ok(SortNode::Key(KeySortNode {
        direction: direction,
    }))
}

fn parse_sort_value(raw: &serde_json::Value) -> Result<SortNode, AppError> {
    let value_type = parse_string_part("value", "type", raw)?.try_into()?;
    let direction = parse_sort_direction(raw)?;
    Ok(SortNode::Value(ValueSortNode {
        value_type: value_type,
        direction: direction,
    }))
}

fn parse_sort_address(raw: &serde_json::Value) -> Result<SortNode, AppError> {
    let direction = parse_sort_direction(raw)?;
    Ok(SortNode::Address(AddressSortNode {
        direction: direction,
    }))
}

fn parse_sort_item(raw: &serde_json::Value) -> Result<Option<SortNode>, AppError> {
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
            .unwrap_or(Err(AppError::ValidationError(
                "Validation Error".to_string(),
                950201,
                ErrorDetails {
                    parameter: "sort".to_string(),
                    reason: "Missing sort key.".to_string(),
                },
            ))),
        _ => Err(AppError::ValidationError(
            "Validation Error".to_string(),
            950201,
            ErrorDetails {
                parameter: "sort".to_string(),
                reason: "Invalid value type, should be an object.".to_string(),
            },
        )),
    }
}

pub fn parse_sort(raw: &serde_json::Value) -> Result<Sort, AppError> {
    match raw {
        serde_json::Value::Array(vs) => {
            let values: Option<Vec<SortNode>> =
                vs.iter()
                    .map(|value| parse_sort_item(value))
                    .collect::<Result<Option<Vec<SortNode>>, AppError>>()?;
            values
                .map(|vs| Sort { children: vs })
                .ok_or(AppError::ValidationError(
                    "Validation Error".to_string(),
                    950201,
                    ErrorDetails {
                        parameter: "sort".to_string(),
                        reason: "Cannot extract sort.".to_string(),
                    },
                ))
        }
        _ => Err(AppError::ValidationError(
            "Validation Error".to_string(),
            950201,
            ErrorDetails {
                parameter: "sort".to_string(),
                reason: "Invalid value type, should be an object.".to_string(),
            },
        )),
    }
}
