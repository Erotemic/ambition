# Agent guide for Ambition

This is the repository operating guide for coding agents. Keep it short, session-agnostic, and focused on routing. Put durable project knowledge in `docs/`, engineering memory in `dev/`, and generated navigation aids in `.agent/`.

## Core Values

* Avoid player-centrism. Value the principle of relativity.
* Find the elegant solution. Jon will push back on hacks.
* Correctness is emergent from elegance.
* **Pre-release engine, zero dependents.** Behavior and feel are NOT sacred until a polish pass — optimize for the elegant unified design, not for preserving current output. Delete duplicates, compat shims, and bridges on sight. Never fold a richer path onto a simpler one to "preserve" it; make the richer/general path universal and delete the rest.
* Unified actors. Player / Enemy / Boss / NPC are controller, capabilities, and authored data—not separate engine types.
* **ONE BODY, ONE PATH.** The player is an actor; controller kind does not define a simulation path. Before adding behavior keyed to player/enemy/boss, check whether the behavior already exists for another controller kind. If so, unify onto one shared body/capability seam and delete the duplicate path. Do not add a parallel implementation “for now.” See `docs/concepts/one-body-one-path.md`.
* All new comments and documentation must be written in ASD-STE100 Simplified Technical English.

## Cold start

For non-trivial work, localize in this order:

1. `README.md`, `AGENTS.md`, `.agent/README.md`, `docs/README.md`.
2. `python scripts/agent_query.py "<task words>"` before broad source search.
3. `docs/concepts/engine-mental-model.md`; skim `docs/concepts/invariants.md`.
4. `docs/planning/vision.md` plus the relevant `docs/planning/tracks.md` entry.
5. The likely crate's generated packet and `MODULES.md`.
6. ONE focused concept/system/recipe/tool doc or ADR.
7. `dev/journals` and `dev/benchmark-candidates` for the symptom or invariant.
8. If you are reviewing, architecturally steering, or inspecting another agent's work, read `docs/reviewer-guide.md` before reviewing the diff.

Do not read all of `docs/`, `dev/`, or a multi-megabyte flat index by default.
See `docs/recipes/fresh-agent-navigation.md`.

## Authoring submodules are part of the architecture

An empty authoring/content submodule directory does NOT mean the capability is absent. Check `.gitmodules`, the root README's authoring-toolchain table, and the canonical repositories:

