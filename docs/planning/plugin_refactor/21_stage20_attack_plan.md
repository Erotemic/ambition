# Stage 20 attack plan — A1 → A2 → A3 → C1 (overnight run)

**Status: PLANNED — awaiting kickoff prompt.** Jon picked the tasks and topology on
2026-06-10; further kickoff instructions are coming. Once the run starts, this doc is
the live progress window (per long-run discipline): the executing agent updates the
Progress Log at every commit point and fills the estimated-vs-actual table at the end.

## Authority & goals (from Jon, 2026-06-10)

The planning documents in this directory (docs 01–20) were written by weaker models.
**The executing agent may overrule any decision in them — including in this plan —
unless it is explicitly marked as Jon's.** When overruling, note it in the Progress Log
with the reasoning. Decisions in this doc explicitly from Jon: the task pick
(A1 → A2 → A3 → C1), the A3 sandbox-as-machinery-lib topology, the overflow order
(B1 then C4), and the goals below. The success criteria everything else serves:

1. **Better compile time.**
2. **Better agent codebase navigation.**
3. **Better idiomatic Bevy ECS with plugins** — module boundaries should be real
   `Plugin`s with their own registration, not facade-only re-export shells.
4. **Software that would pass human-level audits and is actually reusable** — make it
   easier to build great 2D platformers, not just to move files.

## Mission (decided 2026-06-10)

1. **A1** — make machinery content-free + unify the two content modules.
2. **A2** — untangle `content/features/ecs` into a generic combat kit (`crate::mechanics::combat`)
   vs named encounters.
