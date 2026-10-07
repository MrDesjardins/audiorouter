---
applies_to: crates/**/*.rs, src-tauri/**/*.rs
---

# The realtime audio path must not allocate
Code that runs on the audio callback (`RealtimePluginProcessor::process`, anything handed an `AudioBlock` during processing, and functions it calls) must not allocate: no `Vec::new`/`push` that can grow, `collect`, `clone` of heap data, `Box::new`, `format!`, or `String` building. Allocate at construction or graph-preparation time and reuse the storage.

Good:
```rust
impl RealtimePluginProcessor for Gain {
    fn process(&self, block: &mut AudioBlock) {
        let gain = self.gain.load();
        for channel in 0..block.channels() {
            if let Some(samples) = block.channel_mut(channel) {
                for sample in samples {
                    *sample *= gain;
                }
            }
        }
    }
}
```

Bad:
```rust
impl RealtimePluginProcessor for Gain {
    fn process(&self, block: &mut AudioBlock) {
        let gain = self.gain.load();
        let scaled: Vec<f32> = block.channel(0).unwrap().iter().map(|s| s * gain).collect();
        block.channel_mut(0).unwrap().copy_from_slice(&scaled);
    }
}
```

# The realtime audio path must not block, lock, log, or do I/O
The audio callback must never wait for the UI, IPC, disk, network, plugins, or control-plane locks. Do not take a `Mutex`/`RwLock`, sleep, log, read a file, or call a plugin worker from it. Exchange data through preallocated lock-free structures (atomics, `ArrayQueue`) and fail closed when a result is late.

Good:
```rust
fn process(&self, block: &mut AudioBlock) {
    let Some(processed) = self.results.pop() else {
        block.clear();
        return;
    };
    if block.copy_from(&processed).is_err() {
        block.clear();
    }
}
```

Bad:
```rust
fn process(&self, block: &mut AudioBlock) {
    let settings = self.settings.lock().unwrap();
    log::debug!("processing with {:?}", *settings);
    self.worker.send_and_wait(block, &settings);
}
```

# Do not panic across an FFI boundary
Functions exposed as `extern "C"`/`extern "system"` callbacks (plugin hosts, Windows interop) must not let a panic unwind into foreign code. Avoid `unwrap`/`expect`/indexing that can panic there, and wrap the body in `std::panic::catch_unwind` when Rust code that may panic is called.

Good:
```rust
extern "system" fn on_device_event(context: *mut c_void) -> i32 {
    match std::panic::catch_unwind(|| handle_device_event(context)) {
        Ok(Ok(())) => S_OK,
        _ => E_FAIL,
    }
}
```

Bad:
```rust
extern "system" fn on_device_event(context: *mut c_void) -> i32 {
    handle_device_event(context).unwrap();
    S_OK
}
```

# A failed processor on a protected voice path produces silence
When a processor on a protected microphone/voice path fails or is late, the output must be silence until deliberate recovery, never the unprocessed input. A missing microphone must never be silently replaced with another input device.

Good:
```rust
if stage.failed() {
    block.clear();
    return Err(VoicePathError::ProcessorFailed);
}
```

Bad:
```rust
if stage.failed() {
    // Fall back to the raw microphone so the user is still heard.
    return Ok(());
}
```

# Every unsafe block documents invariants, ownership, and lifetimes
Besides the `// SAFETY:` comment, each `unsafe` block in interop code must state who owns any raw pointer or handle, how long it is valid, and which thread may use it. Keep the unsafe region small and covered by a focused test.

Good:
```rust
// SAFETY: `client` is an IAudioClient owned by `self` and released in Drop;
// it is only used on the stream thread, and `fmt` outlives this call.
unsafe { client.Initialize(mode, flags, period, 0, fmt, None)? };
```

Bad:
```rust
unsafe { client.Initialize(mode, flags, period, 0, fmt, None)? };
```
