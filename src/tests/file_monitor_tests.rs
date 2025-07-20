use crate::file_monitor::{FileMonitor, FileSystemEvent};
use std::path::{PathBuf};
use tempfile::{tempdir};
use tokio::time::{sleep, Duration, Instant, timeout}; // Import Instant and timeout
use tokio::sync::mpsc;
use test_log::test; // Import the test_log macro

// Helper function to create a temporary directory and file for tests
async fn setup_test_env() -> (tempfile::TempDir, PathBuf, mpsc::Receiver<FileSystemEvent>) {
    let temp_dir = tempdir().expect("Failed to create temporary directory");
    let mut watch_path = temp_dir.path().to_path_buf();

    // Canonicalize the watch_path immediately to handle OS-specific path variations (e.g., /var vs /private/var on macOS)
    if let Ok(canonical_path) = watch_path.canonicalize() {
        watch_path = canonical_path;
    }
    log::debug!("Watching canonicalized path: {}", watch_path.display());

    let (mut file_monitor, mut rx) = FileMonitor::new(vec![watch_path.clone()])
        .expect("Failed to create FileMonitor");
    file_monitor.start().expect("Failed to start FileMonitor");

    // Give the watcher a moment to initialize and settle, and explicitly drain the directory creation event.
    // This is crucial on OSes like macOS where directory creation triggers an event.
    let dir_created_expected_path = watch_path.clone();
    let dir_created_predicate = |event: &FileSystemEvent| {
        if let FileSystemEvent::Created(p) = event {
            p.canonicalize().ok() == dir_created_expected_path.canonicalize().ok()
        } else {
            false
        }
    };

    let mut dir_event_drained = false;
    let drain_timeout_total = Duration::from_secs(10); // Give it a generous 10 seconds to drain the initial dir creation event
    let drain_start_time = Instant::now();

    while Instant::now().duration_since(drain_start_time) < drain_timeout_total {
        match timeout(Duration::from_millis(500), rx.recv()).await { // Wait for up to 500ms for an event
            Ok(Some(event)) => {
                log::debug!("Drained initial event during setup: {:?}", event);
                if dir_created_predicate(&event) {
                    dir_event_drained = true;
                    log::debug!("Successfully drained expected 'Created' event for directory: {}", dir_created_expected_path.display());
                    break; // Found and drained the specific directory event, can proceed to clear others
                }
            },
            Ok(None) => {
                // Channel closed, no more events (shouldn't happen during setup)
                log::debug!("Channel closed during initial drain.");
                break;
            },
            Err(_) => {
                // Timeout occurred, meaning no events arrived within the 500ms sub-timeout
                log::debug!("No more events in channel for 500ms. Draining complete for this attempt.");
                // If we didn't get the dir creation event yet, continue looping
                if !dir_event_drained {
                    continue;
                } else {
                    break; // If dir event drained, and no more events, we're done.
                }
            }
        }
    }

    if !dir_event_drained {
        log::warn!("Did not receive expected 'Created' event for directory: {} within {:?}. This might cause test flakiness.", dir_created_expected_path.display(), drain_timeout_total);
    }

    // Drain any other lingering events that might have occurred during initial setup
    while rx.try_recv().is_ok() {
        // Continue draining
    }
    tokio::time::sleep(Duration::from_millis(100)).await; // Final small sleep to ensure channel settles

    log::debug!("Setup complete, channel should be clear for test events.");
    (temp_dir, watch_path, rx)
}

// Helper to receive events for a duration and filter for a specific type
async fn receive_events_and_check(
    rx: &mut mpsc::Receiver<FileSystemEvent>,
    expected_event_predicate: impl Fn(&FileSystemEvent) -> bool,
    timeout_secs: u64,
) -> bool {
    let start_time = Instant::now();
    let timeout_duration = Duration::from_secs(timeout_secs);
    let mut collected_events = Vec::new();

    while Instant::now().duration_since(start_time) < timeout_duration {
        tokio::select! {
            event_option = rx.recv() => {
                if let Some(event) = event_option {
                    log::info!("Received event during check: {:?}", event);

                    // Access the Event inside Other to suppress compiler warning
                    if let FileSystemEvent::Other(ref e) = event {
                        log::debug!("Other EventKind: {:?}", e.kind);
                    }

                    collected_events.push(event);
                } else {
                    break; // Channel closed
                }
            }
            _ = sleep(Duration::from_millis(200)) => {
                // Prevent busy-waiting
            }
        }
    }

    collected_events.iter().any(expected_event_predicate)
}



