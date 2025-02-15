use super::parsing::{
    AddressFilter, AndFilter, FragmentType, FragmentValueType, InFilter, InFilterValue,
    InItemFilter, KeyFilter, KeyFragmentFilter, MgetEntries, Operation, OrFilter, RequestFilter,
    RequestSort, SortItem, SortItemDirection, ValueData, ValueFilter, ValueFragmentFilter,
    ValueType,
};
use crate::data_entries::{SqlSort, SqlWhere};
use crate::text_utils::pg_escape;
use base64::encode;
use md5::compute as md5;

impl From<InFilterValue> for SqlWhere {
    fn from(v: InFilterValue) -> Self {
        match v {
            InFilterValue::BinaryVal(b) => format!("'{}'", encode(b)),
            InFilterValue::BoolVal(b) => format!("{}", b.to_owned()),
            InFilterValue::IntVal(n) => format!("{}", n),
            InFilterValue::StringVal(s) => format!("'{}'", s.to_owned()),
        }
    }
}

impl From<FragmentValueType> for SqlWhere {
    fn from(v: FragmentValueType) -> Self {
        match v {
            FragmentValueType::IntVal(n) => format!("{}", n),
            FragmentValueType::StringVal(s) => format!("'{}'", s.to_owned()),
        }
    }
}
impl From<FragmentType> for SqlWhere {
    fn from(v: FragmentType) -> Self {
        match v {
            FragmentType::Integer => "integer".into(),
            FragmentType::String => "string".into(),
        }
    }
}

impl From<Operation> for SqlWhere {
    fn from(v: Operation) -> Self {
        match v {
            Operation::Eq => "=".into(),
            Operation::Gt => ">".into(),
            Operation::Gte => ">=".into(),
            Operation::Lt => "<".into(),
            Operation::Lte => "<=".into(),
        }
    }
}

impl From<RequestFilter> for SqlWhere {
    fn from(v: RequestFilter) -> Self {
        match v {
            RequestFilter::And(n) => n.into(),
            RequestFilter::Or(n) => n.into(),
            RequestFilter::In(n) => n.into(),
            RequestFilter::Fragment(n) => n.into(),
            RequestFilter::ValueFragment(n) => n.into(),
            RequestFilter::Key(n) => n.into(),
            RequestFilter::Value(n) => n.into(),
            RequestFilter::Address(n) => n.into(),
        }
    }
}

impl From<AndFilter> for SqlWhere {
    fn from(v: AndFilter) -> Self {
        if v.0.len() > 0 {
            format!(
                "({})",
                v.0.iter()
                    .map(|n| n.to_owned().into())
                    .collect::<Vec<String>>()
                    .join(" AND ")
            )
        } else {
            "1=1".to_string()
        }
    }
}

impl From<OrFilter> for SqlWhere {
    fn from(v: OrFilter) -> Self {
        if v.0.len() > 0 {
            format!(
                "({})",
                v.0.iter()
                    .map(|n| n.to_owned().into())
                    .collect::<Vec<String>>()
                    .join(" OR ")
            )
        } else {
            "1=1".to_string()
        }
    }
}

impl From<InItemFilter> for SqlWhere {
    fn from(v: InItemFilter) -> Self {
        match v {
            InItemFilter::Fragment {
                position,
                fragment_type,
            } => format!("fragment_{}_{}", position, SqlWhere::from(fragment_type)),
            InItemFilter::Key {} => "key".into(),
            InItemFilter::Value {
                value_type: ValueType::Binary,
            } => "value_binary".into(),
            InItemFilter::Value {
                value_type: ValueType::Bool,
            } => "value_bool".into(),
            InItemFilter::Value {
                value_type: ValueType::Integer,
            } => "value_integer".into(),
            InItemFilter::Value {
                value_type: ValueType::String,
            } => "value_string".into(),
            InItemFilter::Address {} => "address".into(),
        }
    }
}

