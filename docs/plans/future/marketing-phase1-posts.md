# Marketing Phase 1: first posts (drafts)

Drafts for the owner to review, adjust and post from their own accounts.
Part of the [marketing plan](marketing.md), Phase 1. Lines marked
**[confirm]** state something personal that only the owner can confirm.

## Before posting

- **Read each subreddit's rules** in its sidebar (Reddit blocks automated
  reading, so they were not checked here). Look for: a self-promotion rule, a
  required flair (often "Self-promotion" or "Showcase"), a weekly thread for
  projects, and a minimum account age or karma. If the rules say "weekly
  thread only", post there instead.
- **Post on a weekday morning, US Eastern time** (9–11 am), when you can stay
  for the next 2–3 hours to answer every comment. Early replies matter more
  than the post itself.
- **One subreddit per week.** Never paste the same text into several
  subreddits on the same day.
- **Say you're the developer** in the first lines, and ask for feedback, not
  for upvotes.
- **Image post, details in the first comment.** A picture of a route stops
  the scroll better than a text wall. Use `docs/images/streamer-voice-chain.png`
  (or a short screen recording of a route lighting up while you talk), then
  immediately add the comment below.

## Post 1: r/software (week 1)

**Rule (from the sidebar, 2026-10-21):** own software may be promoted only
if it is open source (permanent ban otherwise); closed software only on
Wednesdays with the right flair and something of value. AudioRouter is
GPL-3.0, but to be safe: post on a **Wednesday**, use the self-promotion
flair, keep "open source" in the title and put the GitHub link first.

**Title** (pick one):

1. I made a free, open source audio router for Windows 11 where you draw your audio instead of juggling control panels
2. AudioRouter (free, open source): visual audio routing for Windows. Mic → noise gate → EQ → Discord, headphones and a recording at once

**Image:** the streamer voice chain screenshot.

**First comment** (post it right away):

> Hi, I'm the developer. I game and stream on Windows, and every audio setup
> I tried meant either decoding mixer strips and bus buttons, or clicking
> through several Windows sound panels. I wanted to *see* where my audio goes
> and change it while it plays, so I built AudioRouter. I use it every day
> for my own setup.
>
> You put sources on the left (microphone, a game, Spotify, a file), outputs
> on the right (headphones, Discord through a virtual cable, a recorder) and
> tools in between. The lines are the routing, and they pulse with the level
> while it plays. Everything is live, so you can tweak an EQ while you talk.
>
> What's in it:
>
> - 30+ built-in tools: noise gate, denoise, de-hum, a 16-point EQ,
>   compressor, limiter, pitch, a voice changer, recording to WAV/FLAC/MP3
> - Your own VST2/VST3 effects, each in its own process, so a crashing plugin
>   can't take your audio down
> - Capture a single app (just the game, just the browser)
> - Send audio between two PCs on your network (gaming PC → streaming PC)
> - Stream Deck plugin, a local HTTP API and a command line
>
> It's free, GPL-3.0, with no account and no telemetry.
>
> Honest limits: Windows 11 x64 only; it's an early preview, so expect rough
> edges; the installer isn't code-signed yet, so Windows shows a SmartScreen
> warning (the download page explains it); and sending audio *into* Discord or
> OBS still needs a virtual cable such as VB-Cable.
>
> Source (GPL-3.0): https://github.com/MrDesjardins/audiorouter · Site:
> https://audiorouter.org · 2-minute video: https://youtu.be/IHitHqpi87s
>
> I'd love feedback on what's confusing when you first open it.

## Post 2: r/Windows11 (week 2, after reading what worked in week 1)

**Title:** Windows 11 lets you pick an output per app, but not process it. I built a free tool that does: per-app capture, EQ and routing on one canvas

Check that this title is accurate on your PC before using it: Windows 11's
**Settings → Sound → Volume mixer** sets an output device per app, but offers
no EQ or noise cleanup per app.

**Image:** a route with an Application source (a game or browser) → EQ or
Volume → headphones, ideally beside the Windows volume mixer.

**First comment:** reuse the r/software comment, but lead with per-app
capture and "never changes your Windows default devices or volumes", which
Windows users care about most.

## dev.to cross-post (any time this week)

Don't write a new "why I built it" post: the blog already has six
AudioRouter posts. Cross-post the user-facing one,
[AudioRouter: A Visual Audio Router for Windows, With an API and an MCP Server Built In](https://patrickdesjardins.com/blog/audiorouter-tool-tour-canvas-api-and-mcp),
to dev.to so it reaches a new audience without competing with the original
in search:

1. On dev.to, **Create Post**, paste the article as Markdown.
2. In the post settings (the gear icon), set **Canonical URL** to the blog
   URL above. This tells search engines the blog is the original.
3. Tags (max 4): `windows`, `audio`, `opensource`, `rust`.
4. Add a line at the top: "Originally published on my blog."

Later, the engineering posts (memory diet, JSON-RPC backend, building with
agents) are the material for r/rust and a Show HN, per Phase 4 of the plan.

## Ready answers for common questions

Edit them into your own words; copy-pasted replies read as canned.

- **"Why not just use Voicemeeter?"** Voicemeeter is great and more mature,
  especially for ASIO and MIDI. AudioRouter is for people who want to see the
  route and change it freely, with voice cleanup and crash-safe plugins built
  in. Side by side: https://audiorouter.org/compare.html
- **"Is the unsigned installer safe?"** A code-signing certificate costs
  money every year; sponsorships will pay for it. Every release lists SHA-256
  hashes, and the download page shows how to check them. The source is on
  GitHub.
- **"Why do I still need VB-Cable?"** Windows only lets an app appear as a
  microphone to other apps through an audio driver. AudioRouter's own
  virtual cable is in development, and Microsoft must sign it, which needs a paid
  certificate. Until then, VB-Cable (free) does that part.
- **"Linux or Mac?"** No, it's built on Windows audio APIs, and Windows 11
  only for now.
- **"Latency?"** It uses Windows shared mode, like most apps. It isn't aimed
  at ASIO or DAW recording yet; for that, Voicemeeter or your interface's
  ASIO driver is the better choice.
- **"Was this made with AI?"** Be direct: yes, with AI coding agents under a
  written spec and tests, and the blog explains how
  ([what building AudioRouter taught me](https://patrickdesjardins.com/blog/what-building-audiorouter-taught-me-about-harness-and-rules)).
  Being open about it lands better than being found out.

## After each post

Within 2 days, record in [marketing.md](marketing.md) (Phase 1 table): the
post link, upvotes and comments, new stars and installer downloads (the
Monday metrics row shows the trend), issues or Discussions opened, and the
questions people repeated. Turn repeated questions into FAQ or manual
entries the same week.
