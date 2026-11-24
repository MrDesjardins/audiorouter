//! Deterministic, bounded fuzzing helpers shared by the Q4.3 parser, import and
//! compiler tests (`docs/plans/active/application-quality.md`).
//!
//! Included by each fuzz test with
//! `#[path = "../../../tests/support/fuzz.rs"] mod fuzz;`, so there is one copy
//! and no new crate. Everything is seeded: a failure prints the base seed and
//! the case index, and `case_rng(seed, index)` rebuilds that exact case.
//!
//! Scale: `AUDIOROUTER_FUZZ_SCALE=<n>` multiplies every iteration count (for a
//! longer local or nightly run). The default is 1, which keeps each target
//! well under 5 seconds in a debug build.
#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// SplitMix64: tiny, fast, and good enough to drive mutations.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `0..bound` (`bound == 0` gives 0).
    pub fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next_u64() % bound as u64) as usize
        }
    }

    /// Uniform in `low..=high`.
    pub fn range(&mut self, low: usize, high: usize) -> usize {
        low + self.below(high.saturating_sub(low) + 1)
    }

    /// True once in `n` on average.
    pub fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    pub fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.next_u64() as u8).collect()
    }

    /// Random bytes of a random length in `low..=high`.
    pub fn blob(&mut self, low: usize, high: usize) -> Vec<u8> {
        let length = self.range(low, high);
        self.bytes(length)
    }
}

/// The generator for one case of a run, rebuildable from `(seed, index)`.
pub fn case_rng(seed: u64, index: usize) -> Rng {
    let mut mix = Rng::new(seed ^ (index as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93));
    Rng::new(mix.next_u64())
}

/// `iterations`, times `AUDIOROUTER_FUZZ_SCALE` when that is set.
pub fn iterations(base: usize) -> usize {
    let scale = std::env::var("AUDIOROUTER_FUZZ_SCALE")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|scale| (1..=1_000).contains(scale))
        .unwrap_or(1);
    base.saturating_mul(scale)
}

/// Reports which case was running if the closure panics, so the failure can be
/// replayed with `case_rng(seed, index)`. No panic hook is installed and no
/// panic is caught: the test fails normally.
struct CaseReport<'a> {
    target: &'a str,
    seed: u64,
    index: usize,
}

impl Drop for CaseReport<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!(
                "fuzz target `{}` failed: seed {:#x}, case {} (replay with case_rng)",
                self.target, self.seed, self.index
            );
        }
    }
}

/// Runs `cases` deterministic cases.
pub fn run(target: &str, seed: u64, cases: usize, mut case: impl FnMut(usize, &mut Rng)) {
    for index in 0..iterations(cases) {
        let _report = CaseReport {
            target,
            seed,
            index,
        };
        case(index, &mut case_rng(seed, index));
    }
}

/// Lower-case hex of at most the first 96 bytes, for failure messages.
pub fn hex(bytes: &[u8]) -> String {
    let mut text = String::new();
    for byte in bytes.iter().take(96) {
        text.push_str(&format!("{byte:02x}"));
    }
    if bytes.len() > 96 {
        text.push_str(&format!("...({} bytes)", bytes.len()));
    }
    text
}

const INTERESTING_U32: [u32; 14] = [
    0,
    1,
    2,
    3,
    0x7F,
    0x80,
    0xFF,
    0xFFFF,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
    0x0100_0000,
    0x00FF_FFFF,
];

/// Overwrites a width-1/2/4 integer at a random offset with an edge value (zero,
/// maximum, off-by-one around the data's own length), little or big endian:
/// the length-field and count-field corruption that parsers trip over.
pub fn corrupt_integer(rng: &mut Rng, data: &mut [u8]) {
    if data.is_empty() {
        return;
    }
    let width = *rng.pick(&[1_usize, 2, 4, 4]);
    let width = width.min(data.len());
    let offset = rng.below(data.len() - width + 1);
    let mut value = *rng.pick(&INTERESTING_U32);
    if rng.one_in(3) {
        let length = data.len() as u32;
        value = match rng.below(3) {
            0 => length,
            1 => length.wrapping_add(1),
            _ => length.wrapping_sub(1),
        };
    }
    // Keep the low-order `width` bytes in the chosen byte order.
    if rng.one_in(2) {
        data[offset..offset + width].copy_from_slice(&value.to_le_bytes()[..width]);
    } else {
        data[offset..offset + width].copy_from_slice(&value.to_be_bytes()[4 - width..]);
    }
}

