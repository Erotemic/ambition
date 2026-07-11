# The decomposition playbook — killing the monolith, carve by carve

**Authored by fable 2026-07-05; compressed 2026-07-09 once Phase D-A
completed.** THE highest-priority engineering track (Jon, binding):
`ambition_actors` and the fat app decompose into the crate set of
[`architecture.md`](architecture.md) — referred to below BY ROLE (*[the sim
heart]*, *[the space IR]*) — so that (a) small agents can navigate and modify
any one domain safely, (b) content/demos plug in without touching core, (c)
hot-edit rebuilds shrink.

**Phase D-A is COMPLETE.** The per-carve task cards have been executed and
compressed away; their record is the execution log in
[`../tracks.md`](../tracks.md), and the post-carve verification is
[`fable-final-audit-2026-07-07.md`](fable-final-audit-2026-07-07.md). What
remains here is what still binds: the method rules, the anti-god rules, the
measured ledger, the open residue, D-B, D-C, and the exit criteria.

**Anchor style (evergreen):** cite `path` + SYMBOL, never line numbers. If a
named symbol has moved, `rg` for it; if it's gone, that's drift — update the
doc in the same commit (living-plan discipline), don't guess.

## Method rules (all carves)

- **Measure OUTWARD deps first.** "Names no content" ≠ "extractable"; a module
  with dozens of inbound mechanic deps stays until inversions land.
- **The D2 template:** kill cycles/misplacements INSIDE the crate first
  (compiling, committable steps), then ONE atomic move of the module to its
  crate, then repoint every consumer. Never a lasting facade; delete
  re-export shims in the same arc.
- **Test accounting (BINDING — fable final audit F7):** before the atomic
  move, list every `#[test]` in the moved modules **by name**; after the move,
  every name must exist somewhere. A test that can't run in the new crate gets
  its FIXTURE moved (the `cfg(test)` fixture-manifest pattern), never deleted
  — the W3 carve silently dropped four ruled contract tests this way.
- **Compile-parity gates:** after each carve, `cargo build -p ambition_app
  --features rl_sim` + the suite trio (`ambition_actors` lib, content, app
  rl_sim) + the architecture-boundary tests. **The content suite must run with
  `--features portal`** (CC6 discovery: the portal adapter tests are
  feature-gated and the bare `-p ambition_content` gate silently skips them).
- **Feature discipline:** `ambition_runtime` forwards
  `headless`+`input`+`portal_ldtk`; new crates declare features explicitly;
  never rely on unification accidents.
- **Record compile-time before/after** per carve (`cargo build -p <crate>
  --timings`) — rebuild speed is half the point.

## Anti-god-structure rules (BINDING on every executor)

The failure mode this playbook exists to prevent is re-centralization — an
agent "simplifying" by putting things in one place. These are hard rules;
violating them is wrong even when it compiles and reads cleaner to you:

1. **No `utils`/`common`/`shared`/`prelude`-dump crates, ever.** A type with
   no clear owner means the classification is unfinished — finish it
   (vocabulary moves DOWN to the crate that OWNS the domain; facts invert to
   parameters).
