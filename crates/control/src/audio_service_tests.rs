//! Tests for `audio_service.rs`.

use super::*;

#[test]
fn storage_work_uses_an_independent_connection_off_the_control_thread() {
    let storage = Storage::open_memory().unwrap();
    storage.save_privacy_mute(true).unwrap();
    let mut plane = ControlPlane::with_storage("storage-worker", storage);
    let control_thread = std::thread::current().id();
    plane
        .with_storage_worker(|storage| {
            assert_ne!(std::thread::current().id(), control_thread);
            assert!(storage.load_privacy_mute().map_err(storage_error)?);
            storage.save_privacy_mute(false).map_err(storage_error)
        })
        .unwrap();
    assert!(!plane.storage.as_ref().unwrap().load_privacy_mute().unwrap());
    assert!(plane
        .with_storage_worker::<()>(|_| Err(ControlError::InvalidRequest("fixture".into())))
        .is_err());
    assert!(plane.storage.is_some());
}

#[test]
fn storage_worker_keeps_the_original_connection_available_during_service() {
    let storage = Storage::open_memory().unwrap();
    storage.save_privacy_mute(true).unwrap();
    let target = storage.worker_target();
    let mut passes = 0;
    run_while_servicing(
        || {
            let worker = target.open().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(40));
            assert!(worker.load_privacy_mute().unwrap());
        },
        || {
            storage.load_privacy_mute().unwrap();
            passes += 1;
        },
    );
    assert!(passes >= 5, "only {passes} service passes");
    assert!(storage.load_privacy_mute().unwrap());
}

#[test]
fn long_handler_work_keeps_audio_service_passes_running() {
    let mut passes = 0_u32;
    let value = run_while_servicing(
        || {
            std::thread::sleep(std::time::Duration::from_millis(40));
            7
        },
        || passes += 1,
    );
    assert_eq!(value, 7);
    // 40 ms of work at a 1 ms cadence; a loaded machine still manages a few.
    assert!(passes >= 5, "only {passes} service passes during the work");
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_while_servicing(|| -> u8 { panic!("decoder failed") }, || ())
    }));
    assert!(panicked.is_err());
}

#[test]
fn audio_health_is_absent_while_nothing_plays() {
    assert_eq!(ControlPlane::default().audio_health(), None);
}

#[test]
fn pump_error_categories_keep_variant_names_and_drop_private_text() {
    let category = audio_pump_error_category(&ControlError::InvalidRequest(
        "native input/output pump failed: Feeder(Audio(WindowsOperation { context: \"Microphone (PD200X)\", error: 0x88890004 }))".into(),
    ));
    assert_eq!(category, "Feeder(Audio(WindowsOperation ");
    assert!(!category.contains("PD200X"));
    let category = audio_pump_error_category(&ControlError::InvalidRequest(
        "native multi-input worker is not running".into(),
    ));
    assert_eq!(category, "native multi-input worker is not running");
    assert!(audio_pump_error_category(&ControlError::InvalidRequest("x".repeat(500))).len() <= 120);
}