3. **A3** — promote content to a real `ambition_content` crate, **sandbox-as-machinery-lib
   topology** (Jon's call): `ambition_sandbox`'s lib keeps the machinery; a new thin
   `ambition_app` crate takes the binaries + app wiring; the "sandbox really means
   machinery" rename is deliberately deferred (cheap later task, per doc 17 precedent).
4. **C1** — compile-time deep pass, if time permits.
5. **Overflow** (if the chain lands early or A3 proves infeasible): **B1** `ambition_audio`,
   then **C4** leaf crates (`ambition_math` / `ambition_data`).

## Safety net — gate EVERY commit

```bash
cargo test -p ambition_sandbox --test replay_fixture_regression   # bit-identical; NEVER regenerate fixtures
cargo test -p ambition_sandbox --test scripted_gameplay
cargo test -p ambition_sandbox --test architecture_boundaries     # ~18 guards; add one per boundary won
cargo test -p ambition_sandbox --lib
cargo build -p ambition_sandbox --features visible                # render path compiles
cargo fmt --check
```

Plus the portal integration suites (`portal_*`, `projectile_portal_transit`,
`held_projectile_portal_transit`, `repro_walls`) when touching pickup/portal wiring,
and `cargo run -p ambition_sandbox --bin headless -- 60` as a smoke after big moves.
After A3 these commands change package names — updating them everywhere (AGENTS.md, CI,
scripts, this doc) is **part of A3**, not optional cleanup.

**Constraints:** work on `main`; stage explicit paths (never `git add -A`); commit
trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` — sign as the model
actually executing, not the one doc 20 assumed (Jon's correction; in general the
trailer always names the executing model); keep `regen_sprites.sh` / `regen_assets.sh` working on a fresh clone;
commit each gate-green milestone immediately; never stop for blockers — work around,
note in the log (transient virtiofs EMFILE: retry, then move on).

**Smell discipline (Jon, 2026-06-10):** while working, opportunistically log code
smells to `dev/journals/code_smells.md` instead of chasing them — stay focused on the
big wins. Exception: a very clear fix with zero risk of slowing the main task may just
be done (and noted in the Progress Log).

---

## Phase A1 — machinery content-free + one content module (est. 3h)

### A1.1 The verified coupling map (from 2026-06-10 exploration)

Machinery (non-app) modules importing content, with the inversion for each:

| # | Site | Coupling | Inversion |
|---|------|----------|-----------|
| 1 | `content/features/mod.rs:207` | generic feature schedule calls `ambition_content::bosses::steer_cut_rope_boss_under_anvil` — **the one backward content→ambition_content edge** | content plugin registers the system into a labeled set; the generic schedule defines the set only |
| 2 | `brain/state_machine.rs:177` | `crate::content::features::NPC_PATROL_SPEED` | move the constant into `brain` (generic default) |
| 3 | `presentation/character_sprites/assets.rs:43`, `world/ldtk_world/conversion.rs:761` | character catalog `EMBEDDED_CATALOG` / `display_name_for_character_id` | generic `CharacterDisplayNameRegistry` resource in machinery; content plugin fills it at startup |
| 4 | `abilities/thrown/puppy_slug_gun.rs:97` | `crate::content::features::ActorFaction` | the type is already content-agnostic — relocate it to machinery |
| 5 | `encounter/systems.rs:56`, `boss_encounter/systems.rs:4`, `runtime/reset.rs:57` | `crate::content::quest::QuestRegistry` | split `content/quest.rs`: generic registry resource + advance/reset systems → machinery (joins Bevy-free `crate::quest`); named specs (`default_quest_specs`, pirate treasure rewards) stay content and are installed by the content plugin |
| 6 | `audio/mod.rs:44`, `audio/runtime.rs:100`, `music/intent.rs:17`, `assets/loading.rs:10`, `runtime/setup.rs:31` | `crate::content::data::{SandboxDataSpec, AudioSpec, SoundCueKey}` | `content/data.rs` has **zero named identifiers** — it is generic manifest machinery mis-homed; `git mv` it to machinery (e.g. `runtime/data.rs`), facade re-export from the old path |
| 7 | `items/pickup.rs:100` | `ambition_content::portal::pickup_portal_gun_system` | content plugin registers it inside `ItemPickupSet` (the set-label pattern the portal guard already enforces) |
| 8 | `menu/effects.rs:162` | `ambition_content::portal::equip_portal_gun` | route through a message / effect-handler the content plugin supplies |
| 9 | `mechanics/gravity/plugin.rs:72` | `ambition_content::bosses::reset_cut_rope_*` | content plugin registers the reset systems into the gravity plugin's labeled reset set |
| 10 | `dialog/yarn_bindings.rs:85,390` | `CutRopeHeavyObjectCycle`, `PendingCutRopeRoomReplay` | move those specific yarn bindings content-side via the binding-registry seam (also pre-paves B2) |
| 11 | `assets/sandbox_assets/builders/visuals.rs:148` | `ambition_content::intro::sprites::*` | move the intro-visual builder entries content-side or behind a registry the content plugin fills |
| 12 | `presentation/rendering/actors.rs` (doc-20 suspect) | boss sprite metrics | audit — exploration suggests this already flows through `FeatureViewIndex`; fix only if a real import exists |

NOT couplings (already correct, leave alone): all `app/` / `host/` / `bin/` / `rl_sim/` /
`headless.rs` imports of content — the app layer is the composition layer and is *allowed*
to name content (it moves to `ambition_app` in A3). The `QuestRegistry` resource-read
pattern in `app/hud.rs` / `app/feedback.rs` is correct once the registry type is machinery.

### A1.2 Unify `content/` (24.7k) + `ambition_content/` (5.2k)

Merge `ambition_content/*` **into** `content/` (smaller move; 6 forward import edges to
fix up, 1 backward edge already killed by row 1 above). The unified module keeps
`AmbitionContentPlugin` as its single registration entry point. In-tree name stays
`content/`; it becomes the `ambition_content` **crate** in A3.

### A1.3 Done when

- New `architecture_boundaries` guard: **no machinery module imports
  `crate::content`** — machinery scope = `src/**` minus `content/`, `app/`, `bin/`,
  `host/`, `rl_sim/`, `headless.rs`, `main.rs` (tests.rs files allowlisted where needed).
- One content tree, single dependency direction (content → machinery only).
- All gates green, replay bit-identical.

Commit cadence: one commit per row-group (constants/types, quest split, data.rs move,
catalog registry, portal/menu/gravity/dialog set-registrations, unification, guard).

---

## Phase A2 — `content/features/ecs` trisection (est. 4h)

The knot is smaller than its reputation: 14.3k LOC, of which ~7.9k is generic combat
kit, ~6.4k named, and only **three real generic→named violations**.

### A2.1 Mechanical move (generic kit → `crate::mechanics::combat`)

`git mv` the generic files next to the existing `mechanics::gravity`:
`hitbox.rs` (588), `damage.rs` (1607, after knot 1), `breakables.rs`, `pickups.rs`,
`hazards.rs`, `mount.rs` (after knot 2), `actors.rs` (1277, after knot 3),
`enemy_clusters.rs`, `npc_clusters.rs`, `aggression.rs`, `target_volumes.rs`,
`overlay.rs`, `view_index.rs`, `targeting.rs`, `variation.rs`, `held_items.rs`,
`banner.rs`, `chests.rs`, `falling_chest.rs`, `interact.rs`, `spawn_static.rs`,
`spawn_mounts.rs`. Old `content/features/ecs` keeps a glob facade so inbound
`crate::content::features::…` paths need zero churn. **No parallel god-object** — narrow
public surface, the old `ambition_engine` failure is the anti-pattern.

Per Jon's goal 3, `mechanics::combat` gets a real `CombatKitPlugin` that owns its own
system registration (sets, ordering, message types), mirroring `mechanics::gravity` —
the facade is only a transitional shim for inbound *paths*, never the registration
mechanism. Same applies to anything else that moves tonight: a moved module that can't
register itself via a plugin isn't done moving.

Stays named/content: `brain_effects.rs` (2309), `bosses.rs`, `boss_clusters.rs`,
`spawn_actors.rs`, `anim_helpers.rs`, `encounter_rewards.rs`, `save_sync.rs`, `spawn.rs`
(orchestration/dispatch). Layering note: existing `crate::combat` (979 LOC damage
primitives) stays the primitive layer below `mechanics::combat`.

### A2.2 The three knots

1. **`damage.rs` ~340–460** — named death side-effects (ExplodingMite blast,
   DividingMite split, Sandbag respawn, PirateOnShark held-item special) inline in the
   generic hit loop. Generic side emits a `NamedDeathSideEffect`-style message carrying
   archetype id + position; a content system consumes it. **Determinism risk — see below.**
2. **`mount.rs:32,39–45`** — `is_composite_spawn()` matches `PirateOnShark|PirateHeavyOnShark`.
   Replace with a `CompositeSpawn` marker component inserted by content `spawn_actors.rs`.
3. **`actors.rs:11–46`** — `shark_charge_crashed()` matches `BurningFlyingShark`.
   Replace with a `ChargeAttacker` marker + a pure-geometry crash predicate.

### A2.3 Determinism discipline (replay is bit-identical or the commit doesn't land)

- The message-based death side-effects must fire **the same frame, at the same point in
  the schedule** the inline code ran: register the content consumer immediately
  `.after()` the generic damage system, same schedule. Verify with a targeted
  minimal-App unit test that kills an ExplodingMite/DividingMite and asserts the
  side-effect entity exists that same tick (Bevy testing pattern + pre-poison).
- Moving system *registrations* can silently reorder ambiguous systems → pin explicit
  `.after()`/`.before()`/set labels for everything that moves; the 60f fixture won't
  cover mites, so the unit tests carry the named-side-effect proof.

### A2.4 Done when

- `mechanics::combat` has zero named-content imports — new guard (scan
  `src/mechanics/combat/**` for `EnemyArchetype`, `crate::content`, named boss/enemy ids).
- Named systems register through the content plugin; markers inserted at spawn.
- Replay bit-identical; `--lib` green; new unit tests for the three knots.

Commit cadence: knot 1 (+tests) → knot 2+3 (+tests) → mechanical move + facade → guard.

---

## Phase A3 — the bisection: `ambition_content` crate + thin app crate (est. 4h)

**Topology (decided): sandbox = machinery lib.** Same compile-time payoff as a big
machinery-crate move (content edits stop recompiling machinery either way) at a
fraction of the churn.

```text
ambition_sandbox (lib)   = the machinery (brain, encounter, presentation, world, physics,
        ^                  mechanics, items, menu glue, audio, dialog, dev, …)
        |
ambition_content (crate) = the unified content tree from A1/A2 (named bosses, enemies,
        ^                  quests, banter, intro, character catalog, AmbitionContentPlugin)
        |
ambition_app (crate)     = main.rs, app/, host/, bin/, rl_sim/, headless.rs +
                           the full-stack integration tests
```

### Steps

1. **New crate `crates/ambition_content`**: `git mv` the unified `content/` tree;
   sed `crate::<machinery>` → `ambition_sandbox::<machinery>` inside it. Sandbox keeps a
   facade `pub mod content { pub use ambition_content::*; }`? — **No**: sandbox cannot
   depend on content (that's the whole point). Any machinery reference to content is
   already zero (A1 guard); inbound `crate::content::…` paths exist only in app-layer
   code, which moves in step 2 and seds to `ambition_content::…`.
2. **New crate `crates/ambition_app`**: `git mv` `main.rs`, `app/`, `host/`, `bin/`,
   `rl_sim/`, `headless.rs`; depends on `ambition_sandbox` + `ambition_content` + the
   foundation crates. All `[[bin]]` targets move here **keeping their names**
   (`headless`, `rl_random_walker`, `rl_smoke`, `trace_replay`, the playable bin).
3. **Feature graph**: the 437-line sandbox manifest's personas (`visible`, `rl_sim`,
   `portal*`, `ldtk_runtime`, …) get forwarding features in `ambition_app` (and
   `ambition_content` where content is cfg-gated). This is the gnarliest mechanical
   part — build every persona: default, `--features rl_sim`, `--features visible`.
4. **Tests migrate with their stack**: `replay_fixture_regression` (+fixtures),
   `scripted_gameplay`, `architecture_boundaries`, portal suites → `ambition_app/tests`
   (they exercise the full stack). `architecture_boundaries` path constants update to
   the new layout; sandbox `--lib` tests that touch content move content- or app-side.
5. **Update every command site**: AGENTS.md, `.github/workflows/test.yml`, `scripts/`,
   profile scripts, planning docs — grep for `-p ambition_sandbox` and re-point test/run
   invocations (`cargo run -p ambition_app --bin headless`, etc.). Regen scripts must
   still work on a fresh clone.
6. **New guards** per doc 04 forbidden arrows: `ambition_sandbox` (machinery) has no
   `ambition_content`/`ambition_app` dependency (manifest + source scan);
   `ambition_content` has no `ambition_app` dependency.

### Done when

`ambition_content` builds as its own crate; **touching a content file does not recompile
`ambition_sandbox`** (verify with `touch` + `cargo build` and record the timing); replay
bit-identical from the new test home; all personas build; all suites green; command
sites updated.

Fallback if A3 stalls (e.g. the feature graph fights back past ~2h of debugging): land
whatever boundary is gate-green, revert the rest cleanly, write up the blocker in this
doc, and move to C1/B1. A1+A2 alone are a successful night.

---

## Phase C1 — compile-time deep pass (est. 2h, if time permits)

1. **Measure first** (and record here): clean `cargo build -p ambition_sandbox`,
   incremental after touching one machinery file, incremental after touching one content
   file (post-A3 this should already be the big win), `cargo build --timings` artifact.
2. Attack the worst codegen units: audit `Reflect` derives on hot types (known smell),
   large generic monomorphizations, deep generics → `dyn` where it pays.
3. `[profile.dev]` tuning: `codegen-units`, `split-debuginfo`, `debug = 1`,
   `opt-level` for deps via `[profile.dev.package."*"]`.
4. **No behavior change**: replay bit-identical. Deliverable: before/after wall-clock
   table (clean + both incrementals) in this doc.

## Overflow — B1 then C4

- **B1 `ambition_audio`** (~4.3k): `audio/` + `music/` → new crate. Music director is
  already guard-enforced content-agnostic; A1 row 6 moved `SandboxDataSpec` to
  machinery, which clears the audio module's only content import. Thin sandbox adapter
  maps game events → audio messages. Guard: audio names no content.
- **C4 leaf crates**: `ambition_math` (AABB/geometry/numeric helpers), `ambition_data`
  (IDs/registries/validation). Keep them tiny per doc 04 Layer 0.

## Risks & mitigations

| Risk | Mitigation |
|------|------------|
| Replay divergence from message-ified side-effects or registration reorder | Same-frame `.after()` ordering; targeted unit tests per knot; replay gate per commit |
| A3 feature-graph explosion | Build every persona at each A3 step; the 2h-stall fallback above |
| Slow gate cycle (~10min builds) eats the night | Order gates cheap-first (fmt → boundaries → lib → replay → visible); batch related edits per commit but never skip a gate |
| `dev/` overlay secretly imports content | Audit early in A1; if entangled, allowlist + log it as B4 scope rather than expanding tonight's surface |
| virtiofs EMFILE/EIO | Retry, then route around; never stop the run; log occurrences |
| Fixture corruption temptation | The fixtures are **read-only truth**. A diff means the refactor is wrong. Never regenerate |

## Progress log (live-update during the run)

> Executing agent: append a line per commit/milestone — `HH:MM — <hash> — what landed,
> gates status`. Note every deviation from the plan and every worked-around blocker.

- 2026-06-10 — plan authored; exploration reports captured (coupling map, ecs
  classification, safety-net mechanics). Run not yet started.

## Estimated vs actual (fill at end of run)

| Phase | Est. | Actual | Notes |
|-------|------|--------|-------|
| A1 machinery content-free + unify | 3h | | |
| A2 ecs trisection | 4h | | |
| A3 bisection crates | 4h | | |
| C1 compile-time pass | 2h | | |
| Overflow (B1/C4) | — | | |
