# Agent guide for Ambition

This is the repository operating guide for coding agents. Keep it short, session-agnostic, and focused on routing. Put durable project knowledge in `docs/`, engineering memory in `dev/`, and generated navigation aids in `.agent/`.

## Core Values

* Avoid player-centrism. Value the principle of relativity.
* Find the elegant solution. Jon will push back on hacks.
* Correctness is emergent from elegance.
* **Pre-release engine, zero dependents.** Behavior and feel are NOT sacred until a polish pass — optimize for the elegant unified design, not for preserving current output. Delete duplicates, compat shims, and bridges on sight. Never fold a richer path onto a simpler one to "preserve" it; make the richer/general path universal and delete the rest.
* **Unified actors.** Every actor — the player included — is one body: kinematics + composable ability limbs + a capability mask, driven by a Controller (Human / Brain / RL) and observed via one `WorldView`. Player / Enemy / Boss / NPC are DATA (controller + capabilities), not types or code paths. The player's movement is the good base — make enemies and NPCs *rise to it* (adopt the rich limb pipeline), never drag the player down to a simpler path. Adding a character should be: author capabilities + pick a controller, zero core edits.
* **ONE BODY, ONE PATH — never bifurcate. This is the most-violated rule; read it before any combat/movement/visual/state change.** The player is an actor. Before you write *anything* keyed to "player" or "actor/enemy/boss" — an attack, a hitbox, a damage rule, a VFX/SFX emit, a shield, a reset, a state machine, a brain hook — run the **bifurcation smell test**: *"Does the other controller kind already do this on its own code path?"* If yes, you have found a **FORK**, and your job is to UNIFY onto the single shared seam and delete the other side — NOT to add a second site. **Adding a parallel emission site / state component / system / spec for an effect that already exists elsewhere is a BUG, not a fix — even if it compiles and every test passes.** A green test on a forked path is worthless. If you genuinely cannot complete the merge in one pass, do NOT add the parallel path "for now": route the new caller *through the existing seam* (extract one shared fn/system/event if none exists), and log the remaining merge in `dev/journals/code_smells.md` with `BIFURCATION:` as the first word. Melee is now unified end-to-end: the STATE (`BodyMelee`/`MeleeSwing`), swing MODEL (`AttackSpec`), slash VFX (`emit_melee_slash`), AND the strike SPAWN (`combat::hitbox::spawn_melee_strike` → one gravity-resolved box drives BOTH the damage `Hitbox` entity and the slash) are ONE path for the player and every actor. Do NOT reintroduce a `PlayerAttackState`/`ActorAttackState` split, a second slash emit, or a per-frame player damage loop — route every melee through `spawn_melee_strike`. The remaining seam is the two DRIVER systems (`attack_advance_system` player-tick vs `update_ecs_actors` actor-tick) — they're different orchestrators over the same body (the movement-driver question), not a melee fork. The next melee elevation is the unified action/ability timeline (cancel windows, movement locks, armor/i-frames, resource costs, hurtbox swaps, anim binding) layered on the one strike seam. When a doc/keystone says "unification," it means *delete one path*, not "make them behave similarly."

## Cold start

For non-trivial work, read in this order:

1. `README.md`
2. `AGENTS.md`
3. `dev/README.md`
4. `dev/SEARCH.md`
5. `docs/README.md`
6. `docs/current/state.md`
7. One focused concept, system doc, recipe, tool doc, planning doc, or vision doc for the task

Do not read all of `docs/` or `dev/` by default.

## Source-of-truth order

1. Fresh user instructions.
2. ADRs under `docs/adr/`.
3. Current state under `docs/current/`.
4. Concept pages under `docs/concepts/`.
5. Focused system/tool docs and recipes under `docs/systems/`, `docs/tools/`, and `docs/recipes/`.
6. Planning, vision, and brainstorms under `docs/planning/`, `docs/vision/`, and `docs/brainstorms/`.
7. Engineering memory under `dev/`.
8. Generated navigation indexes under `.agent/`.

Historical notes under `docs/archive/` are evidence, not current authority. Generated indexes aid localization but do not override source files.

## Current architectural stance

- Ambition is Bevy-native. Do not resurrect backend-neutral constraints unless a new ADR says so.
- Prefer data-driven ECS flow: authored/generated data -> Bevy components/entities -> systems -> messages/effects.
- LDtk owns world/level authoring. RON room manifests are historical; RON may still be used for tuning, save/settings, and other data where appropriate.
- Preserve desktop, web, Android/mobile/touch, controller, and Steam Deck paths. iOS is deferred for hardware, not excluded.
- **Layered crate split (Stage 20, 2026-06-10):** `ambition_gameplay_core` is the
  gameplay core library: content-free simulation systems, runtime state, world/LDtk
  integration, player/session systems, combat/items/encounter machinery, persistence,
  schedules, and historical facade re-exports. `ambition_render` is the Bevy
  presentation layer (sprites, camera, parallax, HUD, dialog/cutscene UI, fonts,
  and render-only visual systems). `ambition_content` is the named game content
  (quests, bosses, rosters, dialogue, intro, banter, portal adapters) and depends
  on the machinery. `ambition_app` is the assembly + every binary
  (`ambition_game_bin`, `headless`, `trace_replay`, `rl_*`) + the full-stack
  integration tests, and is the only crate allowed to name both machinery and
  content. Machinery must not import content — `architecture_boundaries` enforces
  it. Schedule vocabulary (`SandboxSet` etc.) stays in
  `ambition_gameplay_core::schedule`.

