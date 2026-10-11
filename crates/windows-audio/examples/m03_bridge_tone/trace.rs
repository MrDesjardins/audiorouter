//! Opt-in impulse diagnostics. Workers only push into preallocated storage;
//! the control thread writes the CSV after both audio workers have stopped.
use crossbeam_queue::ArrayQueue;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug)]
struct Entry {
    stage: &'static str,
    qpc_100ns: u64,
    sequence: u64,
    first_hit_frame: i64,
    peak: f64,
    depth: usize,
    silence: u64,
    dropped: u64,
}

pub(super) struct Trace {
    frequency: u64,
    entries: ArrayQueue<Entry>,
    missed: AtomicU64,
}

impl Trace {
    #[cfg(windows)]
    pub fn new(seconds: u32, frames: u16) -> Result<Self, String> {
        use windows::Win32::System::Performance::QueryPerformanceFrequency;
        let mut frequency = 0;
        // SAFETY: the API writes a single i64 to a live stack variable.
        unsafe { QueryPerformanceFrequency(&mut frequency) }
            .map_err(|error| format!("trace QPC frequency: {error}"))?;
        if frequency <= 0 {
            return Err("trace QPC frequency is invalid".to_owned());
        }
        Ok(Self {
            frequency: frequency as u64,
            entries: ArrayQueue::new(seconds as usize * 48_000 / usize::from(frames) * 4 + 1024),
            missed: AtomicU64::new(0),
        })
    }

    #[cfg(windows)]
    fn qpc_100ns(&self) -> Option<u64> {
        use windows::Win32::System::Performance::QueryPerformanceCounter;
        let mut ticks = 0;
        // SAFETY: the API writes a single i64 to a live stack variable. QPC
        // is observational; it does not wait on control, disk or network I/O.
        unsafe { QueryPerformanceCounter(&mut ticks) }.ok()?;
        convert_qpc(ticks, self.frequency)
    }

    #[cfg(not(windows))]
    fn qpc_100ns(&self) -> Option<u64> {
        let _ = self.frequency;
        None
    }

    pub fn record(
        &self,
        stage: &'static str,
        sequence: u64,
        block: &[f64],
        state: &super::workers::Shared,
    ) {
        let Some(qpc_100ns) = self.qpc_100ns() else {
            self.missed.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let hit = block
            .chunks_exact(2)
            .enumerate()
            .find_map(|(frame, samples)| {
                let peak = samples[0].abs().max(samples[1].abs());
                (peak > 0.01).then_some((frame as i64, peak))
            });
        let (first_hit_frame, peak) = hit.unwrap_or((-1, 0.0));
        if self
            .entries
            .push(Entry {
                stage,
                qpc_100ns,
                sequence,
                first_hit_frame,
                peak,
                depth: state.relay_ready.len(),
                silence: state.relay_silence.load(Ordering::Relaxed),
                dropped: state.relay_dropped.load(Ordering::Relaxed),
            })
            .is_err()
        {
            self.missed.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn write_to(&self, mut out: impl Write) -> std::io::Result<()> {
        let missed = self.missed.load(Ordering::Relaxed);
        writeln!(
            out,
            "# observation QPC, not a DMA presentation timestamp; missed={missed}"
        )?;
        writeln!(
            out,
            "stage,qpc_100ns,sequence,first_hit_frame,peak,relay_depth,relay_silence,relay_dropped"
        )?;
        while let Some(e) = self.entries.pop() {
            writeln!(
                out,
                "{},{},{},{},{},{},{},{}",
                e.stage,
                e.qpc_100ns,
                e.sequence,
                e.first_hit_frame,
                e.peak,
                e.depth,
                e.silence,
                e.dropped
            )?;
        }
        if missed != 0 {
            return Err(std::io::Error::other(format!(
                "trace invalid: {missed} missing observations"
            )));
        }
        Ok(())
    }
}

fn convert_qpc(ticks: i64, frequency: u64) -> Option<u64> {
    if ticks < 0 || frequency == 0 {
        return None;
    }
    u64::try_from(u128::from(ticks as u64) * 10_000_000 / u128::from(frequency)).ok()
}

#[cfg(windows)]
pub(super) fn snapshot_100ns() -> Result<u64, String> {
    use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
    let (mut ticks, mut frequency) = (0, 0);
    // SAFETY: both APIs write one i64 into separate live stack variables.
    unsafe {
        QueryPerformanceFrequency(&mut frequency)
            .map_err(|e| format!("progress QPC frequency: {e}"))?;
        QueryPerformanceCounter(&mut ticks).map_err(|e| format!("progress QPC clock: {e}"))?;
    }
    if frequency <= 0 {
        return Err("progress QPC frequency invalid".to_owned());
    }
    convert_qpc(ticks, frequency as u64).ok_or_else(|| "progress QPC clock invalid".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qpc_units_and_overflow_are_explicit() {
        assert_eq!(convert_qpc(15_000_000, 10_000_000), Some(15_000_000));
        assert_eq!(convert_qpc(15_000_000, 3_000_000), Some(50_000_000));
        assert_eq!(convert_qpc(-1, 10_000_000), None);
        assert_eq!(convert_qpc(1, 0), None);
        assert_eq!(convert_qpc(i64::MAX, 1), None);
    }

    #[test]
    fn trace_capacity_failure_preserves_evidence_and_fails() {
        let trace = Trace {
            frequency: 10_000_000,
            entries: ArrayQueue::new(1),
            missed: AtomicU64::new(1),
        };
        trace
            .entries
            .push(Entry {
                stage: "render",
                qpc_100ns: 123,
                sequence: 4,
                first_hit_frame: 5,
                peak: 0.25,
                depth: 2,
                silence: 1,
                dropped: 0,
            })
            .unwrap();
        let mut out = Vec::new();
        assert!(trace.write_to(&mut out).is_err());
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("missed=1"));
        assert!(text.contains("render,123,4,5,0.25,2,1,0"));
    }

    #[test]
    fn impulse_observation_keeps_stereo_identity_and_counters() {
        let trace = Trace::new(1, 128).unwrap();
        let state = super::super::workers::Shared::new(960, 64);
        let mut block = [0.0; 960];
        block[20] = 0.25;
        block[21] = -0.375;
        trace.record("render", 7, &block, &state);
        let e = trace.entries.pop().unwrap();
        assert_eq!((e.sequence, e.first_hit_frame, e.peak), (7, 10, 0.375));
        assert!(e.qpc_100ns > 0);
        assert_eq!((e.depth, e.silence, e.dropped), (0, 0, 0));
        assert_eq!(trace.missed.load(Ordering::Relaxed), 0);
    }
}
