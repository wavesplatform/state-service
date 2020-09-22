use super::{
    AddressFilter, AddressSort, AndFilter, FragmentFilter, FragmentSort, InFilter, KeyFilter,
    KeySort, OrFilter, RequestFilter, RequestSort, Sort, SortDirection, SqlSort, SqlWhere,
    ValueFilter, ValueSort, ValueSortType, ValueType,
};
use base64::encode;
use std::fmt;

impl ValueType {
    fn to_sql_string(&self) -> String {
        match self {
            ValueType::BinaryVal(b) => format!("'{}'", encode(b)),
            ValueType::BoolVal(b) => format!("{}", b.to_owned()),
            ValueType::IntVal(n) => format!("{}", n),
            ValueType::StringVal(s) => format!("'{}'", s.to_owned()),
        }
    }
}

impl Into<SqlWhere> for RequestFilter {
    fn into(self) -> SqlWhere {
        match self {
            RequestFilter::And(n) => n.into(),
            RequestFilter::Or(n) => n.into(),
            RequestFilter::In(n) => n.into(),
            RequestFilter::Fragment(n) => n.into(),
            RequestFilter::Key(n) => n.into(),
            RequestFilter::Value(n) => n.into(),
            RequestFilter::Address(n) => n.into(),
        }
    }
}

impl Into<SqlWhere> for AndFilter {
    fn into(self) -> SqlWhere {
        if self.children.len() > 0 {
            format!(
                "({})",
                self.children
                    .iter()
                    .map(|n| n.to_owned().into())
                    .collect::<Vec<String>>()
                    .join(" AND ")
            )
        } else {
            "1=1".to_string()
        }
    }
}

impl Into<SqlWhere> for OrFilter {
    fn into(self) -> SqlWhere {
        if self.children.len() > 0 {
            format!(
                "({})",
                self.children
                    .iter()
                    .map(|n| n.to_owned().into())
                    .collect::<Vec<String>>()
                    .join(" OR ")
            )
        } else {
            "1=1".to_string()
        }
    }
}

impl Into<SqlWhere> for InFilter {
    fn into(self) -> SqlWhere {
        let values: Vec<String> = self
            .values
            .clone()
            .into_iter()
            .map(|rows| {
                rows.into_iter()
                    .map(|vt| vt.to_sql_string())
                    .collect::<Vec<String>>()
                    .join(",")
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

impl Into<SqlWhere> for FragmentFilter {
    fn into(self) -> SqlWhere {
        format!(
            "fragment_{}_{} {} {}",
            self.position,
            self.fragment_type,
            self.operation,
            self.value.to_string()
        )
    }
}

impl Into<SqlWhere> for KeyFilter {
    fn into(self) -> SqlWhere {
        format!("key = '{}'", self.value)
    }
}

impl Into<SqlWhere> for ValueFilter {
    fn into(self) -> SqlWhere {
        format!("value = '{}'", self.value)
    }
}

impl Into<SqlWhere> for AddressFilter {
    fn into(self) -> SqlWhere {
        format!("address = '{}'", self.value)
    }
}

impl Into<SqlSort> for RequestSort {
    fn into(self) -> String {
        self.children
            .clone()
            .into_iter()
            .map(|sort_item| match sort_item {
                Sort::Fragment(f) => f.into(),
                Sort::Key(f) => f.into(),
                Sort::Value(f) => f.into(),
                Sort::Address(f) => f.into(),
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

impl Into<SqlSort> for FragmentSort {
    fn into(self) -> SqlSort {
        format!(
            "fragment_{}_{} {}",
            self.position, self.fragment_type, self.direction
        )
    }
}

impl Into<SqlSort> for KeySort {
    fn into(self) -> SqlSort {
        format!("key {}", self.direction)
    }
}

impl Into<SqlSort> for ValueSort {
    fn into(self) -> SqlSort {
        format!("value_{} {}", self.value_type, self.direction)
    }
}

impl Into<SqlSort> for AddressSort {
    fn into(self) -> SqlSort {
        format!("address {}", self.direction)
    }
}