impl From<InFilter> for SqlWhere {
    fn from(v: InFilter) -> Self {
        let values: Vec<String> = v
            .values
            .clone()
            .into_iter()
            .map(|rows| {
                rows.into_iter()
                    .map(|vt| {
                        let v: String = vt.into();
                        pg_escape(v.trim_matches('\'')).into()
                    })
                    .collect::<Vec<String>>()
                    .join("','")
            })
            .map(|row| format!("('{}')", row))
            .collect();

        if v.properties.len() > 0 && values.len() > 0 {
            format!(
                "(({}) IN ({}))",
                v.properties
                    .iter()
                    .map(|p| {
                        let v = SqlWhere::from(p.to_owned());
                        pg_escape(&v.as_str()).into()
                    })
                    .collect::<Vec<SqlWhere>>()
                    .join(","),
                values.join(",")
            )
        } else {
            "1=1".to_string()
        }
    }
}

impl From<KeyFragmentFilter> for SqlWhere {
    fn from(v: KeyFragmentFilter) -> Self {
        format!(
            "fragment_{}_{} {} {}",
            v.position,
            SqlWhere::from(v.fragment_type),
            SqlWhere::from(v.operation),
            SqlWhere::from(v.value)
        )
    }
}

impl From<ValueFragmentFilter> for SqlWhere {
    fn from(v: ValueFragmentFilter) -> Self {
        format!(
            "value_fragment_{}_{} {} {}",
            v.position,
            SqlWhere::from(v.fragment_type),
            SqlWhere::from(v.operation),
            SqlWhere::from(v.value)
        )
    }
}

impl From<KeyFilter> for SqlWhere {
    fn from(v: KeyFilter) -> Self {
        format!("key = '{}'", pg_escape(v.value.as_str()))
    }
}

impl From<ValueFilter> for SqlWhere {
    fn from(v: ValueFilter) -> Self {
        match v {
            ValueFilter {
                value: ValueData::Binary(v),
                ..
            } => {
                let v = encode(v);
                format!(
                    "value_binary = '{}' AND md5(value_binary) = md5('{}')",
                    v, v
                )
            }
            ValueFilter {
                value: ValueData::String(v),
                ..
            } => format!(
                "value_string = '{}' AND md5(value_string) = '{:x}'",
                pg_escape(&v.as_str()),
                md5(&v.as_str())
            ),
            ValueFilter {
                value: ValueData::Bool(v),
                ..
            } => format!("value_bool = {} AND value_bool IS NOT NULL", v),
            ValueFilter {
                operation,
                value: ValueData::Integer(v),
                ..
            } => format!("value_integer {} {}", SqlWhere::from(operation), v),
        }
    }
}

impl From<AddressFilter> for SqlWhere {
    fn from(v: AddressFilter) -> Self {
        format!("address = '{}'", pg_escape(&v.value.as_str()))
    }
}

impl From<RequestSort> for SqlSort {
    fn from(v: RequestSort) -> SqlSort {
        v.0.clone()
            .into_iter()
            .map(|sort_item| sort_item.into())
            .collect::<Vec<String>>()
            .join(",")
    }
}

impl From<SortItem> for SqlSort {
    fn from(v: SortItem) -> Self {
        match v {
            SortItem::Fragment {
                position,
                fragment_type,
                direction,
            } => format!(
                "fragment_{}_{} {}",
                position,
                SqlSort::from(fragment_type),
                SqlSort::from(direction)
            ),
            SortItem::Key { direction } => format!("key {}", SqlSort::from(direction)),
            SortItem::Value { direction } => format!("value {}", SqlSort::from(direction)),
            SortItem::Address { direction } => format!("address {}", SqlSort::from(direction)),
            SortItem::Base { direction } => format!("uid {}", SqlSort::from(direction)),
            SortItem::ValueFragment {
                position,
                fragment_type,
                direction,
            } => format!(
                "value_fragment_{}_{} {}",
                position,
                SqlSort::from(fragment_type),
                SqlSort::from(direction)
            ),
        }
    }
}

impl From<SortItemDirection> for SqlSort {
    fn from(v: SortItemDirection) -> SqlSort {
        match v {
            SortItemDirection::Asc => "ASC".into(),
            SortItemDirection::Desc => "DESC".into(),
        }
    }
}

impl From<MgetEntries> for SqlWhere {
    fn from(v: MgetEntries) -> SqlWhere {
        format!(
            "(address, key) IN ({})",
            v.address_key_pairs
                .into_iter()
                .map(|entry| format!(
                    "('{}', '{}')",
                    pg_escape(entry.address.as_str()),
                    pg_escape(entry.key.as_str())
                ))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}
