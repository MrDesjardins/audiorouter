//! `probe_pipe` against the production control server loop, not a test server.
//!
//! The probe connects and leaves without a request. The production I/O thread
//! counts that as a failed connection and ends after
//! `MAX_CONSECUTIVE_PIPE_FAILURES` of them in a row, so a probe must not cost
//! the backend a pipe instance. Every pipe here has a unique name.
#![cfg(windows)]

use std::time::{Duration, Instant};

use audiorouter_transport::{probe_pipe, round_trip, PipeProbe, SERVER_PIPE_INSTANCES};

fn describe_frame() -> Vec<u8> {
    audiorouter_protocol::encode_frame(
        &serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "system.describe" }),
    )
    .unwrap()
}

#[test]
fn repeated_probes_leave_every_production_pipe_instance_serving() {
    let name = format!(
        r"\\.\pipe\audiorouter-probe-production-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let server_name = name.clone();
    std::thread::spawn(move || {
        let plane = audiorouter_control::ControlPlane::default();
        let grant = audiorouter_control::ClientGrant::with_scopes([
            audiorouter_domain::PermissionScope::Read,
        ]);
        let _ = audiorouter_transport::serve_control_connections_forever_with_grant(
            &server_name,
            plane,
            grant,
        );
    });
    let started = Instant::now();
    while probe_pipe(&name).map_or(true, |probe| probe == PipeProbe::Missing) {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "server never started"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    // Far more probes than one I/O thread tolerates failures in a row.
    let probes = 20 * audiorouter_transport::SERVER_PIPE_INSTANCES as usize * 3;
    for index in 0..probes {
        let probe = probe_pipe(&name).unwrap();
        assert!(probe.is_owned(), "probe {index}: {probe:?}");
    }
    // Every instance still answers: run one request per instance at once.
    let request = describe_frame();
    let handles: Vec<_> = (0..SERVER_PIPE_INSTANCES)
        .map(|_| {
            let (name, request) = (name.clone(), request.clone());
            std::thread::spawn(move || round_trip(&name, &request).is_ok())
        })
        .collect();
    for handle in handles {
        assert!(handle.join().unwrap(), "a request failed after the probes");
    }
}
