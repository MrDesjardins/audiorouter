# Marketing plan for audiorouter.org

Status: proposed 2026-10-20 at the owner's request. Nothing here is
implemented yet. Tasks that change the site, the repository settings or the
release process are ordinary changes and follow `AGENTS.md`; posting to
outside communities is done by the owner, from the owner's own accounts.

## Objective

Get AudioRouter in front of the Windows users who already have the problem it
solves, turn visits into installs that work on the first try, and turn happy
users into stars, issues, word of mouth and sponsors. Budget is the owner's
time (assume 3–4 hours a week) and close to $0 in cash.

## Where we start (2026-10-20 baseline)

| Measure | Value | Source |
| --- | --- | --- |
| GitHub stars / forks | 5 / 1 | `gh repo view` |
| Installer downloads per release | 4–9 (v0.0.13–v0.0.20; 84 in total). Counting every release file gives 18–36, but the other files are mostly fetched by tools, not people | `setup.exe` asset `download_count` |
| Repository views, last 14 days | 82 views, 16 unique visitors | GitHub traffic API |
| Top referrer | reddit.com (6 unique), then github.com, patrickdesjardins.com | GitHub traffic API |
| Repository description, topics, website | empty, none, none | `gh repo view` |
| Site social tags (`og:*`), sitemap, robots.txt | none | `site/` |
| Videos | overview (2 min) and first route, on YouTube | `README.md` |
| Installer | unsigned, so Windows SmartScreen warns | `docs/operations/distribution.md` |

The product is ahead of its audience: 30+ tools, VST hosting, two-PC
streaming, Stream Deck, API/CLI/MCP, guides with playable audio. The work
below is mostly about being findable and about removing first-run friction.

## Positioning

**One line:** a free, open source visual audio router for Windows 11. Draw
your audio on a canvas instead of juggling five control panels.

**Who it is for, in priority order** (each has a ready example in the README
or on the site):

1. **Streamers and Discord users** who want a clean microphone (gate,
   denoise, EQ, compressor) sent to Discord, OBS and headphones at once.
2. **Two-PC streamers** who need game audio on the streaming PC without extra
   software (Network Send/Receive).
3. **Competitive gamers** who want footsteps louder (the Siege guide is a
   concrete, shareable result).
4. **Podcasters and home recordists** who want a safety recording beside the
   live route.
5. **Tinkerers and developers**: Rust real-time audio, local HTTP API, MCP
   server for AI assistants, Stream Deck.

**What people will compare it with:** Voicemeeter (Banana/Potato), SteelSeries
Sonar, Elgato Wave Link, Equalizer APO with Peace, Light Host, OBS filters.
The honest differentiators: a visual graph rather than fixed strips; free and
open source with no account or telemetry; VST plugins isolated in their own
process; per-application capture; two-PC streaming built in; scriptable.
The honest gaps, said up front: Windows 11 only, early preview, unsigned
installer, still needs VB-Cable to feed other apps.

**Rules for every message:** say you are the author; describe what it does,
not superlatives; show a picture or a 20-second clip; link the site, not a
direct download; never disparage a competitor; never ask for upvotes; never
use second accounts or invented reviews.

## Phase 0 — Foundations (week 1, about 4 hours)

Make every visit count before sending traffic. All of these are cheap and
permanent.

| # | Task | Done when |
| --- | --- | --- |
| 0.1 | Set the GitHub description, website (`https://audiorouter.org`) and topics (`windows`, `audio`, `audio-routing`, `virtual-audio`, `voicemeeter-alternative`, `obs`, `discord`, `streaming`, `vst`, `rust`, `tauri`, `dsp`) | `gh repo view` shows them |
| 0.2 | Upload a 1280×640 social preview image (the streamer voice chain screenshot with the tagline) | link previews on Discord/Reddit show the image |
| 0.3 | Add `og:title`, `og:description`, `og:image`, `twitter:card` to every page in `site/`, plus `sitemap.xml` and `robots.txt` | pasting the link in Discord shows a card |
| 0.4 | Register audiorouter.org in Google Search Console and Bing Webmaster Tools; submit the sitemap | both show the site indexed |
| 0.5 | Give each tool page and guide a title that matches what people search ("Noise gate for Discord microphone on Windows", "Hear footsteps better in Rainbow Six Siege") | titles reviewed against real search phrasing |
| 0.6 | Write a "Coming from Voicemeeter?" page: the same three common setups built in AudioRouter, side by side, fair and factual | page live, linked from the home page |
| 0.7 | Add a short "Windows says the installer is unsigned" section to the download page with a screenshot of the SmartScreen steps and why (no certificate yet, link to sponsorship) | a first-time user is not surprised |
| 0.8 | Enable GitHub Discussions (Q&A, Show your route, Ideas) so newcomers have a place that is not an issue | Discussions linked from the README and site |
| 0.9 | Record the numbers weekly with `node tools/marketing/snapshot-metrics.mjs`, which appends to `docs/marketing-metrics.csv`; GitHub keeps traffic only 14 days | first row committed |

