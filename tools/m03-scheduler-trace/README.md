# Offline guest scheduling trace reader

Reads a **saved ETL file only**. It does not create, stop or cancel a tracing
session, access an audio device/driver, or change host settings. Diagnostic
outputs belong in ignored `target/` or the external evidence share, not Git.
Event times and counts are evidence; they do not alone identify a host pause
or prove that a recorded kernel callback caused an audio loss.

Windows `tracerpt` 10.0.26100 leaves guest build 26300's version-5 context-switch
payload unnamed. This reader copies the common first 24 bytes using
[Microsoft's PerfView parser](https://github.com/microsoft/perfview/blob/main/src/TraceEvent/Parsers/KernelTraceEventParser.cs)
(`CSwitchTraceData`). It rejects older/short/null switch records, checks ready
payload length, and emits no pointers or appended unknown fields. Decode
failures, output failures and ETW failures return nonzero. Existing output
paths are refused. Synthetic checks cover version-2 and version-5 prefixes.

Build in an existing **x64 Visual Studio developer command prompt**, from the
repository root (Windows SDK required; no dependency installation):

```powershell
cl.exe /nologo /std:c++17 /EHsc /W4 /WX /O2 /MT tools\m03-scheduler-trace\main.cpp /Fo:target\m03-scheduler-trace.obj /Fe:target\m03-scheduler-trace.exe /link advapi32.lib
.\target\m03-scheduler-trace.exe --self-test
.\target\m03-scheduler-trace.exe 'C:\path\scheduling.etl' 'C:\path\new-switches.csv'
.\target\m03-scheduler-trace.exe 'C:\path\scheduling.etl' 'C:\path\new-switches-qpc.csv' --raw
```

Default timestamps are FILETIME ticks (100 ns). `--raw` preserves the recorded
clock, which is QPC at 10 MHz for the reviewed guest trace; check each ETL
header rather than assuming this frequency for other traces. Correlate thread
IDs to `tracerpt` process/thread lifetime and thread-name records. CSV includes
all context switches and ready-thread events, not just AudioRouter's process.

For each thread, match a switch-out, its subsequent ready event and switch-in
to separate time spent waiting before readiness from ready-to-run scheduling
delay. Account for creation, termination, repeated readiness and trace edges;
do not infer a 1-ms request from ETW alone. The worker source supplies that
requested wait. Priorities, state and reason are raw kernel event fields.
Cross-check switch/ready counts against independent `tracerpt` summary and
require zero rejected/lost events before using a complete-run attribution.

`--silences input.etl output.csv threshold_ms` lists every system-wide interval
of at least the threshold in which **no event of any kind** (switch, ready,
DPC, ISR, timer, process/image; the logfile header excluded) was recorded on
any CPU, plus each CPU's longest gap. Timestamps are raw QPC; a trace whose
clock is not QPC is refused. While audio workers request 1 ms waits, a guest
that executes at all records events every few milliseconds; an all-CPU
silence therefore indicates that the guest was not executing (paused or
starved below the guest OS), not a guest thread or driver callback. Example:

```powershell
.\target\m03-scheduler-trace.exe --silences 'C:\path\scheduling.etl' 'C:\path\new-silences.csv' 20
```

Validated on the saved 2026-10-09 five-minute guest trace: 3,420,580 events,
none lost; three silences ≥ 20 ms (35.173, 27.954, 24.161 ms) of which the
two largest align with that run's two recorded loss events.
