# Android phone audio companion

**Status (2026-10-11):** future proposal added at the user's request. Android
is the first target. This document records a possible app; it does not
authorize implementation or distribution.

## Objective

Send audio captured on an Android phone into an AudioRouter session using the
existing **Network Receive** node. The first slice is the phone microphone,
useful as a temporary wireless input. Reuse the existing `ARNA` network-audio
protocol where possible; do not add a separate desktop driver or change the
network node's audio contract without evidence that the existing one cannot
serve the use case.

Primary flow:

```text
Android microphone → companion app → authenticated UDP on trusted LAN
                    → Network Receive → AudioRouter graph
```

AudioRouter already supports mono/stereo float32 audio at 48 kHz, sequence and
stream IDs, a bounded jitter buffer, packet-loss concealment, replay rejection,
and optional HMAC authentication. The receiver accepts a configured sender IP
and UDP port; the phone needs the PC's private-network address and receiver
port, while AudioRouter must accept the phone's current sender address. See
[GRAPH-16](../../spec/04-graph.md) and the portable protocol implementation in
`crates/protocol/src/network_audio.rs`.

## Proposed scope

### Phase 1 — Android microphone

- Native Android app with explicit microphone permission, clear capture state,
  and an obvious stop control. Any background capture must use the current
  Android foreground-service and privacy-indicator requirements.
- Configure the PC's private IPv4/IPv6 address, Network Receive port, and the
  same optional pairing key used by the node. A QR/setup exchange may simplify
  entry, but it must not expose the pairing key in logs or diagnostics.
- Send only to the configured PC and port; accept no commands, files, or
  executable payloads from the network.
- Handle Wi-Fi changes, app pause/stop, permission revocation, phone sleep,
  packet loss and clock drift without a stuck microphone or unbounded queue.
- Work on a trusted local network only. The current pairing key authenticates
  packets; it does **not** encrypt audio. Do not expose this stream to the
  public internet or imply that pairing provides confidentiality.

### Phase 2 — Android playback capture (optional)

Evaluate capturing other apps' playback using Android's user-approved
`MediaProjection` / `AudioPlaybackCapture` flow. This must be opt-in and
separately labeled from microphone capture. It is limited by Android's usage
and capture-policy rules; some source apps can prohibit capture. Do not promise
universal capture. Recheck platform requirements when this phase is promoted.

### Later platform exploration

iOS is out of the first milestone. If requested, first test whether ReplayKit
can provide the intended app/broadcast audio flow without implying arbitrary
cross-app capture. Do not make iOS parity a dependency for the Android release.

## Requirements and prerequisites

- Related contracts: GRAPH-16, SEC-13, ARCH-03, CAP-01, and UI-17. Revisit the
  stable specifications only if implementation exposes a contract gap.
- Android SDK/Gradle and a supported physical Android test phone; simulator
  tests alone cannot establish microphone, Wi-Fi roaming, background, or
  latency behavior.
- Protocol compatibility must be tested against the existing Rust
  implementation, including authenticated and unauthenticated modes, malformed
  datagrams, replay handling, packet loss, sequence wrap, and clock drift.
- Confirm minimum Android version, whether microphone-only is sufficient for
  the first release, onboarding/pairing UX, and distribution channel at
  milestone promotion. Check current store policies and fees then; this plan
  assumes neither a free public listing nor a particular store.

## Acceptance evidence

1. Android microphone audio reaches a configured Network Receive node and can
   feed an ordinary AudioRouter route without changing the PC's default devices.
2. Matching pairing keys pass; missing/wrong keys and replayed/malformed
   packets are rejected by the existing receiver and produce understandable
   diagnostics without leaking key material.
3. The phone shows an unmistakable active-capture indicator and stops capture
   promptly when stopped, permission is withdrawn, or its capture session ends.
4. A repeatable tone/signal test records end-to-end latency, packet loss,
   jitter-buffer depth and clock-drift behavior on Wi-Fi. State the measured
   network and phone conditions; do not claim a universal latency target from
   a single device.
5. Wi-Fi disconnect/reconnect and phone sleep/restart leave no stale stream
   presented as live audio and recover only through a bounded, visible state.

## Risks and rollback

Risks include Wi-Fi jitter/roaming, battery use, mobile background limits,
source-IP changes, mobile OS capture restrictions, pairing-key handling, and
the unencrypted nature of the current wire protocol. Keep the initial feature
LAN-only and microphone-only. If the companion app is removed or disabled, the
desktop Network Receive node remains usable with other compatible senders;
no session migration should be needed.

## Ordered implementation outline (only after milestone approval)

1. Prototype the existing packet protocol on an Android device and measure
   microphone capture-to-receiver behavior on a controlled local network.
2. Choose native implementation and protocol-sharing strategy; add cross-side
   conformance fixtures so Android and Rust cannot silently drift.
3. Build the microphone capture, authenticated UDP sender, explicit state and
   reconnect handling; keep platform capture and networking off the realtime
   audio callback.
4. Add setup guidance and any narrowly scoped QR/pairing flow, preserving the
   Network Receive node as the desktop routing/configuration authority.
5. Complete physical-device privacy, lifecycle, protocol, three-network
   condition, battery, and latency checks; document supported phone versions.
6. Evaluate Android playback capture and iOS only as separately approved work.

## Platform references

- [Android audio playback capture](https://developer.android.com/media/platform/av-capture): user-approved `MediaProjection`, capture requirements and source-app policy.
- [Apple ReplayKit](https://developer.apple.com/documentation/replaykit): app and microphone audio recording/broadcasting; consult only if an iOS follow-up is approved.