## Phase 1 — Soft launch in small communities (weeks 2–4)

Goal: 20 real users giving feedback, and finding out which message lands.
Post one community at a time, at most two a week, and stay in the thread for
the first few hours to answer every comment. Read each community's
self-promotion rules first; several require a history of ordinary
participation, a flair, or a weekly thread.

| Week | Where | Angle |
| --- | --- | --- |
| 2 | r/software, r/Windows11, r/opensource | "I built a free visual audio router for Windows 11" + the overview video |
| 2 | Owner's blog (patrickdesjardins.com, already a referrer) | Why I built AudioRouter, with screenshots; cross-post to dev.to |
| 3 | r/Twitch, r/obs (check rules; often weekly self-promo threads) | Clean mic to OBS and Discord with one route; two-PC game audio without extra software |
| 3 | r/Rainbow6 or r/SiegeAcademy | The Siege footstep guide itself, with the before/after audio; the app is the footnote |
| 4 | r/podcasting, r/streaming | Live route plus a safety recording |
| 4 | r/rust ("What's everyone working on this week" thread or a project post), Tauri Discord showcase | Engineering angle: real-time audio callback rules, plugin isolation |

After each post, record in the metrics file: post link, upvotes/comments,
site visits that day (Search Console or GitHub referrers), downloads delta,
new stars, issues opened. Turn every repeated question into a manual or FAQ
entry the same week.

## Phase 2 — Be findable everywhere people look (weeks 3–8, in parallel)

Listings are free, permanent and keep sending visitors without effort.

| Task | Notes |
| --- | --- |
| AlternativeTo listing, marked as an alternative to Voicemeeter, SteelSeries Sonar, Wave Link, Equalizer APO | People search "Voicemeeter alternative" constantly; this is likely the single best listing |
| winget manifest (`winget-pkgs`), Scoop `extras` bucket | Check each repository's current rules for unsigned per-user installers before submitting; automate the version bump in the release workflow afterwards |
| awesome lists: awesome-tauri, awesome-rust (applications), awesome-audio / awesome-streaming lists | One pull request each, following each list's format |
| Tauri showcase, This Week in Rust (project updates) | Free reach to developers who star and share |
| Softpedia, MajorGeeks, Neowin software submissions | Windows download sites; submit once, they list updates |
| Stream Deck: link the plugin from the site and consider the Elgato Marketplace once the plugin is stable | Elgato's marketplace has its own review rules |

## Phase 3 — Content engine (from week 5, ongoing)

One small piece a week beats a big launch that is never repeated. Each piece
should solve a searchable problem and end with "AudioRouter does this for
free".

**Short videos (30–60 s, YouTube Shorts / TikTok / Reels):** reuse the
`videos/` HyperFrames setup to produce them.

1. Clean up a noisy mic for Discord in 30 seconds.
2. Send game audio to your streaming PC without extra software.
3. Hear footsteps first in Siege (before/after).
4. Record your podcast while you stream it.
5. A crashing VST plugin does not take your audio down.
6. Ask ChatGPT to build your audio route (MCP).
7. Fun Voice Changer presets on a live mic.

**Written guides on the site** (already a strength: the guides play audio):
one per week, each targeting one search phrase, linked from the tool pages.

