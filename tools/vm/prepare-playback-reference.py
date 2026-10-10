"""Create bounded, offline speaker references; never open an audio endpoint."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import wave

SECONDS = 17
AMPLITUDE = 0.25
SECTIONS = ((0, (997, 47)), (6, (997, 997)), (12, (47, 47)))


def samples(rate):
    if rate not in (44100, 48000):
        raise ValueError("reference rate must be 44100 or 48000")
    data = bytearray(SECONDS * rate * 4)
    fade_frames = round(rate * 0.02)
    for second, frequencies in SECTIONS:
        start = second * rate
        length = 5 * rate
        for frame in range(length):
            # Only the first/last 20 ms are faded, to avoid boundary clicks.
            gain = min(1.0, frame / fade_frames, (length - 1 - frame) / fade_frames)
            values = [
                round(32768 * AMPLITUDE * gain * math.sin(2 * math.pi * hz * frame / rate))
                for hz in frequencies
            ]
            struct.pack_into("<hh", data, (start + frame) * 4, *values)
    return bytes(data)


def write_reference(path, rate):
    # Creating a new file rather than overwriting preserves prior artifacts.
    with path.open("xb") as output:
        with wave.open(output, "wb") as wav:
            wav.setnchannels(2)
            wav.setsampwidth(2)
            wav.setframerate(rate)
            wav.writeframes(samples(rate))


def prepare(destination):
    destination.mkdir(parents=True, exist_ok=False)
    files = []
    for rate in (44100, 48000):
        path = destination / f"reference-{rate}.wav"
        write_reference(path, rate)
        files.append({
            "file": path.name,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "rate": rate,
            "frames": SECONDS * rate,
        })
    report = {
        "qualification": False,
        "encoding": "PCM16",
        "channels": 2,
        "seconds": SECONDS,
        "peakAmplitude": AMPLITUDE,
        "boundaryFadeSeconds": 0.02,
        "sections": [
            {"startSeconds": start, "durationSeconds": 5, "frequenciesHz": hz}
            for start, hz in SECTIONS
        ],
        "files": files,
    }
    (destination / "REFERENCE.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(prepare(args.destination), indent=2))