2. **Every moved module keeps/ships its OWN `Plugin`** registering its own
   systems/messages/resources. The runtime GROUP composes plugins; it never
   absorbs their registrations inline. A carve may leave a crate with no
   plugin (pure vocabulary) — never the reverse (a plugin registering another
   domain's systems).
3. **The `features/` hub facade DIES; it does not migrate.** No new re-export
   hub may be created in `ambition_actors`, the runtime, or anywhere else.
   Consumers import from the owning crate, explicitly. A "convenience" hub is
   the monolith's ghost.
4. **One-way doors:** a lower tier may NEVER import a higher one, and sibling
   domain crates may not import each other except along the arrows
   architecture.md draws. When you want a sideways import, you have found
   either (a) a vocabulary type that belongs a tier down, or (b) a fact that
   should be a parameter/message. There is no (c).
5. **Resources are owned.** A resource is defined + initialized in exactly one
   crate (its plugin); other crates read it via system params. Cross-crate
   `init_resource` of another domain's type is a review flag.
6. **When splitting, split by AUTHORITY (who mutates), not by theme.** "All
   the boss stuff together" is a theme; "the systems that mutate
   `BossPhaseState`" is an authority. Themes produce god crates.

---

## THE LEDGER — measured 2026-07-09 (supersedes the 2026-07-06 projection)

> **UNITS (state them, always): every LOC figure in this document is TOTAL `src`
> lines INCLUDING TESTS.** This matches the 2026-07-06 projection's units — its
> baseline was 101.7k, which is exactly `ambition_gameplay_core`'s total src at
> that commit, and its residual breakdown (`features/` 20.6k + `player/` 6.6k +
> `abilities/` 4.2k) matches today's TOTALS, not today's production-only counts.
> A 2026-07-09 attempt to re-baseline this ledger in production-only lines
> **concluded the opposite of the truth** and was retracted; the units mismatch
> was the whole error. Compare like with like.

Every carve landed. Every carve also left an adapter shell behind, and the
2026-07-06 projection did not model that. **The old numbers said the residual
would bottom out at ≈31–35k and called it "the deliberate floor". It is 64.0k.**
Roughly half the projected ~64k actually left `ambition_actors`.

`ambition_actors` src, by subdirectory. The prod/test split is recorded because
it is genuinely useful for *scoping* a carve (test code travels with its module,
but it is not what makes a module hard to navigate) — it is **not** the
comparison against the projection:

| Subdir | LOC (total) | of which test | Shape |
|---|---:|---:|---|
| `features/` | 25.4k | 10.4k | the real actor domain (spawn/tick/perception/damage-routing/mount/bosses) + the surviving glue |
| `player/` | 6.7k | 2.4k | the last structural player-centrism; folds at S5/S6 |
| `boss_encounter/` | 5.5k | 1.5k | ~~adapter residue after the E6 three-way split~~ **NOT a shell (measured 2026-07-10, R2).** Live boss machinery: attack-geometry math, the phase-script runtime, the encounter entity, the behavior-profile schema. Reaches `crate::features` 53×. See the correction below |
| `abilities/` | 4.1k | 1.9k | D-B carve candidate (`ambition_abilities`), iff measurement is clean |
| `character_sprites/` | 2.7k | 1.4k | actor/content join: animation pickers, authored hitbox resolution, catalog-aware loading |
| `world/` | ~~1.9k~~ **1.5k** | 0.8k | overlay REBUILD (reads live feature components) + the avian physics adapter. The CONSUMPTION side (`CollisionWorld`) left in R3 → `ambition_world::collision`; re-measured 2026-07-10 |
| `projectile/` | 1.8k | 0.9k | the three woven steppers (charge input, victim routing, world collision) |
| `dev/` `items/` `encounter/` | 4.7k | 1.9k | sim-coupled adapters for their carved crates |
| `persistence/` | 1.3k | 0.1k | save-adjacent adapter |
| rest | ~9.9k | ~5.5k | time, session, body_mode, portal glue, gravity, roster, shrine, cutscene, assets tail, menu |
| **total** | ~~64.0k~~ **63.5k** | **27.8k** | after R2 (−0.0k here; it hit content+render) and R3 (−0.4k) |

Destination crates today (2026-07-06 measure, same units): `engine_core` 17.5k,
`characters` 17.0k, `combat` 9.5k, `render` 9.4k, `portal_presentation` 6.5k,
`sprite_sheet` 6.0k, `portal` 5.3k, `ldtk_map` 5.0k, `primitives` 4.1k,
`asset_manager` 4.0k, `persistence` 3.7k, `world` 2.9k, `audio` 2.9k, `sim_view`
2.8k, `menu` 2.4k; nothing else exceeds ~2.3k. `game/ambition_app` is 20.7k (its
`menu/` stayed app-side by the E1e ruling — the host stack + grid backend couple
up to items/player/sfx).

### RULING (Jon, 2026-07-10): the adapter floor IS the floor

The open question — "adapter floor, or a real carve left in `features/`?" — is
CLOSED, on evidence, per fable's own instruction to re-measure (U1) rather than
pre-commit. Three measurements decided it.

**1. The 64.0k is the true post-carve floor, not new code.** `ambition_actors`
was 68.0k total at the F8 audit close (2026-07-07, `3bdbef26`) and is 63.8k
today. It has SHRUNK 4.2k since the carves finished. The gap against the
projected 31–35k is genuine residue.

**2. The missing ~30k is not one carve. It is nine shells.** Projected "LOC out"
versus what actually stayed:

| Subdir | projected out | still resident | the shell |
|---|---:|---:|---|
| `combat/` | 12.8k | **0** | fully left ✅ — the proof a clean carve is possible |
| `world/` | 10.9k | 1.9k | ~~overlay rebuild~~ (LEFT in R3) + the avian adapter |
| `boss_encounter/` | 6.8k | **5.5k** | ⚠️ **NOT a shell — see the R2 correction below** |
| `persistence/` | 5.2k | 1.3k | save-adjacent adapter |
| `projectile/` | 4.4k | 1.8k | the three woven steppers |
| `character_sprites/` | 4.3k | 2.7k | the actor/content join |
| `dev/` `items/` `encounter/` | 8.6k | 4.7k | sim-coupled adapters |
| `menu/` | 3.2k | 0.8k | map UI hydration |

Plus `features/` overshooting its 20.6k projection by ~4.8k. **There is no 25k
carve hiding in `features/`.** There are nine adapter shells between 0.8k and
5.5k, each gated on a DIFFERENT technical precondition. Dissolving them is the
enumerated residue queue below, not a new decomposition phase.

#### CORRECTION (opus, 2026-07-10, executing R2): `boss_encounter/` is NOT one of the shells

The row above was wrong, and executing R2 is what proved it. The table conflated
the **E6 deferred teardown** (the fused `gnu_ton` profile + the split-layer render)
with **`boss_encounter/`'s 5.5k residency**. They are unrelated. The teardown
landed in R2 and removed 511 total src lines repo-wide — **337 from
`game/ambition_content/assets`, 147 from `ambition_render`, and net +26 from
`ambition_actors`.** `boss_encounter/` went 5456 → 5457 total src lines (units:
TOTAL, incl. tests). It did not shrink.

What `boss_encounter/` actually holds: boss attack-GEOMETRY math
(`attack_geometry/`, 2.0k), the phase-script runtime (`encounter_script.rs`), the
encounter entity, the boss behavior-profile schema + registry, and the sim
systems. It reaches `crate::features` 53 times (`BossRef` / `BossConfig` / the
cluster views / spawn). That is the boss half of the ACTOR domain, woven to it —
exactly the shape fable's own floor argument protects ("splitting spawn / tick /
perceive / damage-routing apart would re-fork the actor unification").

**So the shell count is eight, not nine, and the largest remaining one is
`character_sprites/` at 2.7k.** The ruling itself is UNAFFECTED and in fact
strengthened: the residue is not one missing carve, no further crate split is
owed, and `boss_encounter/` is a *reason* the floor is the floor rather than a
line item against it. The knock-on for the chain is that **R2 does not unblock
R4's victim-routing stepper** — boss types never left. See
[`refactor-chain.md`](refactor-chain.md) §R2/§R4.

**3. A further carve of `ambition_actors` buys no compile time.** Every
app-facing crate sits above it (`ambition` umbrella, `content`, `runtime`,
`sim_view`; then `render`, `host`, `touch_input`; then `app` and both demos).
Measured warm-incremental (single sample; read the ratio, not the constants):

- touch a leaf in `ambition_actors` → rebuild `ambition_app`: **104 s**
- touch `ambition_render`, which sits ABOVE actors → rebuild `ambition_app`: **72 s**

So ≥72 of those 104 seconds are the tower above `ambition_actors`, which no carve
of actors touches. And carving `abilities/` out is strictly worse: `app` would
depend on both crates and `actors` on `abilities`, so editing `abilities`
rebuilds abilities, then actors, then the whole tower.

**This confirms fable's own stated reason for a floor** (2026-07-06): *"splitting
spawn/tick/perceive/damage-routing apart would re-fork the actor unification
(U1)… Below the crate line, navigability is won by the D-B internal standard
(every module ≤ ~1.5k lines, one concern, `MODULES.md`), not by more crates."*

**Audit correction (2026-07-10): D-B is reopened.** The crate-split ruling still
stands, but the repo-wide navigability standard does not. Several engine/content
modules exceed ~1.5k lines, the script does not enforce the limit, and at least one
module map is stale. The standard must be made executable rather than cited as an
assumption supporting the adapter-floor ruling.

**Consequences.**
- No further crate split is owed by the ledger. `abilities/` is a *discretionary*
  candidate on navigability grounds only; the numbers do not ask for it.
- The residual shrinks by DISSOLVING SHELLS, one precondition at a time — see
  the residue queue below and
  [`refactor-chain.md`](refactor-chain.md), which sequences them.
- The compile-time lever is the tower (`render` 9.4k, `app` 20.7k, `host`), not
  `ambition_actors`. Do not carve actors expecting a rebuild win.
- Re-baseline this table whenever a shell leaves. **State the units.**

**Efficiency (why the split costs the game nothing):** crate boundaries are
COMPILE-TIME structure — the same systems run in the same schedule (E5's carve
was byte-parity-gated, the precedent). Rust inlines generics across crates;
the kernels already live in `engine_core`; thin-LTO is the lever if a boundary
ever shows in a profile. The costs that DO exist are paid deliberately: the E4
read-model copies view facts once per tick (bought: netcode/RL/render
decoupling — Q32), and the win is INCREMENTAL COMPILE.

### Why these pieces are THE pieces (the elegance argument)

The crate boundaries follow the four real fault lines in the domain, not line
counts: (1) **vocabulary vs. simulation** — schemas/registries/formats
(`entity_catalog`, `sprite_sheet`, `characters`) sit below the systems that
step them (`actors`, `combat`), so content and tools can depend on vocabulary
without dragging the sim; (2) **sim vs. space** — the world IR
(`ambition_world`) is authored INPUT to the sim, never a peer
(backend-agnostic by construction, which is what makes Tiled/Godot importers
additive); (3) **sim vs. observation** — `ambition_sim_view` is the one-way
read-model boundary (render, netcode confirmation, RL observation, and the
slower-light shaders are all THE SAME KIND of consumer); (4) **engine vs. host
vs. content** — `ambition_runtime` (headless sim assembly) / `ambition_host`
(windowed wiring) / content crates (named worlds+rosters+rules). Every demo
and the game compose from exactly these five faces.

### The demo/game → crate support matrix (the proof of sufficiency)

| Consumer | Exercises beyond the shared core |
|---|---|
| Sanic | momentum kernel (`engine_core::surface`), `ambition_world` chains channel, mode-scope seam |
| Super Mary-O | `ambition_items` equipment policies, camera policy knobs, cutscene kit |
| Super Smash Siblings | `ambition_combat` CM stack, N1 slot routing (`ambition_host`), fighter brain (`ambition_characters`), `ambition_sim_view` damage-meter read |
| Hollow Lite | boss pipeline (characters + encounter + combat), `ambition_persistence` (benches), respawn policy (actors) |
| Ambition itself | ALL of the above + portals, dialog, menu, audio, falling-sand content plugin — and hosts each demo via mode scopes |

Shared core in every column: runtime + host + actors + combat + world +
sim_view + characters + entity_catalog + input. If a demo needs a crate edit
outside its column's expectation, that's the oracle firing.

---

## Phase D-A — DONE

E1a–e (persistence, audio, dialog, dev_tools, settings-IR + the first
extension crate), E2 (combat kit + projectile model), E3 (sprite-sheet
absorb), E4 (the observation boundary + `ambition_sim_view`), E5 (the sim
assembly + `ambition_host` — **the demo gate**), E6 (boss tail), E7 (rename +
workspace re-home + facade dissolution), E8 (items), E9 (the `ambition`
umbrella + demo crate homes), W1–W4 (the world/LDtk split, `PlacementRecord`,
the lowering registry, ADR 0021), and the F1–F9 audit queue.

### The E4 contract (permanent, governs all future presentation work)

The carve relocated and SEALED the view types; the rules it fixed still bind:

- Extraction systems are FUNCTIONS of sim state, running LAST in the sim
  schedule. Presentation reads ONLY `SimView`. Plain data — no `Entity`
  borrows beyond opaque ids, no `Handle<T>`, no interior mutability — so
  netcode N3.1 serializes it for free.
- **View rows key by the same stable-id vocabulary the snapshot registry
  uses** (actor `config.id`, player slots, deterministic spawn ids): one
  identity system, two consumers. Render maps its presentation entities off
  those ids, never off sim `Entity` values.
- `PresentationFact` is the ONE event channel presentation consumes; its dedup
  identity is the triple `(tick, source SimId, kind)` — that is what resim
  suppression keys on.
- Render-spawned helper props are PRESENTATION CACHES keyed off view rows
  (despawn/respawn freely; never readable by sim). Anything the sim reads must
  be a sim fact. **A render-inserted component the sim queries (the old
  `BossAnimator` shape) is the boundary violation this carve exists to kill**
  — `ambition_render/tests/observation_boundary.rs` enforces it.

### Open residue (small, enumerated, not blocking)

- ✅ **E6 deferred teardown DONE (2026-07-10, `refactor-chain.md` R2):** the fused
  `gnu_ton` profile, `sync_boss_split_overlay`, `BossOverlayLayer`,
  `BossBodyLayer`, `apply_boss_split_body_z`, the split z-consts, and the
  `{boss_key}_body`/`_hands` sheet convention are gone. GNU-ton IS the ADR-0020
  linked pair: a `gnu_ton_rider` scholar boss aboard a `giant_gnu` mount actor
  whose hand LIMBS his strikes drive. `GIANT_GNU_SHEET` (formerly a byte-identical
  clone of the fused sheet) is now the primary layout. Note this removed nothing
  from `boss_encounter/` — see the correction above.
- **E7 named-content residue:** `dialog/speech_sfx.rs`'s voice table wants a
  content voice-profile registry; the `StartingCharacter` worn-sheet residue
  (`PLAYER_CHARACTER_ID` / `PLAYER_FILE_ROOT` in
  `character_sprites/attack_hitbox.rs`).
- **E-assets tail:** `assets/game_assets` still names gameplay/presentation
  vocabulary; it shrinks with the actor/presentation carves, never by
  reintroducing asset-manager upward deps.
- **E-enc adapters stay by authority:** LDtk loading, the live encounter tick,
  lock-wall contribution, switch-index rebuild, and
  `features/ecs/encounter_rewards.rs` spawn mobs/chests and write
  save/quest/banner state.
- ✅ **`world/overlay_rebuild.rs` LEFT (2026-07-10, `refactor-chain.md` R3).** It
  is now `ambition_world::collision` — `CollisionWorld` + the three composite
  builders + `MovingPlatformSet`. The spike found a dep the analysis below had
  missed (`ambition_portal::pieces::subtract_aabb`); rather than give the space IR
  a dependency on a gameplay mechanic, the pure rectangle set-difference moved
  DOWN to `engine_core::geometry`. `world/overlay.rs` — the REBUILD side — stays,
  as predicted. The `features/` hub re-exports it fed are deleted (anti-god
  rule 3). The original analysis:
  - `overlay.rs` is the REBUILD side. It queries breakables and pogo-target
    volumes and imports `crate::combat::*`. Actor-domain. **It stays.**
  - `overlay_rebuild.rs` is the CONSUMPTION side — it owns `CollisionWorld`, the
    single collision read-API. **Its inputs already became plain**, satisfying
    the condition this bullet always named: it touches `crate::` exactly three
    times, all for `MovingPlatformSet` / `MovingPlatformState` /
    `world_with_moving_platforms` — and the latter two ALREADY live in
    `ambition_world` (`world/platforms/mod.rs` is a `pub use` facade plus visual
    systems). `FeatureEcsWorldOverlay` already lives in
    `ambition_platformer_primitives`. Its inline tests use only `super::*` and
    bevy. `platformer_primitives` depends on nothing but `engine_core`, so
    `ambition_world → platformer_primitives` is acyclic. The only actor-local
    input left is the one-line `MovingPlatformSet` newtype
    (`ambition_actors/src/lib.rs`), which wraps an `ambition_world` type.
    NOT yet proven: that the move compiles once its consumers repoint. Spike it.
    This unblocks `ProjectileCollisionWorld` (see the projectile blockers in
    [`fable-final-audit-2026-07-07.md`](fable-final-audit-2026-07-07.md) F2).
  - `world/physics.rs` (debris/avian) is presentation-adjacent and can join the
    render/host side whenever.

---

## Phase D-B — ✅ RE-CLOSED (2026-07-11): repo-wide navigability standard

**All five re-close criteria are met.** The executable line gate exists and is
poison-tested (criteria 1–3, 5, Series 1), and the last-open half of criterion 4 — split
*or justify* every over-limit module — landed 2026-07-11: `snapshot.rs`, `moveset.rs`,
and `view_cones.rs` were split (see below), `MODULES.md` regenerated, the workspace count
corrected, and the ONE remaining over-limit module (`menu/kaleidoscope_app.rs`, a
declarative Lunex node tree) is *justified* via the named-waiver mechanism (criterion 3),
which criterion 4 explicitly permits ("split **or justify**"). The `engine.module-size`
gate is GREEN (28 policy checks). Nothing in the navigability standard remains open.

The crate-boundary ruling remains: do not split `ambition_actors` merely to chase
line counts or expected compile-time wins. D-B is the independent requirement that
agents can navigate the code *inside* those boundaries.

**Audit state (2026-07-10, since RESOLVED — see the re-closure note above):**

- the documented ~1.5k-line limit was unenforced then (`snapshot.rs`, `moveset.rs`,
  `view_cones.rs` genuinely exceeded it — all three split 2026-07-11; the audit's
  `smash/mod.rs`/`surface.rs` examples were imprecise — measured 1050 / 719, under limit);
- `scripts/modules_md.py` checks concern headers and generated maps, but does not
  enforce line size;
- `game/ambition_demo_smb1/MODULES.md` is stale in the audited tree;
- the workspace has 45 members (44 crates + the `ambition_workspace_policy` test-policy package), not the documented 42.

**Re-close criteria:**

1. Define scope explicitly. Production engine modules and simulation-bearing game
   content are in scope. Tests may have a separate threshold, but cannot silently
   disappear from the policy. Generated/data-heavy files are exceptions only when
   named explicitly.
2. Add a nested-module line-count check to the existing navigation script or a
   neighboring guardrail.
3. Encode exceptions as a **named waiver list with one path and one reviewed reason
   per entry**. Do not infer broad “generated” or “declarative” categories. Adding
   a waiver must be a visible review event.
4. Split or justify every over-limit module, regenerate every stale `MODULES.md`,
   and correct the workspace count.
5. Poison-test the line gate with a synthetic over-limit module and prove that a
   missing/unknown waiver fails.

**Progress:** criteria 1–3 and 5 landed in Series 1 (2026-07-10); criterion 4 COMPLETED
2026-07-11 (all three over-limit non-declarative modules split, `MODULES.md` regenerated,
count corrected, the one declarative module justified). **All five met — D-B re-closed.**
The executable gate is the `engine.module-size` policy
([`tests/ambition_workspace_policy/src/custom/module_size.rs`](../../../tests/ambition_workspace_policy/src/custom/module_size.rs)
+ its waiver list in
[`policies/module_size.toml`](../../../tests/ambition_workspace_policy/policies/module_size.toml);
migrated 2026-07-10 from the retired `crates/ambition_runtime/tests/module_size.rs`):
it walks every production `.rs` under `crates/*/src` and `game/*/src` (test files
excluded by path; inline `#[cfg(test)]` counts), fails any unwaived module over 1500
lines, and — bidirectionally — fails a waiver whose file is no longer oversized.
Exceptions are a named waiver list with one reviewed reason per path; nothing is
inferred. It is poison-tested (`poison_reacts` drives the real walk with a hostile
limit + a stale waiver). The stale `MODULES.md` was regenerated and the 45-member
count corrected. **What re-closed D-B was criterion 4's other half — now done:** the
over-limit debt list is cleared. The gate counts **total** lines (`s.lines().count()`, test
files excluded by path) against the 1500 limit — there is no separate "code-line"
count, so an earlier note calling `moveset.rs` (1536) "under the code-line limit"
was wrong: 1536 > 1500, and the gate flagged it as an **unwaived** violation. All
three over-limit non-declarative modules were split 2026-07-11 (`snapshot.rs`,
`moveset.rs`, `view_cones.rs` — see below). The waiver list is now down to **ONE**:
`kaleidoscope_app.rs` (1814), a declarative Lunex node tree — data-heavy by nature and
exactly the "generated/declarative" class the gate documents as a legitimate permanent
waiver. The gate is GREEN. **D-B's criterion-4 line-size debt is effectively cleared**:
every remaining over-limit module is a justified declarative waiver, not deferred
decomposition work.

**`snapshot.rs` (3684) → four sub-1500 modules — ✅ LANDED 2026-07-11.** The
pre-solved plan ran clean; final shape and the traps it hit:

- `snapshot.rs` → `snapshot/mod.rs` (**1169**), keeping the core: the traits
  (`SnapshotState`/`Cursor`/`Resolve` + `ResolveDecodeError`), wire primitives
  (`put_*`, `Reader`, `paste_put`/`PasteEncode`), `StateHasher`,
  `ApplyOutcome`/`EntryKind`/`StateEntry`, `SimSnapshot`/`take`/`duplicate_live_ids`,
  `RestoreReport`/`RestoreError`, hash/`DesyncReport`/plugin/`register_engine_sim_state`,
  and `canonical_f32_bits` (a wire helper `put_f32` calls — it had to stay with `put_f32`
  in `mod.rs`, NOT travel with the codec block).
- `snapshot/registry.rs` (**713**) ← `struct`+`impl SnapshotRegistry`,
  `SIM_RESOURCE_EXCLUSIONS`.
- `snapshot/restore.rs` (**471**) ← `respawn_from_the_room` + `validate_snapshot` +
  `restore` + `resource_names_available`.
- `snapshot/codecs.rs` (**1379**) ← every `impl SnapshotState/Cursor/Resolve for <T>`,
  the `snapshot_pod!`/`snapshot_unit_enum!` generators, `PasteEncode`/`paste_put`, and
  the `SimId` minting helpers. `use ambition_engine_core::body_clusters as bc;` moved
  with this block and was RE-DECLARED in `mod.rs` (the flagged gotcha — `register_engine_sim_state`
  stays in `mod.rs` and needs `bc`).
- **The gotcha the plan missed: cross-module privacy.** A child module sees its
  parent's private items (so every submodule's `use super::*` reaches the core, and
  `tests.rs` still sees everything), but a PARENT cannot see a child's privates and
  SIBLINGS cannot see each other's. Three items `restore`/`take`/`tests` read across
  that line had to open up: `SnapshotRegistry.entries`/`.messages` → `pub(super)`;
  `MessageChannel` moved from `registry.rs` up to `mod.rs` (parent → visible to all
  submodules, the same as `StateEntry`/`EntryKind`) so `restore` can call its `clear`;
  `SnapshotRegistry::ACTIVE_ROOM_ENTRY`/`ROSTER_ENTRY` → `pub(super)` for the tests.
