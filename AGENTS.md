# Agent guide for Ambition

This is the repository operating guide for coding agents. Keep it short, session-agnostic, and focused on routing. Put durable project knowledge in `docs/`, engineering memory in `dev/`, and generated navigation aids in `.agent/`.

## Core Values

* Avoid player-centrism. Value the principle of relativity.
* Find the elegant solution. Jon will push back on hacks.
* Correctness is emergent from elegance.
* **Pre-release engine, zero dependents.** Behavior and feel are NOT sacred until a polish pass — optimize for the elegant unified design, not for preserving current output. Delete duplicates, compat shims, and bridges on sight. Never fold a richer path onto a simpler one to "preserve" it; make the richer/general path universal and delete the rest.
* **Unified actors.** Every actor — the player included — is one body: kinematics + composable ability limbs + a capability mask, driven by a Controller (Human / Brain / RL) and observed via one `WorldView`. Player / Enemy / Boss / NPC are DATA (controller + capabilities), not types or code paths. The player's movement is the good base — make enemies and NPCs *rise to it* (adopt the rich limb pipeline), never drag the player down to a simpler path. Adding a character should be: author capabilities + pick a controller, zero core edits.
* **ONE BODY, ONE PATH — never bifurcate. This is the most-violated rule; read it before any combat/movement/visual/state change.** The player is an actor. Before you write *anything* keyed to "player" or "actor/enemy/boss" — an attack, a hitbox, a damage rule, a VFX/SFX emit, a shield, a reset, a state machine, a brain hook — run the **bifurcation smell test**: *"Does the other controller kind already do this on its own code path?"* If yes, you have found a **FORK**, and your job is to UNIFY onto the single shared seam and delete the other side — NOT to add a second site. **Adding a parallel emission site / state component / system / spec for an effect that already exists elsewhere is a BUG, not a fix — even if it compiles and every test passes.** A green test on a forked path is worthless. If you genuinely cannot complete the merge in one pass, do NOT add the parallel path "for now": route the new caller *through the existing seam* (extract one shared fn/system/event if none exists), and log the remaining merge in `dev/journals/code_smells.md` with `BIFURCATION:` as the first word. Melee is now unified end-to-end: the STATE (`BodyMelee`/`MeleeSwing`), swing MODEL (`AttackSpec`), slash VFX (`emit_melee_slash` in `combat::util`), AND the strike SPAWN (the moveset path: `combat::moveset::trigger_moveset_moves` → `advance_move_playback` spawns one gravity-resolved volume that drives BOTH the damage `Hitbox` entity and the slash, projected to body state by `project_moveset_melee_to_body_melee`) are ONE path for the player and every actor. Do NOT reintroduce a `PlayerAttackState`/`ActorAttackState` split, a second slash emit, or a per-frame player damage loop — every melee is an `"attack"`-verb moveset move riding `MovePlayback`. The MOVEMENT driver is now unified at the engine entry: the player tick is ONE system (`player_body_tick`) that calls the SAME combined body tick the actor uses (`ae::update_player_with_tuning_clusters` ≈ the actor's `update_body_with_tuning_clusters`), differing only in the input frame and the player respawn POLICY. **The two-clock precision-blink split (responsive aim during bullet-time) is now purely `InputState::control_dt` — an INPUT affordance, not a simulation structure: the human sets `control_dt = real frame dt`; a brain leaves it `0` and runs everything at sim time.** The player tick and `update_ecs_actors` stay SEPARATE Bevy systems on purpose (merging the orchestrators into one god-system is NOT the goal); what's shared is the body-tick engine entry. The next melee elevation is the unified action/ability timeline (cancel windows, movement locks, armor/i-frames, resource costs, hurtbox swaps, anim binding) layered on the one strike seam. When a doc/keystone says "unification," it means *delete one path*, not "make them behave similarly."

## Cold start

For non-trivial work, read and localize in this order: (1) `README.md`,
`AGENTS.md`, `.agent/README.md`, `docs/README.md`; (2) `python
scripts/agent_query.py "<task words>"`, before any broad source search; (3)
`docs/concepts/engine-mental-model.md` for durable ownership/data-flow, skimming
`docs/concepts/invariants.md` — the traps that actually bite; (4)
`docs/planning/vision.md` plus the relevant `docs/planning/tracks.md` entry; (5)
the likely crate's generated packet and `MODULES.md`; (6) ONE focused
concept/system/recipe/tool doc or ADR; (7) `dev/journals` and
`dev/benchmark-candidates` for the symptom or invariant.

Do not read all of `docs/`, `dev/`, or a multi-megabyte flat index by default.
See `docs/recipes/fresh-agent-navigation.md` for the drill-down protocol.

## Generated navigation protocol

`.agent/` holds commit-matched navigation: `.agent/index/catalog.json` for the
overview, per-crate packets under `.agent/index/crates/`, and
`.agent/ecs_inventory/crates/` for Bevy ownership, scheduling, resources,
messages and spawn sites. Reach them through `scripts/agent_query.py` rather than
dumping whole JSON indexes into context. Generated data only LOCALIZES an owner —
confirm in source, which wins for implementation fact (active planning/ADRs win
for intended direction).

## Source-of-truth order

(1) fresh user instructions; (2) **the master plan under `docs/planning/`**, the
primary coordination surface for direction and tasking — keep it current when
work materially changes status or direction (exact same-commit bookkeeping is not
required); (3) ADRs under `docs/adr/` and concepts under `docs/concepts/`; (4)
focused docs under `docs/systems/`, `docs/tools/`, `docs/recipes/`; (5)
brainstorms under `docs/brainstorms/` (Jon's — agents never write there); (6)
engineering memory under `dev/` and generated indexes under `.agent/`.

`docs/current/` is retired; `docs/vision/` holds auxiliary notes only — direction
lives in `docs/planning/`. `docs/archive/` is evidence, not authority.

## Current architectural stance

- Ambition is Bevy-native. Do not resurrect backend-neutral constraints unless a new ADR says so.
- Prefer data-driven ECS flow: authored/generated data -> Bevy components/entities -> systems -> messages/effects.
- LDtk owns world/level authoring. RON room manifests are historical; RON may still be used for tuning, save/settings, and other data where appropriate.
- Preserve desktop, web, Android/mobile/touch, controller, and Steam Deck paths. iOS is deferred for hardware, not excluded.
- **Binary asset payloads are git-ignored but PRESENT on disk.** *Git-ignored is
  not missing* — `ls` before concluding an asset is unavailable, and never build
  fetch/hydration machinery as part of a feature (distribution is Jon's). A
  feature owes only: degrade visibly when a file is absent. ⚠ **they do not
  travel to a `git worktree`** — `crates/ambition_platformer2d_actor_monolith/assets/sprites/` has 972
  files on `main` and 4 in a fresh worktree, so an asset-touching test run there
  fails for reasons that have nothing to do with the change. Full rules:
  `docs/recipes/adding-an-asset.md`.
- **Crate layering:** foundations and domain services feed the unified
  simulation heart; observation/presentation consume it; runtime/provider/host
  compose it; game providers own named content. `ambition_platformer2d_actor_monolith` is not awaiting
  a size-driven carve. Current roles and accepted extractions are in
  `docs/planning/engine/architecture.md` and `docs/planning/tracks.md`.

## Autonomous decision-making

When operating autonomously and you hit an architecture or design fork, **make
the choice Jon would most likely make and act** — read
`docs/planning/decision-principles.md` + `docs/concepts/autonomous-decision-making.md`.
Reserve questions for product/scope, irreversible/outward-facing acts, or true
intent ambiguity; otherwise infer and keep going. Until a polish pass, output/feel
is not a constraint. The gates: it compiles (including `ambition_app`) and
invariants hold.

## Verification

* **Drive the real headless sim — don't say "I can't test it."** Step the actual
  sim (`headless` / `trace_replay`) and observe; if a state can't be exercised
  headlessly, fixing THAT is the priority. Only visual feel ships BLIND.
* **Test invariants/properties, not tuned values or feel** — strongest are
  symmetry/covariance (C4 gravity, through-portal); no regression tests pinning
  unpolished behavior.
* **Replay/bit-identical tests are canaries, not cages** — a failure is info;
  re-baseline when the diff isn't egregious. Full doctrine:
  `docs/planning/engine/headless-verification.md`.
* **`cargo check -p <one_crate>` is not the gate — `cargo check -p ambition_app`
  is.** A per-crate check (even `cargo test -p <crate> --lib`) has been observed
  green on a crate that fails to compile in the app build. "It compiles" means
  the app compiled. App-level tests build into ONE `app_it` target, so
  `--test <file_name>` will not resolve: `cargo test -p ambition_app --test
  app_it -- <module>`.

## The cheapest sufficient command

Pick the narrowest row that covers what you changed. **Every number below was
measured on 2026-08-03 against a warm target directory** (`dev/run_tests_cost.jsonl`
plus direct timings); they are the real loop cost, not an estimate.

| I changed… | run | s | what it does NOT cover |
|---|---|---|---|
| a doc, a plan, a ledger | *nothing* | 0 | a doc a TEST reads — the goal file and `AGENTS.md` are both read by `scripts/tests` |
| a Python tool in `scripts/` | `python -m pytest scripts/tests -q` | 12 | the Rust side, entirely |
| LDtk tooling | `tools/ambition_ldtk_tools/.venv/bin/python -m pytest tests -q` | 6 | anything that consumes what it emits |
| one engine crate's code | `cargo test -p <crate>` | ~5 | **the app build** (see below), and every `#[cfg(feature)]` item |
| anything the app composes | `cargo check -p ambition_app` | 21 | behaviour — it proves compilation and nothing else |
| one app-level behaviour | `cargo test -p ambition_app --test app_it <module>` | 31–90 first, **1.3 warm** | the other modules, and failures that only appear under LOAD |
| all app-level behaviour | `cargo test -p ambition_app --test app_it` | ~101 | non-default features |
| a feature gate | `cargo check -p <crate> --features <combo> --all-targets` | 20 | RUNNING the gated tests — only the union job does |
| every gated test | `cargo test --workspace --features <union>` (see `scripts/run_tests.py`) | 340 | little; it is the widest single graph we have |
| the default sweep | `python scripts/run_tests.py` | 393 (6.5 min) | features, external consumers, the wasm check |
| before a release, or after touching features / the SDK surface / the web path | `python scripts/run_tests.py --run-everything-you-probably-dont-need-this` | 1528 (25.5 min) | `#[ignore]`d tests and acceptance cycles — add `--heavy` |

⭐ **The one that surprises people: a warm filtered `app_it` run is 1.3 seconds.**
The 31–90s figure is the RELINK after an `ambition_app` source edit, so the loop to
optimise is *edit less of `ambition_app`*, not *test less*.

⚠ **`cargo check -p <crate>` is not the gate — `cargo check -p ambition_app` is**,
and the row above says 21s for a reason: it is cheap enough that there is no
excuse. A per-crate check has been observed green on a crate that fails to compile
in the app build.

⛔ **Do not reach for the exhaustive plan out of caution.** It is 4× the default
sweep and there is no CI to satisfy; Jon sweeps it periodically himself. Reach for
it when a row above names your change (features, SDK, web) — and ask
`run_tests.py --list` what a plan actually contains before running it.

⛔ **The 11-second job is the one to stop skipping.** `repo tooling
(scripts/tests)` runs 180 tests including
`test_every_contract_holds_against_the_live_tree` — the 25 architectural absence
contracts, which are the only thing that catches a registration or a dependency
edge landing in the wrong place. On 2026-08-03 two of them sat red through
several commits because targeted `cargo test` filters were run instead of the
plan. ⚠ and `python3 scripts/check_absence_contracts.py` **exits 0 while printing
`2 of 25 violated`** — enforcement needs `--check`.

### The write-ahead worktree (Jon, 2026-08-03)

Builds and tests are not slow because of the compiler alone — they are slow
because **every job reads the LIVE tree**, so a suite running on `main` freezes
editing for its whole duration. A second tree removes that serialization:

```
git worktree add -b workahead /home/agent/code/ambition-workahead main
cd /home/agent/code/ambition-workahead && python3 scripts/mirror_assets_for_worktree.py
```

- **Write ahead in the worktree** while `main` is mid-build. Next feature, next
  refactor — whatever the running job would have blocked.
- **Integrate, build and test on `main`.** Merge the worktree's branch when main
  is clean, start the next job, and go back to writing.
- ⛔ **Do NOT build in the worktree.** A second `target/` cannot be shared (they
  fight over artifacts and lockfiles) so it is a full cold duplicate — and this
  volume has hit 100% three times. Jon's call, and the reason the split is
  write-there / build-here rather than two independent checkouts.
- ⚠ **Mirror the assets first, always.** Generated art and audio are gitignored,
  so a fresh worktree has ~4 sprite files against main's 996 — and the sheet
  registry is baked from those directories, so an assetless tree compiles a binary
  with an EMPTY sheet table and ~40 tests fail for reasons unrelated to the
  change. `mirror_assets_for_worktree.py` links them file by file on purpose: a
  regenerated sprite lands as a REAL file in the worktree and main never sees it,
  which directory symlinks would not give you.

### Sweeping failures: ONE big run, then targeted only (Jon, 2026-08-03)

> *"Run the big suite once, then fix each test individually and verify them with
> local targeted reruns only, and then we DON'T run the entire thing again after.
> We just assume we fixed them because we did locally and move on. If anything
> else broke we catch it on the next big sweep, but we don't spend all day chasing
> those down."*

⛔ **Do not re-run the full suite to confirm a fix the targeted run already
confirmed.** On 2026-08-03 the same agent ran `run_tests.py` or
`cargo test --workspace` five times in one stretch; every failure it found was
then diagnosed and fixed by a single `-p <crate> --test <target> <filter>` run, and
no re-sweep ever caught anything the targeted run had missed. The re-sweeps cost
more than every fix combined.

⚠ **the instinct this overrides is real and still wrong here.** Re-verifying
globally *feels* like diligence; on a pre-release engine with no CI and a
maintainer who sweeps periodically, it buys a confirmation nobody was waiting for
and spends the hour that the next fix needed. A fix verified locally is fixed.

### When a run is slow, get the DISTRIBUTION before theorising

`--report-time` is nightly-only, so per-test timings on stable come from running
serially and timestamping the output:

```
cargo test -p <pkg> --test <target> <filter> -- --test-threads=1 --nocapture \
  | python3 -u -c "
import sys, time
t0=time.time(); last=t0; cur=None; rows=[]
for line in sys.stdin:
    if line.startswith('test ') and ' ... ' in line:
        now=time.time()
        if cur: rows.append((now-last, cur))
        cur=line.split(' ... ')[0][5:]; last=now
for d,n in sorted(rows, reverse=True)[:15]: print(f'{d:7.1f}s  {n}')
"
```

⛔ **A suite total divided by a test count is not a per-test cost, and it reads
exactly like one.** On 2026-08-03 a 67s / 25-test subset produced an apparent
"2.7s per app boot"; the distribution showed boot is 370ms and **one test held
33% of the time, four held 63%**. Two fixes were built against the average before
anyone ran the two commands that show the shape.

⭐ **And the wall clock of a parallel run cannot go below its LONGEST test.** When
a run pins several cores and still feels slow, suspect a single long pole before
suspecting a lock or an I/O bottleneck — the CPU-percentage signature is the same
for both.

## Test placement

A test lives at the **narrowest scope that owns its invariant** — inline for
small local ones, an adjacent `src/foo/tests.rs` for large private modules
(**never widen a production API to move a test**), the crate's `tests/` for
assembled behavior, and `tests/ambition_workspace_policy` for workspace
source/dependency/architecture rules. Full guidance + commands:
`docs/concepts/test-placement.md`.

## The Hall of Characters is NOT a special case

`hall_of_characters` stages ~144 characters — a **dual purpose stress test and
exhibition**. ⛔ **When it is slow, do not fix the Hall. Fix the engine.** ⚠ it is
GENERATED from the character catalog: never hand-edit the level. The rejected
shortcuts (quality variants, load caps, "it's only a debug room") are each
answered in `docs/concepts/hall-of-characters-is-not-special.md` — read it before
optimising anything that touches this room.

## Before a non-trivial patch

- **Spatial questions** (LDtk, gates, hitboxes): read the map, infer the
  component's PURPOSE, place it on the seam that fulfils it, and state the
  reasoning in the commit. Asking "where exactly?" is the wrong default —
  `docs/concepts/llm-spatial-authoring-discipline.md`.
- **Engineering memory:** `rg -n "<subsystem>|<symptom>" dev/journals
  dev/benchmark-candidates` (postmortems + invariant traps). Add durable lessons
  to `dev/benchmark-candidates/` + its index — never transient state.

## Patch discipline

- Commit messages are detailed, and say what PROMPTED the change (the why).
- Prefer reviewable changes with targeted validation; don't hand-edit
  `sandbox.ldtk` (use Ambition LDtk tooling); update concepts/recipes/ADRs/dev
  memory when a durable invariant changes.
- Formatting is advisory rather than an acceptance gate; do not fail or block a
  change solely because `cargo fmt` or `ruff format` was not run.
- Expected working-tree noise, never a mystery: a git hook rewrites
  `.llm_resource_tally/` every turn. Let it ride along with an ordinary commit —
  do not flag, revert, or attribute it to another session. Policy: the managed
  block below.
- A script that writes an artifact ENDS its stdout with a `rich` clickable
  `file://` link to the artifact AND its directory (`[link=file://…]…[/link]`,
  `try/except ImportError` fallback to plain paths). Pattern:
  `scripts/git_debloat.py`, `scripts/archive_agent_source.py`.
- `./run_tests.sh` is the BACKBONE — the repo's Python suites plus one
  `cargo test --workspace`. It is broad-good-enough and it is what a dev cycle
  wants. Narrower is better still when a focused test already covers the touched
  concept: `-p <crate>`, `-k <substr>`.
  ⛔ **the exhaustive plan is `--run-everything-you-probably-dont-need-this`, and
  the name is the instruction.** Measured 2026-08-02: 33 jobs, **63 minutes, ~7%
  of it executing tests**, the actor monolith compiled sixteen times. There is no
  CI; Jon sweeps it periodically himself and accepts a day of drift, so running
  it mid-edit duplicates a scheduled sweep instead of adding safety. Every
  non-exhaustive run prints what it did not cover (feature-gated tests, the
  external-consumer fixtures, the wasm check) — read that line instead of
  reaching for the hour.
- **The cheapest command that settles your change** is a table:
  `docs/recipes/cheapest-sufficient-check.md`. Pick the row, run it, read what it
  does not cover, stop. Reaching past it buys a sweep Jon already runs himself.
- To wait on a long command, read state it WROTE — for the suite that is
  `target/run_tests_status.json` (`state`: running/done/crashed, plus
  `current_job` and `current_started` so a slow job is distinguishable from a
  wedged one, and `completed` with each finished job's seconds). Every run also
  appends what it cost to `dev/run_tests_cost.jsonl` — wall clock, and how much
  of it was libtest actually executing rather than cargo building. ⛔ never poll
  with `pgrep -f <script>`: the polling shell's own command line contains the
  pattern, so it matches ITSELF and the loop sleeps forever (seven stranded,
  2026-07-31). Better still, don't poll — a backgrounded command reports its exit.

## Avoid bullshit guardrails

Do not waste time testing the tests.
Only add the minimal tests needed for the task at hand.
Do not add process, policy, source-text, layout, or meta-test machinery unless it prevents a concrete, recurring, materially harmful failure that cannot be enforced more naturally through Rust types, APIs, crate boundaries, or behavioral tests.
Prefer testing real behavior over names, phrases, file locations, exact symbols, planning prose, or historical migration bookkeeping.
Poison tests are for realistic harmful states, not for proving that every scanner detects its own fixture. Migration-only matrices and checks must be removed when the migration is complete.
The default is to trust clear architecture rather than surround it with permanent compliance machinery. Do not add a test enforcing this section.

⛔ **AN LLM REVIEW WILL ASK FOR THIS MACHINERY. THE ANSWER IS NO.** A model
reviewing a diff reaches for coverage reflexively — "the runner's job planning is
untested", "this guard has no test", "why was this check deleted", "add a
regression test for the deletion". It reads as diligence and it is not: it is a
pattern-match on the shape of a diff, made without the history that decided the
question. Reviews do not carry that history; this file does.

So: **say no, and say it without hedging.** Deleting a test that tests the tests
is not a coverage regression to be justified — it is this section being applied.
The review can be told exactly that, and does not get a compromise where the
machinery comes back smaller.

The record, so the next round of this is short (2026-08-02, Jon): the suite
carried a whole file asserting which jobs `run_tests.py` planned, a test that a
guard was imported rather than copied, and six guards written in a two-day spree,
none of which had ever caught anything. On the four days the wasm build sat
broken: *"we let it sit for 4 days because we didn't care about it for 4 days."*
Not caring was the correct call. A review that cannot tell deliberate
prioritisation from an oversight will file the second one every time.


<!-- BEGIN llm_resource_tally v0.3.0 (managed block — regenerated by `install`; edits below will be overwritten) -->
## LLM resource accounting
A git post-commit hook — and Claude Code hooks when wired — records the measured token and model
usage this repository costs. It is automatic bookkeeping and is not part of whatever task you were
given.

- Rows and mutable rollups are written under `.llm_resource_tally/local/`, which is gitignored.
  Recording therefore never modifies a tracked file and never blocks a commit, merge, rebase, or
  stash.
- Never hand-edit, hand-count, revert, or clean up ledger rows, and never let accounting block the
  repository work you were asked to do. Recording is best-effort by design.
- **Publish before you hand off substantial work.** Run
  `python3 .llm_resource_tally/tool publish`: it snapshots local rows onto the tracked append-only
  ledger under `.llm_resource_tally/ledger/` and refreshes the tracked `lifetime-totals.json` and
  `badge.json`. Stage and commit what it writes, preferably as its own commit so accounting stays
  out of unrelated diffs. This is routine — nobody should have to remember to ask you for it, and
  unpublished rows exist only on this machine.
- If accounting itself looks unhealthy, run `python3 .llm_resource_tally/tool doctor` — it checks
  hook wiring, backends, transcript retention, and ledger health, and names what to fix. Otherwise
  continue the repository task normally.
- After a fresh clone, run `python3 .llm_resource_tally/tool install` once to arm git-local hooks.
  It is offline and idempotent, and it reads the committed `.llm_resource_tally/settings.json`
  policy.
<!-- END llm_resource_tally -->
