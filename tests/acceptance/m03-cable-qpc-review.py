"""Offline regression for timeline units, incomplete snapshots and counter reset."""
import importlib.util
import sys
from pathlib import Path
import unittest

source = Path(__file__).resolve().parents[2] / 'tools/vm/review-cable-continuity.py'
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('review', source)
review = importlib.util.module_from_spec(spec)
spec.loader.exec_module(review)


def line(ticks, frames, gap=3000):
    return (f'progress 1000 ms: interval_capture_gap_us=3000 interval_render_gap_us={gap} '
            f'interval_control_gap_us=3000 capture=NativeBridgeStreamCounters {{ underrun_frames: 0, last_qpc_time: {ticks} }} '
            f'render=NativeBridgeStreamCounters {{ underrun_frames: {frames}, last_qpc_time: {ticks} }}')


class TimelineTests(unittest.TestCase):
    def test_frequency_is_explicit_and_stopped_clock_cannot_extend_interval(self):
        rows = '\n'.join((line(30_000_000, 0), line(33_000_000, 480, 50000), line(33_000_000, 480)))
        result = review.intervals(rows, 3_000_000)
        self.assertEqual(len(result), 1)
        self.assertEqual((result[0]['from'], result[0]['to'], result[0]['added']), (10, 11, 480))
        self.assertTrue(review.nearby_gap(10.5, result))
        self.assertFalse(review.nearby_gap(15, result))

    def test_counter_and_clock_regression_reject_attribution(self):
        for rows in ((line(30, 480), line(33, 0)), (line(30, 0), line(29, 0))):
            with self.assertRaises(ValueError):
                review.intervals('\n'.join(rows), 3)
        with self.assertRaises(ValueError):
            review.intervals('progress 1000 ms: last_qpc_time: 30', 3)


if __name__ == '__main__':
    unittest.main()
