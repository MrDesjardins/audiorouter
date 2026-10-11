"""Host review of a `run-cable-kill.ps1` archive (A7, WP-09). Reads saved
files only.

Usage: review-cable-kill.py <extracted kill-* directory>
       review-cable-kill.py --selftest

Per repeat, on the probe's QPC clock (100 ns):
- silence: the last Cable B arrival after the kill (and before the restart)
  must be at most 500 ms after the kill, and nothing may arrive between
  then and the restart;
- no old audio: every arrival after the restart pairs with an impulse
  emitted after the restart was requested, at the restarted route's steady
  delay (within 15 ms). Identity codes repeat every 16 impulses, so a
  stale block can only be recognised by its timing, not its code;
- the restarted route carries audio again.
"""
import csv
import importlib.util
import json
import statistics
import sys
from pathlib import Path

SILENCE_LIMIT_MS = 500.0
LEVEL_TOLERANCE_MS = 15.0

_stages = importlib.util.spec_from_file_location(
    "review_latency_stages", Path(__file__).with_name("review-latency-stages.py"))
stages = importlib.util.module_from_spec(_stages)
_stages.loader.exec_module(stages)


def judge(emission, period, arrivals, kill, restart):
    """Return (verdict dict, list of failure strings). Times in 100 ns."""
    paired = stages.index_by_code(arrivals, emission, period)
    by_time = sorted((q, k) for k, q in paired.items())
    before = [(q, k) for q, k in by_time if q < restart]
    after = [(q, k) for q, k in by_time if q >= restart]
    failures = []
    last_before = max((q for q, _ in before), default=None)
    silence_ms = None if last_before is None else (last_before - kill) / 1e4
    if last_before is None or last_before < kill:
        silence_ms = 0.0
    if silence_ms > SILENCE_LIMIT_MS:
        failures.append(f"Cable B still carried audio {silence_ms:.1f} ms after the kill")
    late = [q for q, _ in before if q > kill + SILENCE_LIMIT_MS * 1e4]
    if late:
        failures.append(f"{len(late)} arrivals between kill + 500 ms and the restart")
    pre_kill = [(q - emission[k]) / 1e4 for q, k in before if q < kill]
    if not after:
        failures.append("no audio after the restart")
        level = None
    else:
        level = statistics.median((q - emission[k]) / 1e4 for q, k in after)
        stale = [k for q, k in after
                 if emission[k] < restart or abs((q - emission[k]) / 1e4 - level) > LEVEL_TOLERANCE_MS]
        if stale:
            failures.append(f"{len(stale)} arrivals after the restart are old or off the route's delay")
    # Arrivals that pair with nothing (out of window) are suspicious after a
    # restart, except the probe's one extra final impulse: the next code
    # after the window, one period after the last emission, at the route's
    # delay (seen in every probe run since r13).
    paired_times = {q for q, _ in after}
    final_code = len(emission) % 16 + 1
    unpaired_after = sum(
        1 for q, amplitude in arrivals
        if q >= restart and q not in paired_times
        and not (level is not None and round(amplitude * 32) == final_code
                 and abs((q - emission[-1] - period) / 1e4 - level) <= LEVEL_TOLERANCE_MS))
    if unpaired_after:
        failures.append(f"{unpaired_after} unpaired arrivals after the restart")
    return {
        "silence_after_kill_ms": silence_ms,
        "pre_kill_latency_ms": statistics.median(pre_kill) if pre_kill else None,
        "post_restart_latency_ms": level,
        "post_restart_arrivals": len(after),
    }, failures


def review(run):
    result = json.loads((run / "result.json").read_text(encoding="utf-8-sig"))
    print(f"run {result['Run']}: passed={result['Passed']} error={result['Error']!r}")
    ok = bool(result["Passed"])
    for entry in result["Results"]:
        name = f"repeat{entry['Repeat']}"
        clock, arrivals = [], []
        for row in csv.reader((run / f"{name}-raw.csv").open()):
            if row[0] == "meta":
                rate, interval, emitted, period, anchor = (float(row[1]), int(row[2]), int(row[3]),
                                                           float(row[4]), float(row[5]))
            elif row[0] == "clock":
                clock.append((float(row[1]), float(row[2])))
            elif row[0] == "arrival":
                arrivals.append((float(row[1]), float(row[2])))
        emission = stages.render_emission(clock, rate, interval, emitted, anchor, period)
        verdict, failures = judge(emission, period, arrivals, entry["KillQpc100ns"],
                                  entry["RestartRequestQpc100ns"])
        restart_ms = (entry["RestartRunningQpc100ns"] - entry["KillQpc100ns"]) / 1e4
        print(f"  {name}: {verdict}; route 2 running {restart_ms:.0f} ms after the kill; "
              f"probe exit {entry['ProbeCode']}")
        for failure in failures:
            print(f"    FAIL: {failure}")
        ok = ok and not failures and entry["ProbeCode"] == 0
    print("A7 verdict:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


def selftest():
    period = 100_000.0  # 10 ms
    emission = [k * period for k in range(1500)]
    kill, restart = 400 * period, 750 * period

    def arrivals(latency_ms, stop_ms, extra=()):
        out = []
        for k in range(1500):
            q = emission[k] + latency_ms * 1e4
            if q < kill + stop_ms * 1e4 or emission[k] >= restart + 50 * period:
                out.append((q, (1 + k % 16) / 32.0))
        # The probe's extra final impulse, just outside the window.
        out.append((1500 * period + latency_ms * 1e4, (1 + 1500 % 16) / 32.0))
        return sorted(out + list(extra))

    verdict, failures = judge(emission, period, arrivals(40, 45), kill, restart)
    assert not failures and verdict["silence_after_kill_ms"] <= 45, (verdict, failures)
    assert abs(verdict["post_restart_latency_ms"] - 40) < 1e-6
    _, failures = judge(emission, period, arrivals(40, 800), kill, restart)
    assert any("after the kill" in f for f in failures), failures
    # An old impulse (emitted before the kill) replayed after the restart.
    old = (restart + 60 * period + 3e4, (1 + 395 % 16) / 32.0)
    _, failures = judge(emission, period, arrivals(40, 45, [old]), kill, restart)
    assert any("old or off" in f for f in failures), failures
    _, failures = judge(emission, period, [a for a in arrivals(40, 45) if a[0] < restart], kill, restart)
    assert any("no audio after the restart" in f for f in failures), failures
    # An unknown arrival outside the window is still a failure.
    stray = (1500 * period + 400 * 1e4, 3 / 32.0)
    _, failures = judge(emission, period, arrivals(40, 45, [stray]), kill, restart)
    assert any("unpaired" in f for f in failures), failures
    print("review-cable-kill selftest: 5 cases pass")


if __name__ == "__main__":
    if len(sys.argv) == 2 and sys.argv[1] == "--selftest":
        selftest()
        sys.exit(0)
    sys.exit(review(Path(sys.argv[1])))
