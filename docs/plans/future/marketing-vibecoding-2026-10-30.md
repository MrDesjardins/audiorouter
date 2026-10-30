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

- `docs/marketing-metrics.csv` has only the baseline row (twice). The scheduled
  *Marketing metrics (weekly)* run on 2026-10-26 **failed** (run 40386405710),
  so the best weeks of data were never recorded and GitHub keeps traffic for
  only 14 days. Record today's numbers by hand (above) and fix the workflow
  (task A1).
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

## Draft post: a fact outline, not text to paste

Rule 1 removes "fully generated AI slop", and this draft was generated by an
agent. Use it for the facts and structure only. Close it, then write each
section in your own sentences, with your own examples, and cut anything you
would not say to a friend. You may use AI to tidy your own draft, lightly. The
r/software feedback ("copy felt AI-generated") points the same way.

Rule 3 requires the tools, the process and a build insight, so the post cannot
lead with the project or end with just a link.

**Title options** (pick one):

1. I vibe-coded a Windows audio router with real-time audio, plugin hosting and a spec. Here is what kept it from falling apart
2. 8 weeks building a Windows audio app with AI agents: the spec, tests and rules that mattered **[confirm: the first commit is 2026-09-05, so about 8 weeks; say it as it felt to you]**
3. I used AI agents to build a free open-source audio router for Windows. Showing the app and the process

**Image:** the simple route screenshot (step 2). If the sub prefers no
promotional images, use no image.

**Body:**

> I'm the developer. I built AudioRouter, a free, open-source audio router for
> Windows 11: you draw your audio on a canvas (microphone → noise gate → EQ →
> Discord and headphones) and change it while it plays. This post is about how I
> built it with AI agents, because I think that part is more useful to you than
> the app.
>
> **What I used:** **[confirm: which agents and models, how you split the
> work, rough cost]**.
>
> **What kept it from turning into a mess:**
>
> - A written spec before code (`docs/spec/`), with requirement IDs the agents
>   have to cite in every plan and test.
> - One rules file (`AGENTS.md`) that says what the agent may not do, for
>   example "the audio callback never allocates, logs, or waits on a lock".
> - A "validated lessons" list in that file. Every time something broke in a way
>   the agent could not see (a memory leak of about 9 MiB an hour, a plugin that
>   loaded but did nothing, a test that passed while the user's setup failed),
>   the fix and the evidence got written down so the next session would not
>   repeat it.
> - Tests that follow the user's real setup, not a perfect mock. My lesson:
>   "all tests pass" and "it works on my second PC" were different statements.
> - A human gate: the agent does not open pull requests until I approve.
>
> **What did not work:** **[confirm 1–2 real failures you are willing to name,
> for example the audio crackle fixes that shipped on compile-only evidence]**.
>
> The result is early. Windows 11 x64 only, an unsigned installer (Windows will
> warn), and feeding audio into Discord or OBS as a microphone still needs a
> virtual cable such as VB-Cable. It is GPL-3.0, with no account and no
> telemetry.
>
> Source: https://github.com/MrDesjardins/audiorouter · Site:
> https://audiorouter.org
>
> I'd like to hear how you keep agents honest on a larger codebase: what do you
> make them write down, and what do you refuse to let them skip?

**First comment** (details; post immediately):

> More detail if useful. The app: 30+ built-in audio tools, VST2/VST3 plugins
> in their own process (a crashing plugin cannot take your audio down),
> per-application capture, audio between two PCs, a Stream Deck plugin, a local
> HTTP API and an MCP server so an AI assistant can build a route for you.
> Related write-up on my blog: [what building AudioRouter taught me about
> harness and rules](https://patrickdesjardins.com/blog/what-building-audiorouter-taught-me-about-harness-and-rules).
> Happy to share the `AGENTS.md` structure.

Facts checked against the repository on 2026-10-30: `docs/spec/`,
`AGENTS.md` (rules, validated lessons, PR approval gate), GPL-3.0, no
telemetry, unsigned installer, VB-Cable requirement. Facts only the owner can
confirm are marked **[confirm]**; delete a claim rather than guess it.

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
| | r/vibecoding rules read | | | | | | |
| | post | | | | | | |
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
| A1 | Diagnose the failed *Marketing metrics (weekly)* run 40386405710 (likely an expired or missing `METRICS_TOKEN`; read the log first), repair or document it, remove the duplicate baseline row | The next scheduled run commits a row and the CSV has no duplicates |
| A2 | Add today's numbers (table above) to `docs/marketing-metrics.csv`, marked as a manual entry | CSV row exists |
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
