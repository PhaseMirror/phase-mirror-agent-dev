use phase_mirror_gpt::archivum::ArchivumLedger;
use tempfile::tempdir;

#[tokio::test]
async fn test_wal_graceful_shutdown() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("shutdown_test.log");
    let mut ledger = ArchivumLedger::new();
    ledger.init_persistence(log_path.clone()).await.unwrap();

    for i in 0..5 {
        let _ = ledger.commit_event("test_event", format!("id-{}", i), b"payload");
    }

    {
        let l = ledger;
        drop(l);
    }

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut new_ledger = ArchivumLedger::new();
    new_ledger.init_persistence(log_path).await.unwrap();

    assert_eq!(new_ledger.get_entry_count(), 5);
    assert!(!new_ledger.entries[4].prev_hash.is_empty());
}

#[tokio::test]
async fn test_wal_empty_recovery() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("empty_recovery.log");
    let mut ledger = ArchivumLedger::new();
    ledger.init_persistence(log_path.clone()).await.unwrap();
    drop(ledger);

    let mut new_ledger = ArchivumLedger::new();
    new_ledger.init_persistence(log_path).await.unwrap();
    assert_eq!(new_ledger.get_entry_count(), 0);
}