#[test(tokio::test)] // Using test_log::test
async fn test_file_monitor_creation() {
    let paths = vec![PathBuf::from("/tmp/test_dir")]; // This path doesn't need to exist for this test
    let (_file_monitor, mut rx) = FileMonitor::new(paths.clone())
        .expect("FileMonitor creation should succeed");

    // Ensure the receiver is alive and can receive
    assert!(rx.try_recv().is_err()); // Should be empty initially
}

#[test(tokio::test)] // Using test_log::test
async fn test_file_created_event() {
    let (_temp_dir, watch_path, mut rx) = setup_test_env().await;

    let file_path = watch_path.join("test_file_created.txt");
    std::fs::write(&file_path, "hello").expect("Failed to create file");

    // Give the file system some time to register the change and for the debouncer to process
    tokio::time::sleep(Duration::from_secs(3)).await; // Ensure debouncer has time

    let expected_predicate = |event: &FileSystemEvent| {
        if let FileSystemEvent::Created(p) = event {
            p.canonicalize().ok() == file_path.canonicalize().ok()
        } else {
            false
        }
    };

    let received = receive_events_and_check(&mut rx, expected_predicate, 10).await; // Increased timeout for receiving
    assert!(received, "Did not receive expected FileSystemEvent::Created for {}", file_path.display());
}

#[test(tokio::test)] // Using test_log::test
async fn test_file_modified_event() {
    let (_temp_dir, watch_path, mut rx) = setup_test_env().await;

    let file_path = watch_path.join("test_file_modified.txt");
    std::fs::write(&file_path, "initial").expect("Failed to create file");
    tokio::time::sleep(Duration::from_secs(3)).await; // Ensure initial create debounces

    // Clear any initial create/other events that might have been sent
    while rx.try_recv().is_ok() {}

    std::fs::write(&file_path, "modified content").expect("Failed to modify file");
    tokio::time::sleep(Duration::from_secs(3)).await; // Ensure debouncer has time

    let expected_predicate = |event: &FileSystemEvent| {
        if let FileSystemEvent::Modified(p) = event {
            p.canonicalize().ok() == file_path.canonicalize().ok()
        } else {
            false
        }
    };

    let received = receive_events_and_check(&mut rx, expected_predicate, 10).await; // Increased timeout for receiving
    assert!(received, "Did not receive expected FileSystemEvent::Modified for {}", file_path.display());
}

#[test(tokio::test)] // Using test_log::test
async fn test_file_deleted_event() {
    let (_temp_dir, watch_path, mut rx) = setup_test_env().await;

    let file_path = watch_path.join("test_file_deleted.txt");
    std::fs::write(&file_path, "to_delete").expect("Failed to create file");
    tokio::time::sleep(Duration::from_secs(3)).await; // Ensure initial create debounces

    // Clear any initial create/other events that might have been sent
    while rx.try_recv().is_ok() {}

    std::fs::remove_file(&file_path).expect("Failed to delete file");
    tokio::time::sleep(Duration::from_secs(3)).await; // Ensure debouncer has time

    let expected_predicate = |event: &FileSystemEvent| {
        if let FileSystemEvent::Deleted(p) = event {
            p.canonicalize().ok() == file_path.canonicalize().ok()
        } else {
            false
        }
    };

    let received = receive_events_and_check(&mut rx, expected_predicate, 10).await; // Increased timeout for receiving
    assert!(received, "Did not receive expected FileSystemEvent::Deleted for {}", file_path.display());
}