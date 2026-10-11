"""Split a traced repeated-start run's per-impulse latency into stages, to
find which stage holds a standing delay step. Host only; reads saved files.

Usage: review-latency-stages.py <extracted starts-engine-* dir> [start-mode ...]
       (default: every *-raw.csv that has a matching *-cable-a.route.csv)

Stages per impulse k (QPC, 100 ns):
  emission   render-clock dating of frame k * interval (the probe's
             in-run-median method; inherits its payload/device assumption)
  route_in   first `render` route observation containing the impulse
             (route worker read from Cable A's render source)
  route_out  first `capture` route observation containing it (sink
             publication to Cable B)
  arrival    probe capture arrival
  write      QPC after the probe submission containing frame k * interval
             (`submit` rows, present with -TraceRoute)
write -> arrival is the delay a user hears from the moment the application
hands audio to Windows. A render-clock jump (Windows counting a glitch block
in the stream position) moves the clock-dated stages but not this one.
Route observations are worker observation times, not DMA presentation
times; a stage step of one block is still visible in their difference.
"""
import bisect
import csv
import statistics
import sys
from pathlib import Path


def render_emission(clock, rate, interval, emitted, anchor, period):
    adv, last = [], -1.0
    for fr, q in clock:
        if fr > last:
            adv.append((fr, q))
            last = fr
    frames = [a[0] for a in adv]
    offs = [q - fr / rate * 1e7 for fr, q in adv]
    win = 0.1 * rate
    out = []
    for k in range(emitted):
        fr = k * interval
        lo, hi = bisect.bisect_left(frames, fr - win), bisect.bisect_right(frames, fr + win)
        out.append(statistics.median(offs[lo:hi]) + fr / rate * 1e7 if hi > lo else anchor + k * period)
    for k in range(1, len(out)):
        out[k] = max(out[k], out[k - 1])
    return out


def index_by_code(rows, emission, period):
    """Impulse index per row from its code (1..16) and time; first row per
    index wins. Rows: (qpc, peak)."""
    out = {}
    for q, peak in rows:
        r = round(peak * 32)
        if abs(peak * 32 - r) > 1e-4 or not 1 <= r <= 16:
            continue
        k = bisect.bisect_right(emission, q + period / 2) - 1
        if k >= 0:
            k -= ((k - (r - 1)) % 16 + 16) % 16
        if 0 <= k < len(emission) and q - emission[k] >= -period / 2 and k not in out:
            out[k] = q
    return out


def review(run, name):
    clock, arrivals, submits = [], [], []
    for row in csv.reader((run / f"{name}-raw.csv").open()):
        if row[0] == "meta":
            rate, interval, emitted, period, anchor = float(row[1]), int(row[2]), int(row[3]), float(row[4]), float(row[5])
        elif row[0] == "clock":
            clock.append((float(row[1]), float(row[2])))
        elif row[0] == "arrival":
            arrivals.append((float(row[1]), float(row[2])))
        elif row[0] == "submit":
            submits.append([float(x) for x in row[1:]])
    emission = render_emission(clock, rate, interval, emitted, anchor, period)
    lines = (run / f"{name}-cable-a.route.csv").read_text().splitlines()
    route = list(csv.DictReader(lines[1:]))
    stage = {s: [(float(r["qpc_100ns"]), float(r["peak"])) for r in route
                 if r["stage"] == s and int(r["first_hit_frame"]) >= 0] for s in ("render", "capture")}
    depth = [(float(r["qpc_100ns"]), int(r["relay_depth"]), int(r["relay_silence"]), int(r["relay_dropped"]))
             for r in route if r["stage"] == "capture"]
    rin = index_by_code(stage["render"], emission, period)
    rout = index_by_code(stage["capture"], emission, period)
    arr = index_by_code(arrivals, emission, period)
    ks = sorted(k for k in arr if k in rin and k in rout)
    ms = lambda v: v / 1e4
    starts = [s[0] for s in submits]

    def write_qpc(k):
        i = bisect.bisect_right(starts, k * interval) - 1
        if i < 0 or k * interval >= submits[i][0] + submits[i][1]:
            return None
        return submits[i][6]  # QPC sample taken after the submission

    writes = {k: write_qpc(k) for k in ks}
    series = [(k, ms(rin[k] - emission[k]), ms(rout[k] - rin[k]), ms(arr[k] - rout[k]), ms(arr[k] - emission[k]),
               ms(arr[k] - writes[k]) if writes[k] is not None else float("nan")) for k in ks]
    print(f"{name}: {len(ks)} impulses seen at all stages of {emitted}; {lines[0]}")
    # Sustained total-latency steps (> 4 ms over 50 impulses either side).
    total = [s[4] for s in series]
    i, steps = 50, []
    while i < len(series) - 50:
        before, after = statistics.median(total[i - 50:i]), statistics.median(total[i:i + 50])
        if abs(after - before) > 4:
            # refine to the largest single change within the next 50
            j = max(range(i, min(i + 50, len(series))), key=lambda x: abs(total[x] - total[x - 1]))
            steps.append(j)
            i = j + 100
        else:
            i += 1
    def med(a, b, col):
        return statistics.median(s[col] for s in series[a:b])
    print(f"  stage medians at start (ms): emission->route_in {med(0, 50, 1):.2f}, route_in->route_out {med(0, 50, 2):.2f}, "
          f"route_out->arrival {med(0, 50, 3):.2f}, total {med(0, 50, 4):.2f}")
    for j in steps:
        a, b = max(0, j - 50), min(len(series), j + 50)
        k = series[j][0]
        q = emission[k]
        d = [x for x in depth if abs(x[0] - q) < 5e6]
        print(f"  step at impulse {k} (t={ms(q - emission[0]) / 1000:.2f} s):")
        for col, label in ((1, "emission->route_in"), (2, "route_in->route_out"), (3, "route_out->arrival"), (4, "total"),
                           (5, "write->arrival")):
            print(f"    {label:22s} {med(a, j, col):7.2f} -> {med(j, b, col):7.2f}  ({med(j, b, col) - med(a, j, col):+.2f})")
        if d:
            print(f"    relay depth {min(x[1] for x in d)}..{max(x[1] for x in d)}, silence {d[0][2]}->{d[-1][2]}, dropped {d[0][3]}->{d[-1][3]} within ±0.5 s")
    if not steps:
        print("  no sustained step")


if __name__ == "__main__":
    run = Path(sys.argv[1])
    names = sys.argv[2:] or sorted(p.name.removesuffix("-raw.csv") for p in run.glob("*-raw.csv")
                                   if (run / (p.name.removesuffix("-raw.csv") + "-cable-a.route.csv")).exists())
    for name in names:
        review(run, name)
