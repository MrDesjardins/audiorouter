# Marketing: second round, vibe-coding and AI-builder communities (2026-10-30)

Status: proposed 2026-10-30 at the owner's request. Part of the
[marketing plan](marketing.md), Phase 1. The owner posts from their own
accounts; agents draft, prepare assets and make repository changes. Nothing in
this file has been posted. The r/vibecoding rules below were pasted by the
owner on 2026-10-30 (verified). Rules for every other subreddit were **not**
read (Reddit blocks automated reading) and stay marked unverified.

## Objective

Reach people who care about *how* AudioRouter was built, a new audience from
the r/software post, and turn it into feedback, stars and a few installs.
Do it today without breaking the Phase 1 rules: one community per post, at most
two posts a week, authorship and AI use disclosed, no vote requests.

## Where we stand (2026-10-30)

| Measure | 2026-10-20 baseline | 2026-10-30 | Source |
| --- | --- | --- | --- |
| GitHub stars / forks | 5 / 1 | 51 / 2 | `gh api repos/MrDesjardins/audiorouter` |
| Installer downloads, latest releases | 4–9 each | v0.0.21: 108, v0.0.23: 107, v0.0.24: 105 (v0.0.22: 9, replaced within a day) | release asset `download_count` for `setup.exe` |
| Repository views, 14 days | 82 (16 unique) | 438 (179 unique); peak 121 views on 2026-10-24 | traffic API |
| Top referrers, 14 days | reddit.com 6 | audiorouter.org 30, reddit.com 25, Google 16, github.com 9, ilsoftware.it 4, Reddit app 4 (unique visitors) | traffic API |
| Outside issues | 0 | 0 | `gh api repos/.../issues` |

Reading it: the r/software post (84 upvotes) coincides with a 10x jump in
downloads and a 10x jump in stars, and Google now sends visitors. That is
correlation only; no telemetry exists to say which visit became which install.
Downloads include the owner's own and other testers' installs, and a bot or
mirror can inflate counts. Nobody outside the owner has opened an issue, so the
next useful signal is conversation, not volume.

Feedback already received on r/software (from the owner, in the posts file):
the first screenshot showed too much at once, and the website copy read as
AI-generated. The copy was revised in PR #84 (merged 2026-10-28). The simple
screenshot **does not exist yet**; `docs/images/` has only the busy
`streamer-voice-chain.png`.

Open problems this plan notices:

- `docs/marketing-metrics.csv` had only the baseline row (twice). The scheduled
  *Marketing metrics (weekly)* run on 2026-10-26 **failed** (run 40386405710)
  because main's branch rules reject the bot's push, so that week's numbers
  were never recorded and GitHub keeps traffic for only 14 days. This branch
  records today's numbers and repairs the workflow (task A1, awaiting merge).
