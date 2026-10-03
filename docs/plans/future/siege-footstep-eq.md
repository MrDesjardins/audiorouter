# Siege footstep and drone EQ (measured)

Status: user-requested 2026-10-03; recording step pending. Uses existing
tools only (Recorder, Advanced EQ, Compressor, Limiter, Input Switch, Audio
File). No new DSP is in scope unless the analysis shows a gap.

## Objective

Make footsteps and drone movement easier to hear and place in Rainbow Six
Siege on the user's headphones, without hurting direction cues or letting
gunfire and explosions get painfully loud. Derive the EQ from measurements of
the user's own game audio and headphones, not a generic preset.

## Known setup

- Game path: Siege → CABLE-B → session loopback node `siege-in` → Mixer →
  Focusrite headphone output (see the user's confirmed topology).
- Headphones: KZ ZS10 Pro in-ear monitors (4 BA + 1 DD). IEM bass depends on
  the ear-tip seal, so the correction is applied conservatively.
- Siege: Dynamic Range Low, Stun SFX muted.

## Steps

1. **Reference recordings (user).** Raw `siege-in` signal, before any EQ, with
   fixed game and Windows volumes. Labelled takes of targets (steps by distance,
   speed and floor, drone) and maskers (gunfire, breach/explosions, ambience,
   own steps), plus a silent baseline. Private audio; never committed.
2. **Analysis (local tool).** Average spectra per label; per-band
   target-to-masker ratio; onset (transient) energy for steps. Output: boost
   where targets stand out, cut where only maskers live, leave overlap alone.
3. **Headphone correction.** Published ZS10 Pro measurements to a neutral
   target, bass correction limited because of tip seal.
4. **Fit.** At most 16 Advanced EQ bands within schema limits (20–20,000 Hz,
   Q 0.1–20, ±24 dB); boosts capped near +6 dB with matching preamp cut;
   identical left/right; no deep cuts in 2–8 kHz (direction cues).
5. **Chain.** `siege-in` → Advanced EQ → Compressor (moderate, makeup) →
   Limiter → Mixer; optional Input Switch for processed vs. original. Delivered
   as an importable, undoable preset.
6. **Validation.** Replay the references through the chain (Audio File input):
   measured step-to-gunfire level gain, limiter ceiling held, then a blind A/B
   of distance/direction/floor and a real-match check.

## Results so far (2026-10-03)

16 labelled takes (24-bit/48 kHz stereo, raw `siege-in`, Dynamic Range Low)
analysed with `tools/siege-eq/analyze.mjs`; candidate EQ evaluated with
`tools/siege-eq/design.mjs` (RBJ responses identical to `crates/dsp`).

| Sound | Where it lives | Stands out above background |
| --- | --- | --- |
| Steps, same floor (walk/run/crouch) | body 125–630 Hz; scuff 4–12 kHz | scuff 20–30 dB; crouch is almost only the scuff |
| Steps above/below (hard/soft floor) | 100–400 Hz only | 10–20 dB; nothing above 1 kHz |
| Drone | 2–12 kHz, peak 3–5 kHz | 10–42 dB |
| Gunfire, next room | 630 Hz–8 kHz, strongest 1–5 kHz | — |
| Own steps | 63–315 Hz | masks steps above/below; EQ cannot separate |
| Below 50 Hz | rumble only | — |

Game audio peaks at −18 to −37 dBFS, so the EQ's +4.9 dB peak boost
needs no preamp.

Candidate (8 of 16 bands): game layer HP 50 Hz Q 0.707; PK 200 Hz +2 Q 1;
PK 1.6 kHz −3 Q 1; PK 7 kHz +5 Q 0.9. Headphone layer (three AutoEq fits of
the ZS10 Pro agree; narrow fit-dependent corrections above 4 kHz omitted):
PK 165 Hz −3.5 Q 0.65; PK 770 Hz +3 Q 1; PK 2.1 kHz −4 Q 2.5; PK 3.2 kHz
+3.5 Q 2.8.

Measured effect of the game layer: the 4–12 kHz cue band rises about 4.5 dB
while the 1–2.5 kHz gunfire band falls 2–3 dB (about 7 dB relative). A-weighted
broadband level versus gunfire improves only 0.1–1.1 dB per take, because one
loud band dominates broadband level. Larger gains need dynamics (compressor
after the EQ, limiter last), which is the next validation step.

Applied 2026-10-03 at the user's request through the local HTTP API
(`sessions.duplicate`, `graph.plan`/`graph.commit`, no warnings): session
`patrick-main-siege-footstep` ("Patrick Main Session (Siege Footstep EQ)",
revision 1) replaces Siege Advanced EQ with Siege Footstep EQ → Siege
Compressor (−42 dB, 3:1, 3/120 ms, knee 6, makeup +6) → Siege Limiter
(−1 dB) feeding the same Mixer and Game Sound Out. The original session is
unchanged (revision 450).

User listening (v1): footsteps audible but "very muffled overall". Cause: the
user's previous EQ had a +6 dB shelf from 1 kHz, while v1 cut 1–2.5 kHz by
up to 7 dB (gunfire cut plus full headphone peak cut) and the 3 ms compressor
attack dulled step transients. v2 (applied live via `nodes.set`, revision 3):
200 Hz +4; the 1.6 kHz cut replaced by a +3 dB high shelf at 3 kHz; 7 kHz
+6; 2.1 kHz headphone cut −2; compressor attack 15 ms. Response +6 to +9 dB
at 3–10 kHz; peak +9 dB is safe at −18 dBFS game peaks with the limiter.
Trade-off: A-weighted steps versus gunfire change −0.8…+0.3 dB (gunfire is
brighter too); drones +1.1…+1.4 dB. Awaiting the user's v2 listening result.

## Compressor simulation with the engine DSP (2026-10-03)

`cargo run -p audiorouter-dsp --example siege_chain --release -- <folder>
[--grid] [--try thr,ratio,attack,release,makeup ...]` runs the takes through
the engine's own `ParametricEq` (v2 bands), `Compressor` (knee 6 dB) and
`PeakLimiter` (−1 dBFS, 5 ms lookahead, 100 ms release) in 128-frame blocks.
Metric: power mean of the quieter half of each take's events (50 ms RMS
windows ≥6 dB above its floor) minus the gunfire take's 95th-percentile
window. "Step squeeze" is the median compressor gain reduction during step
events.