- Verified: `cargo test -p ambition_runtime --lib` (46 snapshot tests green) + the app
  rl_sim `desync_canary`. Pure code RELOCATION + the visibility widenings above. The
  `snapshot.rs` waiver was deleted from `module_size.toml` (the bidirectional gate
  forces it — a stale waiver for a vanished file also fails).

**`moveset.rs` (1536) → two modules — ✅ LANDED 2026-07-11.** Never waived; a
pre-existing gate RED the corrected status note had masked. Clean builders/runtime
seam, no cross-section coupling:

- `moveset/mod.rs` (**862**) keeps the module's stated identity — the runtime half of
  the Smash model: the components (`MovePlayback`/`ActorMoveset`/`StrikeVolume`/
  `MovesetMelee`/`MoveEventMessage`), the `advance_move_playback`/`trigger_moveset_moves`/
  `dispatch_move_events`/… systems, the verb/VFX/SFX constants, and the tests.
- `moveset/prefabs.rs` (**691**) ← the build-time authoring: `attack_move_from_melee`,
  `Simple{Melee,Ranged,Charge}Params` + their `simple_*` builders + private
  `smp_*`/`srp_*`/`scp_*` defaults, `MovePrefabRegistry`, `Dir`/`directional_attack_variants`,
  and `build_actor_moveset`/`equip_equipment_row`. Re-exported (`pub use prefabs::*`) so
  the public `moveset::<builder>` API and `tests.rs`'s `use super::*` are unchanged.