**Developer posts on the owner's blog** (Hacker News and r/rust material):
real lessons already recorded in `AGENTS.md`, rewritten for outsiders, for
example "The CoTaskMem leak that cost 9 MiB an hour", "Why our audio callback
never takes a lock", "Windows changes your audio endpoint IDs on every driver
update", "Animating 8 live connections cost 135% of a GPU core".

**Release notes as marketing:** each release gets a 2–3 line "what you can
do now" summary and one GIF, posted to Discussions and, for notable releases,
to the communities that asked for that feature.

## Phase 4 — Bigger launch (gated)

Show HN and Product Hunt give one large spike each; do them once the first
run is smooth, not before. Gate:

- [ ] Installer signed (the SignPath Foundation free OSS signing already
      under investigation, or the sponsored certificate), so SmartScreen does
      not warn.
- [ ] Clean-machine, upgrade and uninstall checks done (open in the release
      plan).
- [ ] At least 10 outside users who are not friends, and no open crash issue
      older than a week.
- [ ] Coming-from-Voicemeeter page, FAQ and a 60-second demo ready.

Then: **Show HN** on a weekday morning US Eastern ("Show HN: AudioRouter – a
visual audio router for Windows, in Rust"), with the owner present all day;
**Product Hunt** a week later; then a short email to the Windows/streaming
press (Neowin, Windows Central, streaming YouTubers who review audio tools)
with the video, three screenshots and a one-paragraph pitch. Offer creators
help setting up their own route, never payment.

## Sponsorship

The current ask is concrete (the driver certificate). Keep it concrete:
show a progress bar on the Sponsors page and on the site, thank sponsors in
release notes (with permission), and post an update when the certificate is
bought. Mention it once at the end of posts, never in the title.

## What we will not do

Paid ads, buying stars or reviews, posting from more than one account,
vote requests, posting the same text to many communities in one day,
negative comparisons, telemetry to measure marketing (the product promise is
no telemetry; measure from GitHub, Search Console and post analytics only).

## Measures and review

Review the metrics file every 4 weeks and drop the channels that do not move
anything.

| Measure | Week 4 target | Week 12 target |
| --- | --- | --- |
| Installer downloads per release | 25 | 75 |
| GitHub stars | 30 | 150 |
| Unique repository visitors per 14 days | 100 | 500 |
| Search Console clicks per week | first clicks | 100 |
| Outside issues/discussions opened | 5 | 25 |
| Sponsors | 1 | certificate funded |

These targets are guesses to compare against, not commitments; revise them
after the first review.

## Risks

| Risk | Mitigation |
| --- | --- |
| SmartScreen warning scares visitors away | Explain it on the download page (0.7); pursue signing before Phase 4 |
| A post brings users who hit a bug and leave | Soft launch first; answer threads quickly; Discussions for help |
| Removal or ban for self-promotion | Read rules, participate first, one community at a time, disclose authorship |
| Support load grows beyond the owner's time | FAQ from repeated questions; Discussions so users help each other |
| "Needs VB-Cable" confuses people | One clear sentence and a picture wherever routing to other apps is mentioned |

## Next action

Done 2026-10-20:

- 0.1: description, 14 topics and website link set on GitHub.
- 0.2: custom social preview uploaded (`usesCustomOpenGraphImage` is true);
  the same image is `site/assets/social-preview.png`.
- 0.3: `site/generate-social.mjs` adds canonical, Open Graph and Twitter
  card tags to every page and writes `sitemap.xml` and `robots.txt`; the
  Pages workflow runs it on every deployment. Four pages that had no meta
  description now have one.
- 0.6: `site/compare.html`, "Compare" in the top menu and footer;
  Voicemeeter facts from vb-audio.com, checked 2026-10-20.
- 0.7: "Why Windows warns" section on the download page, with a checksum
  check against `SHA256SUMS.txt`.
- 0.9: `tools/marketing/snapshot-metrics.mjs` and the baseline row in
  `docs/marketing-metrics.csv`.

- 0.8: GitHub Discussions enabled and linked from the README and the site
  footer.

Next, for the owner: 0.4 (register the site in Google Search Console and
Bing Webmaster Tools and submit `https://audiorouter.org/sitemap.xml`; both
need the owner's account), then 0.5
(search-friendly guide titles in `site/generate-guides.mjs`) and Phase 1.
