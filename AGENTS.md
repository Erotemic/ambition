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
7. `dev/journals` and `dev/benchmark-candidates` when engineering memory is relevant.
8. For review or architectural work, read `docs/reviewer-guide.md`.

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
2. The master plan under `docs/planning/`.
3. ADRs under `docs/adr/` and concepts under `docs/concepts/`.
4. Focused docs under `docs/systems/`, `docs/tools/`, `docs/recipes/`.
5. Brainstorms under `docs/brainstorms/` (Jon's — agents never write there).
6. Engineering memory under `dev/` and generated indexes under `.agent/`.

`docs/current/` is retired. `docs/vision/` holds auxiliary notes only. Superseded docs are deleted rather than archived and can be read from Git history.

⛔ **A DONE ITEM IN A PLANNING DOC IS A RECEIPT, NOT A CASE FILE.** Keep what was wrong, what fixed it, the commit, the guard, and any standing prohibition. Investigation belongs in the commit message and Git history. See `docs/planning/README.md#queue-contract`.

## Current architectural stance

* Ambition is Bevy-native. Do not resurrect backend-neutral constraints unless a new ADR says so.
* Prefer data-driven ECS flow: authored/generated data -> Bevy components/entities -> systems -> messages/effects.
* LDtk owns world/level authoring. RON remains appropriate for tuning, save/settings, and other structured data.
* Preserve desktop, web, Android/mobile/touch, controller, and Steam Deck paths. iOS is deferred for hardware, not excluded.
* **Crate layering:** foundations and domain services feed the unified simulation heart; observation/presentation consume it; runtime/provider/host compose it; game providers own named content. Do not carve `ambition_platformer2d_actor_monolith` merely because it is large. See `docs/architecture/engine-architecture.md` and `docs/planning/tracks.md`.

### Assets in worktrees

Binary asset payloads are git-ignored but may be PRESENT on disk. `ls` before concluding an asset is unavailable.

Assets and initialized submodules do not automatically travel to a fresh worktree:

```bash
python3 scripts/mirror_assets_for_worktree.py
```

This also initializes `game/ambition_map_assets`.
See `docs/recipes/adding-an-asset.md`.

Before the first build of every session:

```bash
scripts/setup/target_bindmount.sh --status
```

If the bind is absent:

```bash
scripts/setup/target_bindmount.sh
```

Do not build until the required target bind is active.

* Never use `rm -rf` under `target/`; use Cargo or repository cleanup tools.
* Do not substitute `CARGO_TARGET_DIR` for the main repository target-bind policy.
* If an old shadowed `target/` copy exists underneath the bind, report it rather than deleting it.
* Do not run concurrent Cargo builds against one target directory.

## Autonomous decision-making

When operating autonomously and you hit an architecture/design fork, make the choice Jon would most likely make and act. Read `docs/planning/decision-principles.md` and `docs/concepts/autonomous-decision-making.md`.

Do not stop to ask about an architectural choice you can resolve from those principles. Until a polish pass, current output/feel is not a preservation constraint.

If Jon explicitly asks in that turn to finish and wait:

```bash
python3 scripts/goal_guard.py --pause "Jon asked me to finish X and wait"
```

Extend an armed run with:

```bash
python3 scripts/goal_guard.py --extend 48h
python3 scripts/goal_guard.py --extend
```

Never hand-edit `.goal/active.json`.

## Programmatic Checks

A check that takes more than a minute is not cheap. Batch expensive gates rather than running them after every small edit.

⛔ **FREEZE THE TREE WHILE A GATE RUNS.** A verification result must describe one stable tree.

## Verification

Use the narrowest command that actually covers the change. Full matrix:
`docs/recipes/cheapest-sufficient-check.md`.

* Drive the real headless simulation when behavior matters (`headless` / `trace_replay`). If important state cannot be exercised headlessly, improve the harness.
* Test invariants/properties, not tuned values or unfinished feel.
* Replay/bit-identical tests are canaries, not cages. Re-baseline deliberate changes when appropriate.
* `cargo check -p <one_crate>` is not the assembled-app compile gate:

  ```bash
  cargo check -p ambition_app
  ```

* App integration tests live in one target:

  ```bash
  cargo test -p ambition_app --test app_it -- <module>
  ```

  The trailing argument is a filter. A green filtered run proves only that population.

* `--exact` with a bare test name can match nothing. Use the full module path.
* When a sense-flip is involved, manually read the assertion against the claim.
* A diagnostic probe whose outcomes are both informative should pass; inspect it before encoding the intended invariant.
* Do not routinely run:

  ```bash
  cargo test --workspace --tests
  ```

* Before finalization, when required by the verification recipe:

  ```bash
  cargo test --workspace --lib
  ```

* For character, movement, or combat changes:

  ```bash
  cargo test -p ambition_demo_smash_app
  ```

* For dependency, ownership, motion-authority, or architecture-boundary changes:

  ```bash
  cargo test -p ambition_workspace_policy
  ```

  If policy intentionally changed, update its rationale rather than adding a waiver.

* `cargo nextest` does not run doctests. Do not claim doctest coverage from a nextest-only run.

* Publishing changed art requires:

  ```bash
  ./scripts/regen/quality_variants.sh
  python3 scripts/check_quality_variants_are_fresh.py
  ```

### Generated assets

Large runtime assets are generated from small authored sources and are intentionally git-ignored.

Build generated runtime content with:

```bash
scripts/setup/generated_content.sh
```

Narrower commands:

```bash
./scripts/regen/assets.sh
./scripts/regen/sprites.sh --list
./scripts/regen/sprites.sh --target <target>
python3 scripts/grab_font_assets.py
```

Do not regenerate assets concurrently with a Cargo build.

Details:

* `scripts/regen/README.md`
* `docs/tools/generated-visual-tools.md`
* `docs/tools/generated-audio-tools.md`

### Authored data

A changed Rust type does not typecheck authored RON embedded inside `&str` literals. Search every authored occurrence of changed fields and run tests for each affected crate.

### Repository checks

Repository check scripts may default to advisory mode. Before relying on a zero exit code, confirm whether enforcement requires `--check` or `--strict`, and confirm the checker reached its normal verdict rather than crashing.

When adding or removing a workspace crate, run:

```bash
cargo test -p ambition_workspace_policy --test policy
python3 scripts/check_absence_contracts.py --check
```

Search for other crate-name allowlists, lockfiles, and repository tooling that may need updates.

### Long-running builds

Do not sit and watch a long build during coordinated work. Use an independent worktree for unrelated writing work.

Do not run concurrent Cargo builds against one target directory.

Nested standalone workspaces have their own `target/`. Route them into appropriate bound storage when disk pressure matters.

## Test placement

Tests live at the narrowest scope owning the invariant. Never widen a production API merely to move a test. See `docs/concepts/test-placement.md`.

## The Hall of Characters is NOT a special case

`hall_of_characters` is a generated engine stress test and exhibition.

⛔ **When it is slow, do not fix the Hall. Fix the engine.**

Do not hand-edit the level. Read `docs/concepts/hall-of-characters-is-not-special.md`.

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
* `./run_tests.sh` is the broad repository test backbone. Prefer narrower checks when they cover the touched invariant.
* `cargo test -p <crate>` does not cover the `-D warnings` invariant. Use `scripts/check_no_warnings.py` when that invariant matters.
* If you assemble a narrow verification lane, say what it omits.
* For long-running commands, use their status artifacts rather than polling process names.

## Comments

**Concise, substantive, and unlikely to go stale.**

| location | content |
| --- | --- |
| production source | current invariant, owner, non-obvious ordering reason, consequence of violation |
| test | concrete regression scenario |
| commit / planning / `dev/` | investigation history, measurements, failed theories, dates, review provenance |

* Do not narrate the past in source.
* Do not argue with the comment you replaced.
* Do not restate the code.
* Keep warnings that name non-obvious invariants.
* For real transitional architecture, use a short `TODO(compat-remove)` naming the replacement and deletion condition.

## Push what you commit

Always push to GitHub when credentials exist.

⛔ **Reconcile with `git merge`, not `git rebase`.** Repository evidence can be keyed to commit SHA.

Test reachability when needed:

```bash
git merge-base --is-ancestor <sha> origin/main
```

Push an ahead submodule before committing the superproject pointer:

```bash
git -C <submodule> rev-list --count origin/main..HEAD
```

Do not push another agent's uncommitted submodule work.

Append-only ledgers are shared state. Commit your own appended rows and let repository merge configuration combine independent appends.

## Coordinating subagents and worktrees

Read `docs/tools/agent-worktrees.md` before working in or assigning a worktree.

Use the fixed slots. A coordinator assigns them; do not claim one yourself.

```bash
scripts/agent_worktree.sh list
scripts/agent_worktree.sh setup all
scripts/agent_worktree.sh jobs 2
```

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

Prefer, in order:

1. clear architecture;
2. Rust types and APIs;
3. crate/dependency boundaries;
4. behavioral tests;
5. static/process guards only when the above cannot naturally enforce the invariant.

Migration-only guards should be removed when the migration is complete.

An LLM review asking for redundant coverage or meta-tests is not itself a reason to add them.

⛔ **Do not run `.llm_resource_tally/tool install --claude`.** SessionEnd-only Claude wiring is deliberate.

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