* [sprite renderer](https://github.com/Erotemic/ambition_sprite2d_renderer)
* [music renderer](https://github.com/Erotemic/ambition_music_renderer)
* [SFX renderer](https://github.com/Erotemic/ambition_sfx_renderer)
* [development measurements](https://github.com/Erotemic/ambition_dev_measurements)
* [LDtk map assets](https://github.com/Erotemic/ambition_map_assets)

Read `docs/concepts/agent-native-authoring.md` before designing a new authoring surface.

## Generated navigation protocol

Use `scripts/agent_query.py` to query commit-matched navigation under `.agent/`
rather than dumping whole generated indexes into context. Generated data localizes
likely owners; source wins for implementation fact, and active planning/ADRs win
for intended direction.

## Source-of-truth order

1. Fresh user instructions.
2. **The master plan under `docs/planning/`** — primary coordination surface for direction and tasking.
3. ADRs under `docs/adr/` and concepts under `docs/concepts/`.
4. Focused docs under `docs/systems/`, `docs/tools/`, `docs/recipes/`.
5. Brainstorms under `docs/brainstorms/` (Jon's — agents never write there).
6. Engineering memory under `dev/` and generated indexes under `.agent/`.

`docs/current/` is retired. `docs/vision/` holds auxiliary notes only. Superseded docs are deleted rather than moved to an archive and can be read from git history with `git show <sha>^:<path>`.

⛔ **A DONE ITEM IN A PLANNING DOC IS A RECEIPT, NOT A CASE FILE.** When you close a row, compress it in the same commit: what was wrong in one sentence, what fixed it, the commit, the guard, and any standing prohibition that would otherwise be rediscovered. Investigation belongs in the commit message and git history. For an OPEN row, keep the current model at the top and delete reasoning it supersedes. See `docs/planning/README.md#queue-contract`.

## Current architectural stance

* Ambition is Bevy-native. Do not resurrect backend-neutral constraints unless a new ADR says so.
* Prefer data-driven ECS flow: authored/generated data -> Bevy components/entities -> systems -> messages/effects.
* LDtk owns world/level authoring. RON room manifests are historical; RON remains appropriate for tuning, save/settings, and other structured data.
* Preserve desktop, web, Android/mobile/touch, controller, and Steam Deck paths. iOS is deferred for hardware, not excluded.
* **Crate layering:** foundations and domain services feed the unified simulation heart; observation/presentation consume it; runtime/provider/host compose it; game providers own named content. `ambition_platformer2d_actor_monolith` is not awaiting a size-driven carve. Current roles and accepted extractions are in `docs/architecture/engine-architecture.md` and `docs/planning/tracks.md`.

### Assets in worktrees

**Binary asset payloads are git-ignored but may be PRESENT on disk. Git-ignored is not missing.** `ls` before concluding an asset is unavailable. Do not build fetch/hydration machinery as part of a feature; a feature owes graceful visible degradation when an asset is absent.

Assets and initialized submodules do not automatically travel to a fresh worktree. From inside the worktree:

```bash
python3 scripts/mirror_assets_for_worktree.py
```

This also initializes `game/ambition_map_assets`, which backs symlinked LDtk paths.
Full rules: `docs/recipes/adding-an-asset.md`.

For a fresh clone/worktree, run:

```bash
scripts/setup/target_bindmount.sh
```

⛔⛔ **RUN `scripts/setup/target_bindmount.sh --status` BEFORE YOUR FIRST BUILD,
EVERY SESSION, AND ACT ON WHAT IT SAYS.** The repo may be on virtiofs; the script
shadows `target/` with a directory on local storage. The bind does not survive a
reboot, so an unbound session can silently build into the slow shared filesystem
and create a large second copy of build artifacts.

⇒ If the bind is absent, repair it before building:

```bash
scripts/setup/target_bindmount.sh
```

⚠ Binding stops further growth on the shared filesystem but **does not reclaim an
already-created shadowed copy**. Report that condition rather than deleting it.
If disk pressure is involved, inspect the repository filesystem as well as the
filesystem visible at `target/`:

```bash
df -h .
df -h target
```

If the repo mount remains full with the bind in place, stop and report it rather
than guessing what should be deleted.

⛔ **`rm -rf` under `target/` is never the tool — `cargo clean` is.**

✔ When the target is correctly bound, use Cargo or the repository cleanup tools:

```bash
./scripts/clean_workspace_crates.sh --incremental-only
./scripts/clean_workspace_crates.sh --incremental-only --apply
cargo clean --workspace
cargo clean
```

Run the incremental cleaner without `--apply` first when you only need headroom.

⚠ `./run_tests.sh` refuses to start on an unbound virtiofs target. Do not work
around that safety check.

Do not substitute `CARGO_TARGET_DIR` for the main repository target-bind policy.

## Autonomous decision-making

When operating autonomously and you hit an architecture/design fork, **make the choice Jon would most likely make and act**. Read `docs/planning/decision-principles.md` and `docs/concepts/autonomous-decision-making.md`.

Do not stop to ask about an architectural choice you can resolve from those principles. Until a polish pass, current output/feel is not a preservation constraint.

**Handing the turn back on an armed run.** Never stop an autonomous run to ask, except when Jon explicitly asks in that turn to finish and wait. Then:

```bash
python3 scripts/goal_guard.py --pause "Jon asked me to finish X and wait"
```

Do not use `--pause` to end a turn early. Clearing an armed run is Jon's call.

Extend an armed run with:

```bash
python3 scripts/goal_guard.py --extend 48h   # also 2d, 90m, or ISO timestamp
python3 scripts/goal_guard.py --extend       # print clocks
```

Never hand-edit `.goal/active.json`; the run has multiple release clocks and
`--extend` updates them consistently.

## Programmatic Checks

A check that takes more than a minute is not cheap. A check that takes more
than 2 minutes is expensive. Many iterations of 10s-of-seconds checks add real
latency and have a large negative impact on throughput.

* **Batch the gate**: do not run the full gate every micro-edit. It belongs
  before a big commit, not between edits. Sometimes not even between commits,
  when they are part of a larger campaign.
* Do not double check with cargo. Tee its output to a file once if you need
  multiple operators over its output.
* ⛔⛔ **FREEZE THE TREE WHILE A GATE RUNS — including docs, including a merge.**
  A verification result must describe one stable tree. Do not edit or merge
  underneath a running gate.

## Verification

Use the narrowest command that actually covers the change. Full matrix:
`docs/recipes/cheapest-sufficient-check.md`.

* **Drive the real headless sim — don't say "I can't test it."** Step the actual
  sim (`headless` / `trace_replay`) and observe. If important state cannot be
  exercised headlessly, improve the harness. Only visual feel ships blind.
* **Test invariants/properties, not tuned values or unfinished feel.** Prefer
  symmetry/covariance where applicable.
* **Replay/bit-identical tests are canaries, not cages.** Re-baseline deliberate
  changes when the diff is not egregious.
* **`cargo check -p <one_crate>` is not the compile gate.**

  ```bash
  cargo check -p ambition_app
  ```

  A crate-local check can be green while the assembled app fails.

* App-level integration tests live in one `app_it` target:

  ```bash
  cargo test -p ambition_app --test app_it -- <module>
  ```

  ⛔ **The trailing `-- <module>` is a FILTER, not a lane.** A green filtered run
  proves only the selected population. Before claiming the target passes, remove
  the filter and run the whole target.

  Likewise, `--exact` with a bare test name can match nothing and still exit 0;
  use the full module path.

* ⛔ **A green suite cannot certify an assertion's direction.** When a sense-flip
  is involved, read the assertion against the claim and failure message.

* A probe whose two outcomes are both informative must not be a deliberately
  failing test. Use a passing diagnostic probe, inspect it, then encode the
  intended invariant.

* ⛔⛔ **DO NOT SWEEP `cargo test --workspace --tests`.** It links many
  integration targets at once. Prefer `--workspace --lib` plus named integration
  targets.

* ⛔ **THE PER-TURN GATE DOES NOT RUN `--workspace --lib`.** Before
  push/finalization, when the verification recipe calls for it:

  ```bash
  cargo test --workspace --lib
  ```

  Keep this as a separate validation tier. Do not add it to every turn.

* When touching character, movement, or combat:

  ```bash
  cargo test -p ambition_demo_smash_app
  ```

* When moving dependency, ownership, motion-authority, or other architecture boundaries:

  ```bash
  cargo test -p ambition_workspace_policy
  ```

  If a policy fires because architecture intentionally changed, update its
  rationale. Do not add a waiver merely to silence it.

* `cargo nextest run` is installed and `./run_tests.sh` uses it when available.
  Prefer it for diagnosis because it reports individual test durations.

  ⛔ **NEXTEST RUNS NO DOCTESTS.** Do not claim doctest coverage from a nextest
  run unless doctests ran separately.

* ⛔ **A compiling game can still draw stale quality-variant art.** Publishing art means:

  ```bash
  ./scripts/regen/quality_variants.sh
  python3 scripts/check_quality_variants_are_fresh.py
  ```

### Generated assets

⭐⭐ **THE BIG ASSETS ARE GENERATED FROM SMALL AUTHORED SOURCES, AND THAT IS WHY
THEY ARE GITIGNORED.** Spritesheets, portraits, backgrounds, quality tiers,
music cues and the packed SFX bank are all output. Git carries the small,
reviewable source used to rebuild them.

One command rebuilds all generated runtime content, fonts included:

```bash
scripts/setup/generated_content.sh
```

Narrower commands:

```bash
./scripts/regen/assets.sh
./scripts/regen/sprites.sh --list
./scripts/regen/sprites.sh --target george_booul_vfx
./scripts/regen/sprites.sh --check-toolchain
python3 scripts/grab_font_assets.py
```

⛔ **An assetless tree can fail tests that do not name assets.** Common symptoms:

| symptom | likely cause | fix |
|---|---|---|
| many unrelated `"unknown cosmetic effect"` errors | baked sprite-sheet table is empty | `./scripts/regen/sprites.sh` |
| missing bundled font at compile time | fonts were not fetched | `python3 scripts/grab_font_assets.py` |
| portrait manifest expected but absent | baked portrait table is empty | `./scripts/regen/sprites.sh` |
| art draws at a stale quality tier | quality variants are stale | `./scripts/regen/quality_variants.sh` |

⚠ `regen/assets.sh` does not fetch fonts. `scripts/setup/generated_content.sh`
does both generated assets and fonts.

`build.rs` declares asset directories with `cargo:rerun-if-changed`, so Cargo
re-bakes after regeneration. Do not run regeneration concurrently with a Cargo
build.

Details:
[`scripts/regen/README.md`](scripts/regen/README.md),
[`docs/tools/generated-visual-tools.md`](docs/tools/generated-visual-tools.md),
[`docs/tools/generated-audio-tools.md`](docs/tools/generated-audio-tools.md).

### Authored data

A changed Rust type does not typecheck authored RON embedded inside `&str` literals.
Search every authored occurrence of changed fields and run tests for each affected
crate.

Git-ignored sprite RON can be skipped by ordinary recursive search and symlink
handling. For sprite assets use:

```bash
find . -path '*/assets/sprites/*.ron' -not -path './target/*' | \
    xargs grep -l '<field>'
```

### Repository checks

Repository check scripts may default to advisory mode. Before relying on a zero
exit code, check whether enforcement requires `--check` or `--strict`.

Known advisory-by-default scripts include:

* `check_absence_contracts.py` — enforce with `--check`
* `check_doc_link_ratchet.py` — enforce with `--check`
* `check_planning_citations.py --vanished REF` — enforce with `--strict`
* `check_planning_line_citations.py` — enforce with `--strict`, repair with
  `--fix`

`compile_ratchet.py` is intentionally the counterexample and fails by default.

⛔ **A checker that crashes is not a checker that fails.** If a checker shells
out to other tools, confirm that it reached its normal verdict output rather
than trusting the exit code or traceback alone.

Do not duplicate a check in CI without first searching the workflow at the
parent commit.

Adding or removing a workspace crate can affect repository policy files and
nested lockfiles outside the changed crate. After adding or removing a crate,
run:

```bash
cargo test -p ambition_workspace_policy --test policy
python3 scripts/check_absence_contracts.py --check
```

Relevant files are discoverable:

```bash
git grep -l <an existing sibling crate name> -- '*.toml' '*.lock' '*.py'
```

### Long-running builds

⛔ **NEVER SIT AND WATCH A BUILD DURING COORDINATED WORK.** Give a worker
independent writing work in a worktree while `main` verifies.

⛔ **DO NOT RUN CONCURRENT BUILDS AGAINST ONE TARGET DIR.** Agent worktrees each
bind-mount their own, so builds in different slots are fine — see
`docs/tools/agent-worktrees.md`. Pace your `-j` to your slot.

⛔ **Do not run concurrent Cargo builds with different feature sets against the
same target directory.** Shared artifacts can be rebuilt incompatibly and leave
misleading link failures.

If a long feature-specific run is in flight, do documentation or other
non-Cargo work instead of launching a competing build.

⛔ **A nested workspace's `target/` is not the main bind-mounted target.**
`examples/capability_demo`, `fixtures/minimal_game`, and
`fixtures/external_consumer` carry their own workspaces. Route those builds into
the main bound target explicitly, for example:

```bash
CARGO_TARGET_DIR=$PWD/target/capability_demo \
  cargo test --manifest-path examples/capability_demo/Cargo.toml ...
```

`scripts/setup/target_bindmount.sh --status` reports the main target only. If
disk pressure matters, check both:

```bash
df -h .
df -h target
```

The exhaustive plan `--run-everything-you-probably-dont-need-this` is
intentionally exceptional. Use it only when the verification recipe names your
change.

## Test placement

Tests live at the narrowest scope owning the invariant. Never widen a production
API merely to move a test. See `docs/concepts/test-placement.md`.

## The Hall of Characters is NOT a special case

`hall_of_characters` is a generated engine stress test and exhibition.

⛔ **When it is slow, do not fix the Hall. Fix the engine.**

Do not hand-edit the level. Read
`docs/concepts/hall-of-characters-is-not-special.md` before optimizing anything
that touches it.

## Before a non-trivial patch

For LDtk, gates, hitboxes, and other spatial authoring, follow
`docs/concepts/llm-spatial-authoring-discipline.md`.

Search engineering memory with:

```bash
rg -n "<subsystem>|<symptom>" dev/journals dev/benchmark-candidates
```

Add durable lessons to `dev/benchmark-candidates/`; never transient project state.

## Patch discipline

* Do not hand-edit generated LDtk content.
* Formatting is advisory, never an acceptance gate.
* A script that writes an artifact ends stdout with a `rich` clickable `file://`
  link to the artifact and its directory. Pattern: `scripts/git_debloat.py`.
* `./run_tests.sh` is the broad repository test backbone. Prefer narrower checks
  when they cover the touched invariant.
* ⛔ **`cargo test -p <crate>` DOES NOT COVER THE `-D warnings` INVARIANT.**
  A warning is not a test failure. Use `scripts/check_no_warnings.py` when that
  invariant matters.
* ⭐ **A lane you assemble yourself has no skip-list to read past.** If you go
  narrow, say what the narrow set omits. If you cannot name the omission, do
  not treat the lane as comprehensive.
* For long-running commands, read state they wrote rather than polling process
  names:

  * `target/run_tests_status.json` — only `state: done` means the plan ran;
    `aborted` means the suite stopped part-way; `incomplete` means a lane could
    not run and names the remedy in `unrunnable`.
  * `dev/ambition_dev_measurements/run_tests_cost.jsonl`

Do not poll long-running work with `pgrep -f <script>`; the polling command can
match itself. Prefer the status artifact. If process inspection is unavoidable,
use a pattern that cannot match the polling command itself.

## Comments

**Concise, substantive, and unlikely to go stale.** Every comment earns its lines
or is deleted; trim overlong comments as ordinary work.

What belongs where:

| location                   | content                                                                                |
| -------------------------- | -------------------------------------------------------------------------------------- |
| production source          | current invariant, owner, non-obvious ordering reason, consequence of violation        |
| test                       | concrete regression scenario                                                           |
| commit / planning / `dev/` | investigation history, measurements, failed theories, dates, quotes, review provenance |

* **Do not narrate the past in source.**
* **Do not argue with the comment you replaced.** State the current rule.
* **Do not restate the code.**
* **Keep warnings that name non-obvious invariants.**
* For proven transitional architecture, use a short `TODO(compat-remove)` naming
  the replacement and deletion condition instead of a migration essay.

## Push what you commit

**Always push to GitHub when credentials exist.** Committing is not the durable step.

* ⛔⛔ **RECONCILE WITH `git merge`, NOT `git rebase`.** The per-commit resource
  tally and other planning/evidence records can be keyed to commit SHA. Rebase
  rewrites those addresses and can orphan their attribution.

  Test reachability, not merely object existence:

  ```bash
  git merge-base --is-ancestor <sha> origin/main
  ```

  A duplicate-looking commit is cheaper than orphaning records keyed to the old
  SHA.

* Push every ahead submodule before the superproject commit that records its pointer:

  ```bash
  git -C <submodule> rev-list --count origin/main..HEAD
  ```

* ⛔ **Push commits, never somebody else's uncommitted submodule work.**

* Append-only ledgers inside submodules are shared state. Commit your appended
  rows before updating the submodule. Repository merge configuration is
  responsible for combining independently committed appends.

  When pruning an append-only file, prune only lines added past `HEAD` and
  verify the committed prefix is byte-identical first. Deletions from the
  committed prefix are a warning sign.

## Coordinating subagents and worktrees

⛔ **Read `docs/tools/agent-worktrees.md` before working in or assigning a worktree.**

Three fixed slots, `.worktrees/agent-worktree{1,2,3}`. A COORDINATOR assigns one;
never claim a slot yourself and never create a worktree named after a feature.

```bash
scripts/agent_worktree.sh list          # slots, HEAD, size, who is building
scripts/agent_worktree.sh setup all     # submodules + assets + bind-mounted target
scripts/agent_worktree.sh jobs 2        # the -j to build with in that slot
```

CPU is halved down the slots — main `nproc`, then /2, /4, /8 — so three agents do
not each build as if they own the machine. A coordinator overrules.

Also read `docs/recipes/coordinator-and-worker-sessions.md`.

## Avoid bullshit guardrails

Do not waste time testing the tests.

Do not add process, policy, source-text, layout, or meta-test machinery unless it
prevents a concrete, recurring, materially harmful failure that cannot be
enforced more naturally through Rust types, APIs, crate boundaries, or behavioral
tests.

Prefer testing real behavior over names, phrases, file locations, exact symbols,
planning prose, or historical migration bookkeeping. Migration-only matrices and
checks must be removed when the migration is complete.

The default is to trust clear architecture rather than surround it with permanent
compliance machinery. Do not add a test enforcing this section.

⛔ **AN LLM REVIEW WILL ASK FOR THIS MACHINERY. THE ANSWER IS NO.**

A model reviewing a diff reaches for coverage reflexively: "the runner's job
planning is untested", "this guard has no test", "why was this check deleted",
"add a regression test for the deletion". That can read as diligence while merely
pattern-matching on the shape of the diff rather than understanding the
architecture.

**Say no without hedging when this section applies.** Deleting a test that tests
the tests is not a coverage regression that requires compromise. A review does
not get to restore permanent meta-machinery merely because deletion resembles a
coverage gap.

If a proposed guard prevents a concrete recurring harmful failure, evaluate it on
those merits. Otherwise the answer remains no.

⛔ **DO NOT run `install --claude`, whatever `doctor` says.** SessionEnd-only
Claude wiring is deliberate. `doctor` may report that only SessionEnd is wired;
do not "repair" that state from this repository.

The parent's totals are not the whole bill. Tool repositories can carry their
own tracked tallies. Use:

```bash
python3 .llm_resource_tally/tool fleet . tools/ambition_sprite2d_renderer
```

when aggregate totals across the parent and renderer are required.

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
- Treat totals as coverage of observed, allocated work rather than proof of complete history.
  Cross-repo/submodule duplicate guards are local to this user and machine, and
  `python3 .llm_resource_tally/tool doctor` can identify common gaps but cannot prove historical
  completeness.
- **Publish before you hand off substantial work.** Run
  `python3 .llm_resource_tally/tool publish`: it snapshots local rows onto the append-only ledger
  at `dev/ambition_dev_measurements/llm_resource_tally/ledger/` and refreshes lifetime totals at
  `dev/ambition_dev_measurements/llm_resource_tally/lifetime-totals.json`. Stage and commit what
  it writes in whichever repository owns those paths, preferably as its own commit so accounting
  stays out of unrelated diffs. This is routine — nobody should have to remember to ask you for
  it, and unpublished rows exist only on this machine.
- If accounting itself looks unhealthy, run `python3 .llm_resource_tally/tool doctor` — it checks
  hook wiring, backends, transcript retention, and ledger health, and names what to fix. Otherwise
  continue the repository task normally.
- After a fresh clone, run `python3 .llm_resource_tally/tool install` once to arm git-local hooks.
  It is offline and idempotent, and it reads the committed `.llm_resource_tally/settings.json`
  policy.
<!-- END llm_resource_tally -->