- Verified: `cargo test -p ambition_combat --lib` (102 tests green), `cargo check
  -p ambition_app --features rl_sim` clean, and the module-size gate GREEN.

**`view_cones.rs` (2206) → runtime + diagnostics — ✅ LANDED 2026-07-11.** The waiver
had said "no natural seam extracted yet"; inspection found a clean one — the F1/F3
debug overlay and the text/PNG dump machinery are ~1080 lines that never run in a
normal render frame:

- `view_cones.rs` (**1145**) keeps the render path: the config types, `PortalViewRig`,
  the `sync_portal_view_cones` system and its `sync_cone_material_tint` helper, and the
  `geometry`/`mesh` submodules (unchanged).
- `view_cones/debug.rs` (**1078**) ← `debug_portal_view_zones` (gizmo overlay), the
  `handle_*`/`flush_*`/`write_*`/`*_debug_dump_text` F-key dump chain, `SourceClipDebug`,
  `PortalViewConeDebugRow`/`selected_portal_view_cone_debug_rows`, and the `fmt_*`
  formatting helpers. `view_cones.rs` stays a FILE (not `mod.rs`) — it already hosts
  `mod geometry;`/`mod mesh;`, so it just gained `mod debug; pub use debug::*;`, leaving
  the crate-level `view_cones::` path (lib.rs re-exports, plugin.rs registration)
  unchanged. `sync_cone_material_tint` (called by `sync_portal_view_cones`, line 856)
  stayed on the render side even though it sits among the diagnostics in source order.
