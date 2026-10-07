//! Engine soak: a long multi-tool route through `RealtimeMixerFanout`, the
//! path native outputs use, for a fixed wall-clock duration. No WASAPI, no
//! private audio; it runs on any platform.
//!
//! ```text
//! cargo run --release -p audiorouter-engine --example soak
//! AUDIOROUTER_SOAK_SECONDS=1200 cargo run --release -p audiorouter-engine --example soak
//! ```
//!
//! The route is a heavier W1 (spec 14): a voice chain of nine tools that
//! fans out to a voice sink and a headphone Mixer, and a desktop source with
//! two tools that feeds the same Mixer and a recording sink. Quanta are
//! processed back to back (not paced to real time), so a 20-minute run
//! covers several hours of audio.
//!
//! It prints a Markdown report on stdout (progress goes to stderr) and exits
//! with status 1 when a check fails:
//!
//! - **Heap growth after warm-up** (NFR-06 allows <10 MiB over 8 hours).
//!   Live heap bytes are counted by this binary's global allocator; the
//!   default limit is 1 MiB (`AUDIOROUTER_SOAK_MAX_HEAP_GROWTH_KIB`).
//! - **Allocations while processing after warm-up** (AGENTS.md realtime
//!   rule): must be 0.
//! - **p99 time per quantum** (NFR-04 asks p99.9 < 50 % of the quantum on the
//!   reference PC). Shared CI runners are noisy, so the gate is p99 against
//!   `AUDIOROUTER_SOAK_P99_PERCENT` (default 50) of the 2.67 ms quantum;
//!   p99.9 and the maximum are reported only.
//!
//! The mean time per quantum is also shown as a share of one core at real
//! time and of the whole machine, for comparison with NFR-05 (W1 backend
//! CPU ≤10 % of the machine on average). That is the engine alone, without
//! WASAPI, the recorder or the control plane, so it is a lower bound.
//! `AUDIOROUTER_SOAK_METRICS=<file>` also writes the key numbers as
//! `key=value` lines. Resident memory is reported where the OS exposes it
//! cheaply (Linux); the quality workflow samples the Windows private bytes
//! and working set from outside the process.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{
    compile_native_paths_with_plugins_and_audio, AudioBlock, AudioBlockRing, RealtimeMixerFanout,
    RuntimeGeneration,
};

const FRAMES: usize = 128;
const SAMPLE_RATE: f64 = 48_000.0;
/// Histogram resolution and range: 1 µs buckets up to 20 ms.
const BUCKETS: usize = 20_000;
/// Report rows (one per interval), fixed before the run starts.
const INTERVALS: usize = 10;
const KIB: i64 = 1024;

/// Live heap bytes of the whole process.
static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);

thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

// SAFETY: every call forwards to `System` with the caller's layout and
// pointer unchanged; the counters are plain atomics and thread-locals that
// never allocate (`try_with` tolerates thread-local teardown).
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation(layout.size() as i64);
        // SAFETY: same contract as `GlobalAlloc::alloc`, upheld by our caller.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation(layout.size() as i64);
        // SAFETY: same contract as `GlobalAlloc::alloc_zeroed`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size() as i64, Ordering::Relaxed);
        // SAFETY: `ptr` was returned by `System` for this `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_allocation(new_size as i64 - layout.size() as i64);
        // SAFETY: `ptr` was returned by `System` for this `layout`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

fn count_allocation(delta_bytes: i64) {
    LIVE_BYTES.fetch_add(delta_bytes, Ordering::Relaxed);
    let _ = TRACKING.try_with(|tracking| {
        if tracking.get() {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        }
    });
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn env_number(name: &str, default: f64) -> f64 {
    match std::env::var(name) {
        Ok(value) => value
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite() && *number > 0.0)
            .unwrap_or_else(|| panic!("{name} must be a positive number, got {value:?}")),
        Err(_) => default,
    }
}

fn port(name: &str, direction: PortDirection) -> Port {
    Port {
        name: name.into(),
        direction,
        channels: 2,
    }
}

fn node(id: &str, kind: NodeKind) -> Node {
    let ports = match kind {
        NodeKind::PhysicalInput => vec![port("out", PortDirection::Output)],
        NodeKind::PhysicalOutput => vec![port("in", PortDirection::Input)],
        _ => vec![
            port("in", PortDirection::Input),
            port("out", PortDirection::Output),
        ],
    };
    Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports,
    }
}

fn edge(source: &str, destination: &str) -> Edge {
    Edge {
        id: EntityId::new(format!("{source}->{destination}")),
        source_node: EntityId::new(source),
        source_port: "out".into(),
        destination_node: EntityId::new(destination),
        destination_port: "in".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    }
}

