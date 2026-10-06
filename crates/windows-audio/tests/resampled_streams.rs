//! Live check that the multi-path engine's 48 kHz streams work on an endpoint
//! whose shared mix rate is not 48 kHz (for example a 96 kHz 7.1 virtual
//! device): render silence to it at 48 kHz and loopback-capture it at 48 kHz.
//! Silence only, so nothing is heard. Opt-in:
//! `AUDIOROUTER_LIVE_RESAMPLE_ENDPOINT=<exact render endpoint id>`
//! `cargo test -p audiorouter-windows-audio --test resampled_streams -- --ignored --nocapture`

#![cfg(windows)]

use audiorouter_windows_audio::{
    enumerate_active_endpoints, EndpointDirection, SharedCapture, SharedRender,
};
use std::time::{Duration, Instant};

#[test]
#[ignore = "opens a real non-48 kHz render endpoint (silence only); explicit opt-in"]
fn non_48k_endpoint_renders_and_loopback_captures_at_48k() {
    let endpoint_id = std::env::var("AUDIOROUTER_LIVE_RESAMPLE_ENDPOINT")
        .expect("set AUDIOROUTER_LIVE_RESAMPLE_ENDPOINT to an exact render endpoint id");
    let endpoint = enumerate_active_endpoints()
        .expect("endpoint inventory")
        .into_iter()
        .find(|e| e.id == endpoint_id && e.direction == EndpointDirection::Render)
        .expect("active render endpoint");
    println!(
        "endpoint mix format: {} Hz, {} channel(s), float32 {}",
        endpoint.sample_rate_hz,
        endpoint.channels,
        endpoint.is_ieee_float32()
    );
    assert_ne!(
        endpoint.sample_rate_hz, 48_000,
        "choose an endpoint whose mix rate is not 48 kHz"
    );

    let mut render = SharedRender::open_with_headroom_at_rate(&endpoint_id, 500_000, 48_000)
        .expect("render at 48 kHz");
    let mut capture =
        SharedCapture::open_loopback_at_rate(&endpoint_id, 48_000).expect("loopback at 48 kHz");
    render.submit_silence().expect("prefill silence");
    render.start().expect("start render");
    capture.start().expect("start loopback");

    // Discard the first half second (engine warm-up), then count for two seconds.
    let warmup_until = Instant::now() + Duration::from_millis(500);
    let (mut counted, mut counting_since) = (0u64, None::<Instant>);
    let end = warmup_until + Duration::from_secs(2);
    while Instant::now() < end {
        render.submit_silence().expect("keep rendering silence");
        while let Some(packet) = capture.next_packet().expect("loopback packet") {
            if Instant::now() >= warmup_until {
                counting_since.get_or_insert_with(Instant::now);
                counted += u64::from(packet.frames);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let seconds = counting_since
        .expect("loopback delivered packets")
        .elapsed()
        .as_secs_f64();
    let rate = counted as f64 / seconds;
    println!("delivered {counted} frames in {seconds:.3} s = {rate:.0} frames/s");
    capture.stop().expect("stop loopback");
    render.stop().expect("stop render");
    assert!(
        (rate - 48_000.0).abs() < 48_000.0 * 0.03,
        "expected about 48000 frames/s, got {rate:.0}"
    );

    // The existing constructors keep the endpoint's own rate (the single-path
    // bridge runs its graph at that rate).
    let mut render =
        SharedRender::open_with_headroom(&endpoint_id, 500_000).expect("render at mix rate");
    let mut capture = SharedCapture::open_loopback(&endpoint_id).expect("loopback at mix rate");
    render.submit_silence().expect("prefill silence");
    render.start().expect("start render");
    capture.start().expect("start loopback");
    let warmup_until = Instant::now() + Duration::from_millis(500);
    let (mut counted, mut counting_since) = (0u64, None::<Instant>);
    let end = warmup_until + Duration::from_secs(2);
    while Instant::now() < end {
        render.submit_silence().expect("keep rendering silence");
        while let Some(packet) = capture.next_packet().expect("loopback packet") {
            if Instant::now() >= warmup_until {
                counting_since.get_or_insert_with(Instant::now);
                counted += u64::from(packet.frames);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let native = counted as f64 / counting_since.expect("packets").elapsed().as_secs_f64();
    println!("unconverted loopback: {native:.0} frames/s");
    capture.stop().expect("stop loopback");
    render.stop().expect("stop render");
    let expected = f64::from(endpoint.sample_rate_hz);
    assert!(
        (native - expected).abs() < expected * 0.03,
        "expected about {expected} frames/s, got {native:.0}"
    );
}