- Verified: `cargo test -p ambition_portal_presentation` (45 tests green), `cargo check
  -p ambition_app --features rl_sim` clean; the module-size gate is GREEN with the
  `view_cones.rs` waiver deleted.

`MODULES.md` generation remains useful and the dissolved hub globs remain done;
those mechanisms are not sufficient to label the whole D-B standard complete.

## Phase D-C — the demo-hosting seam (ambition runs the demos)

**✅ DONE (2026-07-10, = `refactor-chain.md` R1).** Vision §5's last
decomposition artifact — the **scoped game-mode pattern**: a demo's rules crate
gates its systems on an area/room tag, not on global state, so several rulesets
coexist in one binary. Design detail in [`../demos/README.md`](../demos/README.md).

The shipped surface:

```rust
// ambition_world (RoomMetadata):          pub mode: Option<String>,  // merge: first Some wins
// ambition_platformer_primitives::lifecycle:
#[derive(Component)] pub struct ModeScopedEntity(pub String);  // + SpawnScopedExt::spawn_mode_scoped
// ambition_runtime::mode_scope:
pub fn in_mode(name: &'static str) -> impl FnMut(Option<Res<ActiveRoomMetadata>>) -> bool + Clone;
pub fn despawn_departed_mode_entities(..);  // ModeScopePlugin, in SandboxSet::Progression
```

