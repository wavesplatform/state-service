use super::parsing::{
    AddressFilter, AndFilter, FragmentFilter, FragmentOperation, FragmentType, FragmentValueType,
    InFilter, InItemFilter, KeyFilter, OrFilter, RequestFilter, RequestSort, SortItem,
    SortItemDirection, ValueFilter, ValueFragmentFilter, ValueType,
};
use crate::data_entries::{SqlSort, SqlWhere};
use base64::encode;

impl From<ValueType> for SqlWhere {
    fn from(v: ValueType) -> Self {
        match v {
            ValueType::BinaryVal(b) => format!("'{}'", encode(b)),
            ValueType::BoolVal(b) => format!("{}", b.to_owned()),
            ValueType::IntVal(n) => format!("{}", n),
            ValueType::StringVal(s) => format!("'{}'", s.to_owned()),
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

impl From<FragmentOperation> for SqlWhere {
    fn from(v: FragmentOperation) -> Self {
        match v {
            FragmentOperation::Eq => "=".into(),
            FragmentOperation::Gt => ">".into(),
            FragmentOperation::Gte => ">=".into(),
            FragmentOperation::Lt => "<".into(),
            FragmentOperation::Lte => "<=".into(),
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
            InItemFilter::Value {} => "value".into(),
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
                    .map(|vt| vt.into())
                    .collect::<Vec<String>>()
                    .join(",")
            })
            .map(|row| format!("({})", row))
            .collect();

        if v.properties.len() > 0 && values.len() > 0 {
            format!(
                "(({}) IN ({}))",
                v.properties
                    .iter()
                    .map(|p| SqlWhere::from(p.to_owned()))
                    .collect::<Vec<SqlWhere>>()
                    .join(","),
                values.join(",")
            )
        } else {
            "1=1".to_string()
        }
    }
}

impl From<FragmentFilter> for SqlWhere {
    fn from(v: FragmentFilter) -> Self {
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
        format!("key = '{}'", v.value)
    }
}

impl From<ValueFilter> for SqlWhere {
    fn from(v: ValueFilter) -> Self {
        match v {
            ValueFilter::Binary(v) => format!("value_binary = '{}'", encode(v)),
            ValueFilter::String(v) => format!("value_string = '{}'", v),
            ValueFilter::Bool(v) => format!("value_bool = {}", v),
            ValueFilter::Integer(v) => format!("value_integer = {}", v),
        }
    }
}

impl From<AddressFilter> for SqlWhere {
    fn from(v: AddressFilter) -> Self {
        format!("address = '{}'", v.value)
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
