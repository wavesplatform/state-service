use crate::api::parsing::{
    AddressNode, AddressSortNode, AndNode, FragmentNode, FragmentSortNode, InNode, KeyNode,
    KeySortNode, Node, OrNode, Sort, SortDirection, SortNode, ValueNode, ValueSortNode,
    ValueSortType, ValueType,
};
use base64::encode;
use std::fmt;

pub trait ToSqlWhereString {
    fn to_sql_where_string(&self) -> String;
}

pub trait ToSqlSortString {
    fn to_sql_sort_string(&self) -> String;
}

impl ToSqlWhereString for Node {
    fn to_sql_where_string(&self) -> String {
        match self {
            Node::And(n) => n.to_sql_where_string(),
            Node::Or(n) => n.to_sql_where_string(),
            Node::In(n) => n.to_sql_where_string(),
            Node::Fragment(n) => n.to_sql_where_string(),
            Node::Key(n) => n.to_sql_where_string(),
            Node::Value(n) => n.to_sql_where_string(),
            Node::Address(n) => n.to_sql_where_string(),
        }
    }
}

impl ToSqlWhereString for AndNode {
    fn to_sql_where_string(&self) -> String {
        if self.children.len() > 0 {
            format!(
                "({})",
                self.children
                    .iter()
                    .map(|n| n.to_sql_where_string())
                    .collect::<Vec<String>>()
                    .join(" AND ")
            )
        } else {
            "1=1".to_string()
        }
    }
}

impl ToSqlWhereString for OrNode {
    fn to_sql_where_string(&self) -> String {
        if self.children.len() > 0 {
            format!(
                "({})",
                self.children
                    .iter()
                    .map(|n| n.to_sql_where_string())
                    .collect::<Vec<String>>()
                    .join(" OR ")
            )
        } else {
            "1=1".to_string()
        }
    }
}

impl ToSqlWhereString for InNode {
    fn to_sql_where_string(&self) -> String {
        let values: Vec<String> = self
            .values
            .clone()
            .into_iter()
            .map(|rows| {
                rows.into_iter()
                    .fold("".to_string(), |acc, value| match value {
                        ValueType::IntVal(n) => {
                            if acc.len() > 0 {
                                format!("{}, {}", acc, n)
                            } else {
                                format!("{}", n)
                            }
                        }
                        ValueType::StringVal(s) => {
                            if acc.len() > 0 {
                                format!("{}, '{}'", acc, s.to_owned())
                            } else {
                                format!("'{}'", s.to_owned())
                            }
                        }
                        ValueType::BoolVal(b) => {
                            if acc.len() > 0 {
                                format!("{}, {}", acc, b.to_owned())
                            } else {
                                format!("{}", b.to_owned())
                            }
                        }
                        ValueType::BinaryVal(b) => {
                            if acc.len() > 0 {
                                format!("{}, '{}'", acc, encode(b))
                            } else {
                                format!("'{}'", encode(b))
                            }
                        }
                    })
            })
            .map(|row| format!("({})", row))
            .collect();

        if self.properties.len() > 0 && values.len() > 0 {
            format!(
                "(({}) IN ({}))",
                self.properties.join(","),
                values.join(",")
            )
        } else {
            "1=1".to_string()
        }
    }
}

impl ToSqlWhereString for FragmentNode {
    fn to_sql_where_string(&self) -> String {
        format!(
            "fragment_{}_{} {} {}",
            self.position,
            self.fragment_type,
            self.operation,
            self.value.to_string()
        )
    }
}

impl ToSqlWhereString for KeyNode {
    fn to_sql_where_string(&self) -> String {
        format!("key = '{}'", self.value)
    }
}

impl ToSqlWhereString for ValueNode {
    fn to_sql_where_string(&self) -> String {
        format!("value = '{}'", self.value)
    }
}

impl ToSqlWhereString for AddressNode {
    fn to_sql_where_string(&self) -> String {
        format!("address = '{}'", self.value)
    }
}

impl ToSqlSortString for Sort {
    fn to_sql_sort_string(&self) -> String {
        self.children
            .clone()
            .into_iter()
            .map(|sort_item| match sort_item {
                SortNode::Fragment(f) => f.to_sql_sort_string(),
                SortNode::Key(f) => f.to_sql_sort_string(),
                SortNode::Value(f) => f.to_sql_sort_string(),
                SortNode::Address(f) => f.to_sql_sort_string(),
            })
            .collect::<Vec<String>>()
            .join(",")
    }
}

impl fmt::Display for SortDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortDirection::Asc => write!(f, "ASC"),
            SortDirection::Desc => write!(f, "DESC"),
        }
    }
}

impl fmt::Display for ValueSortType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueSortType::Binary => write!(f, "binary"),
            ValueSortType::Bool => write!(f, "bool"),
            ValueSortType::Integer => write!(f, "integer"),
            ValueSortType::String => write!(f, "string"),
        }
    }
}

impl ToSqlSortString for FragmentSortNode {
    fn to_sql_sort_string(&self) -> String {
        format!(
            "fragment_{}_{} {}",
            self.position, self.fragment_type, self.direction
        )
    }
}

impl ToSqlSortString for KeySortNode {
    fn to_sql_sort_string(&self) -> String {
        format!("key {}", self.direction)
    }
}

impl ToSqlSortString for ValueSortNode {
    fn to_sql_sort_string(&self) -> String {
        format!("value_{} {}", self.value_type, self.direction)
    }
}

impl ToSqlSortString for AddressSortNode {
    fn to_sql_sort_string(&self) -> String {
        format!("address {}", self.direction)
    }
}
