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

1. `README.md`
2. `AGENTS.md`
3. `dev/README.md`
4. `dev/SEARCH.md`
5. `docs/README.md`
6. `docs/planning/README.md` → `docs/planning/vision.md` + `docs/planning/tracks.md` (the master plan + live queue)
7. The crate's `MODULES.md` (its modules + the ONE concern each declares)
8. One focused concept, system doc, recipe, tool doc, or planning doc for the task

Do not read all of `docs/` or `dev/` by default.

## Source-of-truth order

1. Fresh user instructions.
2. **The master plan under `docs/planning/`** — the single source of truth
   for direction and tasking ("implement the plan in docs/planning" is the
   standing job). Its living-plan discipline (`docs/planning/README.md`) is
   binding: work commits update the plan in the same commit.
3. ADRs under `docs/adr/`.
4. Concept pages under `docs/concepts/`.
5. Focused system/tool docs and recipes under `docs/systems/`, `docs/tools/`, and `docs/recipes/`.
6. Brainstorms under `docs/brainstorms/` (Jon's — agents never write there).
7. Engineering memory under `dev/`.
8. Generated navigation indexes under `.agent/`.

`docs/current/` is retired (archived 2026-07-05); `docs/vision/` holds
auxiliary vision notes only — direction lives in `docs/planning/`.

Historical notes under `docs/archive/` are evidence, not current authority. Generated indexes aid localization but do not override source files.

## Current architectural stance

- Ambition is Bevy-native. Do not resurrect backend-neutral constraints unless a new ADR says so.
- Prefer data-driven ECS flow: authored/generated data -> Bevy components/entities -> systems -> messages/effects.
- LDtk owns world/level authoring. RON room manifests are historical; RON may still be used for tuning, save/settings, and other data where appropriate.
- Preserve desktop, web, Android/mobile/touch, controller, and Steam Deck paths. iOS is deferred for hardware, not excluded.
- **Crate layering:** foundations ← machinery (`ambition_actors`, being
  decomposed) ← presentation (`ambition_render`) ← content (`ambition_content`)
  ← app (`ambition_app`, the only crate naming both machinery and content;
  `architecture_boundaries` enforces it). The target stack and the teardown
  playbook: `docs/planning/engine/architecture.md` +
  `docs/planning/engine/decomposition.md`.

## Autonomous decision-making

When operating autonomously and you hit an architecture or design fork, **make
the choice Jon would most likely make and act** — read
`docs/planning/decision-principles.md` (Jon's criteria) and
`docs/concepts/autonomous-decision-making.md`. Reserve questions for
product/scope, irreversible/outward-facing acts, or true intent ambiguity;
otherwise infer and keep going — do not stall to ask. Until a polish pass,
output/feel is not a constraint. The gates: it compiles (including
`ambition_app`) and invariants hold.

## Verification

* **Drive the real headless sim — don't say "I can't test it."** The game runs
  headless (`headless` / `trace_replay` binaries); step the actual simulation
  and observe. If the real sim can't be exercised headlessly from some state,
  fixing THAT is the priority. Only visual feel is exempt (ships BLIND).
* **Test invariants and properties, not tuned values or feel** — the strongest
  are symmetry/covariance (C4 gravity rotation, through-portal). No new
  regression tests pinning unpolished behavior.
* **Bit-identical / replay tests are canaries, not cages** — a failure is
  information; re-baseline when the diff isn't egregious.
  Full doctrine: `docs/planning/engine/headless-verification.md`.

## Spatial authoring discipline (LDtk, gates, hitboxes)

Before asking "where exactly?", read
`docs/concepts/llm-spatial-authoring-discipline.md`: read the map, infer the
component's PURPOSE, place it on the seam that fulfils it, state the
reasoning in the commit message. Asking "where?" is the wrong default.

## Engineering memory and benchmark candidates

Before a non-trivial patch: `rg -n "<subsystem>|<symptom>" dev/journals
dev/benchmark-candidates` (postmortems + invariant traps). Add durable
lessons to `dev/benchmark-candidates/` + its index — never transient state.

## Generated indexes

`.agent/index/` is generated + git-ignored; each crate root's `MODULES.md` is
generated + committed. Regenerate/check: `python scripts/generate_agent_index.py
&& python scripts/check_agent_kb.py && python scripts/check_doc_links.py && python
scripts/modules_md.py`.

## Commit messages

- Make detailed commit messages as you might normally do it, but also include a
  summary of the prompt that inspired them. I.e. why the change is being made.

## Patch discipline

- Prefer reviewable changes with targeted validation.
- Do not hand-edit `sandbox.ldtk`; use Ambition LDtk tooling.
- Update concepts, recipes, ADRs, or dev memory when a durable invariant changes.

## Style: `cargo fmt` on modified Rust files; `ruff format` on modified Python files.

## Common validation commands

```bash
cargo fmt --check
cargo test -p ambition_actors --lib
cargo test -p ambition_content --all-features
cargo run -p ambition_app --bin headless
python scripts/check_agent_kb.py && python scripts/check_doc_links.py
python scripts/modules_md.py          # each crate's MODULES.md is current
```

Use narrower tests when a focused test already covers the touched concept.


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
