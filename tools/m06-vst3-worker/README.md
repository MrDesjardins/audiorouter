# M06 native VST3 worker

This Windows-only worker is the production-facing single-stream VST3 process
owner used by the supervised Rust worker client. It loads one verified x64
VST3 bundle in a separate process, sends the existing length-prefixed JSON
`Hello`/`Ready` handshake, accepts bounded `Process` frames and parameter
events, calls `IAudioProcessor::process`, and returns `Processed` frames. It
uses restricted DLL search paths and never opens an audio device.

Build it from the repository root after the pinned SDK fixture has been built:

    .\tools\m06-vst3-worker\build.ps1

The generated executable and object file are ignored and must not be committed.
The app ships this worker as `audiorouter-vst3-worker.exe` beside the shell;
control selects it for VST3 nodes (override: `AUDIOROUTER_VST3_WORKER_PATH`).
`tools/release/prepare-artifacts.ps1` builds it with
`build.ps1 -Release -Output target\release\audiorouter-vst3-worker.exe`
(optimized, static C runtime).

Since 2026-10-22 the worker also:

- moves the protocol to duplicated stdin/stdout handles and points the
  process's standard handles and CRT fds 0/1 at `NUL` before any plugin code
  loads, so a plugin that prints (UADx does) cannot corrupt a frame;
- calls `setBusArrangements` for the requested mono or stereo layout and,
  when the plugin keeps its own 1- or 2-channel layout, adapts in `process`
  (mono input copied to both channels; stereo output summed as 0.5·(L+R));
- describes only automatable, visible, writable, non-program-change
  parameters, at most 256;
- refuses classes whose VST3 sub-categories contain `Instrument`;
- answers `Latency` with `getLatencySamples`.
The worker accepts either one mono/stereo input and output bus through
`Process`, or a bounded auxiliary-bus layout through `HelloBuses` and
`ProcessBuses` (up to four buses and eight aggregate channels per direction).
In the `HelloBuses` layout, bus counts and per-bus channel counts must match
the plugin exactly; auxiliary
buses are never flattened into the main stream. Native process buffers and the
framed protocol also supports bounded VST3 component state through `StateSave`
and `StateRestore`; state is exchanged as a versioned byte asset with a
SHA-256 digest. Native process buffers and the framed protocol are currently
worker-thread implementation details, not proof of realtime graph scheduling
or physical-latency performance.

The worker answers `DescribeParameters` from the VST3 edit controller with a
bounded normalized descriptor catalog.

Threads: the main thread is the plugin's UI thread (OLE STA, a message loop,
per-monitor DPI aware) and makes every controller and `IPlugView` call. A
second thread reads the pipe and processes audio; every other request is
handed to the UI thread with `SendMessageTimeout` (15 s, after which the
worker ends so the supervisor contains it). The worker calls the module's
`InitDll`/`ExitDll`, which VSTGUI editors need.

Editors: `DescribeEditor` creates the `editor` view to report its size (or
no editor). `EditorOpen` checks that the parent HWND exists and belongs to
`parent_process_id`, answers at once, and attaches the view in a posted
message; the view sizes the parent to fit and follows its resizes.
`EditorClose` answers at once and detaches in a posted message (the shell's
window thread waits on the close request while the child window would wait
on it). An invalid parent returns `editorParentInvalid`; a plugin without a
Windows view returns `editorUnavailable`. Edits made in the window reach the
processor with the next quantum, and host parameter events update the
controller so the window follows. WebView2-based editors (UADx) need the
host to raise the worker job's process limit while open (SEC-07).
`native_vst3_editor_opens_in_a_pumping_parent` checks an editor end to end;
set `AUDIOROUTER_VST3_EDITOR_HOLD_MS` to look at it.