A rules crate attaches every system `.run_if(in_mode("sanic"))` when hosted,
or unconditionally when standalone — the APP chooses via
`SanicRulesPlugin::hosted()` vs `::global()` (a constructor flag, not two
plugins). Mode resources live on a mode-owner entity carrying
`ModeScopedEntity`; leaving the mode's rooms tears them down through the same
lifetime-scope vocabulary a room-scoped entity uses.

The marker lives with its lifetime-scope siblings a tier below the sweep that
consumes it (the sweep reads `ActiveRoomMetadata`) — the same marker/sweep split
`RoomScopedEntity` already has. `ambition_runtime` therefore gained a direct
`ambition_world` dep, which `architecture_boundaries.rs` now allows by name.
Pinned by `ambition_runtime/tests/mode_scope.rs` + the umbrella oracle in
`game/ambition_demo_sanic`. Rationale for both deviations from the sketch above:
[`refactor-chain.md`](refactor-chain.md) §R1.

## Exit criteria (the whole playbook)

1. ✅ The monolith no longer exists (renamed residue included); every crate in
   architecture.md's stack is real with imports flowing downward, enforced by
   `game/ambition_app/tests/architecture_boundaries.rs`.
2. ✅ The named-content grep over engine crates hits zero (test fixtures
   allowed only under `cfg(test)`; the one `include_str!` is the sanctioned
   fixture pattern).
