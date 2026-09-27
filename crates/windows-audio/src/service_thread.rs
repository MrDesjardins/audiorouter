//! Scheduling setup for the backend-owned native audio service thread.
//!
//! Prepared native workers are passive: capture, graph processing and render
//! only advance when a caller pumps them. The control backend owns one
//! dedicated thread that performs that pumping continuously, so audio
//! continuity no longer depends on a UI timer or IPC round trip. This module
//! only configures that thread; it opens no stream and changes no endpoint.

/// Keeps the per-thread scheduling configuration alive. Dropping it restores
/// the timer resolution, leaves the MMCSS task and uninitializes COM, in the
/// reverse order of acquisition. It must be dropped on the thread that
/// created it, which `!Send` enforces.
pub struct AudioServiceThreadGuard {
    #[cfg(windows)]
    mmcss: Option<windows::Win32::Foundation::HANDLE>,
    #[cfg(windows)]
    timer_period: bool,
    #[cfg(windows)]
    com: bool,
    _not_send: std::marker::PhantomData<*const ()>,
}

/// What the current thread obtained. Each field is best effort: a missing
/// capability degrades scheduling precision but never prevents pumping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioServiceThreadCapabilities {
    pub com_multithreaded: bool,
    pub mmcss_pro_audio: bool,
    pub one_millisecond_timer: bool,
}

impl AudioServiceThreadGuard {
    /// Configure the calling thread for native audio service: a
    /// multithreaded COM apartment (WASAPI clients are free-threaded), the
    /// MMCSS "Pro Audio" task, and 1 ms timer resolution so short sleeps do
    /// not round up to the 15.6 ms default tick.
    pub fn enter() -> (Self, AudioServiceThreadCapabilities) {
        #[cfg(windows)]
        {
            // SAFETY: each call configures only the calling thread (COM,
            // MMCSS) or a process-wide reference-counted timer request. Every
            // successful acquisition is recorded and released exactly once by
            // `Drop` on the same thread; failures are recorded as absent.
            unsafe {
                let com = windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_MULTITHREADED,
                )
                .is_ok();
                let mut task_index = 0_u32;
                let mmcss = windows::Win32::System::Threading::AvSetMmThreadCharacteristicsW(
                    windows::core::w!("Pro Audio"),
                    &mut task_index,
                )
                .ok()
                .filter(|handle| !handle.is_invalid());
                let timer_period = windows::Win32::Media::timeBeginPeriod(1) == 0;
                let capabilities = AudioServiceThreadCapabilities {
                    com_multithreaded: com,
                    mmcss_pro_audio: mmcss.is_some(),
                    one_millisecond_timer: timer_period,
                };
                (
                    Self {
                        mmcss,
                        timer_period,
                        com,
                        _not_send: std::marker::PhantomData,
                    },
                    capabilities,
                )
            }
        }
        #[cfg(not(windows))]
        {
            (
                Self {
                    _not_send: std::marker::PhantomData,
                },
                AudioServiceThreadCapabilities::default(),
            )
        }
    }
}

impl Drop for AudioServiceThreadGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        // SAFETY: releases only what `enter` recorded as acquired, on the
        // same thread (the guard is `!Send`), in reverse acquisition order.
        unsafe {
            if self.timer_period {
                let _ = windows::Win32::Media::timeEndPeriod(1);
            }
            if let Some(handle) = self.mmcss.take() {
                let _ = windows::Win32::System::Threading::AvRevertMmThreadCharacteristics(handle);
            }
            if self.com {
                windows::Win32::System::Com::CoUninitialize();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn service_thread_setup_is_reversible_and_reports_capabilities() {
        std::thread::spawn(|| {
            let (guard, capabilities) = AudioServiceThreadGuard::enter();
            assert!(capabilities.com_multithreaded);
            assert!(capabilities.one_millisecond_timer);
            drop(guard);
            // A second acquisition on the same thread must still succeed
            // after the first guard released everything it took.
            let (_guard, again) = AudioServiceThreadGuard::enter();
            assert!(again.com_multithreaded);
        })
        .join()
        .unwrap();
    }
}
