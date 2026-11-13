# CI selection and parallel execution — 2026-11-12

Owner approved all three phases. Requirements: ENG-01/02/04/05; M08
verification tooling, without changing hardware or release acceptance gates.
Objective: select affected checks, retain a daily full inventory, shard Edge
across four isolated runners, and separate Rust, UI and release work.

Prerequisites: pinned Node/Rust, Windows native build tools, GitHub artifact
and cache services. Hosted timing/cache evidence requires Actions. Preserve
the unrelated dirty checkout; work on improve/ci-speed from origin/main.

Ordered tasks:
1. Test docs assets, named workflows and known root scripts in the classifier;
   unknown paths remain full. Add daily full CI alongside quality metrics.
2. Build browser fixtures once, transfer them to four one-worker runners.
   Select real-backend specs through transitive local imports for Rust-only
   changes; fail closed on unresolved imports. Full runs retain all specs.
3. Separate Linux UI, Windows Rust and Windows release checks; restore Rust
   caches on PRs and save only trusted main runs. Keep Quality gate fail closed.
4. Run policy/runner tests, workflow syntax validation, documentation and diff
   checks; exercise browser preparation/discovery and available local checks.

Decisions: ordinary shell Rust changes do not build installers on PRs;
packaging manifests, build scripts, release tooling and driver changes do.
Nightly/manual runs exercise every area. No change to branch protection.
No PR opening authorized by implementation approval alone.

Validation matrix: classifier exact enabled sets; gate rejects failed, missing,
cancelled and skipped required jobs; workflow wiring covers full inventory;
browser selection follows helper imports; artifact paths match consumers;
cache saves exclude PRs. Windows and Linux/Actions evidence are distinguished.

Risks: nightly can detect packaging or unmodeled browser coupling later than a
PR. Four runners increase concurrency demand. Cache eviction reduces speed,
never correctness. Artifacts include only generated fixtures, no user data.
Rollback: revert these scoped commits; existing Quality gate name stays stable.
Next: run the branch's manual full CI, inspect every job/annotation and record
hosted results before requesting PR-opening approval. One cohesive branch
keeps the classifier, gate and renamed jobs compatible throughout rollout.

## Local evidence — Windows, 2026-11-12

- `node --test tools/quality/policy.test.mjs tools/quality/verify.test.mjs
  tools/metrics/code-size.test.mjs`: 79/79 pass. Exact path-area regressions,
  transitive selection and failures/skips of every new required job covered.
- `node tools/quality/browser-ci.mjs prepare`: pass. Fresh e2e_backend build,
  bundled Vite build, positive/negative browser guard and asset guard pass.
  Guard artifacts: `%TEMP%/audiorouter-browser-guard-zU7VGK` and
  `%TEMP%/audiorouter-bundled-asset-guard-5tASfs` (local transient evidence).
- `CI=1 BROWSER_SCOPE=backend BROWSER_SHARD=1/4 node
  tools/quality/verify.mjs --checks browser`: 91 pass, 10 existing explicit
  skips, 2.4 minutes. Report:
  `%TEMP%/audiorouter-verification/1794531957272-8e2def71-a0b7-4256-a635-6c045e69745d.json`.
- Playwright JSON discovery for each shard compared by spec/project identity:
  all 757 tests split 191/189/188/189; backend 293 tests in 38 files split
  101/51/97/44. Each union is complete and disjoint. Local probe/results in
  `target/check-shards.mjs` and `target/shard-discovery.txt`, intentionally
  untracked. Conservative dynamic imports include some mocked specs.
- UI clean npm install, `typecheck:all`, `format`, `format:check`, `lint` pass.
  No product UI/style changes. `actionlint` 1.7.12 passes ci.yml; downloaded
  Windows archive SHA256 matches its published checksum.
- `node tools/docs/validate.mjs`: 260 Markdown files / 1,475 links pass.
  `git diff --check` passes. Complete tracked diff and new helpers inspected.

An initial discovery invocation from the repository root exposed a cwd-dependent
selector path; the config now resolves both specs and assets relative to its
own URL. Discovery passes from the root and the real UI working directory.
The local terminal inherited conflicting NO_COLOR/FORCE_COLOR values; the
browser command now removes NO_COLOR when FORCE_COLOR is set for its children.
No Rust product code changed. Hosted Linux UI, artifact transfer, cache size,
installer/full inventory and end-to-end Actions timing remain unmeasured.
