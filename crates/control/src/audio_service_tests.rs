//! Tests for `audio_service.rs`.

use super::*;

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
