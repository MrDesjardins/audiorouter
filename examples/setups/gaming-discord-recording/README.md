# Gaming, Discord and clip recording with three virtual cables

A working AudioRouter setup for playing a game with Discord voice chat while a
clip recorder (Outplayed in this example) captures your voice, the game and
Discord, with your voice heard once.

Verified on 2026-10-03 on Windows 11 with Rainbow Six Siege, Discord and
Outplayed. If you use different applications, follow the same rules for each
cable.

## What you need

- A microphone. This setup uses a USB podcast microphone (PD200X).
- Headphones. This setup uses a Focusrite Scarlett interface.
- Three VB-Audio virtual cables: **VB-Cable** (named "CABLE"), **VB-Cable A**
  and **VB-Cable B**.

## The rule that makes it work

Each cable carries one kind of audio, and **your voice is on exactly one
cable**:

| Cable | Carries | Who plays into it | Who listens to it |
|---|---|---|---|
| **CABLE-A** | Your processed voice only | AudioRouter | Discord mic, game voice-chat mic, recorder mic |
| **CABLE-B** | Game audio only | The game | AudioRouter |
| **CABLE** | Discord + game, **no voice** | Discord, and AudioRouter (game copy) | The recorder's system audio |

Your headphones get the full mix (game + Discord + your voice) from
AudioRouter.

A recorder records a microphone and a system-audio device. If both contain your
voice, it records your voice twice, slightly out of time, and sounds like an
echo. Typical causes are recording the headphone mix together with the mic, or
recording CABLE-A as system audio while the mic is also on. Keeping voice off
CABLE prevents this.

## Windows settings

Sound settings:

| Setting | Device |
|---|---|
| Default playback | Speakers (Focusrite USB Audio) — your headphones |
| Default recording | CABLE-A Output |

Since the default playback is your headphones, notifications and browser audio
go to your ears and not to your teammates or your clips. Never make CABLE-A
Input the default playback device, or every app's sound is sent through your
mic.

## Application settings

| Application | Input (microphone) | Output (speaker) |
|---|---|---|
| Discord | CABLE-A Output | CABLE Input |
| Siege (game) | CABLE-A Output | CABLE-B Input |
| Outplayed (recorder) | CABLE-A Output | System audio: CABLE Input |

Discord plays into CABLE and not into your headphones. AudioRouter captures
Discord with an application capture and puts it in your headphone mix, so you
hear it once.

If your recorder also has a separate "game audio" or game-capture option, turn
it off. The game is already in CABLE.

## AudioRouter session

```text
Voice
  Microphone (PD200X)
    → Spectral Gate → Advanced EQ → Compressor → Gate → Meter
        → Output: CABLE-A Input ............ Discord, game and recorder mic
        → "Game and Mic" mixer

Game
  Input: CABLE-B (what the game plays)
    → Siege Advanced EQ
        → Output: CABLE Input .............. recorder system audio (with Discord)
        → "Game and Mic" mixer

Headphones
  "Game and Mic" mixer → Duck (bypassed)  ┐
  Discord.exe application capture ────────┴→ "Discord, Game, Mic" mixer
        → Output: Speakers (Focusrite USB Audio)
```

| Node | Kind | Purpose |
|---|---|---|
| Microphone (PD200X) | Input | Your mic, mono. |
| Spectral Gate | Spectral gate | Removes steady background noise using a learned noise profile (reduction 40 dB). |
| Advanced EQ 1 | Parametric EQ | Voice tone: high-pass at 70 Hz, +5 dB at 117 Hz (warmth), +6 dB at 3 kHz (presence), +2 dB at 10.3 kHz (air). |
| Voice Compressor | Compressor | Evens out loudness: threshold −8 dB, ratio 3.3:1, attack 10.6 ms, release 154 ms, +1.2 dB makeup. |
| Voice Gate | Gate | Mutes the mic between words: threshold −31 dB, range 58 dB, hold 50 ms. |
| Meter 1 | Meter | Shows the level sent to CABLE-A. |
| Voice to CABLE-A | Output → CABLE-A Input | Your processed voice for Discord, the game and the recorder. |
| Siege game | Input ← CABLE-B | The game's audio. |
| Siege Advanced EQ | Parametric EQ | Game EQ that brings out footsteps. Tune it to taste. |
| Game Sound Out | Output → CABLE Input | Copy of the game for the recorder. Discord already plays here. |
| Game and Mic | Mixer | Game + your voice for the headphones. |
| Duck Siege | Duck | Optional game ducking, bypassed in this setup. |
| Discord.exe capture | Application capture | Discord's audio for the headphones. |
| Discord, Game, Mic | Mixer | Full headphone mix. |
| Headphone (Scarlett) | Output → Focusrite speakers | What you hear. |

## Check it

1. Talk while the game is playing sound. You hear yourself once in the
   headphones, with the game and Discord.
2. Ask someone in Discord whether they hear your voice and nothing else (no game
   sounds or notifications).
3. Record a short clip, talking over game sound. Play it back: your voice
   should be heard once, without echo, with the game and Discord.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Voice twice or echo in clips | Two recorder sources both contain your voice | Recorder mic = CABLE-A Output, system audio = CABLE Input; nothing else |
| No game in clips | Nothing sends the game to CABLE | Add the Output node to CABLE Input fed from the game EQ |
| Teammates hear notifications or music | CABLE-A Input is the Windows default playback | Set default playback to your headphones |
| Discord twice in your headphones | Discord's speaker is your headphones and the application capture also adds it | Set Discord's output to CABLE Input |
| No game in your headphones | The game isn't playing into CABLE-B | Set the game's output device to CABLE-B Input |
