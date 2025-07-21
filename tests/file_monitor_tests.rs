use std::sync::mpsc::channel;
use std::time::Duration;
use std::fs::{File, remove_file};
use std::thread::sleep;

use endpoint_protection_agent::file_monitor::monitor_directories;
use tempfile::tempdir;

#[test]
fn test_monitor_detects_file_creation() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let dir_path = temp_dir.path().to_str().unwrap().to_string();

    let (tx, rx) = channel();

    let _watcher = monitor_directories(&[dir_path.clone()], tx)
        .expect("Failed to start monitor");

    // Give time for watcher to initialize
    sleep(Duration::from_secs(1));

    // Trigger a file event
    let file_path = temp_dir.path().join("test_file.txt");
    let _ = File::create(&file_path).expect("Failed to create test file");

    // Wait and check for event
    let event = rx.recv_timeout(Duration::from_secs(3));
    assert!(
        event.is_ok(),
        "Did not receive file system event on file creation"
    );

    // Clean up
    remove_file(file_path).ok();
}
