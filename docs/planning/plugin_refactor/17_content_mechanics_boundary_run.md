# Stage 17: Content / ability boundary run (autonomous)

**Status:** DONE (2026-06-08) — executed autonomously across S1–S6 (see §9 log).
**Author intent:** make the repo easier for new agents to navigate and pull
named content out of the reusable core. This is **Phase 1** of the content-crate
path (draw the boundaries *inside* `ambition_sandbox` via Bevy plugins); the crate
split + `ambition_sandbox` rename + asset retargeting are **Phase 3**, explicitly
deferred (see §8).

---

## 1. Why this scope (findings that shaped it)

A survey of the current tree (2026-06-08) found:

- **Bosses are already well-separated.** `assets/data/boss_profiles.ron` holds the
  data; `crate::boss_encounter` is generic machinery; bespoke per-boss behavior
  (gnu_ton, cut-rope/smirking_behemoth) already lives in
  `crate::ambition_content::bosses`. Not a useful target.
- **Enemy/NPC rosters are *interwoven*, not movable as-is.**
  `content/features/enemies.rs` (1153 LOC) mixes 55 named enemy identifiers with
  reusable surface-walker / cluster machinery (`EnemyMut`, `ae::Block`, predicates).
  Splitting named-from-machinery here is real work → **Phase 2**, deferred.
- **The crate root is the #1 navigability problem: 61 `.rs` files.** 14 of them are
  loose **player ability / weapon mechanics** (each a self-contained, plugin-shaped
  system tied to a `crate::items::Item` ability). They have no shared home.
- **`intro/` is a near-leaf** named-story submodule (`IntroPlugin`); its only inbound
  importers are content-side (`ambition_content`, `content/banter`, sprite/asset
  registration).
- Duplicate/redundant subsystems exist (e.g. enemy vs player projectile) — noted by
  the owner as a *separate* axis; **out of scope** here (§8).

The highest-leverage **safe, autonomous** chunk is therefore: **(a) give the 14
loose abilities one clear home behind one plugin, and (b) move the `intro/` named
story content into the `ambition_content` nucleus** — then ratchet guards so the
root can't regrow. This declutters the root (61 → ~46 files), establishes the
ability layer the Phase-2 mechanic-crate extraction needs, and extends the proven
content-nucleus pattern — with zero behavior change and a strong existing test net.

---

## 2. Target in-crate layout (Phase 1 end state)

```text
crate::abilities/                         NEW — Ambition's player ability/weapon kit
  mod.rs            — AmbitionAbilitiesPlugin (umbrella; composes the sub-plugins)
  traversal/        — blink, dive, grapple, possession, mark_recall
  ranged/           — beam, meteor, shockwave, vortex, volley, bomb, sentry
  thrown/           — gravity_grenade, puppy_slug_gun
  (each file keeps its own Plugin/registration fn; mod.rs just re-exports + composes)

crate::ambition_content/
  intro/            — MOVED from crate::intro (named story slice + IntroPlugin)
  bosses/ items/ quests/ dialogue/ portal/   (unchanged; already here)
  plugin.rs         — AmbitionContentPlugin now also adds IntroPlugin

crate::mechanics/   — unchanged this run (gravity mechanic); abilities are a sibling
```

Rationale for `crate::abilities` as a **top-level** module (not under
`ambition_content`): the abilities are imported by several layers today (combat,
presentation, items); a neutral top-level ability layer avoids a content→reusable
inversion while still grouping them. Plan doc `04_crate_topology.md` already
sanctions "an Ambition-specific abilities module." Whether abilities ultimately fold
into the content crate or a dedicated `ambition_abilities` crate is a **Phase 3**
decision, not this run's.

---

## 3. Per-file classification & destination (the cut-lines)

| Current (crate root) | LOC | Destination | Notes |
|---|---|---|---|
| `blink.rs` | 313 | `abilities/traversal/blink.rs` | reachability-tested (`blink_run_reachability`) |
| `dive.rs` | 370 | `abilities/traversal/dive.rs` | reachability-tested (`dive_drill_reachability`) |
| `grapple.rs` | 223 | `abilities/traversal/grapple.rs` | |
| `possession.rs` | 332 | `abilities/traversal/possession.rs` | takes over a non-boss actor |
| `mark_recall.rs` | 330 | `abilities/traversal/mark_recall.rs` | `Item::MarkRecall` |
| `beam.rs` | 249 | `abilities/ranged/beam.rs` | |
| `meteor.rs` | 265 | `abilities/ranged/meteor.rs` | |
| `shockwave.rs` | 295 | `abilities/ranged/shockwave.rs` | highest inbound (9) — verify importers |
| `vortex.rs` | 231 | `abilities/ranged/vortex.rs` | |
| `volley.rs` | 179 | `abilities/ranged/volley.rs` | |
| `bomb.rs` | 161 | `abilities/ranged/bomb.rs` | |
| `sentry.rs` | 263 | `abilities/ranged/sentry.rs` | deployable turret |
| `gravity_grenade.rs` | 160 | `abilities/thrown/gravity_grenade.rs` | gravity-well grenade |
| `puppy_slug_gun.rs` | 184 | `abilities/thrown/puppy_slug_gun.rs` | named (puppy-slug summon) |
| `intro/` (8 files) | 1425 | `ambition_content/intro/` | named story slice |