const VOICE_CHAIN: &[(&str, NodeKind)] = &[
    ("v-dehum", NodeKind::Dehum),
    ("v-declick", NodeKind::Declick),
    ("v-denoise", NodeKind::Denoise),
    ("v-eq", NodeKind::ParametricEq),
    ("v-gate", NodeKind::Gate),
    ("v-compressor", NodeKind::Compressor),
    ("v-bass-treble", NodeKind::BassTreble),
    ("v-limiter", NodeKind::Limiter),
    ("v-meter", NodeKind::Meter),
];
const DESKTOP_CHAIN: &[(&str, NodeKind)] =
    &[("d-gain", NodeKind::Gain), ("d-meter", NodeKind::Meter)];

/// mic → voice chain → voice sink, and → headphone Mixer (monitor);
/// desktop → desktop chain → headphone Mixer, and → recording sink.
fn soak_session() -> Session {
    let mut nodes = vec![
        node("mic", NodeKind::PhysicalInput),
        node("desktop", NodeKind::PhysicalInput),
        node("mix-headphones", NodeKind::Mixer),
        node("out-voice", NodeKind::PhysicalOutput),
        node("out-headphones", NodeKind::PhysicalOutput),
        node("out-recording", NodeKind::PhysicalOutput),
    ];
    let mut edges = Vec::new();
    for (chain, source) in [(VOICE_CHAIN, "mic"), (DESKTOP_CHAIN, "desktop")] {
        let mut previous = source;
        for &(id, kind) in chain {
            nodes.push(node(id, kind));
            edges.push(edge(previous, id));
            previous = id;
        }
    }
    edges.extend([
        edge("v-meter", "out-voice"),
        edge("v-meter", "mix-headphones"),
        edge("d-meter", "mix-headphones"),
        edge("d-meter", "out-recording"),
        edge("mix-headphones", "out-headphones"),
    ]);
    Session {
        id: EntityId::new("engine-soak"),
        name: "Engine soak".into(),
        schema_version: 1,
        revision: 1,
        nodes,
        edges,
    }
}

/// A voice-like tone with harmonics and a little noise; other sources get a
/// different fundamental.
fn source_block(source: &str, quantum: usize) -> AudioBlock {
    let fundamental = if source == "mic" { 140.0 } else { 330.0 };
    let mut block = AudioBlock::new(2, FRAMES).unwrap();
    let mut seed = 0x2545_f491_u32.wrapping_add(quantum as u32);
    for channel in 0..2 {
        for (frame, sample) in block.channel_mut(channel).unwrap().iter_mut().enumerate() {
            let t = (quantum * FRAMES + frame) as f64 / SAMPLE_RATE;
            let tone = (1..=5)
                .map(|harmonic| {
                    (std::f64::consts::TAU * fundamental * harmonic as f64 * t).sin()
                        / harmonic as f64
                })
                .sum::<f64>();
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let noise = (seed as f64 / u32::MAX as f64 - 0.5) * 0.02;
            *sample = (0.15 * tone + noise) as f32;
        }
    }
    block
}

/// Per-quantum processing times in 1 µs buckets (the last one is overflow).
struct Histogram {
    buckets: Vec<u64>,
    count: u64,
    total: Duration,
    max: Duration,
}

impl Histogram {
    fn new() -> Self {
        Self {
            buckets: vec![0; BUCKETS],
            count: 0,
            total: Duration::ZERO,
            max: Duration::ZERO,
        }
    }

    fn clear(&mut self) {
        self.buckets.fill(0);
        self.count = 0;
        self.total = Duration::ZERO;
        self.max = Duration::ZERO;
    }

    fn record(&mut self, elapsed: Duration) {
        let bucket = (elapsed.as_micros() as usize).min(BUCKETS - 1);
        self.buckets[bucket] += 1;
        self.count += 1;
        self.total += elapsed;
        self.max = self.max.max(elapsed);
    }

    fn merge(&mut self, other: &Self) {
        for (mine, theirs) in self.buckets.iter_mut().zip(&other.buckets) {
            *mine += theirs;
        }
        self.count += other.count;
        self.total += other.total;
        self.max = self.max.max(other.max);
    }

    fn mean_us(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total.as_secs_f64() * 1e6 / self.count as f64
        }
    }

    /// Upper edge of the bucket holding the `quantile` sample, in µs.
    fn percentile_us(&self, quantile: f64) -> f64 {
        let rank = ((self.count as f64 * quantile).ceil() as u64).max(1);
        let mut seen = 0;
        for (bucket, &count) in self.buckets.iter().enumerate() {
            seen += count;
            if seen >= rank {
                return (bucket + 1) as f64;
            }
        }
        BUCKETS as f64
    }
}

#[derive(Clone, Copy)]
struct Row {
    elapsed: Duration,
    quanta: u64,
    mean_us: f64,
    p99_us: f64,
    max_us: f64,
    live_heap: i64,
    rss: Option<u64>,
}

