# Stage 20 — Mythos task menu (autonomous overnight)

> **EXECUTED 2026-06-10** — see `21_stage20_attack_plan.md` (Morning handoff)
> for results. Status: A1 ✅ A2 ✅ (kit + knots-as-data; actor core blocked on
> the EnemyConfig.archetype field, smell-logged) A3 ✅ (the bisection: machinery
> lib ← ambition_content ← ambition_app) C1 ✅ (measured; see honest numbers)
> B1 ✅ (ambition_audio) B2/C4 triaged out (thin-crate, doc 15) B3/B4/C2/C3
> remain (C2-gravity found already-effectively-done).

A menu of high-value, **mythos-level** tasks for a top-tier autonomous agent to
**choose from** and run overnight. Each is large but bounded, self-verifying via the
differential harness, and either independent or with prerequisites stated. Pick by
appetite: one big chain, or one self-contained extraction, or the compile-time pass.

The strategic context: `ambition_sandbox` is **133k LOC — 84% of the codebase, one
crate**, so *any* edit recompiles all of it and agents can't navigate it. Nine
foundation crates are already cleanly extracted; the runtime-substance pass is done;
the machinery→content coupling is now light and guard-enforced. The vocabulary has
stabilized — the precondition for going further (doc 15) is met. The end-state target
is doc `04_crate_topology.md` (~20 crates: `machinery ← ambition_content ← app`).

---

## Shared context (applies to every task)

**The safety net — gate EVERY commit on all of these:**
- `replay_fixture_regression` — **bit-identical sim**. A correct refactor keeps it
  byte-for-byte. NEVER regenerate the fixtures.
- `scripted_gameplay`, `architecture_boundaries` (~20 boundary guards — add a new one
  for every boundary you win), `cargo test -p ambition_sandbox --lib`.
- `cargo build -p ambition_sandbox --features visible` (the render path compiles).
- The relevant portal/menu integration suites when you touch those areas.

**Method (proven across Stages 16–19):**
- **Facade pattern** — re-export from the old path so inbound `crate::x::…` sites need
  zero churn; move by `git mv`; the old module becomes a glob facade.
- **Interrogate entanglement** — for each coupling ask "is it essential or incidental,
  and what's the penalty to invert it?" Invert incidental couplings (lower crate owns
  the generic; consumer adapts via a marker/message), don't force essential ones.
- **Differential harness first, then port boldly** — large mechanical moves are *less*
  risky than they feel once the harness is green; don't over-deliberate.

**Constraints:** work on `main`; stage explicit paths (never `git add -A`); commit
trailer `Co-Authored-By: {your_model}` (e.g. {your_model} might be Claude Opus 4.8 <noreply@anthropic.com>); keep regen scripts
working on a fresh clone; commit completed, gate-green work without waiting to be asked.

