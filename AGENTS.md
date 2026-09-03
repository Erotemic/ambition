# Agent guide for Ambition

This is the repository operating guide for coding agents. Keep it short, session-agnostic, and focused on routing. Put durable project knowledge in `docs/`, engineering memory in `dev/`, and generated navigation aids in `.agent/`.

## Core Values

* Avoid player-centrism. Value the principle of relativity.
* Find the elegant solution. Jon will push back on hacks.
* Correctness is emergent from elegance.
* **Pre-release engine, zero dependents.** Behavior and feel are NOT sacred until a polish pass — optimize for the elegant unified design, not for preserving current output. Delete duplicates, compat shims, and bridges on sight. Never fold a richer path onto a simpler one to "preserve" it; make the richer/general path universal and delete the rest.
* Unified actors. Player / Enemy / Boss / NPC are controller, capabilities, and authored data—not separate engine types.
* **ONE BODY, ONE PATH.** The player is an actor; controller kind does not define a simulation path. Before adding behavior keyed to player/enemy/boss, check whether the behavior already exists for another controller kind. If so, unify onto one shared body/capability seam and delete the duplicate path. Do not add a parallel implementation “for now.” See `docs/concepts/one-body-one-path.md`.

## Cold start

For non-trivial work, localize in this order:

1. `README.md`, `AGENTS.md`, `.agent/README.md`, `docs/README.md`.
2. `python scripts/agent_query.py "<task words>"` before broad source search.
3. `docs/concepts/engine-mental-model.md`; skim `docs/concepts/invariants.md`.
4. `docs/planning/vision.md` plus the relevant `docs/planning/tracks.md` entry.
5. The likely crate's generated packet and `MODULES.md`.
6. ONE focused concept/system/recipe/tool doc or ADR.
7. `dev/journals` and `dev/benchmark-candidates` for the symptom or invariant.

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

`docs/current/` is retired. `docs/vision/` holds auxiliary notes only. `docs/archive/` is evidence, not authority.

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
EVERY SESSION, AND ACT ON WHAT IT SAYS.** The repo is on virtiofs; the script
SHADOWS `target/` with a directory on local ext4. The bind does not survive a
reboot and nothing re-establishes it, so an unbound session silently builds
through the shared mount into the slow directory underneath — minutes per check,
and a second full copy of every artifact accumulating where nobody looks.

⛔ **AND THE BOUND DISK FILLS — WHICH TAKES `/tmp` WITH IT.** On a VM where the
bind target lives on the same device as `/` (the calculex VM: `/dev/vda1`), a
full `target/` is a full ROOT. The symptom is not a cargo error: the harness's
own task files start failing with `ENOSPC` and command output is lost
mid-session, which looks like tooling breakage. One day of multi-profile work
reached 309 G of 309 G — `target/debug` 217 G, `target/notrace` 25 G,
`target/profiling` 18 G, `target/wasm32-unknown-unknown` 9 G.
⇒ **WHERE THE SPACE GOES, so you can see the shape of it without deleting
anything**: `target/debug/incremental` reached **156 G** here, more than half
the disk on its own, and the measurement targets (`notrace`, `profiling`,
wasm32) held another 50 G between them.
⛔⛔⛔ **THAT IS A SYMPTOM READING, NOT A REMEDIATION, and this paragraph used to
end with `rm -rf target/debug/incremental` as "safe — cargo rebuilds it".** It
contradicted the standing rule ten lines below it, and the contradiction was
LOAD-BEARING: an agent pruning `target/debug/{deps,examples,incremental}` by
mtime on 2026-09-03 was following this paragraph. Removed 2026-09-03. The rule
below is the one that stands; nothing here licenses a deletion.
⇒ **What to actually do:** run `scripts/setup/target_bindmount.sh --status`. A
`target/` that has grown enormous is almost always an ABSENT BIND, and repairing
it returns the space without deleting anything, because the duplicate was never
supposed to exist. Run `df -h /tmp` BEFORE starting a second target or profile
combination, not after — the cheap fix is not starting the second copy.
⚠ **A full disk can crash the gate mid-run**, and the traceback is an `OSError:
[Errno 28]` from `run_tests.py`'s own status writer rather than anything about
your change — `./run_tests.sh` had 6957/6957 tests passing before it fell over
writing its status file.

⛔⛔⛔ **AND NEVER `rm -rf` ANYTHING UNDER A `target/`. NOT `incremental`, NOT
`deps`, NOT "superseded" artifacts, NOT AS A FAVOUR WHEN THE DISK IS FULL.**
A target directory that has grown enormous is a SYMPTOM and the cause is almost
always this bindmount being absent. Run `--status` and fix the mount; the space
comes back on its own because the duplicate was never supposed to exist. If the
disk is genuinely short after that, SAY SO AND STOP — the reclaim is Jon's call,
on Jon's machine, and `cargo clean` is his to run.

⛔ **THIS IS WRITTEN FROM A REAL INCIDENT, 2026-08-27.** An agent skipped the
status check, built all day through virtiofs, was asked to "mark sweep the target
directory to clear some space", and deleted 205GB from the live target instead of
asking why 246GB was sitting there. Everything it removed was rebuildable and
that is not the point: the check it skipped names the problem in one line
(`state ⚠ NOT BOUND, and this worktree is on virtiofs`) and prints the command
that fixes it.