/// Resident set size in bytes, read without allocating (Linux only).
fn resident_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        use std::io::Read;
        let mut buffer = [0u8; 128];
        let mut file = std::fs::File::open("/proc/self/statm").ok()?;
        let length = file.read(&mut buffer).ok()?;
        let text = std::str::from_utf8(&buffer[..length]).ok()?;
        let pages: u64 = text.split_whitespace().nth(1)?.parse().ok()?;
        Some(pages * 4096)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn mib(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0))
}

fn main() {
    let seconds = env_number("AUDIOROUTER_SOAK_SECONDS", 10.0);
    let max_growth_kib = env_number("AUDIOROUTER_SOAK_MAX_HEAP_GROWTH_KIB", 1024.0);
    let p99_percent = env_number("AUDIOROUTER_SOAK_P99_PERCENT", 50.0);
    if cfg!(debug_assertions) {
        eprintln!("note: unoptimized build; use --release for real timings");
    }
    let duration = Duration::from_secs_f64(seconds);
    let warm_up = (duration / 10)
        .max(Duration::from_secs(1))
        .min(duration / 2);
    let interval = (duration - warm_up) / INTERVALS as u32;
    let quantum = Duration::from_secs_f64(FRAMES as f64 / SAMPLE_RATE);

    let set = compile_native_paths_with_plugins_and_audio(
        &soak_session(),
        RuntimeGeneration::new(1),
        &Default::default(),
        &Default::default(),
    )
    .unwrap_or_else(|error| panic!("soak route does not compile: {error:?}"));
    let path_count = set.paths().len();
    let inputs = set
        .input_node_ids()
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect::<Vec<_>>();
    let channels = vec![2; inputs.len()];
    let mut fanout = RealtimeMixerFanout::from_paths(set, 4, &channels, FRAMES)
        .unwrap_or_else(|error| panic!("soak route does not prepare: {error:?}"));
    let rings = (0..fanout.branch_count())
        .map(|_| AudioBlockRing::new(4, 2, FRAMES).unwrap())
        .collect::<Vec<_>>();
    let destinations = rings.iter().collect::<Vec<_>>();
    let blocks = (0..64)
        .map(|quantum| {
            inputs
                .iter()
                .map(|id| source_block(id, quantum))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let generation = fanout.generation();

    // Everything the loop touches is allocated above or here.
    let mut window = Histogram::new();
    let mut overall = Histogram::new();
    let mut rows: Vec<Row> = Vec::with_capacity(INTERVALS + 2);
    let mut quanta: u64 = 0;
    let mut delivered: u64 = 0;
    let mut baseline_heap = None;
    let mut next_row = warm_up;

    eprintln!(
        "soak: {} s ({} s warm-up), {path_count} paths, {} inputs, {} outputs",
        duration.as_secs_f64(),
        warm_up.as_secs_f64(),
        inputs.len(),
        rings.len()
    );
    let started = Instant::now();
    loop {
        let warm = baseline_heap.is_some();
        let set_tracking = |on: bool| TRACKING.with(|tracking| tracking.set(on));
        if warm {
            set_tracking(true);
        }
        let begin = Instant::now();
        for (input, block) in blocks[quanta as usize % blocks.len()].iter().enumerate() {
            let accepted = fanout
                .try_submit_input(input, generation, block)
                .expect("submit input");
            assert!(accepted, "input ring {input} full");
        }
        delivered += fanout.process_once(&destinations).expect("process") as u64;
        for ring in &rings {
            while let Some(block) = ring.try_receive() {
                let _ = ring.try_recycle(block);
            }
        }
        let elapsed = begin.elapsed();
        if warm {
            set_tracking(false);
        }
        quanta += 1;
        let now = started.elapsed();
        if !warm {
            if now >= warm_up {
                baseline_heap = Some(LIVE_BYTES.load(Ordering::Relaxed));
                ALLOCATIONS.with(|count| count.set(0));
                rows.push(Row {
                    elapsed: now,
                    quanta,
                    mean_us: 0.0,
                    p99_us: 0.0,
                    max_us: 0.0,
                    live_heap: LIVE_BYTES.load(Ordering::Relaxed),
                    rss: resident_bytes(),
                });
            }
            continue;
        }
        window.record(elapsed);
        if now >= next_row + interval || now >= duration {
            overall.merge(&window);
            rows.push(Row {
                elapsed: now,
                quanta,
                mean_us: window.mean_us(),
                p99_us: window.percentile_us(0.99),
                max_us: window.max.as_secs_f64() * 1e6,
                live_heap: LIVE_BYTES.load(Ordering::Relaxed),
                rss: resident_bytes(),
            });
            window.clear();
            next_row += interval;
            eprintln!("soak: {:.0} s, {quanta} quanta", now.as_secs_f64());
            if now >= duration {
                break;
            }
        }
    }
    let wall = started.elapsed();
    let allocations_after_warm_up = ALLOCATIONS.with(|count| count.get());

    let baseline_heap = baseline_heap.unwrap_or(0);
    let final_heap = LIVE_BYTES.load(Ordering::Relaxed);
    let growth = final_heap - baseline_heap;
    let quantum_us = quantum.as_secs_f64() * 1e6;
    let p99 = overall.percentile_us(0.99);
    let p999 = overall.percentile_us(0.999);
    let mean = overall.mean_us();
    let cores = std::thread::available_parallelism().map_or(1, |cores| cores.get());
    let core_share = 100.0 * mean / quantum_us;
    let audio_seconds = quanta as f64 * quantum.as_secs_f64();
    let expected_deliveries = quanta * rings.len() as u64;

    println!("## Engine soak");
    println!();
    println!(
        "Route: two sources, {} tools, one Mixer, three outputs ({path_count} compiled path(s)), \
         {FRAMES} frames at {} Hz. Wall time {:.0} s ({:.0} s warm-up); {quanta} quanta \
         = {:.2} h of audio ({:.0}x real time). {cores} logical CPUs.",
        VOICE_CHAIN.len() + DESKTOP_CHAIN.len(),
        SAMPLE_RATE,
        wall.as_secs_f64(),
        warm_up.as_secs_f64(),
        audio_seconds / 3600.0,
        audio_seconds / wall.as_secs_f64(),
    );
    println!();
    println!("| Elapsed (s) | Quanta | Mean µs | p99 µs | Max µs | Live heap (KiB) | RSS (MiB) |");
    println!("| ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
    for (index, row) in rows.iter().enumerate() {
        // The first row is the baseline taken at the end of the warm-up.
        let timing = if index == 0 {
            "warm-up end | – | –".to_owned()
        } else {
            format!("{:.1} | {:.0} | {:.0}", row.mean_us, row.p99_us, row.max_us)
        };
        println!(
            "| {:.1} | {} | {timing} | {} | {} |",
            row.elapsed.as_secs_f64(),
            row.quanta,
            row.live_heap / KIB,
            row.rss.map_or_else(|| "n/a".to_owned(), mib),
        );
    }
    println!();

    let growth_limit = (max_growth_kib * KIB as f64) as i64;
    let p99_limit = quantum_us * p99_percent / 100.0;
    let checks = [
        (
            "Heap growth after warm-up (NFR-06)",
            format!("< {max_growth_kib:.0} KiB"),
            format!("{:+.1} KiB", growth as f64 / KIB as f64),
            growth < growth_limit,
        ),
        (
            "Allocations while processing after warm-up",
            "0".to_owned(),
            allocations_after_warm_up.to_string(),
            allocations_after_warm_up == 0,
        ),
        (
            "p99 time per quantum (NFR-04 shape)",
            format!("≤ {p99_limit:.0} µs ({p99_percent:.0} % of {quantum_us:.0} µs)"),
            format!("{p99:.0} µs"),
            p99 <= p99_limit,
        ),
        (
            "Every quantum delivered to every output",
            expected_deliveries.to_string(),
            delivered.to_string(),
            delivered == expected_deliveries,
        ),
    ];
    let info = [
        (
            "p99.9 / max time per quantum (NFR-04: p99.9 < 50 %)",
            format!("{p999:.0} µs / {:.0} µs", overall.max.as_secs_f64() * 1e6),
        ),
        (
            "Mean time per quantum, share of one core at real time",
            format!("{mean:.1} µs, {core_share:.1} %"),
        ),
        (
            "Same, share of the whole machine (NFR-05: ≤10 % average)",
            format!("{:.2} %", core_share / cores as f64),
        ),
    ];
    println!("| Check | Limit | Measured | Result |");
    println!("| --- | --- | --- | --- |");
    for (name, limit, measured, passed) in &checks {
        let result = if *passed { "pass" } else { "**FAIL**" };
        println!("| {name} | {limit} | {measured} | {result} |");
    }
    for (name, measured) in &info {
        println!("| {name} | – | {measured} | info |");
    }
    // Optional key=value file for the workflow's trend history.
    if let Ok(path) = std::env::var("AUDIOROUTER_SOAK_METRICS") {
        let metrics = format!(
            "soak_p99_us={p99:.0}\nsoak_p999_us={p999:.0}\nsoak_mean_us={mean:.1}\n\
             soak_heap_growth_kib={:.1}\nsoak_quanta={quanta}\n",
            growth as f64 / KIB as f64
        );
        std::fs::write(&path, metrics).unwrap_or_else(|error| panic!("{path}: {error}"));
    }
    if checks.iter().any(|(_, _, _, passed)| !passed) {
        std::process::exit(1);
    }
}
