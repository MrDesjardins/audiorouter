"""Offline review of a driver-timing.csv from the r16+ diagnostic drivers
(host only; reads a saved CSV). The MoreData follow-up line is meaningful
only for r17+ drivers (oldest-intact selection; r16 never set MoreData);
re-signal records (event 7) exist only from r18.

Usage: review-driver-timing.py driver-timing.csv [skip_ms]

Per stream: timer intervals, notification lateness against the true packet
boundary (RUN + n * packet duration), the delay from notification to
Windows' GetReadPacket return (capture only), and every capture packet
number the driver skipped, classified by whether its DMA slot was still
intact at that fetch. Assumes two packets per WaveRT buffer, as recorded
by the RUN event of the reviewed guest streams.
"""
import csv
import sys
from collections import defaultdict


def pct(values, p):
    if not values:
        return float("nan")
    s = sorted(values)
    return s[min(len(s) - 1, int(p * len(s)))]


def summary(name, values):
    if not values:
        print(f"  {name}: none")
        return
    print(f"  {name}: n={len(values)} min={min(values):.3f} p50={pct(values, .5):.3f} "
          f"p95={pct(values, .95):.3f} p99={pct(values, .99):.3f} max={max(values):.3f} ms")


def main(path, start_skip_ms=0.0):
    rows = []
    with open(path, newline="") as f:
        for r in csv.DictReader(f):
            rows.append({k: int(v, 0) for k, v in r.items()})
    rows.sort(key=lambda r: (r["qpc"], r["sequence"]))
    freq = 10_000_000
    streams = defaultdict(list)
    for r in rows:
        streams[r["stream_id"]].append(r)
    for sid, rs in sorted(streams.items()):
        run = [r for r in rs if r["event"] == 6][0]
        rate = run["extra"] >> 32
        packet_bytes = run["extra"] & 0xFFFFFFFF
        bytes_per_ms = rate * run["linear_bytes"] / 1000.0
        packet_ms = packet_bytes / bytes_per_ms
        t0 = run["qpc"]
        ms = lambda q: (q - t0) * 1000.0 / freq
        direction = "capture" if run["direction"] == 2 else "render"
        print(f"stream {sid} bus {run['bus']} {direction}: rate={rate} packet={packet_bytes} B = {packet_ms:.4f} ms")

        timers = [r for r in rs if r["event"] == 1 and ms(r["qpc"]) >= start_skip_ms]
        intervals = [ms(b["qpc"]) - ms(a["qpc"]) for a, b in zip(timers, timers[1:])]
        summary("timer interval", intervals)

        # Position must equal floor(elapsed * bytes_per_ms / 384) * 384 if it
        # is linear from RUN; count deviations (clamps/catch-up).
        step = bytes_per_ms  # one 1 ms step
        dev = 0
        for r in rs:
            if r["event"] == 2:
                expect = int(ms(r["extra"]) * bytes_per_ms // step * step)
                if r["linear_bytes"] != expect:
                    dev += 1
        print(f"  position deviations from linear-from-RUN: {dev}")

        notes = {}
        for r in rs:
            if r["event"] == 3:
                notes.setdefault(r["extra"], r["qpc"])
        late = []
        late_by_n = {}
        for n, q in notes.items():
            boundary = n * packet_ms
            if boundary < start_skip_ms:
                continue
            v = ms(q) - boundary
            late.append(v)
            late_by_n[n] = v
        summary("notify lateness after true boundary", late)
        skipped = sum(1 for a, b in zip(sorted(notes), sorted(notes)[1:]) if b != a + 1)
        print(f"  notification count jumps (coalesced): {skipped}")

        if direction == "capture":
            returns = {}
            for r in rs:
                if r["event"] == 5 and r["status"] == 0:
                    returns.setdefault(r["packet_number"], r["qpc"])
            fetch = []
            worst = []
            for n, q in notes.items():
                p = n - 1  # notification count n completes packet n-1
                if p in returns and n * packet_ms >= start_skip_ms:
                    d = ms(returns[p]) - ms(q)
                    fetch.append(d)
                    worst.append((d, p, ms(q)))
            summary("notify -> GetReadPacket return", fetch)
            total = [late_by_n[n] + (ms(returns[n - 1]) - ms(notes[n]))
                     for n in notes if n in late_by_n and n - 1 in returns]
            summary("true boundary -> fetch return", total)
            failures = sum(1 for r in rs if r["event"] == 5 and r["status"] != 0)
            print(f"  GetReadPacket non-success returns: {failures}")
            # A capture slot holds packet p until the DMA writer reaches the
            # start of packet p + 2 (two packets per WaveRT buffer). Classify
            # every packet number the driver skipped by what remained of its
            # slot at that fetch (post-update linear position).
            ok = [r for r in rs if r["event"] == 5 and r["status"] == 0]
            intact = reused = 0
            reused_bytes = []
            reused_into = 0
            for a, b in zip(ok, ok[1:]):
                for p in range(a["packet_number"] + 1, b["packet_number"]):
                    over = b["linear_bytes"] - (p + 2) * packet_bytes
                    if over <= 0:
                        intact += 1
                    else:
                        reused += 1
                        reused_bytes.append(over)
            for r in ok:
                if r["linear_bytes"] > (r["packet_number"] + 2) * packet_bytes:
                    reused_into += 1
            within_packet = sum(1 for b in reused_bytes if b < packet_bytes)
            print(f"  skipped packets: {intact + reused} (slot intact {intact}, partly reused {reused}: "
                  f"{within_packet} by under one packet, max {max(reused_bytes, default=0)} B)")
            print(f"  returned packets whose slot reuse had begun: {reused_into}")
            # MoreData follow-up: did the next fetch come within 0.2 ms, and
            # was it woken by a boundary notification or not (r17/r18)?
            ordered = [r for r in rs if r["event"] in (3, 5, 7)]
            more_returns = quick = after_note = resignals = 0
            pending, note = None, False
            for r in ordered:
                if r["event"] == 7:
                    resignals += 1
                elif r["event"] == 3:
                    note = True
                elif r["status"] == 0:
                    if pending is not None:
                        quick += ms(r["qpc"]) - pending < 0.2
                        after_note += note
                    more = r["packet_count"] - 1 > r["packet_number"]
                    more_returns += more
                    pending, note = (ms(r["qpc"]) if more else None), False
            print(f"  MoreData returns: {more_returns}; next fetch within 0.2 ms: {quick}; "
                  f"after a boundary notification: {after_note}; re-signals (event 7): {resignals}")
            worst.sort(reverse=True)
            print("  largest fetch delays (ms, packet, notify at ms):",
                  [(round(d, 3), p, round(t, 1)) for d, p, t in worst[:8]])
        big = sorted(((round(v, 3), n, round(n * packet_ms, 1)) for n, v in late_by_n.items()), reverse=True)[:8]
        print("  largest notify lateness (ms, n, boundary ms):", big)


if __name__ == "__main__":
    main(sys.argv[1], float(sys.argv[2]) if len(sys.argv) > 2 else 0.0)
