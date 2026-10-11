"""Per-impulse latency series from a probe raw.csv (replicates the probe's
in-run-median dating) and per-step correlation with route progress counters.
Usage: review-latency-steps.py <extracted run dir> <pair, e.g. A-to-B>
Host only; reads saved files. Dating inherits the probe's payload/device
frame assumption."""
import bisect
import re
import statistics
import sys
from pathlib import Path

run, pair = Path(sys.argv[1]), sys.argv[2]
clock, arrivals = [], []
for line in open(run / f"{pair}-raw.csv"):
    f = line.rstrip().split(",")
    if f[0] == "meta":
        rate, interval, emitted, period, anchor = float(f[1]), int(f[2]), int(f[3]), float(f[4]), float(f[5])
    elif f[0] == "clock":
        clock.append((float(f[1]), float(f[2])))
    elif f[0] == "arrival":
        arrivals.append((float(f[1]), float(f[2])))
# advancing samples: frames strictly increasing
adv, last = [], -1
for fr, q in clock:
    if fr > last:
        adv.append((fr, q)); last = fr
frames = [a[0] for a in adv]
offs = [q - fr / rate * 1e7 for fr, q in adv]
win = 0.1 * rate
emission = []
for k in range(emitted):
    fr = k * interval
    lo, hi = bisect.bisect_left(frames, fr - win), bisect.bisect_right(frames, fr + win)
    if hi > lo:
        emission.append(statistics.median(offs[lo:hi]) + fr / rate * 1e7)
    else:
        emission.append(anchor + k * period)
for k in range(1, len(emission)):
    emission[k] = max(emission[k], emission[k - 1])
lat = {}
for q, amp in arrivals:
    r = round(amp * 32)
    if abs(amp * 32 - r) > 1e-4 or not 1 <= r <= 16:
        continue
    i = r - 1
    k = bisect.bisect_right(emission, q + period / 2) - 1
    if k >= 0:
        k -= ((k - i) % 16 + 16) % 16
    if k < 0 or k >= emitted:
        continue
    l = q - emission[k]
    if l < -period / 2 or l >= 15.5 * period or k in lat:
        continue
    lat[k] = l / 1e4
t0 = emission[0]
# Level per 10 s window (median) and steps between consecutive windows.
ks = sorted(lat)
windows = {}
for k in ks:
    windows.setdefault(int((emission[k] - t0) / 1e7 // 10), []).append(lat[k])
levels = [(w, statistics.median(v)) for w, v in sorted(windows.items())]

# Progress counters by QPC.
prog = []
for line in open(run / f"{pair}.progress.txt"):
    if not line.startswith("progress "):
        continue
    s = re.search(r"snapshot_qpc_100ns=(\d+)", line)
    if not s:
        continue
    c = re.search(r"capture=NativeBridgeStreamCounters \{ underrun_frames: (\d+), overrun_frames: (\d+)", line)
    r = re.search(r"render=NativeBridgeStreamCounters \{ underrun_frames: (\d+), overrun_frames: (\d+)", line)
    p = re.search(r"accepted: (\d+), late: (\d+), overrun: (\d+)", line)
    w = re.search(r"written=(\d+) read=(\d+) ack=(\d+)", line)
    g = max(int(x) for x in re.findall(r"interval_(?:capture|render|control)_gap_us=(\d+)", line))
    prog.append((int(s.group(1)), dict(cap_under=int(c.group(1)), ren_under=int(r.group(1)), ren_over=int(r.group(2)),
                                      late=int(p.group(2)), pkt_over=int(p.group(3)), written=int(w.group(1)),
                                      read=int(w.group(2)), gap_ms=g / 1000)))
pq = [p[0] for p in prog]


def counters_between(a, b):
    i, j = bisect.bisect_left(pq, a), bisect.bisect_right(pq, b)
    if i == 0 or j >= len(prog):
        return None
    x, y = prog[i - 1][1], prog[min(j, len(prog) - 1)][1]
    d = {k: y[k] - x[k] for k in ("cap_under", "ren_under", "ren_over", "late", "pkt_over")}
    d["max_gap_ms"] = max(p[1]["gap_ms"] for p in prog[i - 1:j + 1])
    d["route_depth"] = y["written"] - y["read"]
    return d


print(f"{pair}: matched {len(lat)}/{emitted}; first level {levels[0][1]:.2f} ms, last {levels[-1][1]:.2f} ms")
print("level steps between 10 s windows (>= 4 ms):")
for (w1, l1), (w2, l2) in zip(levels, levels[1:]):
    if abs(l2 - l1) >= 4:
        # locate the impulse where it changes
        a, b = t0 + w1 * 1e7, t0 + (w2 + 1) * 1e7
        print(f"  t={w2*10:5d} s  {l1:6.2f} -> {l2:6.2f} ms  counters {counters_between(a, b)}")
# Impulse-level step localisation: any jump > 4 ms sustained for 50 impulses.
print("sustained impulse-level steps (> 4 ms, held 50 impulses):")
prev = None
for idx in range(len(ks) - 50):
    k = ks[idx]
    if prev is not None:
        before = statistics.median(lat[x] for x in ks[max(0, idx - 50):idx])
        after = statistics.median(lat[x] for x in ks[idx:idx + 50])
        if abs(after - before) > 4 and (prev_step is None or k - prev_step > 100):
            c = counters_between(emission[k] - 2e7, emission[k] + 2e7)
            print(f"  impulse {k} t={(emission[k]-t0)/1e7:7.1f} s  {before:6.2f} -> {after:6.2f}  {c}")
            prev_step = k
    else:
        prev_step = None
    prev = k