## Autonomous decision-making

When operating autonomously and you hit an architecture or design fork, **make the
choice Jon would most likely make and act** — read
`docs/concepts/autonomous-decision-making.md`. The short version: most
architecture/implementation forks are yours to decide (reserve questions for
product/scope, irreversible/outward-facing acts, or true intent ambiguity); score
candidates by elegance (obvious single source of truth, follows seams, no hidden
ordering), the layer boundaries (Rust=behavior, RON=content, LDtk=space, machinery
imports no named content), runtime efficiency, maintainability, and conciseness;
refactor toward the better-scoring option rather than taking the easy path; prefer
single-commit replacement over compatibility shims (pre-release); and on a timed
or autonomous run, **infer and keep going — do not stall to ask.** Until a polish
pass, output/feel is not a constraint — refactor for elegance even when behavior
changes. The gates are: it compiles (including `ambition_app`) and invariants hold.

## Verification

* **Drive the real headless sim — don't say "I can't test it."** The game runs
  headless (`ambition_app` `headless` / `trace_replay` binaries): step the actual
  simulation from any state and observe how it progresses. The only thing you may
  be unsure of is visuals (and those are headed for headless render-to-disk
  spot-checks). If the real sim can't be exercised headlessly from some state,
  fixing *that* is the priority — never settle for a proxy/approximation.
* **Test invariants and properties, not tuned values or feel.** The strongest
  tests are SYMMETRY / COVARIANCE under the relativity principle — an action
  behaving identically under C4 gravity rotation and through portals — because
  those survive feel tweaks. Also: no OOB / wedge / NaN, determinism, feature
  composition. Do NOT write new regression tests to pin unpolished behavior.
* **Bit-identical / replay tests are canaries, not cages.** Their job is to flag
  when a change you *expected* to be behavior-neutral actually wasn't — a smell
  worth a look. Expect them to fail over time as elegance changes behavior; when
  the diff isn't egregious, just re-baseline the target (script the update if it's
  tedious). A failing canary is information, not a wall.

## Spatial authoring discipline (LDtk, gates, hitboxes)

If you are placing entities, gates, walls, hitboxes, or other map
geometry, read `docs/concepts/llm-spatial-authoring-discipline.md`
before asking the user "where exactly?". The short version: read the
map, infer the *purpose* of the component (block exit / block entry
/ gate progression), place it along the seam that fulfils that
purpose, and state the reasoning in the commit message. Asking
"where?" is the wrong default.

## Engineering memory and benchmark candidates

Before a non-trivial patch, search prior mistakes:

```bash
rg -n "<subsystem>|<symptom>|<failure class>" dev/journals dev/benchmark-candidates
```

Use `dev/journals/` for symptom postmortems and `dev/benchmark-candidates/` for invariant traps before refactors.

If you notice a reusable failure mode, invariant trap, or repo-specific question that would catch a future agent mistake, opportunistically add or update a benchmark candidate under `dev/benchmark-candidates/` and link it from `dev/benchmark-candidates/index.md`. Do this only for durable lessons, not transient task state.

## Generated indexes

`.agent/index/` is generated, intentionally ignored by Git, and should not be committed.

If `.agent/index/` is missing, stale, or needed for file/symbol/test lookup,
regenerate it before using it:

```bash
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
python scripts/check_doc_links.py
```

## Commit messages

- Make detailed commit messages as you might normally do it, but also include a
  summary of the prompt that inspired them. I.e. why the change is being made.

## Patch discipline

- Prefer reviewable changes with targeted validation.
- Do not hand-edit `sandbox.ldtk`; use Ambition LDtk tooling.
- Update concepts, recipes, ADRs, or dev memory when a durable invariant changes.

## Style

To keep merge conflicts simple to resolve use a style formatter.

- Use `cargo fmt` on any modified Rust files.
- Use `ruff format` on any modified Python files.

## Common validation commands

```bash
cargo fmt --check
cargo test -p ambition_gameplay_core --lib
cargo test -p ambition_content --all-features
cargo run -p ambition_app --bin headless
python scripts/check_agent_kb.py
python scripts/check_doc_links.py
```

Use narrower tests when a focused test already covers the touched concept.