**Reference docs:** `04_crate_topology.md` (target), `15_post_task_k_revision.md`
(sequencing wisdom — don't proliferate thin crates around a shifting core),
`16_ecs_layer_extraction_plan.md` (the trisection method), `runtime_extraction_backlog.md`.

---

## TIER A — The monolith bisection (the keystone; A1→A2→A3 is a chain)

The single highest-leverage arc for BOTH goals: split the monolith into
`machinery ← ambition_content (crate) ← ambition_sandbox (thin app)`. Isolating the
~30k LOC of most-churned code (content) means content edits recompile ~30k not 133k,
and it gives agents the clean "reusable machinery vs named Ambition content" axis.

### A1 — Make machinery content-free + unify the two content modules *(de-risking keystone)*
- **Value:** unblocks the entire bisection; immediately improves navigability.
- **Difficulty:** medium-hard (surgical inversions + a real dependency-direction fix).
- **Scope:** Two parts.
  1. **Invert the ~8 machinery→content couplings** so generic modules name no content
     (same pattern as portal/brain/music): boss sprite metrics in
     `presentation/rendering/actors.rs`; character catalog in
     `presentation/character_sprites/assets.rs` + `world/ldtk_world/conversion.rs`;
     brain boss patterns in `brain/boss_pattern.rs` + `brain/state_machine.rs`; quest
     hooks in `encounter/systems.rs`; the portal-pickup wiring in `items/pickup.rs`.
     Each: move the named bit to a content-side adapter, or invert via a
     registry/marker/message the content layer fills.
  2. **Unify `content/` (24.7k) and `ambition_content/` (5.2k)** — today they are
     **bidirectionally coupled** (7↔4 imports). Consolidate into ONE inward-facing
     content module with a single dependency direction (content → machinery only).
- **Done when:** a new `architecture_boundaries` guard asserts no generic/machinery
  module imports the content module; the two content trees are one; replay bit-identical.

### A2 — Untangle `content/features/ecs` into generic combat-kit vs named encounters *(THE knot)*
- **Value:** the hardest blocker in the repo; unblocks the Layer-2 mechanics crates
  (combat, encounter, boss-runtime) AND the content crate. Doc 16 deferred this twice.
- **Difficulty:** the flagship mythos task — sustained surgery across ~21.5k LOC.
- **Scope:** `content/features/ecs` mixes a thin GENERIC combat kit (hitbox / damage /
  health / teams / target-volumes / mount / breakable / pickup / overlay) with NAMED
  bosses, enemies, encounters, and reward chests. Separate them: the generic kit becomes
  `crate::mechanics::combat` (content-free, ECS-native, narrow public surface — NO
  parallel god-object, the old `ambition_engine` failure); the named encounters/bosses
  stay content and consume the kit via the established adapter seam. Keep the boss
  runtime (phase/attack primitives) generic where it already is; leave named attack
  *data* in content.
- **Prereq:** A1 (one content module, machinery content-free).
- **Done when:** the generic combat kit has no named-content imports (guarded); named
  bosses/enemies register through the content plugin; replay bit-identical; `--lib` green.

### A3 — Promote `ambition_content` to a crate + thin `ambition_sandbox` app shell *(the payoff)*
- **Value:** the bisection lands. Content (most-churned) compiles independently; the app
  shell becomes thin and obviously "wiring."
- **Difficulty:** large mechanical port, harness-verified (low conceptual risk after
  A1/A2, high volume).
- **Scope:** lift the unified content module into a real `ambition_content` crate
  depending only on the machinery crates; `ambition_sandbox` keeps the binaries + app
  wiring (`app/`, `host/`, `bin/`, `rl_sim/`) and depends on `ambition_content`. Facade
  re-exports keep inbound churn near-zero. Add the boundary guards (`content → apps`
  forbidden, etc. per doc 04 "Forbidden arrows").
- **Prereq:** A1 + A2.
- **Done when:** `ambition_content` builds as its own crate; a content-only edit no longer
  recompiles the machinery; replay bit-identical; all suites green; `--features visible` builds.

---

## TIER B — Big stable adapter-crate extractions (compile-time wins; mostly independent)

Large + stable = stays cached while you iterate elsewhere. Each is a self-contained
overnight pick. Order within is by independence (cleanest first).

### B1 — Extract `ambition_audio` (audio + music, ~4.3k) *(cleanest warm-up, still real)*
- **Value:** removes a stable 4.3k from the hot recompile path; clean win.
- **Difficulty:** medium (real runtime: Kira, music director, cues, radio).
- **Scope:** `audio/` + `music/` → `ambition_audio`. The music director is already
  **guard-enforced content-agnostic**, so the seam is mostly clean; route the few
  content-named cues through IDs/registries the content layer fills. Sandbox keeps a
  thin adapter that maps game events → audio messages.
- **Prereq:** none (independent). **Done when:** crate builds; sandbox depends on it;
  guard that audio names no content; replay bit-identical.

### B2 — Extract `ambition_dialogue` (dialog, ~2.1k)
- **Value:** stable, self-contained Yarn runtime out of the monolith.
- **Difficulty:** medium. **Scope:** `dialog/` → `ambition_dialogue` (Yarnspinner
  runtime + bindings); content-named commands route through a binding registry.
- **Prereq:** none. **Note:** confirm no other agent is mid-flight in `dialog/**`.

### B3 — Extract `ambition_render` / generic presentation primitives (~10.6k) *(biggest compile win)*
- **Value:** the largest single stable chunk; major incremental-rebuild win.
- **Difficulty:** hard — `presentation/` mixes generic 2D primitives with
  content-specific visuals (named character sprites, boss metrics).
- **Scope:** split generic presentation (sprite/quad/camera/fx primitives, the menu
  render glue) from content visuals; the generic half → `ambition_render`; named visuals
  stay content and feed it via components/messages.
- **Prereq:** A1 (the presentation→content couplings must be inverted first).

### B4 — Extract `ambition_devtools` (dev, ~4.9k)
- **Value:** removes the debug/trace/inspector layer (changes often during debugging,
  but its *consumers* shouldn't recompile when it changes).
- **Difficulty:** hard — the debug overlay reads *everything*; needs a read-only
  observation seam so devtools depends on machinery, not vice versa.
- **Scope:** `dev/` → `ambition_devtools`; define a narrow read-only state seam (the
  overlay/trace consume snapshots, not internals).
- **Prereq:** lighter after A1/A3 (clearer layering to observe).

---

## TIER C — Cross-cutting / visionary (ambitious; some independent)

### C1 — Compile-time deep pass *(independent, helps TODAY)*
- **Value:** faster builds now, before any crate split lands; orthogonal to the bisection.
- **Difficulty:** mythos if it requires real codegen-bloat surgery, not just config.
- **Scope:** run `cargo build --timings`; attack the worst codegen units; **audit
  `Reflect` derives** and large generic monomorphizations on hot paths (a known smell
  here — see `feedback_compile_time`); reduce deep generics / trait-object where it pays;
  tune `[profile.dev]` (codegen-units, `split-debuginfo`, `debug=1`, `opt-level` of deps).
  Report a before/after wall-clock table for clean + incremental builds.
- **Prereq:** none. **Done when:** measured incremental-rebuild improvement, no behavior
  change (replay bit-identical), documented.

### C2 — Generalize a NAMED mechanic out of content into a reusable mechanic crate *(north-star)*
- **Value:** directly serves "get named content out of core, generalized where
  possible" — turns a bespoke boss/ability into reusable machinery others can build on.
- **Difficulty:** high — requires designing the *right* generic abstraction (the project
  values "narrow specific types over wide generic ones; add knobs when use cases land").
- **Scope:** pick a strong candidate — e.g. the cut-rope boss arena, the gravity *mechanic*
  (`crate::mechanics::gravity` switch/zones/visuals → `ambition_mechanics_gravity`), or
  the held-items pickup/throw model → `ambition_mechanics_held_items`. Extract the generic
  mechanic + a content adapter that supplies the named instance. Prove reusability with a
  second tiny content instance or a scripted reachability test.
- **Prereq:** none for gravity/held-items (already mostly content-free); A1/A2 for combat-y ones.

### C3 — Time-domains / drivers-adapters seam for RL + multiplayer *(deep architecture)*
- **Value:** the long-term architecture target (shared sim-time, per-player input-feel
  time, drivers/adapters split — ADR 0010/0011). Future-unblocking, not compile-time.
- **Difficulty:** very high, design-heavy. **Scope:** formalize the sim-time vs
  player-input-feel-time split and the input *driver* (local / RL / network) adapter seam
  so multiple controllable entities can be driven by different sources on a shared sim
  clock. Land it behind the existing `WorldTime`/`SimDt`/`ClockState` and the
  universal-brain `ActorControl` seams. **Prereq:** none structurally; highest ambiguity —
  best for the most capable model, and may produce a design + first slice rather than a
  finished feature.

### C4 — Layer-0 leaf crates (`ambition_math`, `ambition_data`) *(finish-early / lower difficulty)*
- **Value:** dependency-graph clarity; modest compile win. **Difficulty:** low-medium.
- **Scope:** extract the pure AABB/geometry/numeric helpers (`ambition_math`) and the
  IDs/registries/validation types (`ambition_data`) per doc 04 Layer 0. Keep them tiny.
- **Prereq:** none. Good as a warm-up or an "if time remains" task.

---

## Recommended picks by appetite

- **Maximum impact, willing to go deep:** A1 → A2 (and A3 if the night is long). This is
  the largest single advance the refactor can make.
- **One clean, high-confidence win:** B1 (`ambition_audio`) or C2-gravity — self-contained,
  guard-verifiable, low ambiguity.
- **Help builds immediately, no crate risk:** C1 (compile-time pass).
- **Visionary, design-forward:** C3 (time-domains / RL+multiplayer seam).

Independent (pick any, no prereqs): A1, B1, B2, C1, C2 (gravity/held-items), C3, C4.
Sequenced: A2 (needs A1), A3 (needs A1+A2), B3/B4 (cleaner after A1).