- Main contains the Soundboard (PR #89) but release 0.0.24 does not. A visitor
  from today's post installs 0.0.24 and finds no Soundboard; do not mention it
  as available until it ships.

## Decision: what to post today

**Primary: r/vibecoding, a build-story post, not a product pitch.**

Why this one, today:

1. A different audience and a different angle from r/software. People there
   ask "did a real, finished thing come out of an AI-assisted build?" The
   evidence is in the repository: written specs in `docs/spec/`, requirement
   IDs, an `AGENTS.md` rule set, validated lessons with evidence links, CI,
   CodeQL triage. That is a concrete story the other posts only mention in one
   sentence.
2. It is honest about AI use, which is also the lesson from r/software
   ("copy felt AI-generated"): say it first, in the owner's own words.
3. It fits the cadence: r/software 2026-10-21, today (Fri), the r/Windows11
   Sunday thread on 2026-11-01 is the second post this week, then r/SideProject
   no earlier than 2026-11-08.

What to expect, so it is not judged unfairly: this audience builds apps; few
run a Windows audio router. Expect comments, stars and developer feedback, and
few installs. Do not compare its downloads with r/software's.

**r/vibecoding rules (read by the owner 2026-10-30) and what each means here:**

| Rule | What it says | Consequence for this post |
| --- | --- | --- |
| 1. No low effort posts | Keep AI use to a minimum; AI may help shape thoughts, but fully generated text is removed. "A place for humans who use AI to code" | **The draft below was written by an AI agent, so it must not be posted as written.** Treat it as a fact outline and write the post yourself, in your own sentences. This is the largest risk to the post |
| 2. Vibe coding dev tools must be approved | Vibe-coding *startups and dev tools* must be approved through the Vibe Coding community on X before one introduction post | AudioRouter is an audio app built with AI, not a vibe-coding tool, so rule 3 applies, not rule 2. If a moderator says otherwise, do not argue or go around them; skip the sub and use an alternative |
| 3. No shilling | Vibe-coded projects are welcome if the post teaches how it was built: **the tools you used, your process and workflow, code, design or build insights**. A bare link is removed. Format: "here's the project, here's how I made it" | The post is the build story with the project as the example, which is the plan's angle. Tools, process and at least one concrete insight are **required**, so the **[confirm]** lines cannot be left vague. Put the link after the story, not first |
| 4. No vibe coding pessimism | Constructive critique only; gatekeeping, insults and fundamental pessimism about building software with little code review earn a permanent ban | Describe your own process and its guardrails without implying other people's way is wrong. Say "what worked for me", not "what stops vibe coding from failing". Title option 1 is the riskiest on this point; prefer 2 or 3 |
| 5. No assholes | | Answer harsh comments once, politely, then stop |

Flair, karma and account-age requirements were not in the pasted rules; the
submit screen will show any flair that is required. The sub says it will add
flairs for approved dev tools, which do not apply to this project.

**Alternatives if a moderator removes it or the rules change:**

| Option | Notes (all unverified) |
| --- | --- |
| r/ChatGPTCoding promo thread | A recurring self-promotion thread existed in 2024–2025 snapshots; one post per project. Comment in the current thread only |
| r/ClaudeAI "Built with Claude" showcase | A megathread exists (April 2026 snapshot). A feed post needed total karma of at least 50 then; below that you are sent to the megathread. Rule 7 sets the showcase requirements. Only if the project was in fact built with Claude **[owner confirms which tools]** |
| dev.to cross-post | No community gate. Already planned in [marketing-phase1-posts.md](marketing-phase1-posts.md); safe to do the same day because it is a different channel and the canonical URL protects the blog |

**Not today, and why:**

- r/opensource and open-source subs generally: a May 2026 snapshot describes
  moderators there restricting "vibecoded" projects (minimum commit history,
  megathread-only). Read their rules first; wait.
- r/Windows11: Sunday thread only (2026-11-01), already drafted.
- r/SideProject: one week after the Windows11 thread.
- Show HN and Product Hunt: gated on the signed installer and 10 outside users
  ([marketing.md, Phase 4](marketing.md)). Not met: the installer is still
  unsigned and there are no outside issues.
- r/Twitch, r/obs, Siege subs: Phase 1 weeks 3–4; keep them in order.

## Today's timeline (owner, about 1 hour of prep plus 3 hours present)

| Step | Who | Done when |
| --- | --- | --- |
| 1. Read the r/vibecoding rules | Owner | Done 2026-10-30: project posts allowed under rule 3 with build details; no approval needed for an app (rule 2 is for dev tools) |
| 2. Take the simple screenshot: one Application source → EQ → headphones, no other nodes, dark theme, 1600 px wide | Owner (an agent must not launch a second shell; see AGENTS lessons) | File saved as `docs/images/simple-route-app-eq-headphones.png` |
| 3. Write the post yourself from the outline below (rule 1: no generated text); fill every **[confirm]** line with your real tools, workflow and one failure; read the README once as a stranger | Owner | The post contains nothing you would not say aloud, and names the tools, the process and one concrete build insight (rule 3) |
| 4. Post on a weekday morning in US Eastern time if possible; if posting this afternoon, stay at least 3 hours | Owner | Post URL recorded |
| 5. First comment (details and honest limits) within 2 minutes | Owner | Posted |
| 6. Answer every comment for 3 hours using the reply bank, in your own words | Owner | No unanswered question |
| 7. Same evening: record post link, upvotes, comments, stars, views and referrers (commands below) | Owner or agent | Row added to the Evidence table |
| 8. After 48 hours: repeat step 7, list repeated questions, turn each into FAQ or manual text | Owner + agent | Edits merged |

Optional, same day: the dev.to cross-post (steps in
[marketing-phase1-posts.md](marketing-phase1-posts.md)), after the Reddit post,
not before, so replies there are not neglected.

## Draft post (written for you to revise, not to paste)

Rule 1 removes "fully generated AI slop", and an agent wrote this draft at the
owner's request (2026-10-30), in the owner's voice, as a starting point. The
owner will revise it before posting. Revise it for real: rewrite the sentences
that are not yours, add your own examples, cut what you would not say to a
friend, and read it aloud. Moderators and readers judge by feel, and the
r/software feedback ("copy felt AI-generated") points the same way. Light AI
help on your own text is allowed; posting generated text as is, is not.

Rule 3 requires the tools, the process and a build insight, so the post cannot
lead with the project or end with just a link.

**Voice:** the draft below is written to sound like you, from your own blog
post (informal first person, short plain sentences next to long ones, colons,
"I don't want to overstate it", candid caveats, an occasional comma splice).
It is still a draft by an agent: read it aloud, change anything that does not
sound like you, and keep what is true. Facts come from your blog post
(posted 2026-10-13) and the repository; anything that may have changed since is
marked **[confirm]**.

**Title options** (pick one; rule 4 favors the constructive ones):

1. I built a free Windows audio router by directing AI agents: two $20 subscriptions, a written spec, and the mistakes that taught me the most
2. How I built AudioRouter (free, open source, Windows) with AI agents: the spec, the rules file and three failures
3. I wrote almost none of the code of my audio app. Here is what I did instead

**Image:** the simple route screenshot (step 2), with the app name and the
word "Windows" visible. The video goes in the first comment (see
"Video" below).

**Body:**

> Hi, I'm the developer. Here is the project, and here is how I made it.
>
> AudioRouter is a free, open source audio router for Windows 11. You draw your
> audio on a canvas: microphone, noise gate, EQ, then Discord and your
> headphones at the same time. You can change everything while it plays.
>
> I wrote almost none of the code myself. I directed AI agents, with two
> subscriptions, Anthropic and OpenAI, twenty dollars a month each
> **[confirm: still true, and say which tool did what]**. The first version took
> about three weeks of calendar time and maybe fifteen hours of my own
> attention **[confirm: the repository is now about 8 weeks old; say what is
> true for the whole project so far]**.
>
> **What I did before any code:** I wrote a specification. Testable
> requirements with MUST and SHOULD, an ID for each one (ARCH-04, for example),
> each tied to a milestone and an acceptance scenario. The agents must cite the
> IDs in their plans and their tests. It is boring to write, but I think it is
> the part that worked the most.
>
> **One rules file:** `AGENTS.md`, short and specific. For example: the audio
> callback never allocates, never logs, never waits on a lock. I learned that
> strict and specific beats layered and generic.
>
> **One boundary:** the canvas and the audio engine talk only through one
> versioned API (TypeScript and React on one side, Rust on the other). One agent
> worked on the canvas, another on the audio code, in separate passes, so the
> canvas agent could not create a bug in the audio thread.
>
> **What I did not use:** agents reviewing each other, a planner model, test
> loops before each commit. I don't want to overstate it, but I think a
> dedicated verification agent would have made this faster. Instead I was the
> verification, and it was slow.
>
> **Where it went wrong** (this is the real lesson):
>
> - A release passed every automated check and failed on my second computer.
>   The only machine where the tests ran had a developer variable set, always.
> - Recording passed all its tests, then wrote unplayable files the first time
>   it met my real setup with three audio streams. My tests used one perfect
>   stream.
> - I shipped several "crackling fixes" with "it compiles" as the only proof.
>   Now every audio change must pass a test that plays a sine wave through a
>   real route and allows zero glitches.
>
> Each time, I wrote the lesson in the rules file with the evidence, so the next
> agent session does not repeat it.
>
> The project is early: Windows 11 only, the installer is not signed (Windows
> will warn you), and to send audio into Discord or OBS as a microphone you
> still need a virtual cable like VB-Cable. It is GPL-3.0, with no account and
> no telemetry.
>
> Source: https://github.com/MrDesjardins/audiorouter, site:
> https://audiorouter.org
>
> My question: when your agent says "done", what do you check before you
> believe it?

**First comment** (post it right away):

> A 13-minute tour of the app, if you prefer to see it first **[confirm this is
> the 13-minute video and add chapter timestamps in its description]**:
> https://youtu.be/_hasa_zfDL8
>
> What is in it: 30+ audio tools, VST plugins that run in their own process (a
> crashing plugin cannot take your audio down), capture of one single app,
> audio between two PCs, a Stream Deck plugin, a local HTTP API, and an MCP
> server so an AI assistant can build a route for you.
>
> I wrote more about the rules and the harness on my blog: [what building
> AudioRouter taught me](https://patrickdesjardins.com/blog/what-building-audiorouter-taught-me-about-harness-and-rules).
> I can share the structure of `AGENTS.md` if it is useful.

Facts checked on 2026-10-30: the specification with requirement IDs and
`AGENTS.md` are in the repository; the failures and the sine-wave test are in
`AGENTS.md` "Validated lessons" and in your blog post; GPL-3.0, no telemetry,
unsigned installer and the VB-Cable requirement match the release notes. Two
items are not facts of the repository: the subscriptions and your hours. Delete
a claim rather than guess it.

### Video: should the 13-minute tour go in the post?

Yes, but as support, not as the centerpiece.

- **Rule 3 wants the build story.** The tour (`https://youtu.be/_hasa_zfDL8`,
  "Full feature tour: tools, MCP and Stream Deck" in the README) shows what the
  app does, not how it was built. Put the story in the post and the video in
  the first comment, so the post still stands on its own and is not "just
  dropping a link".
- **Thirteen minutes is long for someone who just met the project.** Say in one
  line what it contains, and add chapter timestamps in the video description so
  a visitor can jump to the part they care about.
- **Have a short option too.** The 2-minute overview is
  `https://youtu.be/IHitHqpi87s`; offer it first to people who ask "what does it
  look like".
- **Check that the tour shows the current app.** It replaced the homepage
  walkthrough in PR #83, which predates the Soundboard (PR #89), so it cannot
  show it. Confirm nothing in it contradicts the current release.
- **Do not upload the video natively to Reddit.** A YouTube link keeps the
  views and the description with the chapters; Reddit would also make the
  video the post, which pushes the build story down.

## Reply bank for this audience

Edit into your own words; copied replies read as canned.

- **"Is it actually production quality?"** No: it is an early preview. Say what
  has been verified (CI on Windows, a fresh-install test of the built app,
  long-run audio continuity tests) and what has not (clean-machine and upgrade
  checks, the unsigned installer, hardware qualification), as listed in the
  release notes of v0.0.24.
- **"How do you stop the AI from breaking real-time audio?"** The rules file
  forbids blocking calls in the audio path, and tests play a tone through a
  real route and check that it stays continuous. Mention the specific case
  where a "fix" shipped with only compile evidence.
- **"Which model or tool?"** **[confirm]**; give the real answer, including
  what you switched away from.
- **"Is the code secure?"** It has CodeQL scanning; 38 alerts were found and
  each was fixed or given a recorded disposition (see
  `docs/plans/active/code-scanning-remediation.md`). Check the current open
  count with `gh api repos/MrDesjardins/audiorouter/code-scanning/alerts?state=open`
  before quoting a number (it showed 1 on 2026-10-30). Do not claim a security
  audit.
- **"Can I just prompt this into existence?"** Say what the spec and
  evidence loop cost in your own time and where it saved you.
- **"Why Windows only / unsigned / VB-Cable?"** Use the answers in
  [marketing-phase1-posts.md](marketing-phase1-posts.md).
- **A harsh comment:** thank them once, answer facts, do not argue, and do not
  delete anything. Never ask for upvotes.

## Evidence (fill in)

Commands (read-only, any time within 14 days of the event, since traffic
history expires):

```
gh api repos/MrDesjardins/audiorouter --jq '[.stargazers_count,.forks_count]'
gh api repos/MrDesjardins/audiorouter/traffic/views
gh api repos/MrDesjardins/audiorouter/traffic/popular/referrers
gh api repos/MrDesjardins/audiorouter/releases --jq '.[0:3][]|[.tag_name,([.assets[]|select(.name|test("setup.exe"))|.download_count]|add)]|@tsv'
```

| When | Where | Link | Upvotes / comments | Stars | Views / uniques (day) | Setup downloads | Repeated questions |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-10-30 (before) | baseline | | | 51 | 23 / 15 on 10-29 | v0.0.24: 105 | |
| 2026-10-30 | r/vibecoding rules read by the owner (rules 1 and 3 shape the post) | | | | | | |
| 2026-10-30 | r/vibecoding post (build story; question about signing a kernel driver cheaply) | [post](https://www.reddit.com/r/vibecoding/comments/1xjv4i5/how_i_built_audiorouter_free_open_source_windows/) | | 51 at posting | | v0.0.24: 105 at last check | |
| +48 h | | | | | | | |

## Success measures

Guesses to compare against, not commitments:

| Measure (48 hours after posting) | Good | Weak |
| --- | --- | --- |
| Comments from people other than the owner | 10 or more | fewer than 3 |
| New stars | 10 or more | fewer than 3 |
| Outside issues or Discussions opened | 1 or more | 0 |
| Referrers showing the post | reddit.com up by 20+ unique visitors | no change |
| Downloads | any rise over the previous 2 days; not the goal | |

If weak: do not repost. Record why (angle, flair, timing, rules), keep the
build-story as a blog post, and move on to the Windows11 thread as planned.

## Repository tasks for an agent (this branch)

These change the repository and follow `AGENTS.md`; none opens a PR without the
owner's approval.

| # | Task | Done when |
| --- | --- | --- |
| A1 | **Done in this branch, awaiting merge.** Cause: the token worked and the numbers were collected, but the push to `main` was rejected (GH013: main requires a pull request and the "Quality gate" check; the ruleset has no bypass). Fix: the workflow now keeps the history on an unprotected `metrics-data` branch (a CSV-only orphan branch, seeded from main's CSV on the first run); `snapshot-metrics.mjs` replaces a same-day row instead of duplicating it; the duplicate baseline row is removed. The git steps were rehearsed locally against a bare remote (first run, second run, branch holds only the CSV); the workflow itself has not run | After merge, dispatch *Marketing metrics (weekly)* once by hand (Actions, Run workflow) and confirm `metrics-data` has the row. The owner decides whether to dispatch |
| A2 | **Done:** today's row (2026-10-30: 51 stars, 438 views / 179 unique, v0.0.24 105 installer downloads, 530 in total) is in `docs/marketing-metrics.csv`, taken by the script itself | CSV row exists |
| A3 | Add the r/vibecoding post and its results to `marketing-phase1-posts.md` and `marketing.md` "Next action" after the owner posts | The files link the post and numbers |
| A4 | Turn repeated questions into FAQ, manual or compare-page text within a week | Merged |
| A5 | Release the Soundboard (0.0.25) so the next round has real news: a 20-second clip of a Stream Deck key playing a sound | Release published after the usual gates |

## Sequence after today

| When | What |
| --- | --- |
| Fri 2026-10-30 | r/vibecoding (or an alternative), optional dev.to |
| Sun 2026-11-01 | r/Windows11 Software Showoff Sunday thread (draft in the posts file), simple screenshot reused |
| Mon 2026-11-02 | Review 48-hour results; confirm the metrics workflow works; list repeated questions |
| From Sat 2026-11-08 | r/SideProject with the project story, using what the first three taught; r/Twitch / r/obs after that |
| After 0.0.25 | Soundboard clip as the news for a creator-focused post |

## Risks

| Risk | Mitigation |
| --- | --- |
| Removed under rule 1 (generated text) or rule 3 (no build details) | Write it yourself; name tools, process and one real insight; the link comes last. If removed, do not repost: ask the moderators politely or move on |
| Moderators remove a promotional post | Disclose authorship; never repost; use a promo thread where one exists |
| The build-story invites hostility about AI code | Say AI use first, show the verification (specs, tests, evidence), admit what is unqualified; don't argue |
| Visitors install 0.0.24, hit a first-run problem and leave | The download page explains SmartScreen; stay in the thread; Discussions for help |
| Stale claims (Soundboard, "signed", test counts) | Only claim what the release notes and repository show today |
| Over-posting | Maximum two community posts a week; this plan uses one new community today and one thread on Sunday |
