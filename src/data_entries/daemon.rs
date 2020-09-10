use super::{DataEntriesRepo, DataEntriesSource, DeletableDataEntry};
use crate::log::APP_LOG;
use slog::info;
use std::time::Duration;

/*
Algorithm:
1. Get last height from db
2. Request a batch of T block updates from last_height
3. Transform updates into rows for insertion
4. Insert updates
5. Upadte last handled height in db
5. If less than T updates have been received, sleep for some time before continuing
*/

pub async fn start<T: DataEntriesSource + Send + Sync, U: DataEntriesRepo + Send + Sync>(
    updates_src: T,
    mut dbw: U,
    min_height: u32,
    blocks_per_request: usize,
) {
    loop {
        let last_handled_height = dbw.get_last_handled_height().unwrap();

        let from_height = if last_handled_height < min_height {
            min_height
        } else {
            last_handled_height + 1
        };
        let to_height = from_height + (blocks_per_request as u32) - 1;

        info!(
            APP_LOG,
            "updating data entries from {} to {}", from_height, to_height
        );

        let updates = updates_src
            .fetch_updates(from_height, to_height)
            .await
            .unwrap_or_else(|_| (Vec::new(), Vec::new()));

        if updates.0.len() > 0 {
            dbw.insert_entries(&updates.0).unwrap();
        }

        if updates.1.len() > 0 {
            let entries_to_delete: Vec<DeletableDataEntry> = updates
                .1
                .clone()
                .into_iter()
                .map(|dde| DeletableDataEntry {
                    address: dde.address,
                    key: dde.key,
                })
                .collect();
            dbw.delete_entries(&entries_to_delete).unwrap();
        }

        let last_inserted = updates.0.last();
        let last_deleted = updates.1.last();

        let mut last_updated_height = None;
        if let Some(last_inserted) = last_inserted {
            if let Some(last_deleted) = last_deleted {
                if last_deleted.height > last_inserted.height {
                    last_updated_height = Some(last_deleted.height as u32);
                } else {
                    last_updated_height = Some(last_inserted.height as u32);
                }
            } else {
                last_updated_height = Some(last_inserted.height as u32);
            }
        }

        match last_updated_height {
            Some(last_updated_height) => dbw.set_last_handled_height(last_updated_height).unwrap(),
            None => (),
        }

        let entries_inserted = updates.0.len();
        let entries_deleted = updates.1.len();

        info!(APP_LOG, "inserted {} entries", entries_inserted);
        info!(APP_LOG, "deleted {} entries", entries_deleted);

        if entries_inserted + entries_deleted == 0 {
            tokio::time::delay_for(Duration::from_secs(5)).await;
        }
    }
}