**Not moved this run** (root files that are engine/shell/runtime, not abilities or
named content): everything else (`combat`, `kinematic`, `item_pickup`, `save`,
`actor*`, `interaction`, `portal_pieces`, `lunex_kaleidoscope_app`, `headless`,
`falling_sand`, the facade shims `physics`/`runtime`/`world`/`inventory`, etc.).
`shrine.rs` (healing/save shrine — a world interactable, not an Item ability) stays
at root this run; revisit when a `world_features` home is defined.

---

## 4. Strategy: import-rewrite, not facade

Intra-crate moves use **import rewrite** (`crate::blink::X` → `crate::abilities::traversal::blink::X`),
NOT root facade re-exports. The compiler guarantees completeness and we avoid
permanent alias cruft. Inbound counts are small (1–9 per file); each move greps all
`crate::<name>` references and updates them. `lib.rs` drops the per-file `pub mod`
and gains `pub mod abilities;`.

**Inversion check (per ability):** before moving, confirm the inbound importers are
acceptable. Because `crate::abilities` is a neutral top-level layer, ANY module may
import it — so moves cannot create a build break or a *new* guard violation. The one
thing to watch is intro (§5, S5) where a presentation/asset site imports content.

---

## 5. Execution slices (each = one build-green, test-gated commit)

> Ordering: abilities first (purely mechanical, lowest risk), intro next (one small
> possible inversion), guards + docs last.

- **S1 — Stand up `crate::abilities` + move traversal abilities.**
  Create `abilities/mod.rs` with `AmbitionAbilitiesPlugin` (composes the existing
  per-ability plugins/registration fns). `git mv` blink, dive, grapple, possession,
  mark_recall into `abilities/traversal/`; rewrite imports; register
  `AmbitionAbilitiesPlugin` in `app/plugins.rs` in place of the individual ability
  registrations it now owns. Gate (incl. `blink_run_reachability`,
  `dive_drill_reachability`).

- **S2 — Move ranged abilities.** beam, meteor, shockwave, vortex, volley, bomb,
  sentry → `abilities/ranged/`; rewrite imports; fold their registrations into
  `AmbitionAbilitiesPlugin`. Gate.

- **S3 — Move thrown abilities.** gravity_grenade, puppy_slug_gun →
  `abilities/thrown/`; rewrite imports; fold registrations. Gate.

- **S4 — Move `intro/` → `ambition_content/intro/`.** `git mv`; rewrite the ~6
  inbound imports; `AmbitionContentPlugin` adds `IntroPlugin` (drop any direct
  `IntroPlugin` add elsewhere). If a *presentation/asset* site reaches INTO intro
  (presentation→content inversion), invert it via the existing registration pattern
  (intro registers its sprites/bindings INTO presentation, content-pushes — intro's
  mod.rs already does this for `CharacterSpriteAssets`/`RoomCutsceneBindings`); if
  that inversion proves non-trivial, **leave `intro/` at root, record why in this
  doc, and continue** (do not block the run). Gate.

  **DONE (2026-06-08).** Landed cleanly — no inversion needed. `IntroPlugin` was
  already composed by `AmbitionContentPlugin` (`ambition_content/plugin.rs`); no
  separate add existed in `app/plugins.rs`, so only the path changed
  (`crate::intro::IntroPlugin` → `crate::ambition_content::intro::IntroPlugin`).
  The presentation/asset → intro sites (`presentation/character_sprites/assets.rs`,
  `assets/sandbox_assets/builders/visuals.rs`, `assets/sandbox_assets/tests/identity.rs`)
  are in-crate references that still build with just the rewritten path — intro's own
  registration INTO presentation (`CharacterSpriteAssets`/`RoomCutsceneBindings`) is
  unchanged and did not break, so the content-pushes inversion was not required.
  The `architecture_boundaries_named_content_registers_through_content_plugin` guard's
  two `crate::intro::IntroPlugin` needles were updated to the new path. All gates green.

- **S5 — Guardrails ratchet** (`tests/architecture_boundaries.rs`): add
  (a) **no loose ability files at the crate root** — assert the 14 names no longer
  exist as `src/*.rs` and live under `src/abilities/`; (b) **the abilities layer is
  composed by exactly one plugin** — `AmbitionAbilitiesPlugin` is the only place
  `app/plugins.rs` adds ability systems (grep guard); (c) keep the existing
  `ambition_menu` content-free + portal guards. Gate.

