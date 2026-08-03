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
- To wait on a long command, read state it WROTE — for the suite that is
  `target/run_tests_status.json` (`state`: running/done/crashed). ⛔ never poll
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


<!-- BEGIN llm_resource_tally v1.0.0 (managed block — regenerated by `install`; edits below will be overwritten) -->
## Resource accounting — the LLM compute this work costs (CRITICAL: do this every commit)

Every commit in this repo is produced by an LLM agent; we keep a **measured** per-commit
record of the tokens/model it cost (inference-time, energy & carbon are derived later).

- **After cloning**, wire the hook once (offline, idempotent): `python3 .llm_resource_tally/tool install`
- Thereafter every `git commit` auto-records. To record by hand: `python3 .llm_resource_tally/tool record`
- **At session end** (captures planning/chat that produced no commit): `python3 .llm_resource_tally/tool reconcile && python3 .llm_resource_tally/tool rollup`
- Codex agents: `python3 .llm_resource_tally/tool record --backend codex`
- Other non-Claude agents: `python3 .llm_resource_tally/tool record --backend <name> --transcript <path>`

**Tag what the work was** with `--label` (e.g. `record --label implementation`, or
`reconcile --label planning`) so non-code work is counted and attributable.

Tokens/model are MEASURED from your session transcript (deduped by message id — do NOT
hand-count). The ledger `.llm_resource_tally/ledger/` (at this repo's root) is append-only,
per-session, concurrency-safe, and stores measurements only.
<!-- END llm_resource_tally -->
