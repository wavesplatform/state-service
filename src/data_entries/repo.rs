use super::{
    sql::ToSqlSortString, sql::ToSqlWhereString, DataEntriesRepo, DataEntry, DeletableDataEntry,
    DoubleVarCharTupleType, InsertableDataEntry,
};
use crate::db::PgPool;
use crate::error::Error;
use crate::schema::data_entries;
use crate::schema::last_handled_height;
use crate::schema::last_handled_height::dsl::*;
use crate::APP_LOG;
use diesel::prelude::*;
use diesel::sql_types::Array;
use slog::info;

#[derive(Clone)]
pub struct DataEntriesRepoImpl {
    pool: PgPool,
}

impl DataEntriesRepoImpl {
    pub fn new(pg_pool: PgPool) -> DataEntriesRepoImpl {
        DataEntriesRepoImpl { pool: pg_pool }
    }
}

impl DataEntriesRepo for DataEntriesRepoImpl {
    fn get_last_handled_height(&self) -> Result<u32, Error> {
        Ok(last_handled_height
            .select(last_handled_height::height)
            .first::<i32>(&self.pool.get()?)? as u32)
    }

    fn set_last_handled_height(&mut self, h: u32) -> Result<(), Error> {
        diesel::update(last_handled_height::table)
            .set(last_handled_height::height.eq(h as i32))
            .execute(&self.pool.get()?)?;

        Ok(())
    }

    fn insert_entries(&mut self, entries: &[InsertableDataEntry]) -> Result<(), Error> {
        diesel::insert_into(data_entries::table)
            .values(entries)
            .on_conflict((data_entries::address, data_entries::key))
            .do_nothing()
            .execute(&self.pool.get()?)?;

        Ok(())
    }

    fn delete_entries(&mut self, entries: &[DeletableDataEntry]) -> Result<(), Error> {
        let query = diesel::sql_query("delete from data_entries where (address, key) = ANY($1)")
            .bind::<Array<DoubleVarCharTupleType>, _>(entries);

        query.execute(&self.pool.get()?)?;
        Ok(())
    }

    fn search_data_entries<W: ToSqlWhereString, S: ToSqlSortString>(
        &self,
        query_where: Option<W>,
        query_sort: Option<S>,
        query_limit: u64,
        query_offset: u64,
    ) -> Result<Vec<DataEntry>, Error> {
        let mut query_where_string = query_where.map_or("".to_string(), |query_where| {
            query_where.to_sql_where_string()
        });
        if query_where_string.len() > 0 {
            query_where_string = format!("WHERE {}", query_where_string);
        }
        let mut query_sort_string =
            query_sort.map_or("".to_string(), |query_sort| query_sort.to_sql_sort_string());
        if query_sort_string.len() > 0 {
            query_sort_string = format!("ORDER BY {}", query_sort_string);
        }
        let query = diesel::sql_query(format!(
            "SELECT address, key, height, value_binary, value_bool, value_integer, value_string FROM data_entries {} {} LIMIT {} OFFSET {}",
            query_where_string,
            query_sort_string,
            query_limit,
            query_offset
        ));

        info!(
            APP_LOG,
            "{}",
            diesel::debug_query::<diesel::pg::Pg, _>(&query)
        );

        Ok(query.get_results(&self.pool.get()?)?)
    }
}

// #[cfg(test)]
// pub(crate) mod tests {
//     use super::*;
//     use crate::{
//         data_entries::{BalancesRepo, UsdnBalanceUpdate},
//         db::tests::PG_POOL_LOCAL,
//     };
//     use chrono::{Duration, NaiveDateTime};
//     use once_cell::sync::Lazy;

//     pub static REPO: Lazy<BalancesRepoImpl> =
//         Lazy::new(|| BalancesRepoImpl::new(PG_POOL_LOCAL.clone()));

//     #[test]
//     fn last_handled_height_on_empty_db() {
//         reset_pg();
//         let last_handled_height_on_empty = REPO.get_last_handled_height().unwrap();
//         assert_eq!(last_handled_height_on_empty, 0);
//         reset_pg();
//     }

//     #[test]
//     fn set_and_get_last_handled_height() {
//         reset_pg();
//         REPO.clone().set_last_handled_height(100).unwrap();
//         let h = REPO.get_last_handled_height().unwrap();
//         assert_eq!(h, 100);
//         reset_pg();
//     }

//     #[test]
//     fn insert_updates() {
//         reset_pg();

//         // 1 Jan 2020
//         let time = NaiveDateTime::from_timestamp(1577836800, 0);

//         let updates = vec![
//             UsdnBalanceUpdate {
//                 address: "address1".to_owned(),
//                 timestamp: time,
//                 balance: 995.5,
//                 origin_transaction_id: "tx1".to_owned(),
//                 height: 1,
//             },
//             UsdnBalanceUpdate {
//                 address: "address2".to_owned(),
//                 timestamp: time + Duration::seconds(5),
//                 balance: 201.4,
//                 origin_transaction_id: "tx2".to_owned(),
//                 height: 2,
//             },
//         ];

//         REPO.clone().insert_updates(&updates).unwrap();

//         let h = REPO.get_last_handled_height().unwrap();
//         assert_eq!(h, 2);

//         reset_pg()
//     }

//     #[test]
//     fn joins_addresses_correctly() {
//         let addresses = vec![
//             String::from("qwe"),
//             String::from("asd"),
//             String::from("zxc"),
//         ];
//         let joined = join_addresses(&addresses);
//         assert_eq!(joined, "'qwe','asd','zxc'".to_owned());
//     }

//     fn reset_pg() {
//         diesel::sql_query("truncate usdn_balance_updates restart identity;")
//             .execute(&PG_POOL_LOCAL.get().unwrap())
//             .unwrap();
//     }
// }