| Chain | Steps − gunfire | Drones − gunfire | Step squeeze | Gunfire GR | Ambience |
| --- | --- | --- | --- | --- | --- |
| Raw | −9.5 dB | −10.2 dB | — | — | 0 |
| EQ v2 only | −10.3 | −12.1 | — | — | +0.8 |
| v2 chain: −42 dB, 3:1, 15/120 ms, +6 | −4.8 | −6.0 | 0.5 | 7.1 | +6.7 |
| Balanced: −42, 6:1, 10/120 ms | −3.0 | −3.5 | 1.1 | 9.5 | +6.4 |
| Maximum: −42, 6:1, 5/80 ms | −1.7 | −2.9 | 1.4 | 9.8 | +6.3 |
| −46, 4:1, 10/120 ms | −1.1 | −2.1 | 4.1 (too much) | 11.6 | +4.5 |

The current v2 chain already improves every step take by +3.7 to +5.6 dB
relative to gunfire, confirming the user's listening. A 452-setting grid
(threshold −50…−34, ratio 2–6, attack 5–30 ms, release 80–200 ms, makeup
4–10 dB; limits: step squeeze ≤2 dB, peak ≤ −1 dBFS) ranks 6:1 with 5 ms
attack first. Makeup gain does not change the balance, only overall level and
background. Thresholds below −42 dB start compressing the steps themselves.
Recommendation: try **Balanced** first (keeps step clicks, which a 3 ms attack
dulled in v1), then Maximum if steps still need help; listen for pumping with
the 80 ms release. Not yet applied to the session.

## Risks

Boosting step bands also boosts guns that share them; gains are bounded and
dynamics do part of the work. Siege's mix and patch changes can move the
target. Hearing safety: the Limiter is mandatory on this path.

## Next action

User records the reference takes (checklist in the active conversation and
this plan's step 1); then build the analysis tool.