3. ✅ **MET (2026-07-10).** A demo app builds from runtime+host groups + its
   content crate with zero engine edits — `game/ambition_demo_sanic_app`, whose
   whole manifest is `ambition` + `ambition_demo_sanic` + `bevy`. It boots
   `add_headless_foundation` + `PlatformerEnginePlugins::fixed_tick()` +
   `PlatformerHostPlugins` + `SanicDemoContentPlugin` + `SanicRulesPlugin::global()`
   and steps the REAL sim: the body falls and lands on the authored speedway floor,
   and the mode-scoped act timer runs. **Gate-enforced** by
   `game/ambition_demo_sanic_app/tests/exit_3.rs`, so a future engine change that
   breaks a demo's ability to boot fails a test rather than a person.
   `cargo run -p ambition_demo_sanic_app --bin sanic_demo -- --ticks 600`.

   Fable ruled the demo binary "interactive work". That conflated the SHELL with
   the FEEL (see tracks.md §1's counter-argument, vision §7). The shell is
   architecture and ships now; the momentum tuning, the character sheet, and the
   drawn frame remain the interactive build. **The shell draws nothing, and that
   is an ENGINE gap, not a demo shortcut** — room/block visuals are still spawned
   by `ambition_app`'s room-flow. Filed as oracle-violation OV1 in tracks.md.
4. ✅ Workspace green: `cargo test --workspace --all-targets --features
   rl_sim` — 44/44 suites, zero failures as of 2026-07-09.
5. ✅ **REWRITTEN and met (opus, 2026-07-09).** The old criterion compared the
   hot-path rebuild against "the monolith baseline recorded before D-A" — a
   baseline that was never recorded, and which could no longer be reconstructed
   honestly anyway (the dependency tree has moved on; a pre-D-A checkout would
   time a different Bevy).

   A relative criterion against a lost number is not a criterion. What it was
   reaching for is *absolute and forward-checkable*: **does an edit reach a test
   quickly, and does authoring content cost less than editing the engine?**
   Measured on this machine, warm incremental, `dev` profile:

   | Loop | Edit | Rebuild | Wall |
   |---|---|---|---:|
   | A. sim TDD | leaf module in `ambition_actors` | `-p ambition_actors` | **3.2 s** |
   | B. foundation blast radius | `engine_core::geometry` | `-p ambition_actors` | **5.4 s** |
   | C. play loop | leaf module in `ambition_actors` | `-p ambition_app` | **104 s** |
   | D. **authoring loop** | module in `ambition_content` | `-p ambition_app` | **9.4 s** |

   **D is the decomposition's payoff, and it is the number to protect.** Content
   sits near the top of the DAG, so authoring a quest, a boss, or a room rebuilds
   in nine seconds rather than the whole game. C is the residual cost: everything
   above `ambition_actors` — sim_view, render, host, content, the umbrella, the
   app — must relink, and that is what shrinking `ambition_actors` further would
   buy. B shows the foundation is cheap to touch, which is why `engine_core` can
   keep absorbing vocabulary.

   **The criterion, going forward:** D stays under ~15 s and A under ~5 s. Both
   are ratchets — re-measure when a crate boundary moves, and record the new
   numbers here rather than deleting the old ones.
