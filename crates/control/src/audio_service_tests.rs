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