/// One random edit: bit flip, byte overwrite, integer corruption, truncation,
/// insertion, deletion, range duplication, or a block of repeated bytes.
pub fn mutate_once(rng: &mut Rng, data: &mut Vec<u8>, max_len: usize) {
    match rng.below(8) {
        0 if !data.is_empty() => {
            let index = rng.below(data.len());
            data[index] ^= 1 << rng.below(8);
        }
        1 if !data.is_empty() => {
            let index = rng.below(data.len());
            data[index] = *rng.pick(&[0x00, 0x01, 0x7F, 0x80, 0xFF, b'[', b'{', b'"']);
        }
        2 => corrupt_integer(rng, data),
        3 if !data.is_empty() => data.truncate(rng.below(data.len())),
        4 => {
            let at = rng.below(data.len() + 1);
            let count = rng.range(1, 16);
            let fill = rng.bytes(count);
            data.splice(at..at, fill);
        }
        5 if data.len() > 1 => {
            let start = rng.below(data.len());
            let end = (start + rng.range(1, 32)).min(data.len());
            data.drain(start..end);
        }
        6 if !data.is_empty() => {
            let start = rng.below(data.len());
            let end = (start + rng.range(1, 64)).min(data.len());
            let copy = data[start..end].to_vec();
            let at = rng.below(data.len() + 1);
            data.splice(at..at, copy);
        }
        7 => {
            let at = rng.below(data.len() + 1);
            let value = *rng.pick(&[0x00_u8, 0xFF, 0x41]);
            let count = rng.range(1, 64);
            data.splice(at..at, std::iter::repeat_n(value, count));
        }
        _ => {}
    }
    data.truncate(max_len);
}

/// One to four stacked edits of `seed`, never longer than `max_len`.
pub fn mutate(rng: &mut Rng, seed: &[u8], max_len: usize) -> Vec<u8> {
    let mut data = seed.to_vec();
    for _ in 0..rng.range(1, 4) {
        mutate_once(rng, &mut data, max_len);
    }
    data
}

// ---------------------------------------------------------------------------
// Allocation accounting
// ---------------------------------------------------------------------------

thread_local! {
    // `const` thread-locals of `Cell<usize>` need no lazy allocation and no
    // destructor, so reading them inside the allocator is sound.
    static LARGEST_REQUEST: Cell<usize> = const { Cell::new(0) };
    static TRACKING: Cell<bool> = const { Cell::new(false) };
}

/// Forwards to the system allocator and records, per thread, the largest single
/// request made while a [`measure_allocation`] call is running. Install it in a
/// fuzz test binary with `#[global_allocator]`. Per-thread, so tests running in
/// parallel do not see each other's allocations.
pub struct TrackingAllocator;

fn note_request(size: usize) {
    // `try_with` fails only while the thread is shutting down; ignore that.
    let _ = TRACKING.try_with(|tracking| {
        if tracking.get() {
            let _ = LARGEST_REQUEST.try_with(|largest| {
                if size > largest.get() {
                    largest.set(size);
                }
            });
        }
    });
}

// SAFETY: every method forwards unchanged arguments to `System`, which upholds
// the `GlobalAlloc` contract; the only additional work reads and writes
// allocation-free thread-local `Cell`s and never allocates or panics.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note_request(layout.size());
        // SAFETY: `layout` is the caller's, forwarded unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note_request(layout.size());
        // SAFETY: `layout` is the caller's, forwarded unchanged.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: `pointer` and `layout` come from a matching `alloc` above.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note_request(new_size);
        // SAFETY: arguments are the caller's, forwarded unchanged.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

/// Runs `work` and returns its result with the largest single allocation
/// request (bytes) the current thread made inside it. Requires
/// [`TrackingAllocator`] to be the test binary's global allocator.
pub fn measure_allocation<R>(work: impl FnOnce() -> R) -> (R, usize) {
    LARGEST_REQUEST.with(|largest| largest.set(0));
    TRACKING.with(|tracking| tracking.set(true));
    let result = work();
    TRACKING.with(|tracking| tracking.set(false));
    let largest = LARGEST_REQUEST.with(Cell::get);
    (result, largest)
}
