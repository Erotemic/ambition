# Agent guide for Ambition

This is the repository operating guide for coding agents. Keep it short, session-agnostic, and focused on routing. Put durable project knowledge in `docs/`, engineering memory in `dev/`, and generated navigation aids in `.agent/`.

## Core Values

* Avoid player-centrism. Value the principle of relativity.
* Find the elegant solution. Jon will push back on hacks.
* Correctness is emergent from elegance.
* **Pre-release engine, zero dependents.** Behavior and feel are NOT sacred until a polish pass — optimize for the elegant unified design, not for preserving current output. Delete duplicates, compat shims, and bridges on sight. Never fold a richer path onto a simpler one to "preserve" it; make the richer/general path universal and delete the rest.
* **Unified actors.** Every actor — the player included — is one body: kinematics + composable ability limbs + a capability mask, driven by a Controller (Human / Brain / RL) and observed via one `WorldView`. Player / Enemy / Boss / NPC are DATA (controller + capabilities), not types or code paths. The player's movement is the good base — make enemies and NPCs *rise to it* (adopt the rich limb pipeline), never drag the player down to a simpler path. Adding a character should be: author capabilities + pick a controller, zero core edits.
* **ONE BODY, ONE PATH — never bifurcate. This is the most-violated rule; read it before any combat/movement/visual/state change.** The player is an actor. Before you write *anything* keyed to "player" or "actor/enemy/boss" — an attack, a hitbox, a damage rule, a VFX/SFX emit, a shield, a reset, a state machine, a brain hook — run the **bifurcation smell test**: *"Does the other controller kind already do this on its own code path?"* If yes, you have found a **FORK**, and your job is to UNIFY onto the single shared seam and delete the other side — NOT to add a second site. **Adding a parallel emission site / state component / system / spec for an effect that already exists elsewhere is a BUG, not a fix — even if it compiles and every test passes.** A green test on a forked path is worthless. If you genuinely cannot complete the merge in one pass, do NOT add the parallel path "for now": route the new caller *through the existing seam* (extract one shared fn/system/event if none exists), and log the remaining merge in `dev/journals/code_smells.md` with `BIFURCATION:` as the first word. Melee is now unified end-to-end: the STATE (`BodyMelee`/`MeleeSwing`), swing MODEL (`AttackSpec`), slash VFX (`emit_melee_slash`), AND the strike SPAWN (`combat::hitbox::spawn_melee_strike` → one gravity-resolved box drives BOTH the damage `Hitbox` entity and the slash) are ONE path for the player and every actor. Do NOT reintroduce a `PlayerAttackState`/`ActorAttackState` split, a second slash emit, or a per-frame player damage loop — route every melee through `spawn_melee_strike`. The MOVEMENT driver is now unified at the engine entry: the player tick is ONE system (`player_body_tick`) that calls the SAME combined body tick the actor uses (`ae::update_player_with_tuning_clusters` ≈ the actor's `update_body_with_tuning_clusters`), differing only in the input frame and the player respawn POLICY. **The two-clock precision-blink split (responsive aim during bullet-time) is now purely `InputState::control_dt` — an INPUT affordance, not a simulation structure: the human sets `control_dt = real frame dt`; a brain leaves it `0` and runs everything at sim time.** The player tick and `update_ecs_actors` stay SEPARATE Bevy systems on purpose (merging the orchestrators into one god-system is NOT the goal); what's shared is the body-tick engine entry. The next melee elevation is the unified action/ability timeline (cancel windows, movement locks, armor/i-frames, resource costs, hurtbox swaps, anim binding) layered on the one strike seam. When a doc/keystone says "unification," it means *delete one path*, not "make them behave similarly."

## Cold start

For non-trivial work, read in this order:

1. `README.md`; `AGENTS.md`; `dev/README.md`; `dev/SEARCH.md`; `docs/README.md`.
2. `docs/planning/README.md` → `docs/planning/vision.md` + `docs/planning/tracks.md` (the master plan + live queue)
3. The crate's `MODULES.md` (its modules + the ONE concern each declares)
4. One focused concept, system doc, recipe, tool doc, or planning doc for the task

Do not read all of `docs/` or `dev/` by default.

## Source-of-truth order

1. Fresh user instructions.
2. **The master plan under `docs/planning/`** — the primary coordination surface
   for direction and tasking. Keep it current when work materially changes status
   or direction; exact same-commit bookkeeping is not a universal requirement.
3. ADRs under `docs/adr/`; concept pages under `docs/concepts/`.
4. Focused system/tool docs and recipes under `docs/systems/`, `docs/tools/`, `docs/recipes/`.
5. Brainstorms under `docs/brainstorms/` (Jon's — agents never write there).
6. Engineering memory under `dev/`; generated navigation indexes under `.agent/`.

`docs/current/` is retired (archived 2026-07-05); `docs/vision/` holds auxiliary
vision notes only — direction lives in `docs/planning/`. Historical notes under
`docs/archive/` are evidence, not authority; generated indexes aid localization
but do not override source files.

## Current architectural stance

- Ambition is Bevy-native. Do not resurrect backend-neutral constraints unless a new ADR says so.
- Prefer data-driven ECS flow: authored/generated data -> Bevy components/entities -> systems -> messages/effects.
- LDtk owns world/level authoring. RON room manifests are historical; RON may still be used for tuning, save/settings, and other data where appropriate.
- Preserve desktop, web, Android/mobile/touch, controller, and Steam Deck paths. iOS is deferred for hardware, not excluded.
- **Crate layering:** foundations and domain services feed the unified
  simulation heart; observation/presentation consume it; runtime/provider/host
  compose it; game providers own named content. `ambition_actors` is not awaiting
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

## Test placement

A test lives at the **narrowest scope that owns its invariant**: small local
invariants inline; large private modules in an adjacent `src/foo/tests.rs`
(**never widen a production API to move a test**); public/assembled-system
behavior in the owning crate's `tests/`; workspace source/dependency/module-size/
architecture rules ONLY in `tests/ambition_workspace_policy` (links no production
crate, so never compiles `ambition_app`). Use poison/non-vacuity checks for
reusable scanner infrastructure or realistic harmful cases, not automatically
for every declarative rule. Full guidance + commands:
`docs/concepts/test-placement.md`.

## Spatial authoring discipline (LDtk, gates, hitboxes)

Before asking "where exactly?", read
`docs/concepts/llm-spatial-authoring-discipline.md`: read the map, infer the
component's PURPOSE, place it on the seam that fulfils it, state the reasoning in
the commit. Asking "where?" is the wrong default.

## Engineering memory and benchmark candidates

Before a non-trivial patch: `rg -n "<subsystem>|<symptom>" dev/journals
dev/benchmark-candidates` (postmortems + invariant traps). Add durable lessons to
`dev/benchmark-candidates/` + its index — never transient state.

## Commit messages: detailed, plus a summary of the prompt that inspired them (why).

## Patch discipline

- Prefer reviewable changes with targeted validation; don't hand-edit
  `sandbox.ldtk` (use Ambition LDtk tooling); update concepts/recipes/ADRs/dev
  memory when a durable invariant changes.
- Formatting is advisory rather than an acceptance gate; do not fail or block a
  change solely because `cargo fmt` or `ruff format` was not run.
- Expected working-tree noise, never a mystery to investigate or stage on its
  own: a git hook rewrites `.llm_resource_tally/` (the `ledger.jsonl`, rollup,
  and badge) on every turn as resource-accounting bookkeeping. Leave those
  changes to ride along with an ordinary commit; do not flag them as suspicious,
  revert them, or treat them as another session's work. See the managed
  "LLM resource accounting" block below for the full policy.

## Script output convention

Any script that writes a file/artifact (a tool, not a pure library) ENDS its
stdout with a `rich` clickable `file://` link to the artifact AND its containing
directory, so it is one click to navigate to. Use `[link=file://…]…[/link]`
markup via `rich.print`, with a graceful `try/except ImportError` fallback to
plain paths. Pattern: `scripts/git_debloat.py` (`file_uri`/`format_path`),
`scripts/archive_agent_source.py` (`path_link`/`print_output_location`).

## Common validation commands

```bash
./run_tests.sh
```

Use narrower tests when a focused test already covers the touched concept.

## Avoid bullshit guardrails

Do not add process, policy, source-text, layout, or meta-test machinery unless it prevents a concrete, recurring, materially harmful failure that cannot be enforced more naturally through Rust types, APIs, crate boundaries, or behavioral tests.
Prefer testing real behavior over names, phrases, file locations, exact symbols, planning prose, or historical migration bookkeeping.
Poison tests are for realistic harmful states, not for proving that every scanner detects its own fixture. Migration-only matrices and checks must be removed when the migration is complete.
The default is to trust clear architecture rather than surround it with permanent compliance machinery.
Do not add a test enforcing this section.


<!-- BEGIN llm_resource_tally v1.0.0 (managed block — regenerated by `install`; edits below will be overwritten) -->
## LLM resource accounting
A git hook records measured token/model usage for commits and a Claude SessionEnd hook can sweep
non-committing work. The hook intentionally updates `.llm_resource_tally/`; these generated ledger, rollup, and badge changes are expected bookkeeping. Do not spend time investigating, reverting, rewriting, or cleaning them up unless a tally command reports an error; include intended accounting changes with an ordinary commit.

After cloning, run `python3 .llm_resource_tally/tool install` once to arm git-local hooks; it reads the committed `.llm_resource_tally/settings.json` policy. Do not hand-edit or hand-count ledger rows. Use `python3 .llm_resource_tally/tool doctor`
when accounting itself appears unhealthy; otherwise continue the repository task normally.
<!-- END llm_resource_tally -->
