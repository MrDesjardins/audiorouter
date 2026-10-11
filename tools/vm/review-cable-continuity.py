"""Offline QPC review of a saved continuity ZIP. Never opens an audio endpoint.

Loss dates still inherit the probe's payload/device-frame assumption. The
four-route gap labels describe correlation, not proof of a scheduling cause.
"""
import argparse
import json
import re
import subprocess
import zipfile
from pathlib import Path


PAIRS = ('A-to-B', 'C-to-D', 'E-to-F', 'G-to-H')


def intervals(text, frequency):
    result = []
    previous = None
    for line in text.splitlines():
        if not line.startswith('progress '):
            continue
        gaps = re.findall(r'interval_(?:capture|render|control)_gap_us=(\d+)', line)
        qpc = re.findall(r'last_qpc_time: (\d+)', line)
        if len(gaps) != 3 or len(qpc) != 2:
            raise ValueError('Incomplete progress snapshot')
        end = max(map(int, qpc)) / frequency
        errors = sum(map(int, re.findall(
            r'\b(?:underrun_frames|overrun_frames|sequence_gaps|non_finite_samples|format_mismatches|late|overrun): (\d+)', line)))
        if end == 0:
            continue
        if previous is not None:
            if end < previous[0] or errors < previous[1]:
                raise ValueError('Progress clock/counter regression')
            if end > previous[0]:
                result.append({'from': previous[0], 'to': end,
                               'gap_ms': max(map(int, gaps)) / 1000,
                               'added': errors - previous[1]})
        previous = (end, errors)
    return result


def nearby_gap(at, timeline):
    return any(x['gap_ms'] >= 20 and x['from'] <= at + 1.5
               and x['to'] >= at - 1.5 for x in timeline)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('--probe', type=Path, required=True)
    parser.add_argument('--driver-qpc-frequency', type=int, required=True,
                        help='Known guest driver QPC frequency in ticks/second; do not guess')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.driver_qpc_frequency <= 0:
        parser.error('Driver QPC frequency must be positive')
    args.output.mkdir(parents=True, exist_ok=True)
    probe = args.probe.resolve()
    with zipfile.ZipFile(args.archive) as archive:
        # Select only known flat files, never extract arbitrary archive paths.
        files = {Path(name.replace('\\', '/')).name: name for name in archive.namelist()}
        original = json.loads(archive.read(files['result.json']))
        timelines = {pair: intervals(archive.read(files[pair + '.progress.txt']).decode('utf-8-sig'),
                                    args.driver_qpc_frequency) for pair in PAIRS}
        routes = []
        for pair in PAIRS:
            raw = args.output / (pair + '-raw.csv')
            raw.write_bytes(archive.read(files[pair + '-raw.csv']))
            run = subprocess.run([str(probe), 'cable-impulse-reanalyze', str(raw)],
                                 text=True, capture_output=True, check=True)
            (args.output / (pair + '-reanalyzed.txt')).write_text(run.stdout, encoding='utf-8')
            events = []
            for match in re.finditer(r'cable_lost_run first_impulse=(\d+) count=(\d+) first_qpc_100ns=([0-9.]+) last_qpc_100ns=([0-9.]+)', run.stdout):
                first, count, at, end = match.groups()
                at, end = float(at) / 1e7, float(end) / 1e7
                hits = [name for name in PAIRS if nearby_gap(at, timelines[name])]
                events.append({'first_impulse': int(first), 'count': int(count), 'qpc_seconds': at,
                               'last_qpc_seconds': end, 'nearby_gap_routes': hits, 'all_four': len(hits) == 4})
            saved = next(route for route in original['Results'] if route['Pair'] == pair)
            total = sum(x['count'] for x in events)
            if total != saved['Lost']:
                raise ValueError(f'{pair}: QPC loss runs total {total}, expected {saved["Lost"]}; truncated/changed analysis')
            nominal = [e for e in saved['Events'] if e['Kind'] == 'lost-impulses']
            origin = nominal[0]['At'] - events[0]['qpc_seconds'] if events else 0
            drift = [(old['At'] - new['qpc_seconds']) - origin for old, new in zip(nominal, events)]
            route = {'pair': pair, 'lost': total, 'loss_runs': len(events),
                     'qpc_all_four_runs': sum(e['all_four'] for e in events),
                     'qpc_unexplained_impulses': sum(e['count'] for e in events if not e['all_four']),
                     'nominal_date_drift_seconds': [min(drift), max(drift)] if drift else [], 'events': events}
            routes.append(route)
            print(pair, {k: v for k, v in route.items() if k != 'events'})
    report = {'scope': 'offline correlation; inherited payload/device-frame assumption, not causal attribution',
              'driver_qpc_frequency': args.driver_qpc_frequency, 'routes': routes}
    (args.output / 'qpc-review.json').write_text(json.dumps(report, indent=2), encoding='utf-8')


if __name__ == '__main__':
    main()