⚠ `./run_tests.sh` refuses to start on an unbound virtiofs target for this
reason. Do not work around it by any means other than running the script.

Do not substitute `CARGO_TARGET_DIR`; it applies only to commands launched from
that shell and does not establish the repository-wide target policy.

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
than 2 minutes is expensive. Agents have a bad habit of thinking that 20
seconds here or 30 seconds there is cheap enough. It is not. Many iterations of
these 10s of seconds checks adds real latency and has a massive negative impact
on throughput.

* **Batch the gate**: do not run the full gate every micro-edit. It belongs
  before a big commit, not between edits. Sometimes not even between commits,
  when they are part of a larger campaign.

* Do not double check with cargo. Tee its outputs to a file once if you need
  multiple operators over its output.

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
* ⛔⛔ **DO NOT SWEEP `cargo test --workspace --tests` — IT FILLS THE DISK.**
  It links many integration targets at once. Simultaneous linker failures across
  unrelated crates are likely an environment-resource problem; rerun one crate
  before diagnosing mass code failure. Prefer `--workspace --lib` plus named
  integration targets.
* ⛔⛔ **THE PER-TURN GATE DOES NOT RUN `--workspace --lib`.** Before push/finalization:

  ```bash
  cargo test --workspace --lib
  ```

  Keep this as a separate validation tier. ⛔ **DO NOT "FIX" THE CHEAP GATE BY
  ADDING THIS SWEEP TO EVERY TURN.**
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
* `cargo nextest run` is installed and `./run_tests.sh` uses it when it is.
  Every recipe above works with `nextest run` in place of `test`, and the reason
  to prefer it is DIAGNOSIS, not speed: it names each test's duration and calls
  out anything past 30s. Measured 2026-08-27, the app suite takes the same 265s
  under both runners because ONE test spends 164 of them
  (`a_player_death_reset_survives_the_rollback_window`) — libtest reports a
  total, so that had never been attributed.
  * ⛔⛔ **NEXTEST RUNS NO DOCTESTS.** It executes compiled test binaries and
    rustdoc's tests are not among them. `./run_tests.sh` carries a separate
    `cargo test --workspace --doc` job for exactly that reason. ⚠ The suite had
    no such job until 2026-08-27, and `ambition_sim_harness`'s only doctest had
    been failing to compile the entire time — nothing goes red when a runner
    silently stops covering a class of test.
  * Filters translate: `-k foo` is a bare `foo`, `--ignored` is
    `--run-ignored only`, `--include-ignored` is `--run-ignored all`,
    `--nocapture` is `--no-capture`.
* ⛔ **A compiling game can still draw stale quality-variant art.** Publishing art means:

  ```bash
  ./scripts/regen/quality_variants.sh
  python3 scripts/check_quality_variants_are_fresh.py
  ```

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
exit code, check whether enforcement requires `--check`.

Known advisory-by-default scripts include:

* `check_absence_contracts.py` — enforce with `--check`
* `check_doc_link_ratchet.py` — enforce with `--check`
* `check_planning_citations.py --vanished REF` — enforce with `--strict`.
  ⛔ Without it this one PRINTS every finding and still exits **0**, which is how
  it nearly shipped into `--maintenance` as a job that lists real problems and
  reports success (measured 2026-09-03: 13 findings, exit 0 bare, exit 1 with
  `--strict`).

`compile_ratchet.py` is intentionally the counterexample and fails by default.

⚠ `check_roadmap_evidence.py` was listed here until 2026-09-03 and **does not
exist** — deleted 2026-08-13 in `5e382342d` with nothing replacing it. The
enforcement flag is not the only thing to check before trusting a zero: so is
the script.

Do not duplicate a check in CI without first searching the workflow at the parent
commit.

### Long-running builds

⛔ **NEVER SIT AND WATCH A BUILD DURING COORDINATED WORK.** Give a worker
independent writing work in a worktree while `main` verifies.

⛔ **DO NOT RUN CONCURRENT BUILDS AGAINST ONE TARGET DIR.** Agent worktrees each
bind-mount their own, so builds in different slots are fine — see
`docs/tools/agent-worktrees.md`. Pace your `-j` to your slot.

The exhaustive plan `--run-everything-you-probably-dont-need-this` is intentionally
exceptional. Use it only when the verification recipe names your change.

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
* For long-running commands, read state they wrote rather than polling process
  names:

  * `target/run_tests_status.json` — only `state: done` means the plan ran;
    `aborted` is a suite the disk floor stopped part-way, and its `failed` list
    is empty because every job that started passed.
  * `dev/ambition_dev_measurements/run_tests_cost.jsonl`

Do not poll with `pgrep -f <script>`; the polling command can match itself.
This includes waiting for ABSENCE (`until ! pgrep -f run_tests.py`), which reads
as the opposite and strands the same way — the rule is that the pattern matches
the waiter, not that the test points one way. Six shells stranded that way
2026-09-03. Use `pgrep -f "[r]un_tests.py"` if you must, or read the status file.

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

* Push every ahead submodule before the superproject commit that records its pointer:

  ```bash
  git -C <submodule> rev-list --count origin/main..HEAD
  ```
* ⛔ **Push commits, never somebody else's uncommitted submodule work.**


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
