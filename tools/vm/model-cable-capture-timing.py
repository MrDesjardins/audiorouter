"""Offline packet/slot timing model. No Windows, audio, or system operations."""
from fractions import Fraction


def position(ms):
    # Production UpdatePosition carries fractional milliseconds; equivalent
    # to total whole-millisecond progress at 48 frames/ms in this model.
    return int(ms) * 48


def latest(ms):
    return position(ms) // 128 - 1


def retained_packet(ms, packet):
    # Two physical slots; an older packet is unsafe as soon as any sample of
    # its slot is reused. Do not equate old slot contents with a valid packet.
    end = position(ms)
    return (packet + 1) * 128 <= end <= (packet + 2) * 128


def checks():
    assert latest(Fraction(8, 3)) == -1  # 1 ms progress quantum, not a second clock
    assert latest(3) == 0
    assert latest(6) == 1
    assert latest(8) == 2
    assert retained_packet(5, 0)
    assert not retained_packet(6, 0)  # 32 frames of packet 2 overwrite slot 0
    assert retained_packet(6, 1)
    # An OS fetch only 2 ms after the 6 ms notification sees packet 2;
    # packet 1 has become stale. Client read can follow immediately.
    assert latest(6 + 2) == 2
    assert retained_packet(8, 1)  # exact overwrite boundary, no reused sample yet
    assert not retained_packet(9, 1)
    # Same returned packet, two different causes: timer late vs OS fetch late.
    assert latest(6) == latest(3 + 3) == 1
    # A 264-frame WASAPI ring can retain two complete 128-frame packets, but
    # not three. A delayed reader is a distinct loss path after OS fetch.
    assert 2 * 128 <= 264 < 3 * 128
    # Sweep every microsecond around slot reuse; no partially overwritten
    # packet is reported safe by the model.
    for us in range(5000, 10001):
        t = Fraction(us, 1000)
        end = position(t)
        for packet in range(3):
            safe = retained_packet(t, packet)
            if end > (packet + 2) * 128:
                assert not safe
    print("Capture timing model passed: boundaries, slot reuse, delayed notification/fetch, reader retention.")
    print("128 frames = 2.667 ms; 1 ms notification quantization leaves less than one period for OS fetch.")
    print("Capture rows alone cannot distinguish a late notification from a late OS fetch; correlate scheduler readiness.")


if __name__ == "__main__":
    checks()
