use endpoint_protection_agent::file_monitor::monitor_directories;
use tempfile::tempdir;
use std::fs::File;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

#[tokio::test]
async fn test_monitor_detects_file_creation_async() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let dir_path = temp_dir.path().to_str().unwrap().to_string();

    let (tx, mut rx) = mpsc::channel(10);
    let _watcher = monitor_directories(&[dir_path.clone()], tx)
        .expect("Failed to start monitor");

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Trigger event
    let file_path = temp_dir.path().join("test_async.txt");
    File::create(&file_path).expect("Failed to create file");

    // Wait with timeout
    let received = timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(received.is_ok() && received.unwrap().is_some(), "Did not receive event");
}