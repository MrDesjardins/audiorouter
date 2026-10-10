"""Offline WAV checks only; no microphone, endpoint or driver access."""
import importlib.util
import math
from pathlib import Path
import struct
import sys
import tempfile
import unittest
import wave

SCRIPT = Path(__file__).resolve().parents[2] / "tools/vm/prepare-playback-reference.py"
# Keep generated test artifacts out of the source directory.
sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location("reference", SCRIPT)
REFERENCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REFERENCE)


class PlaybackReferenceTests(unittest.TestCase):
    def test_rates_headers_and_new_file_ownership(self):
        base = SCRIPT.parents[2] / "target"
        base.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=base) as directory:
            self.assertTrue(Path(directory).resolve().is_relative_to(base.resolve()))
            destination = Path(directory) / "new"
            report = REFERENCE.prepare(destination)
            self.assertFalse(report["qualification"])
            for rate in (44100, 48000):
                with wave.open(str(destination / f"reference-{rate}.wav"), "rb") as wav:
                    self.assertEqual((wav.getnchannels(), wav.getsampwidth(), wav.getframerate()), (2, 2, rate))
                    self.assertEqual(wav.getnframes(), 17 * rate)
                    self.assertEqual(wav.getcomptype(), "NONE")
            with self.assertRaises(FileExistsError):
                REFERENCE.prepare(destination)

    def test_tones_quantization_silence_and_boundary_fades(self):
        for rate in (44100, 48000):
            data = REFERENCE.samples(rate)
            values = list(struct.iter_unpack("<hh", data))
            self.assertEqual(len(values), rate * 17)
            self.assertLessEqual(max(abs(s) for frame in values for s in frame), 8192)
            for start, hz in REFERENCE.SECTIONS:
                self.assertEqual(values[start * rate], (0, 0))
                self.assertEqual(values[(start + 5) * rate - 1], (0, 0))
                for frame in range(rate, 2 * rate):
                    actual = values[start * rate + frame]
                    for channel, frequency in enumerate(hz):
                        expected = 8192 * math.sin(2 * math.pi * frequency * frame / rate)
                        self.assertLessEqual(abs(actual[channel] - expected), 0.5)
            for first, last in ((5, 6), (11, 12)):
                self.assertTrue(all(frame == (0, 0) for frame in values[first * rate:last * rate]))

    def test_unapproved_rates_are_rejected(self):
        for rate in (0, 16000, 22050, 96000):
            with self.assertRaises(ValueError):
                REFERENCE.samples(rate)


if __name__ == "__main__":
    unittest.main()
