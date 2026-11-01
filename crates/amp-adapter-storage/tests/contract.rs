//! The redb library keeps the `LibraryStoragePort` contract (TASK-187).

#![forbid(unsafe_code)]

use amp_adapter_storage::RedbStorageAdapter;
use amp_test_support::check_library_storage;

#[tokio::test]
async fn the_redb_library_keeps_the_contract() {
    let dir = tempfile::tempdir().unwrap();
    let store = RedbStorageAdapter::new(dir.path().join("library.redb")).unwrap();
    check_library_storage("redb", &store).await;
}
