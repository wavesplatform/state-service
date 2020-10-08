#[macro_use]
extern crate diesel;

pub mod api;
pub mod config;
pub mod data_entries;
pub mod db;
pub mod error;
pub mod log;
pub mod schema;

use data_entries::repo::DataEntriesRepoImpl;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    let config = config::load()?;

    let service_pg_pool = db::pool(&config.postgres)?;

    let data_entries_repo = DataEntriesRepoImpl::new(service_pg_pool.clone());

    api::start(config.port, data_entries_repo).await;

    Ok(())
}
