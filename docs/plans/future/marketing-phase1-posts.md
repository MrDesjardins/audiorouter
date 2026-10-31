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

## Next: r/Windows11 Sunday thread

Status: not posted. The owner confirmed that r/Windows11 only allows this
in the Software Showoff Sunday thread. Find the current thread using the
[Sunday thread search](https://www.reddit.com/r/Windows11/search/?q=author%3Afroggypwns+Software+Showoff+Sunday&restrict_sr=on&sort=new&t=week),
check its instructions and comment there; do not create a standalone post.
Wait until [PR #84](https://github.com/MrDesjardins/audiorouter/pull/84) is
merged and the revised site is live. Prepare a simple screenshot of an
application source → EQ → headphones, and be available to answer replies.

Draft comment (review in your own words before posting):

> Hi, I'm the developer of AudioRouter, a free, open-source audio router for
> Windows 11.
>
> One thing I wanted to do was capture audio from a single app, run it through
> an EQ, and send it to my headphones without affecting every other app.
> AudioRouter lets you build that route on a canvas and adjust it while it's
> playing.
>
> It's an early preview. The installer is unsigned, and sending audio into an
> app as a microphone still requires a virtual cable such as VB-Cable. I built
> it with substantial AI coding assistance and continue to test and maintain it.
>
> Project: https://github.com/MrDesjardins/audiorouter
> Website: https://audiorouter.org
>
> If you try the per-app route, I'd like to know whether it's clear how to set
> it up.

## Later: r/SideProject

Status: not posted. After the Windows11 thread, wait about a week, review its
feedback, and check r/SideProject's current rules, eligible flair and any
account requirements. Those rules have not been verified. Use the same simple
screenshot, but tell the project story rather than repeating the Windows post.
Only say the website has been revised once the changes are live.

Suggested title: I built an open-source Windows audio router because I wanted to see where my sound was going

Draft post (review in your own words before posting):

> I use Windows for gaming and wanted to route my microphone and application
> audio without having to keep a mental map of several mixer panels. So I built
> AudioRouter: you connect sources, effects and outputs on a canvas and can
> adjust them while audio is running.
>
> I shared it in r/software recently. One useful piece of feedback was that my
> first screenshot showed too much at once, so I'm working on a simpler
> example. People also pointed out that the website copy felt too AI-generated;
> I've been revising it.
>
> The app is free and open source. It's still an early Windows 11 preview, the
> installer is unsigned, and routing into Discord as a microphone requires a
> virtual cable. I used AI coding assistance extensively and continue to test
> and maintain it.
>
> I'd appreciate feedback on the first impression: looking at this simple
> route, is it clear what the app does?
>
> Project: https://github.com/MrDesjardins/audiorouter
> Website: https://audiorouter.org

After each post, record its link, responses and any measurable changes in
stars or downloads; don't infer installs from votes. The original
[r/software post](https://www.reddit.com/r/software/comments/1xc69ah/audiorouter_free_open_source_visual_audio_routing/)
received 84 upvotes as reported by the owner on 2026-10-27.

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
