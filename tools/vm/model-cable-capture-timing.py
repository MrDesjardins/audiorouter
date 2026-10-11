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


def select(ms, last_read):
    # r17 GetReadPacket: the oldest unread packet whose slot is intact, else
    # the oldest intact one; MoreData when a newer packet is complete.
    newest = latest(ms)
    if newest < 0 or last_read >= newest:
        return None
    packet = last_read + 1
    while packet < newest and not retained_packet(ms, packet):
        packet += 1
    return packet, packet < newest


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
    assert 2 * 128 <= 280 < 3 * 128  # r15 actual client buffers
    # r16 observations distinguish timer delivery from fetch delay. Same
    # returned packet 1, but different timer/notification histories.
    late_timer = {"timer_entry": 6, "notification": 6, "fetch_entry": 6}
    late_fetch = {"timer_entry": 3, "notification": 3, "fetch_entry": 6}
    assert latest(late_timer["fetch_entry"]) == latest(late_fetch["fetch_entry"]) == 1
    assert late_timer["fetch_entry"] - late_timer["notification"] == 0
    assert late_fetch["fetch_entry"] - late_fetch["notification"] == 3
    # Timely OS fetch followed by late user read is different again; kernel
    # returns 0 then 1, while the client can skip once its ring overflows.
    assert [latest(t) for t in (3, 6)] == [0, 1]
    # Coalesced notification explicitly records progress >1 packet. Returning
    # the newest packet does not authorize replay from a reused older slot.
    assert latest(9) - latest(3) == 2
    assert not retained_packet(9, 0) and not retained_packet(9, 1)
    assert retained_packet(9, 2)
    # Entry and lock-acquired timestamps distinguish waiting on position
    # serialization from a late OS call. Return time is never sample time.
    entry, locked, returned = Fraction(6), Fraction(8), Fraction(81, 10)
    assert locked - entry == 2 and returned > locked
    # Sweep every microsecond around slot reuse; no partially overwritten
    # packet is reported safe by the model.
    for us in range(5000, 10001):
        t = Fraction(us, 1000)
        end = position(t)
        for packet in range(3):
            safe = retained_packet(t, packet)
            if end > (packet + 2) * 128:
                assert not safe
    # r17 selection. r16 packet 55 pattern in this model: last read 0; the
    # fetch's own update reaches 8 ms = 384 frames = start of packet 3, the
    # exact reuse boundary of packet 1's slot, so packet 1 is still intact.
    assert select(8, 0) == (1, True)       # sample returned 2 and dropped intact 1
    assert select(8, 1) == (2, False)      # MoreData follow-up
    assert select(9, 0) == (2, False)      # packet 1's slot partly reused: skipped
    assert select(3, -1) == (0, False)     # first read
    assert select(5, 0) is None            # nothing new
    # Every returned packet is intact at the fetch time; only overwritten
    # packets are passed over, at microsecond resolution.
    for us in range(2667, 20001):
        t = Fraction(us, 1000)
        for last in range(-1, latest(t)):
            packet, more = select(t, last)
            assert retained_packet(t, packet)
            assert all(not retained_packet(t, p) for p in range(last + 1, packet))
            assert more == (packet < latest(t))
    print("Capture timing model passed: boundaries, slot reuse, delayed notification/fetch, reader retention.")
    print("r17 selection returns the oldest intact unread packet; only overwritten packets are skipped.")
    print("128 frames = 2.667 ms; 1 ms notification quantization leaves less than one period for OS fetch.")
    print("Capture rows alone cannot distinguish a late notification from a late OS fetch; correlate scheduler readiness.")


if __name__ == "__main__":
    checks()
