use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

fn failure() -> ControlError {
    ControlError::Audio {
        code: "other",
        operation: None,
        hresult: 0x8000_4005,
        retryable: true,
        remediation: "",
        message: "enumeration failed".into(),
        resource_ids: Vec::new(),
    }
}

fn wait_until(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn the_list_is_built_on_the_inventory_thread_and_answered_at_once() {
    let threads = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::new(AtomicU64::new(0));
    // The second enumeration blocks until released, like a slow PC.
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let enumerate: Enumerate = {
        let (threads, calls, release) = (threads.clone(), calls.clone(), release.clone());
        Arc::new(move || {
            threads
                .lock()
                .unwrap()
                .push(std::thread::current().name().map(str::to_owned));
            let call = calls.fetch_add(1, Ordering::SeqCst) + 1;
            if call == 2 {
                let (released, wake) = &*release;
                let guard = released.lock().unwrap();
                drop(wake.wait_while(guard, |released| !*released).unwrap());
            }
            Ok(json!([call]))
        })
    };
    let inventory = AppInventory::spawn(enumerate).unwrap();
    assert_eq!(inventory.list().unwrap(), json!([1]));
    // Within the refresh limit, reads never enumerate again.
    for _ in 0..20 {
        assert_eq!(inventory.list().unwrap(), json!([1]));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // An aged list asks for a refresh; while it runs, reads still answer
    // at once with the list they have.
    std::thread::sleep(REFRESH_AFTER + Duration::from_millis(50));
    assert_eq!(inventory.list().unwrap(), json!([1]));
    wait_until(|| calls.load(Ordering::SeqCst) == 2);
    let started = Instant::now();
    assert_eq!(inventory.list().unwrap(), json!([1]));
    assert!(started.elapsed() < Duration::from_millis(100));
    *release.0.lock().unwrap() = true;
    release.1.notify_all();
    // The read during the slow refresh found the list still aged and queued
    // one more, so the newer list may already be the third.
    wait_until(|| inventory.list().unwrap() != json!([1]));
    assert!(
        calls.load(Ordering::SeqCst) <= 3,
        "at most one queued refresh"
    );
    assert!(threads
        .lock()
        .unwrap()
        .iter()
        .all(|name| name.as_deref() == Some("audiorouter-apps")));
}

#[test]
fn a_failed_refresh_keeps_the_last_list_until_it_is_stale() {
    let fail = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicU64::new(0));
    let enumerate: Enumerate = {
        let (fail, calls) = (fail.clone(), calls.clone());
        Arc::new(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            if fail.load(Ordering::SeqCst) {
                Err(failure())
            } else {
                Ok(json!(["discord"]))
            }
        })
    };
    let stale_limit = REFRESH_AFTER + Duration::from_millis(400);
    let inventory = AppInventory::spawn_with(enumerate, stale_limit).unwrap();
    assert_eq!(inventory.list().unwrap(), json!(["discord"]));
    fail.store(true, Ordering::SeqCst);
    std::thread::sleep(REFRESH_AFTER + Duration::from_millis(50));
    assert_eq!(inventory.list().unwrap(), json!(["discord"]));
    wait_until(|| calls.load(Ordering::SeqCst) == 2);
    // The refresh failed: the last list is still served while recent...
    assert_eq!(inventory.list().unwrap(), json!(["discord"]));
    // ...and then the failure is reported instead of an old list.
    std::thread::sleep(stale_limit);
    assert_eq!(inventory.list(), Err(failure()));
    // A later success clears it.
    fail.store(false, Ordering::SeqCst);
    wait_until(|| inventory.list().is_ok());
}

#[test]
fn a_first_enumeration_failure_is_reported_without_waiting() {
    let inventory = AppInventory::spawn(Arc::new(|| Err(failure()))).unwrap();
    let started = Instant::now();
    assert_eq!(inventory.list(), Err(failure()));
    assert!(started.elapsed() < FIRST_LIST_WAIT);
}

#[test]
fn dropping_the_inventory_stops_its_thread() {
    let owned = Arc::new(());
    let enumerate: Enumerate = {
        let owned = owned.clone();
        Arc::new(move || {
            let _ = &owned;
            Ok(json!([]))
        })
    };
    let inventory = AppInventory::spawn(enumerate).unwrap();
    inventory.list().unwrap();
    assert_eq!(Arc::strong_count(&owned), 2);
    drop(inventory);
    // The thread has returned and released the enumerator it held.
    assert_eq!(Arc::strong_count(&owned), 1);
}
