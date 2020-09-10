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
use log::APP_LOG;
use tokio::try_join;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    let config = config::load()?;

    let service_pg_pool = db::pool(&config.postgres)?;

    let data_entries_repo = DataEntriesRepoImpl::new(service_pg_pool.clone());

    let web_join_handle = {
        let port = config.port.clone();
        let repo = data_entries_repo.clone();
        tokio::spawn(async move {
            api::start(port, repo).await;
        })
    };

    if let Err(err) = try_join!(web_join_handle) {
        panic!(err);
    }

    Ok(())
}