- **S6 — Docs + memory.** Update this doc's status to DONE with per-slice commits +
  an est-vs-actual table; update `runtime_extraction_backlog.md` / `README.md`
  success-criteria checkboxes; note the new root file count. Commit.

---

## 6. Verification gate (run after EVERY slice; all must pass)

```bash
~/.cargo/bin/cargo build -p ambition_sandbox
~/.cargo/bin/cargo test  -p ambition_sandbox --lib
~/.cargo/bin/cargo test  -p ambition_sandbox --test architecture_boundaries
~/.cargo/bin/cargo test  -p ambition_sandbox --test scripted_gameplay
~/.cargo/bin/cargo test  -p ambition_sandbox --test replay_fixture_regression
~/.cargo/bin/cargo test  -p ambition_sandbox --test blink_run_reachability \
                                              --test dive_drill_reachability \
                                              --test movement_axis
```
Final (once, after S6): `~/.cargo/bin/cargo build -p ambition_sandbox --no-default-features --features visible`.
Never regenerate replay fixtures. `cargo fmt -p ambition_sandbox` before each commit.

---

## 7. Autonomous-run rules

- **Do not stop for questions.** Work around blockers; if a slice's clean form is
  infeasible, take the stated fallback (e.g. S4) and record it here, then continue.
- **Stage explicitly; never `git add -A`.** Another agent owns
  `crates/ambition_sandbox/src/dialog/**` and `crates/ambition_sandbox/src/dev/dev_tools.rs`
  — never touch or stage those.
- Retry transient EMFILE; one commit per slice; commit messages end with the
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` trailer.
- Keep §5/§9 of this doc live-updated as the progress window (the owner reads, can't
  interject). Record wall-clock per slice for the est-vs-actual table.

---

## 8. Explicitly deferred (NOT this run)

- **Phase 2 — extract reusable machinery crates** (enemy/combat ECS kit, brain,
  mechanics, presentation adapter) by splitting named-from-machinery in
  `content/features/ecs/*`, `boss_encounter/`, etc. Required *before* a content crate.
- **Phase 3 — promote `ambition_content` to its own crate**, retarget its assets via
  `ambition_asset_manager`, and rename `ambition_sandbox` → e.g. `ambition_app` /
  `ambition_game` (the composer). Mechanical once Phase 2 lands.
- **Duplicate-subsystem unification** (e.g. enemy vs player projectile) — a separate
  generalization axis.
- `shrine.rs` and other world-interactable root files — await a `world_features` home.

## 9. Definition of done / live log

**DONE (2026-06-08).** All of S1–S6 committed, every gate green (including the final
`--no-default-features --features visible` Phase-1 persona build), the 14 ability files
live under `crate::abilities/{traversal,ranged,thrown}/` behind `AmbitionAbilitiesPlugin`,
`intro/` lives in `crate::ambition_content::intro`, the crate root is **47** `.rs` files
(down from 61: 14 abilities moved out; `intro/` was already a directory), and the new
guard (`architecture_boundaries_abilities_live_under_abilities_layer`) passes alongside
the existing 12.

**Ability registration reality:** 13 of the 14 abilities register **nothing** with the
Bevy `App` — they are pure-logic modules invoked from combat / item-pickup / projectile
systems. Only `traversal::possession` owns `App` state (its `PossessionState` resource,
init'd by `AmbitionAbilitiesPlugin`; the possession *systems* stay chained in
`register_player_simulation_systems` to preserve interleaved run-condition ordering). So
`AmbitionAbilitiesPlugin` is **intentionally thin** — one `init_resource` today — and
exists primarily as the single composition point the guard enforces and that future
ability plugins fold into.

| Slice | Est | Actual | Commit | Notes |
|---|---|---|---|---|
| S1 traversal abilities | 25m | ~consolidated | `425b3210` | S1–S3 landed as ONE commit (the three ability moves were mechanically uniform and gated together) |
| S2 ranged abilities | 25m | ~consolidated | `425b3210` | (consolidated into S1's commit) |
| S3 thrown abilities | 15m | ~consolidated | `425b3210` | (consolidated into S1's commit) |
| S4 intro → content | 30m | ~15m | `9ce0585d` | clean move, no inversion; updated boundary guard needles |
| S5 guards | 20m | ~15m | (this commit) | new `..._abilities_live_under_abilities_layer` guard: 14 names absent at root + present under `abilities/`, and `AmbitionAbilitiesPlugin` registered in `app/plugins.rs` |
| S6 docs | 10m | ~10m | (this commit) | this doc → DONE; README/backlog had no natural Stage-17 checkbox (left untouched per "don't invent"); memory updated |
