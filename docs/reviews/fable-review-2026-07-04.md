# Fable review — 2026-07-04: the architecture consolidation

**Authored by fable** after a full-repo review (four parallel deep audits:
gameplay_core module map + coupling histogram, workspace dep graph, planning-doc
reconciliation, content-in-core hunt) plus a front-to-back read of the
2026-07-02 review's E-log (E1–E66) and Jon's 2026-07-04 direction.

**What this doc is:** the verdict on the 2026-07-02→04 execution, the
adjudication of every fork queued for fable, and the roadmap from here to the
target architecture. The TARGET itself (crate map, plugin shape, content seams)
is `docs/planning/engine/architecture.md` — **rewritten today; treat the old
version as history.** The phase framing (P1–P5, demo matrix, M/U/Q registers)
in `docs/planning/roadmap.md` still stands; this doc is its execution
front-end.

**Relation to `fable-review-2026-07-02.md`:** that doc remains the RECORD
(audits A–D, adjudications AD1–AD5, E-log E1–E66). Its task sections were
already stale before this review; **do not work from it — work from THIS doc.**

---

## 1. VERDICT on the 2026-07-02→04 execution

**The run was excellent, and the log is honest.** Spot-checks of the headline
claims against code found no drift: the moveset subsumption is real (melee,
specials, ranged, and boss strikes all execute through `advance_move_playback`;
the flat paths are deleted, not shadowed), the boss body genuinely moves
through the shared flight limb (`step_floating_body` is gone), and
`BossAttackState` is a pure projection with the brain-write retired (E66) —
the intent/projection split (E65) was exactly the right prerequisite. The
verification discipline (parity nets first, full-workspace gates, the E39
leaf-rot finding) should be kept as standing practice.

**Where the state actually is** (trust this over every older doc):

- **§A actor unification: ~95% done.** One victim resolver, one knockback/
  stagger, one movement seam, one moveset executor, one perception port
  (non-boss). The REMAINING tail is A1's driver fold — three named slices with
  named blockers (E66): the integrate fold (blocked on the render-envelope vs
  collision-footprint fork — adjudicated below, AJ5), the brain fold (param
  ceiling + snapshot absorption), and `BossAnim`→`CharacterAnim` (BLIND).
- **§B frame bugs: DONE** with C4-harness canaries. B8/B12 residuals are LOW.
- **§C content-out-of-core: the seams are proven, the residue is enumerable.**
  C1 (items), C6 (sheet-specs + strike geometry), C7-render landed. What's
  left is a bounded inventory (§4, R3) — id consts, the world files, the
  roster embed, a handful of string matches.
- **§D decomposition: prepped but not carved.** D1/D2 done, D3 materialization
  done to the clean boundary, D4's linchpin (`RoomGeometry`) re-homed. No new
  crate has been cut yet — correctly, because the taxonomy and read-model
  shape had to settle first. They now have.

**Flagged-for-fable items, all closed this review:** E64 mount fork → AJ3.
E66 possessed-geometry-strike faction → R1.4. The `unified_melee` rl_sim RED →
stays in Jon's feel queue (it is a moveset-cadence tuning gap, not an
architecture defect; do not chase it in an architecture run). The BULK REVIEW
QUEUE deferred-tuning items remain Jon's feel pass, unchanged.

## 2. THE STATE, measured (2026-07-04)

- 25 workspace crates. `ambition_gameplay_core` = **~99.5k LOC** — half the
  workspace, 5× the next crate. `ambition_app` = 21k, of which **10k is a
  misplaced menu host stack** and 2.7k dev tools. `ambition_characters` 17k,
  `ambition_engine_core` 13.7k, `ambition_content` 10.6k, `ambition_render` 9.9k.
- **The dep graph is already cleanly layered**: only app/content/render/
  touch_input sit above gameplay_core; nothing below reaches up. The carve is
  therefore an *internal* decomposition problem, not an untangling of the
  workspace.
- **gameplay_core's real internal weight** (facades excluded): `features/`
  19.7k (the actor ECS sim — 480 inbound refs), `world/` 10.2k (`rooms` is a
  139-inbound universal spine), `combat/` 10.2k (the kit; mutually re-exporting
  with `features` BY CONSTRUCTION), `boss_encounter/` 6.3k, `player/` 6.2k,
  `persistence/` 4.5k (132 inbound, reaches UP into menu — the one god-dep),
  `character_sprites/` 4.3k, `abilities/` 4.1k, projectile pair 4.4k, plus a
  ~5k near-leaf harvest (time, quest, body_mode, host, inventory_ui,
  asset_publish, gravity, ability_cooldown, camera_snapshot).
- **Parallel-name split states:** portal = complete (the exemplar);
  cutscene/time/interaction = coherent; **combat = stalled at 1k of an 11k
  concern; menu = fragmented across three crates.** These two are the
  half-finished seams to finish (R4c/R4d).
- **Named-content residue in engine crates**, ranked by extraction cost
  (production only; full detail in R3): **(1) the baked asset payload** —
  `gameplay_core/assets/` IS the game (4 `.ldtk` worlds, 7 story `.yarn`
  files, the 56-track music registry, 213 build.rs-baked sprite RONs, biome
  parallax art, boss art) embedded via `include_str!`/build.rs; **(2) the
  `Item` enum's closed save-keyed SET** (`ITEM_COUNT=24` — C1 opened the
  metadata, not the set); **(3) `character_roster.rs`** (embeds the roster
  RON + Res-less free-fn API); **(4) `features/npcs.rs`** ~61-arm hardcoded
  bark tables (~450 lines, legacy fallback); **(5) `boss_encounter/sprites`**
  per-boss sheet defaults + enumerated boss arrays; **(6) `ParallaxTheme`**
  closed biome enum + alias table; **(7) render's `pirate_weapon.rs`** (a
  whole content weapon-visual module); **(8) `sync.rs`** boss-id→sheet match
  arms; **(9)** the 9 thin named boss constructors; **(10)**
  `PLAYABLE_ROSTER`; plus `features/{bosses,arena}.rs` id consts,
  `projectile/visual_kind.rs` (apple/glider), `falling_sand.rs` room/switch
  ids. Verified CLEAN (no action): `shrine.rs`, `quest/`, `music` director,
  `dialog` known-ids (derived), `ambition_engine_core`, `ambition_menu`,
  `ambition_audio` prod.

## 3. FABLE ADJUDICATIONS — every queued fork, resolved

### AJ1. The ability model (JD1) — the binding spec

Three tiers, all entering through data; core never matches a content key:

- **Tier DATA:** a full `MoveSpec` authored in RON (exists today).
- **Tier PREFAB (new):** character data may author `Prefab { key, params }`
  instead of a literal `MoveSpec`; a string-keyed **prefab registry** of
  constructors `(params) -> MoveSpec` expands it at roster install. The engine
  ships the standard kit — `simple_melee`, `simple_u_tilt`, `simple_ranged`,
  `simple_charge`, … — which are exactly `attack_move_from_melee` /
  `fire_move_from_ranged` generalized and made authorable; a game registers
  more. `sword_slash` = `simple_melee` + sword params, zero new code.
- **Tier TECHNIQUE:** `Effect { key, params }` events/sustains on the timeline
  fire content-owned Bevy systems (the proven `register_required_components`
  seam), now WITH params.

**Params value type — decided: (A) an opaque serde value.**
`EffectRef { key: String, params: ParamValue }` where `ParamValue` wraps
`ron::Value`; each effect/prefab hydrates its own `#[derive(Deserialize)]`
struct (`params.hydrate::<SwordSlashParams>()`). Rationale: typed AT the
effect, core stays ignorant (decomposable), zero registration ceremony, and
**it is not a corner**: the authored RON is byte-identical under option (B)
Bevy `Reflect`, so if a visual move editor lands later, swapping hydration to
the type registry is a mechanical migration — the data survives. (C)
`HashMap<String,f32>` is rejected: it cannot express structured params
(vectors, curves, nested tables). To keep (A) honest, add an **install-time
validation hook**: each registered technique/prefab may register a
param-schema check the content-validation pass runs against every authored
use — typos fail at startup, not mid-fight.

**Schema changes** (`ambition_entity_catalog`):
- `MoveEventKind::Effect { key }` → `MoveEventKind::Effect(EffectRef)`.
- `MoveWindow.sustain_effect: Option<String>` → `Option<EffectRef>`.
- **NEW `HitVolume.on_hit: Option<EffectRef>`** — fires with hit context
  (owner, victim, contact) when the volume LANDS. This is the missing
  conditional primitive: pogo, lifesteal, on-hit status, launch modifiers.
- Volumes gain a sprite-derived source (`VolumeShape::FromSpritePart { part }`
  or a parallel `source` field — executor's call) resolved per-tick by the
  frame-driven hitbox pipeline (AD2 generalized) — per-frame volumes are
  canonical (M7); this closes the "manifest box is richer" deferred-tuning
  item.

**Input→move mapping:** stays in the published character data via
`MovesetContract.verbs`, extended with directional intent: the trigger
resolves `(base verb, attack_axis, grounded)` → the most-specific authored
verb id with a documented fallback chain
(`attack_air_down` → `attack_down` → `attack`). The sprite generator emits
default mappings; smash-style tilt/smash variants later are MORE VERBS (data),
never a schema fork.

**Pogo — dissolved into the model:** a down-air move's Active volume carries
`on_hit: Effect("pogo_bounce", { rise })`; the pogo technique applies the
owner-frame bounce through the shared impulse seam, gated on the victim's
pogo-target capability. Generic platformer kit → ships as an ENGINE-provided
technique in the standard library (the registry is open either way).

**Items ↔ params — both, as Jon suspected:** numeric modifiers MERGE into the
params value at trigger-resolve (an equipment-modifiers component read where
the move/prefab is expanded); behavioral overrides are components the
technique reads. Numeric = data merge; behavioral = ECS.

**Dispatch shape — keep the message.** `Effect` events bridge to the existing
`ActorActionMessage::Special` channel, extended to carry the `EffectRef`
(params ride along). It is proven, ordered, and deterministic. The
marker-component + observer alternative is noted as a possible future
ECS-native reshape — revisit only when a real consumer needs per-entity
observation, not before.

**The player-melee fold rides this** (R2.5): directional variants = authored
moves selected by the verb map; pogo = the on_hit technique; the manifest
hitbox = sprite-derived volumes. The flat directional player path is then
DELETED — the last combat fork, and the player becomes the flagship
data-driven fighter (I7 made real).

### AJ2. The world seam (JD4) — the binding spec

- **`WorldManifest`** (roster-install pattern): content installs
  `{ entry_world, entry_room, worlds: [{ id, source }] }` where source is
  embedded bytes (web/Android) or a path (desktop hot-reload). Core keeps the
  `RoomSpec`/`RoomSet` kit + projection + validators and ships ZERO worlds;
  `secondary_world_ids()` and the `include_str!` embeds move to
  `ambition_content`. The hardcoded `"central_hub_complex"` start room dies
  with it.
- **Content-registered LDtk entity converters** (ADR 0009 — the crux): a
  registry `ldtk identifier → converter fn` producing the domain rows
  (`Authored<T>` lists / spawn plans). The engine registers the standard
  vocabulary (Solid, LoadingZone, Portal, GravityZone, EnemySpawn, …); a game
  adds its own at plugin-build time without touching the loader. This is the
  multi-session piece and the real "second game ships its own world" oracle.
- **Per-room mechanics, split by kind** (Jon adjudicated; the lightest seam
  each): hall-of-characters → pure `Authored<T>` data + content dialogue;
  falling-sand → a **self-gating content plugin** (gates on its room's
  presence; also resolves its `Res<Time>`/world-down VFX smells in the move);
  duel-arena staging → a content system consuming a **new `RoomLoaded
  { room_id }` message** emitted at the end of room staging. Start with the
  message — it is already the Bevy way; add a same-frame hook registry ONLY if
  a real consumer proves the one-frame delay load-bearing.

### AJ3. Mount authoring (E64 fork) — the `mount:` field wins

Author `mount: String` (optional) on the LDtk `EnemySpawn` entity, naming a
mount **archetype id**. The loader composes rider archetype + mount archetype;
the fused `pirate_on_shark`/`pirate_heavy_on_shark` brain keys retire (rider
keeps its own brain); the rider's display name IS the spawn name — 
`composite_rider_name`'s suffix-strip and `rider_name_suffix` are deleted.
Rationale: LDtk owns spatial/identity authoring (M8); a fused brain-key hides
a composition the data model should state; archetype-id (not a new mount
registry) because mounts are already roster rows. Execution = the 5-step plan
in E64 (the ldtk_tools subcommands exist). The 7 sandbox spawns re-author in
the same slice; `roundtrip` + `validate` gate it.

### AJ4. `BossAttackProfile` — collapse the 11 geometry variants to string keys

The enum's data half is already gone (E58 strike-geometry table, E62 sheet
RON, string-derived `move_id`). Finish it: profile identity becomes a plain
string key end-to-end — the 11 variants become built-in default entries in
the strike-geometry/sheet tables, `Special(String)` stops being special
(every profile is a key), and the anim-row/overlay keying resolves through the
RON sheet spec. ~72 refs / 8 files, a bounded rename+re-key slice gated on
the existing byte-identical RON pins + the four boss suites. After this, a
new boss is 100% RON: profile keys + strike rects + sheet rows + pattern.

### AJ5. A1 tail — the three remaining slices, shapes decided

1. **Integrate fold — split the envelope (the elegant option in E66, chosen).**
   `kin.size` IS the collision footprint for every body and `CenteredAabb`
   publishes from it universally (ONE rule); the boss's gross render/composite
   envelope becomes an explicit component (extend `ActorRenderSize` /
   introduce `BodyEnvelope` — executor measures which reads exist) consumed by
   `refresh_boss_damageable_volumes`' coarse bound and the boss sprite path.
   Then `integrate_boss_bodies` folds into `integrate_sim_bodies` with NO boss
   arm, and the deliberate `(0,0)` stagger gate becomes per-body
   `BodyHitFeel`-style DATA, not a branch. Gates: the four boss suites +
   `boss_motion_parity`.
2. **Brain fold:** absorb the remaining boss-only snapshot inputs (E30 started
   this), bundle params (the tuple pattern `tick_actor_brains` already uses),
   fold `tick_boss_brains` in, drop `Without<BossConfig>`. The boss's
   omniscient targeting joins the `WorldView` port here (the A7 boss
   remainder) — after which `BrainSnapshot.target_pos` can finally die.
3. **`BossAnim` → `CharacterAnim` — via the move clock (the deep fix).** The
   E37 render→sim write-back (`BossAnimationFrameSample`) exists because the
   RENDER animator owns the drawn frame. The moveset already carries
   `ClipBinding` + `phase_at` — the drawn attack frame becomes a SIM-side
   sample of the live `MovePlayback` phase, presentation reads it, and the
   write-back dies. Boss anim rows become `CharacterAnim` rows in the
   (already-RON) sheet spec. BLIND for visuals; mechanics pinned by
   frame-sample tests. This is also the moveset's clip-by-phase seam landing
   for EVERY actor — the last piece of "the move timeline is authoritative for
   gameplay AND presentation."

Plus **R1.4** (small): restore the possessed boss's geometry strike as a real
moveset-routed strike with the possessor's EFFECTIVE faction (E66's carve-out
(a) made honest).

### AJ6. The target crate map — ratified

`docs/planning/engine/architecture.md` (rewritten today) is binding: 6 tiers,
~30 crates, short names (no `_runtime` suffix scheme), grow-don't-mint,
mechanics core stays ONE crate (`ambition_actors`, renamed LAST). Key
reconciliations against the old lineup: `ambition_actor_control` /
`_actor_runtime` / `_combat_runtime` / `_game_runtime` do not happen as
named; their concerns land in `ambition_characters` / `ambition_actors` /
`ambition_combat` / `ambition_runtime`. The persistence↔menu knot resolves by
LAYERING (persistence below menu), not by one mega-crate. `falling_sand` is
CONTENT (a self-gating plugin), not an engine mechanic crate.

### AJ7. Housekeeping adjudications

- **`unified_melee` rl_sim RED:** feel-pass queue (moveset cadence), not
  architecture. Leave the test red and documented; do not loosen it further.
- **`ambition_touch_input`'s upward deps** (gameplay_core/render via
  menu-bridge): a later inversion rider on the menu consolidation (R4c); not
  its own arc.
- **`ambition_content`'s portal adapter glue:** stays — it is the *visible
  adapter* pattern the exemplar prescribes; the `content::features` re-export
  compat shim, however, deletes with the features-hub dissolution.
- **Stale docs — swept this review:** `docs/current/state.md`/`next.md`
  (2026-06-13) and `boss-system.md` now carry freshness banners pointing here;
  ADR 0016's faction section annotated as partially superseded (relational
  model landed, `ProjectileFaction` retired).

## 4. THE ROADMAP — R-phases from here to the target

Ordering logic: finish unification while the surface is hot (R1/R2 — every
later extraction gets cheaper with the forks gone), then evict content + build
the world seam (R3 — so crate labels become honest), then carve in dependency
order (R4), then assemble the engine face (R5) and prove it (R6). R1/R2
(combat+boss surface) and R3 (world+content surface) are largely DISJOINT —
safe to run as parallel agents if desired.

### R1 — close the unification arc (≈1–2 sessions, autonomous, BLIND bits marked)
R1.1 envelope split + integrate fold (AJ5.1) → R1.2 brain fold + boss
perception (AJ5.2) → R1.3 BossAnim via move-phase sampling (AJ5.3, BLIND) →
R1.4 possessed-strike effective faction → R1.5 the `Without<BossConfig>` /
player-branch sweep (exit: only documented POLICY remains — the P1 exit).

### R2 — the ability model (≈2 sessions, autonomous; player fold BLIND)
R2.1 `EffectRef` schema (events/sustain/on_hit unified) → R2.2 params
plumbing + install-time validation → R2.3 prefab registry (generalize the
existing constructors) → R2.4 directional verb selection → R2.5 the player
melee fold (directional moves + pogo technique + sprite-frame volumes; DELETE
the flat path; BLIND) → R2.6 equipment→params merge. Exit: the player is a
data-driven fighter; `MoveSpec`+prefabs+techniques express every shipped move.

### R3 — content eviction + the world seam (≈3–4 sessions, autonomous)
- **R3.1** `WorldManifest` + converter registry + `RoomLoaded` (the
  multi-session crux).
- **R3.2** the ASSET-PAYLOAD move (violation #1): `gameplay_core/assets/` →
  content, seam by seam on the proven "empty default = built-in" override
  pattern — worlds/start-room (rides R3.1), dialogue `.yarn` set, music/sfx
  registries, the build.rs sprite-RON bake, backgrounds + boss art,
  `character_roster.rs` data + a non-Bevy install seam for the LDtk parser
  (violation #3), `PLAYABLE_ROSTER` (#10).
- **R3.3** room mechanics by kind (falling_sand → self-gating content plugin
  incl. its room/switch ids, duel-arena → `RoomLoaded` content system, hall →
  authored data).
- **R3.4** named-residue sweep: `features/{bosses,arena}.rs` id consts; the
  npcs.rs bark tables → catalog `barks` (delete ~450 lines, #4); boss sheet
  defaults + enumerated arrays → `boss_sheets.ron` (#5); the `sync.rs` id→
  sheet arms → a `sprite_target` field in boss data (#8); the 9 named boss
  constructors → `from_data` callers (#9); `ParallaxTheme` → string-keyed
  themes (#6); projectile visual kinds → C5 string-keyed art registry;
  render's `pirate_weapon.rs` → data-driven held-weapon visual or content
  presentation (#7).
- **R3.5** mount field (AJ3) → **R3.6** profile-key collapse (AJ4).
- **DEFERRED, known-L (violation #2):** opening the `Item` enum's save-keyed
  SET (string/dynamic item ids across persistence/menu/pickup/equip). Per
  design-balance, land it when the R6 demo game demands its own items — it
  will — not speculatively; note it in the demo's adversarial log day one.

Exit: `rg 'gnu_ton|pca|mockingbird|shark|duel_arena|noether|pirate'` in
engine crates hits test fixtures only, then zero.

### R4 — the carve (≈4–6 sessions, autonomous, dependency order)
Each slice = move a family to its leaf home, redirect every consumer, delete
the facade (the proven D2 template); gate = `cargo test --workspace` + the
boundary suite; **record compile-time before/after per slice** (the carve
exists to buy rebuild speed — measure the purchase).
- **R4a** near-leaf harvest: `time/`→`ambition_time`; `quest/`+`host/`→ the
  new `ambition_persistence`; `inventory_ui/`→items; `asset_publish/`→
  asset_manager; `camera_snapshot`+`camera_ease` wait for sim_view.
- **R4b** `ambition_world` (rooms+LDtk+platforms+physics+gravity zones; the
  `RoomTransitioned` inversions; the 139-inbound repoint). Needs R3.1.
- **R4c** support ring: `ambition_persistence` (save+settings); the menu
  consolidation (gameplay_core IR + the app's 10k host stack →
  `ambition_menu`); audio/music → `ambition_audio`; `ambition_dev_tools`
  (core dev/ + app dev/); dialog runtime → `ambition_dialog`.
- **R4d** finish `ambition_combat` (cut the 23-ref features back-edge, move
  the 10k kit incl. the moveset runtime) + `ambition_projectiles` (the pair).
- **R4e** sprite metadata: `character_sprites` + boss sprites/attack_geometry
  → `ambition_sprite_sheet` (the ONE pipeline, M7).
- **R4f** `ambition_sim_view` + cut the render edge (D3.7 — the lever fires;
  render/portal_presentation leave the hot rebuild path).
- **R4g** rename the ~30–35k residue → `ambition_actors`; dissolve the
  `features/` hub facade (its 634 refs redirect family-by-family as homes
  land — this happens *throughout* R4, R4g is the final sweep).

### R5 — the engine face (≈1 session, autonomous)
`ambition_runtime::PlatformerEnginePlugins` (sim/presentation/headless
groups, subsystem-owned ordering); app assembly collapses onto it; boundary
tests extended to assert app thinness. The `App::new().add_plugins(...)`
moment (C4/M12).

### R6 — the first proof clone (≈2–3 sessions; Jon picks the target — Q12)
`demos/demo_smb` or `demo_moneyseize`: one content crate + ~100-line app
against `ambition_runtime`, built adversarially — every needed core edit files
an oracle-violation issue and gets fixed as engine work. Exit: the demo's
`git log --stat` touches zero engine crates.

## 5. JON'S OPEN DECISIONS (deliberately short — nothing here blocks R1–R5)

1. **Q12 (first demo game):** SMB1 or MoneySeize for R6? (Roadmap proposal:
   MoneySeize for feel-calibration, SMB1 for recognizability — pick one.)
2. **The `ambition_actors` rename** of the gameplay_core residue (R4g):
   endorse the name or supply a better one. Pure mechanical churn either way;
   scheduled last.
3. **Standing Q1–Q11** in roadmap.md remain open (engine naming/repo Q3,
   determinism-as-guarantee Q4, slopes Q6, streaming Q7 …) — none gate this
   roadmap; they gate P4/P5 scope.

## 6. HANDOFF — rules of engagement (unchanged, distilled)

- Work from THIS doc + `architecture.md` + `roadmap.md`. The 2026-07-02 review
  is the E-log record; append new E-entries THERE or start an E-log here —
  keep exactly ONE live log (recommend: new entries append HERE as R-entries,
  e.g. `R1.1-a`, so the 07-02 doc freezes).
- Commit each verified slice; stage explicit paths; feel-touching changes ship
  BLIND in marked commits; frame-agnostic always (new reaction seams get a C4
  scenario); ONE BODY ONE PATH; keep this doc's log current — Jon can only
  read, not ask.
- Verify: `cargo test --workspace --all-targets` is the only gate that sees
  all configs (E39/E40 lessons); the ten app integration suites + the four
  boss suites + `boss_motion_parity --features rl_sim` are the fast core;
  known RED: `unified_melee::a_hostile_actor…` (feel-reserved, documented).
- Estimates vs actuals: multi-session runs record wall-clock per phase and a
  final table (Jon's standing ask).

---

# EXECUTION LOG (live — start here, newest last)

*Executor: opus. Signed per repo convention.*

### R1.1 — the boss body integrates through the ONE shared `integrate_actor_body` ✅ (byte-identical)
The bespoke boss integration (`integrate_boss_bodies`'s inline `em.update` + a
render-sized `CenteredAabb` publish) is DELETED; the boss now flows through the
SAME `integrate_actor_body` every actor body does. The real duplication was the
integration ALGORITHM, not the query — so `integrate_boss_bodies` stays a thin
system in its chain-1 slot (a third disjoint-archetype arm beside the player's
`integrate_home_body` and the actor arm, all sharing the one integrator), which
preserves the boss's presentation ordering exactly.
- **The envelope split (AJ5.1) landed as data:** new body-generic
  `BodyEnvelope(Vec2)` component (`combat/components/actors.rs`) = the coarse
  render/composite footprint. `integrate_actor_body` gained an
  `envelope: Option<Vec2>` param and publishes `CenteredAabb` from
  `footprint = envelope ?? kin.size` — the ONE universal rule. A boss carries
  `BodyEnvelope(render_size)` (inserted at its sole production spawn,
  `spawn_actors.rs`); every ordinary actor passes `None` (its collision box IS
  its footprint) → byte-identical. `kin.size` is the collision box for every
  body; the boss's coarse-hurtbox envelope is no longer conflated with it.
- **Byte-identical, verified by construction + tests:** a floating boss produces
  no jump/dash/land move-events (no movement FX), never `shark_charge_crash`es
  (its caps lack `charge_crash_explodes`), and its stagger timers are always zero
  (the boss victim path arms none), so every extra thing `integrate_actor_body`
  does is a no-op for a boss; the `CenteredAabb` comes out identical because
  `collision_aabb(SimpleActorGeometry{size: render_size, frame_down: -surface_normal})`
  == the old `boss_frame.to_world_half(render_size*0.5)` (a non-surface-walker's
  `-surface_normal == gravity_dir`, kept live by §B2). The boss's `kin.size`
  self-heal onto `combat_size` is preserved (still in the boss arm before the
  shared call).
- **Files:** `combat/components/actors.rs` (+`BodyEnvelope`),
  `features/ecs/actors/update.rs` (`integrate_actor_body` param + `pub(crate)`),
  `features/ecs/bosses/tick.rs` (`integrate_boss_bodies` → shared call),
  `features/ecs/spawn_actors.rs` (`BodyEnvelope` insert).
- **Verified:** gameplay_core --lib 1134; app suites (rl_sim) boss_lifecycle 8,
  boss_contact_iframes 4, boss_motion_parity 2, boss_possession_specials 1,
  duel_arena 4, enemy_attacks_player 1, player_robot_fights_player 1,
  possession_end_to_end 3, plugin_minimal_app 8 — all green.
- **REMAINING toward "no boss arm at all" (the last step of AJ5.1):** merge the
  boss query INTO `integrate_sim_bodies`' actor query (drop `Without<BossConfig>`
  there) and DELETE `integrate_boss_bodies`. That needs the chain-2 movement
  phase reordered AHEAD of the chain-1 boss presentation systems
  (`update_ecs_bosses`/`sync_boss_actor_components`/`sync_actor_poses_from_feature_aabbs`,
  which read this frame's moved position), so it's a schedule change kept
  separate from this integrator-sharing one. The `BodyEnvelope` column would then
  move onto the actor query (`Option<&BodyEnvelope>`).

### R1.4 — a possessed boss's geometry strike fires through the moveset with the possessor's EFFECTIVE faction ✅ (BLIND feel)
Retires the §A1-slice-1b suppression that kept a possessed boss's GEOMETRY strike
inert (parity with the deleted `sync_boss_strike_hitboxes`). A possessed boss now
strikes like any other move — possession grants the full kit (invariant I2).
- **The load-bearing fix is one line + its enforcement:** `advance_move_playback`
  stamped the strike `Hitbox.source` from the owner's RAW `ActorFaction` — an
  outlier violating `effective_faction`'s OWN documented contract ("every hitbox
  stamp resolves through it, so a possessed body attacks its former allies, not
  its possessor"). Now it stamps `effective_faction(*faction, brain)`: identity
  for every ordinary actor + the player's own body (no `Brain::Player` ⇒ authored
  faction), `Player` for a controlled body. So the un-suppressed geometry strike
  hits the boss's former allies, not the controlling player.
- **Suppression removed:** `trigger_boss_attack_moves` dropped its
  `!is_special() && brain.is_player()` skip (and the now-unused `Brain` query
  column). A possessed geometry strike starts its moveset move like any other.
- **Files:** `combat/moveset.rs` (effective-faction stamp + `Option<&Brain>`
  query column), `features/ecs/bosses/tick.rs` (drop the suppression),
  `tests/boss_possession_specials.rs` (flip the assertion: the geometry strike
  now FIRES and its hitbox carries `Player`; wait it out — a committed move — before
  the special press).
- **BLIND** on feel (a possessing player can now deal geometry-strike damage); the
  MECHANICS are pinned: the possession test asserts the strike's hitbox `source ==
  ActorFaction::Player`. Every non-possessed body is byte-identical (the identity
  case), confirmed by gameplay_core --lib 1134 + duel_arena 4 all green.
- **Verified:** gameplay_core --lib 1134; boss_possession_specials 1 (with the
  effective-faction assertion), boss_lifecycle 8, boss_contact_iframes 4,
  duel_arena 4 — green. `unified_melee`'s `a_hostile_actor…` stays the DOCUMENTED
  pre-existing red (non-possessed melee-cadence gap, untouched by this identity-
  preserving change).

### R1.3 — the SIM owns the boss animation frame; the render→sim write-back is retired ✅ (BLIND feel)
The E37 architectural smell — RENDER's `animate_bosses` ticked the `BossAnimator`
AND inserted `BossAnimationFrameSample` onto the sim entity (render writing sim
state, consumed by the boss strike geometry) — is gone. A new sim system
`drive_boss_animators` (gameplay_core) picks the anim from the projected
`BossAttackState`, runs the animator's `request_for_phase` + `tick`, and writes the
sample; the renderer now only READS the sim-driven frame (`current_flat_index()`)
to draw. The drawn pose and the strike geometry share ONE sim-owned frame.
- **Byte-identical where it's tested, blind where it isn't.** The `BossAnimator`
  is still render-inserted (it holds the loaded sheet asset), so headless has no
  animator ⇒ no sample ⇒ the geometry keeps its elapsed-time fallback — exactly as
  the headless boss suites already ran (they never had a render sample). So the
  suites are unaffected. In WINDOWED play the sample is now sim-written one phase
  earlier (WorldPrep vs presentation) — a one-frame geometry-timing shift, BLIND
  (Jon feel-checks); the frame algebra itself is the SAME `BossAnimator.tick`.
- **Files:** `boss_encounter/sprites/mod.rs` (pub `current_flat_index`),
  `features/ecs/bosses/tick.rs` (`drive_boss_animators`),
  `features/{ecs/,}mod.rs` (export + register `.after(project_boss_attack_state_from_move)`),
  `ambition_render/.../actors/boss.rs` (`animate_bosses` reads the frame, no tick,
  no write-back). Also fixed the stale R1.4 doc-comment on `trigger_boss_attack_moves`.
- **Verified:** gameplay_core --lib 1134; render --lib 24; app suites (rl_sim)
  boss_lifecycle 8, boss_contact_iframes 4, boss_motion_parity 2,
  boss_possession_specials 1, duel_arena 4 — all green.
- **REMAINING (the "no boss anim island" tail):** the `BossAnimator` frame STATE
  (`current`/`drive_phase`/`frame`/`elapsed`/`clip_held`) can split fully sim-side
  (a `BodyEnvelope`-style split: a sim `BossAnimFrame` component + the draw-only
  render half), dropping the sim's read of a render-inserted component. And boss
  anim ROWS becoming `CharacterAnim` rows (the actor animator) is the deeper
  `BossAnim`→`CharacterAnim` convergence — both follow-ups; this slice retired the
  load-bearing smell (the write-back).

### R1.2 — the boss perceives its foe through the world-out `WorldView` port ✅ (byte-identical)
The A7 boss remainder: the boss brain read its target STRAIGHT from the omniscient
`ActorTarget` (`select_actor_targets`' global nearest-foe), the last actor still
carved out of the perception seam every other body uses. Now `tick_boss_brains_system`
(the autonomous BossPattern arm) builds the boss's own headless `WorldView` via the
SAME `build_world_view` `tick_actor_brains` uses, and targets `nearest_hostile()` —
the boss OBSERVES its foe, it is no longer told where it is.
- **Arena-wide awareness, sourced from the arena.** A boss fight fills the room, so
  the boss's viewport half-extent is the **whole world size** (`world.0.size`): the
  viewport then spans 2× the arena centered on the boss and (inclusive `contains`)
  ALWAYS holds the entire room, wherever the boss floats. So `nearest_hostile` sees
  exactly the foe `select_actor_targets` would pick (both resolve hostility through
  the shared `FactionRelations`) — **byte-identical** target in any real fight, while
  the omniscient read is gone. No magic number: "arena-wide" is derived from the
  arena. This is why the boss needs no `PerceptionMemory` (it never loses sight of the
  foe) — that `Without<BossConfig>` in `ensure_perception_memory` is now documented
  POLICY, not a parallel-system carve-out.
- **Honest fallbacks.** With the perception collectors present (the real run + the
  full-plugin boss suites) the boss uses the view; when the arena holds no live foe it
  holds at self (as `select_actor_targets` points a foe-less actor at itself). The
  omniscient `target.pos` survives ONLY as the fallback for perception-less boss UNIT
  fixtures (no `PerceptionPeers` resource) — those stay byte-identical. The possession
  arm is untouched (a possessed boss steers from controller input, never targets).
  `BrainSnapshot.target_pos` is still WRITTEN (now from perception) — it can DIE once
  the boss brain consumes the `WorldView` directly (a later slice).
- **Files:** `features/ecs/bosses/tick.rs` (`tick_boss_brains_system`: +3 perception
  resources, +`ActorFaction`/`ActorAggression` query columns, the `WorldView` build,
  `front_wall_clearance` + `snapshot.target_pos` now read the perceived target),
  `features/ecs/perception.rs` (`ensure_perception_memory` doc: the boss exclusion is
  now arena-wide-awareness POLICY, not omniscience).
- **Verified:** gameplay_core --lib 1134; app suites (rl_sim) boss_lifecycle 8,
  boss_contact_iframes 4, boss_motion_parity 2, boss_possession_specials 1,
  duel_arena 4 — all green.
- **⚠ SUPERSEDED by R1.2b** (Jon's redirect): the arena-wide-`WorldView` boss build
  above STILL carried a fallback (`if PerceptionPeers present … else ActorTarget`) —
  and so did `tick_actor_brains`. Jon flagged the fallback as bloat: "enforce a
  perception system, and maybe the most basic type of perception is omniscience." R1.2b
  below dissolves it by making omniscience a first-class typed mode; the boss reverts to
  reading `ActorTarget` — but now as the blessed `Perception::Omniscient`, not a hidden
  fallback. The arena-wide viewport (and the boss's perception resources) are GONE.

### R1.2b — perception is a typed policy; omniscience is the BASIC mode (no more resource-presence fallback) ✅ (byte-identical)
Both `tick_actor_brains` and (R1.2's) `tick_boss_brains_system` chose the target with
the SAME smell: `if the PerceptionPeers resource happened to exist { sighted WorldView }
else { omniscient ActorTarget }` — an implicit resource-presence FALLBACK, two ways to
learn where your foe is bridged by an accident of init. Jon's reframe: **make perception
a deliberate, typed per-body policy, and let omniscience be its basic mode.**
- **The type.** New `enum Perception { Omniscient, Sighted { viewport_half } }`
  (`features/ecs/perception.rs`), `Default = Omniscient`. A body WITHOUT the component
  reads as `Omniscient` — the basic perception: it simply knows the nearest hostile
  anywhere (the global `ActorTarget`), no viewport / sight / forgetting. `Sighted` is the
  world-out `WorldView` port (bounded viewport + `PerceptionMemory` pursuit).
- **Who is what.** `ensure_perception` (was `ensure_perception_memory`) GRANTS ordinary
  non-boss actors `Sighted { DEFAULT_VIEWPORT_HALF }` + memory — they can be juked, lose
  sight, give up. Everything else defaults `Omniscient`: the **player** (steers from
  input, never perceive-targets), a **boss** (relentless — it knows where you are in its
  arena; the canonical basic-perception body), and any **fixture** that wires up no
  perception. So there is NO fallback: the target derivation branches on the typed policy.
- **Byte-identical today.** `viewport_half == DEFAULT_VIEWPORT_HALF` for every current
  body, and the branch maps exactly onto the old resource-presence split (production
  actors were peers-present→Sighted; fixtures peers-absent→Omniscient), so behavior is
  unchanged. The actor tick still builds its `WorldView` ALWAYS (the brain's line-of-fire
  needs it); only the TARGET source is policy-gated. The one seam: a production actor is
  `Omniscient` for the 1 frame before `ensure_perception` attaches `Sighted` (same
  accepted gap the memory `Option` already had; deterministic, washes out — duel_arena's
  1800-frame fights are green).
- **The boss got SIMPLER.** R1.2's arena-wide `WorldView` build + its 3 perception
  resources + 2 query columns are DELETED; the boss reads `target.pos` (Omniscient),
  now a blessed mode rather than a carve-out. No per-frame view build for the boss.
- **Files:** `features/ecs/perception.rs` (`Perception` enum; `ensure_perception`
  grants Sighted+memory), `features/ecs/actors/update.rs` (query `+Option<&Perception>`;
  target branches on the policy), `features/ecs/bosses/tick.rs` (reverts R1.2 to the
  Omniscient `target.pos`), `features/mod.rs` (registration rename).
- **Verified:** full `cargo test --workspace --all-targets` — 43 test binaries green;
  the ONLY failure is the documented pre-existing RED `unified_melee::a_hostile_actor`
  (confirmed identical on the clean baseline). gameplay_core --lib 1134; boss + duel
  suites (rl_sim) all green.

### R1.5 — every surviving `Without<BossConfig>` is documented POLICY ✅
The ratchet (`rg 'Without<.*BossConfig>' gameplay_core/src`, excl. comments/tests): ~12
real query filters. All are genuine boss POLICY, not "the boss has a parallel system":
- **Domain policy (self-evident):** pickups (a boss doesn't collect), target-volumes +
  view-index (boss geometry from sprite-metrics), damage-predicates + damage routing
  (environmental-kill-only etc.), reset (bosses REVIVE via encounter reset, not actor
  respawn), projectiles (bosses fire via the moveset). Each already reads as a
  domain difference.
- **The load-bearing trio (now annotated):** `tick_actor_brains` (:182),
  `integrate_sim_bodies` actor arm (:719), `sync_actor_read_model` (:827) — the boss runs
  its OWN chain-1 (`tick_boss_brains_system` / `integrate_boss_bodies` /
  `sync_boss_actor_components`). Each carve-out now carries a POLICY comment: the boss is
  a NON-SWARM actor (no slot-board / anti-clump), it integrates through the SHARED
  `integrate_actor_body` but from a chain-1 slot kept for byte-identical presentation
  ordering, and its read-model sync also carries boss-only encounter fields. Folding the
  brain-tick would ADD a swarm-skipping boss branch (adapter, not canonicalization);
  folding the integrate arm ("no boss arm") needs a chain reorder for a BLIND pose lag and
  the boss chain-1 presentation stays regardless. So they are documented POLICY.
- `ensure_perception`'s `Without<BossConfig>` is now the "boss = Omniscient basic mode"
  policy (R1.2b), superseding R1.2's arena-wide-awareness note.

**R1 COMPLETE.** The boss island's parallel forks are all dissolved (integrate, animator
write-back, possessed-strike, targeting), and every remaining `Without<BossConfig>` is
documented boss policy. Optional DEEP-convergence follow-ups (Jon's call, design-gated,
NOT blocking): the "no boss arm" integrate fold (blind); the `BossAttackIntent` →
general-move-intent generalization that would let the boss brain-tick truly fold into the
actor path; boss anim ROWS → `CharacterAnim`. Next roadmap phase: R2 (ability model).

### R1 HANDOFF — remaining slices (R1.2, R1.5), with the analysis done
Executor note (opus): R1.1 + R1.4 landed + verified + committed
(`a8b5f3fb`, `ec4168ae`). The three remaining slices are each a substantial
focused effort; a fresh context should take them one at a time. The
groundwork:

- **The exit criterion is measurable.** `rg 'Without<.*BossConfig>'
  crates/ambition_gameplay_core/src` (excl. tests) = **17 carve-outs / 11
  files** today. The three LOAD-BEARING ones are the actor-tick systems the
  boss is excluded from only because it has parallel systems:
  `features/ecs/actors/update.rs:177` (`tick_actor_brains`), `:701`
  (`integrate_sim_bodies` actor arm — now shares the integrator via R1.1 but
  still a separate query), `:809` (`sync_actor_read_model`). The rest are
  damage/victim/reset/perception/view carve-outs. R1.5's exit = after R1.2/R1.3
  land, every surviving `Without<BossConfig>` is genuine boss POLICY (a real
  behavioral difference), not "the boss has a parallel system." Re-run the grep
  as the ratchet.

- **R1.2 brain fold** (`tick_boss_brains_system` → `tick_actor_brains`): **the
  naive merge is the WRONG shape — do NOT do it.** `tick_actor_brains` is a
  swarm-specific system (per-player-target slot-board arbitration + anti-clump
  crowding); a boss doesn't participate in any of that, so folding it in would
  add a big boss branch that SKIPS the swarm machinery — an adapter that pollutes
  the actor system, not canonicalization. The boss brain LOGIC is already unified
  (E30: the universal `Brain::tick`); `tick_boss_brains_system` is legitimately
  different NON-SWARM orchestration (boss-only snapshot fields, `BossAttackIntent`
  output, possession→special mapping), the same way the player's `integrate_home_body`
  is a separate arm. So the `Without<BossConfig>` at `:177` is arguably DOCUMENTED
  POLICY (a boss is a non-swarm actor), satisfying R1.5's exit. The genuinely
  elegant fold would first GENERALIZE the boss-specific bits — `BossAttackIntent`
  → a general "move intent" the actor moveset trigger reads; the possession→special
  map → a general controller→move map — a bigger DESIGN slice, Jon's call, not a
  mechanical merge. **Recommendation: reclassify `:177` as policy (document it) and
  invest R1.2 in the boss WorldView-targeting migration instead** (the A7 boss
  remainder): the boss still reads the omniscient `ActorTarget`; route it through
  `WorldView.nearest_hostile` with a large authored `viewport_half` (a boss wants
  arena-wide awareness — DATA, per E56's viewport knob), removing the perception
  carve-out and letting `BrainSnapshot.target_pos` eventually die. Gate: the 4 boss
  suites + `duel_arena` (chase determinism is fragile per E39 — assert ranges).

- **R1.3 write-back — DONE** (see the R1.3 entry above; the render→sim write-back
  is retired, the sim owns the boss animation frame). The two remaining R1.3-adjacent
  follow-ups are LOWER value: (a) split the `BossAnimator` frame-state fully sim-side
  (drop the sim's read of a render-inserted component) and (b) boss anim ROWS →
  `CharacterAnim` rows (the actor animator) — the deeper `BossAnim`→`CharacterAnim`
  convergence. Neither is load-bearing; the smell (render writing sim state) is gone.

- **Finish R1.1's "no boss arm"** (optional, folds into R1.2): merge the boss
  query INTO `integrate_sim_bodies` (drop `Without<BossConfig>` at `:701`, move
  `BodyEnvelope` onto that query as `Option<&BodyEnvelope>`) and DELETE
  `integrate_boss_bodies`. Requires reordering the chain-2 movement phase AHEAD
  of the chain-1 boss presentation systems (`update_ecs_bosses` reads only
  health/timers — safe; `sync_actor_poses_from_feature_aabbs` reads CenteredAabb
  → a one-frame ActorPose lag, presentation-only/BLIND). Cleanest to land WITH
  R1.2 (both touch the boss chain-1 tuple + the schedule).

*(R1.1 + R1.3 + R1.4 done. Remaining: R1.2 — reclassify/WorldView (see above); R1.5 sweep.)*

---

## R2 — the ability model (executor: opus, 2026-07-04)

The R2 ability-model ENGINE is landed as data + primitives; the player-melee
fold (R2.5) is the remaining consumer.

### R2.1 — `EffectRef` schema: the ONE ability-vocabulary reference ✅ (byte-identical) — `68d1f328`
`ambition_entity_catalog` gains `ParamValue(ron::Value)` (opaque params, hydrate
via `Value::into_rust`; `Default` = empty `{}` table) + `EffectRef { key, params }`.
Schema changes (AJ1): `MoveEventKind::Effect(EffectRef)`, `MoveWindow.sustain_effect:
Option<EffectRef>`, NEW `HitVolume.on_hit: Option<EffectRef>`. RON authoring uses the
anonymous-struct form `Effect((key: "x"))`. Dispatch bridge still drops params
(no consumer until R2.2 threads `Special`). All construction migrated.

### R2.4 — directional verb selection ✅ (byte-identical) — `d238f4cc`
`AttackDir { Neutral, Up, Down, Back }` + `directional_verb_chain(base, dir,
grounded)` (`attack_air_down → attack_down → attack_air → attack`) +
`MoveGates::permits` + `MovesetContract::move_for_directional_verb` (pure,
7 unit tests). `trigger_moveset_moves` reduces `ControlFrame.attack_axis`
(body/gravity-local) → `AttackDir`, reads `BodyGroundState.on_ground`, resolves
the melee verb directionally. Byte-identical: every current body authors only
`"attack"`, which every direction resolves to.

### R2 on-hit primitive + engine pogo ✅ (byte-identical) — `eff54cd2`
New `combat/on_hit.rs`: `HitboxOnHit` sidecar + `dispatch_hitbox_on_hit`
(decoupled from the damage resolvers — re-tests overlap, reuses `damage_lands`;
covers every hitbox source uniformly) + `OnHitEffectMessage` + `apply_pogo_bounce`
+ `PogoTarget` capability. A down-air authoring `on_hit: Effect("pogo_bounce",
(rise:…))` rebounds the OWNER off a `PogoTarget` victim. First live exercise of
`ParamValue::hydrate` (empty params → default rise). 2 headless tests. No-op
until a move authors `on_hit`.

### R2 self-motion + acceptance ✅ (byte-identical) — `92cb3f64`, `4e784a43`
`MoveSpec.start_impulse: Option<(f32,f32)>` — the flat `AttackSpec.self_impulse`
as move DATA, applied in `trigger_moveset_moves` (now mut `BodyKinematics` +
`GravityCtx`), facing-mirrored + gravity-rotated. Closes the biggest fold
expressivity gap; general (any move can lunge). Plus the I7 acceptance canary:
one RON `EntityCatalogDoc` authors a fighter's whole kit (directional verbs +
`start_impulse` + `on_hit` pogo) and parses/validates/resolves — a fighter is
DATA, not code.

### R2.5 — the player-melee fold ✅ LANDED — `6806c16b` (the R2 capstone / I7)
The last combat fork is dissolved: the player's melee is the SAME moveset runtime
every actor uses, driven by the controlled CHARACTER's kit (Jon: "whatever
character is chosen behaves like the character; the controller — human, brain, or
RL — just attaches" — the relativity / non-player-centrism principle).
- **The derive** (`directional_attack_variants` in `build_actor_moveset`): a
  body's ONE authored swing → up-/down-tilt + four aerials + pogo down-air, by
  TRANSFORMING the base volume (reach rotates up/down, mirrors behind), scaled by
  the character's own reach. Every controlled body resolves these through the
  shared directional trigger; a neutral grounded attacker (every enemy) still
  resolves `"attack"` → byte-identical swing.
- **The player**: `PlayerSimulationBundle` gains `ActorMoveset` (from
  `action_set.melee` — a worn starting character's swing drives the derive,
  respecting `overlay_character_moveset` — the character-override decision
  RESOLVED) + `MovesetMelee`. `pogo_pressed` → the down-air.
  `project_moveset_melee_to_body_melee` recognizes the whole `attack*` melee
  family (`is_melee_swing_move`) so the `BodyMelee`/`BodyCombat.attacking`
  read-models keep working. FollowOwner strikes stamp facing-directed knockback.
- **Pogo unified** (the world-orb decision RESOLVED): entity targets via the
  on-hit dispatcher + `apply_pogo_bounce` (`632f96de`); world `PogoOrb` blocks via
  `pogo_moveset_off_world_orbs`. One pogo — `PogoTarget` entities + `PogoOrb`
  blocks. Shared `pogo_rise_from` (default 720 = flat `pogo_speed`).
- **BLIND feel deltas** (Jon's to tune): derived directional geometry vs the old
  per-intent table, knockback direction, no vertical-commit floors, no attack
  self-recoil. Mechanics headless-verified: full gate 70 binaries green, only the
  documented pre-existing RED `unified_melee::a_hostile_actor`.

**Historical note** — the fold's two design decisions (below) were adjudicated by
Jon mid-run and are now RESOLVED in the landing above:

**Open design decision (needs Jon):** world-orb pogo reconciliation. Breakable
pogo-orbs (`spawn_breakable`) carry `CenteredAabb` + `PogoTargetVolumes` but NO
`ActorFaction`, so the on-hit dispatcher (gates on `damage_lands` over factioned
victims) can't see them. Victim-pogo (off enemies via `PogoTarget`) and
world-orb-pogo (off environmental breakables) are genuinely different mechanics.
Options: (a) widen the dispatcher's eligibility to factionless capability
targets; (b) keep a small world-orb-pogo check that fires for a `MovesetMelee`
down-air (relocated from `attack.rs:450`). This is a modelling call, not a
mechanical one — hence deferred to a fresh session with Jon's steer.

**Reassuring (NOT blockers):** offense scaling survives — the settings
`player_damage_multiplier` scales at `resolve_body_hit` downstream of authored
damage, and `BodyOffense.damage_multiplier` has no in-game upgrade wired, so
authored `damage` matches the flat base. The affordance HUD stays (labels only).

**Prototyped-then-reverted (the authoring is trivial once the design lands):**
a `player_moveset()` builder from the parity table (jab / u-&d-tilt / 4 aerials
+ pogo down-air) resolved every direction correctly in a unit test, and stamping
`knock_x = facing * volume.knockback` for a FollowOwner strike gives a folded
player knockback. Both were REVERTED — they are speculative until the two design
decisions below land (the code is a ~30-min re-author, not the hard part).

**SECOND open conflict, found by wiring the live switch (needs design):** the
`from_scratch_as_character` path (the landed "player wears any catalog character"
feature) overrides the player's melee via `overlay_character_moveset` on the
`ActionSet`. Attaching a FIXED `player_moveset()` + `MovesetMelee` unconditionally
REGRESSES that feature (a starting character loses its custom melee) — a
functional regression, not feel. The fold must build the player's `ActorMoveset`
so it RESPECTS the character override (derive directional variants from the
character kit, or merge). So the live switch is deferred with TWO design
decisions — world-orb pogo eligibility + character-moveset reconciliation — plus
the BLIND feel deltas. The R2 ENGINE + the parity table are in hand; the fold is
a focused next session that starts from these two decisions. **Scope discovered** (from the
flat `attack_spec_from_view` parity table in `ambition_combat`): the moveset must
grow to match the flat directional swing, OR author approximate moves and let Jon
tune feel (pre-release license). Gaps between `HitVolume`/`MoveSpec` and the flat
`AttackSpec`:
1. **self_impulse** — per-intent body-local lunge at swing start. The moveset has
   no move-start self-motion. Add `MoveSpec.start_impulse: Option<(f32,f32)>`
   applied at trigger (mut `BodyKinematics` + gravity rotation), OR drop and tune.
2. **knockback VECTOR** — flat `AttackSpec.knockback` is a per-intent Vec2 (up-air
   knocks up, back-air knocks back). `HitVolume.knockback` is a SCALAR with
   resolver-derived direction → approximate for up/down. Widen or accept.
3. **vertical commit** — Up-attacks floor a min ascend; AirDown a min descend.
   No moveset equivalent; drop and tune, or a commit primitive.
4. **damage_kind** — flat carries `DamageKind::Slash`/`Pogo`; `HitVolume` has
   only `damage: i32`. Accept Slash default, or add.

**Tractable plan** (affordance HUD STAYS — it only labels input→`AttackVariant`;
`MovesetMelee` bypasses only the flat swing EXECUTION, not the HUD):
(a) author `player_moveset()` from the parity table (jab→`attack`, u/d-tilt→
`attack_up`/`attack_down`, airs→`attack_air`/`attack_air_up`/`attack_air_back`,
d-air→`attack_air_down` + `on_hit: pogo_bounce`); (b) attach `ActorMoveset` +
`MovesetMelee` to the player bundle; (c) reconcile pogo — enemies gain
`PogoTarget` (victim pogo); the WORLD-ORB pogo (`BlockKind::PogoOrb` via
`PogoTargetVolumes`) is a SEPARATE traversal mechanic tied to the flat down-air's
active window (`attack.rs:450`) — relocate it to fire on a `MovesetMelee` body's
down-air, or keep as a small general check; (d) headless-verify the player attack
lands + BLIND feel; (e) later: retire the player-only flat directional execution
(the shared `start_attack` stays for enemies).

### DEFERRED until the fold (no consumer yet — avoid speculative generality)
R2.2 (thread `EffectRef` params through the `Effect→Special` dispatch + install-time
param validation) — real once a move-start/technique authors params. R2.3 (prefab
registry — generalize `attack_move_from_melee`/`fire_move_from_ranged` into keyed
`simple_melee`/`simple_ranged`) — DRY once the player moves give concrete shape.
R2.6 (equipment→params merge) — once params thread through.

---

## R3 — content eviction + the world seam (executor: fable, 2026-07-04)

### R3.1 — the world seam's three pieces, all landed ✅
The multi-session crux (AJ2/JD4) is in: the loader has NO private knowledge of
entity vocabulary, world list, or start room — all three enter through install
seams a second game can use.

- **R3.1a converter registry** (`ee48719e`, byte-identical): `entity_to_runtime`'s
  closed match → a converter REGISTRY (`identifier → fn(&LdtkEntityCtx) →
  Result<RuntimeEntityEmission, String>`). The engine's 31-identifier standard
  vocabulary registers through the SAME table shape content uses;
  `install_ldtk_entity_converters` (OnceLock, first-install-wins — the
  install_enemy_roster contract) adds game converters at plugin-build time.
  Validation (`known_entity`) consults the registry, so a content-registered
  entity passes the validator + converts with zero loader edits. Emission struct
  + per-family ctors + field accessors are pub for converter authors.
  Portal-compiled-out stays a LOUD error (error converters under
  cfg(not(portal_ldtk))). Tests: standard table ↔ `AMBITION_LDTK_ENTITY_IDENTIFIERS`
  drift pin; an installed converter validates + converts end-to-end; unknown
  identifiers still fail.
- **R3.1b WorldManifest** (`4c3d5717`, byte-identical): the FIVE sites that each
  privately knew the world list (secondary_world_ids, the include_str! trio, the
  hand-built catalog rows, the duplicate include_bytes! embedded-registry inserts,
  the hardcoded `"central_hub_complex"` start room) now all derive from ONE
  installed `WorldManifest { entry_room, worlds }` via `install_world_manifest`.
  A `WorldSource` row = `{ id, asset_path, loose_path, embedded_text,
  embedded_bevy_path, required }` and drives: the catalog entry (missing policy
  from `required`), the serde loader's disk→embedded chain, the Bevy
  EmbeddedAssetRegistry insert (from the SAME text — the 4MB of duplicated
  include_bytes! world JSON is gone), hot-reload (primary row), the
  bevy_ecs_ldtk asset path, and `to_room_set`'s entry room. The BUILT-IN default
  manifest still names the sandbox worlds UNTIL R3.2 moves the payload — seam
  first, payload second.
- **R3.1c RoomLoaded** (`2d9ec893`): `rooms::RoomLoaded { room_id }`, written by
  `spawn_room_feature_entities` — the one choke point all four staging paths
  flow through (initial build, transition, reset, hot-reload restage). The JD4
  seam for imperative per-room content staging; duel-arena moves onto it in
  R3.3. Registered app-side beside RoomTransitionRequested; staging test asserts
  exactly one emission with the staged room id.
- **Verified per slice:** gameplay_core --lib 1143; app rl_sim checks; duel_arena 4
  + plugin_minimal_app 8 green after R3.1c.

### R3.2a — the DATA payload evictions: audio registries, yarn, character catalog ✅
Three of R3.2's asset-payload seams landed on the proven install pattern; the
engine now ships no tracks, no cues, no dialogue, and no characters.

- **Audio registries** (`9cb5e72b`): `music_registry.ron` + `sfx_registry.ron`
  → `ambition_content/assets/audio/`. Core seam:
  `install_music_registry`/`install_sfx_registry` + `authored_*_registry()`
  (empty registry in a content-less binary; cross-crate cfg(test) fixture =
  the game's real data). `for_desktop_dev_default` reads the seam; the dead
  `*_REGISTRY_ASSET` consts died; regen_music_registry.py writes the new path.
- **Yarn dialogue** (`6987ecaf`): the 7 `.yarn` → `ambition_content/assets/
  dialogue/`. `ambition_content::dialogue::yarn` owns YARN_SOURCES +
  `yarn_spinner_plugin()` — now IN-MEMORY `YarnFileSource` (asset-root
  coupling gone; the Android folder-scan caveat dissolves) — and
  `known_dialogue_ids()`. Core dialog keeps only the runtime. Both guards
  moved WITH the content, unweakened: `yarn_compile` (whole-project compile,
  ui-gated) + `dialogue_lint` (arity + markup) as content tests;
  dialogue_lint.py follows.
- **Character catalog + playable cast** (`05952f40`, violations #3 + #10):
  `character_catalog.ron` → content; core's `character_roster` becomes the
  non-Bevy install seam (`install_character_catalog` + `catalog()` parse
  cache) the LDtk NpcSpawn converter / spawn paths / sprite joins read.
  Install chokes: content plugin, init_sandbox_resources, the sim-resources
  plugin (immediately before `character_roster_plugin()`), headless + rl_sim
  entries. PLAYABLE_ROSTER/next_playable → `content::character_catalog` with
  their rot-pins. Python tooling paths swept (incl. one commit in the
  sprite2d_renderer submodule).
- **Gate:** full `cargo test --workspace --all-targets` — the ONLY failure was
  the app's stale "asset root must contain dialogue/" expectation (fixed with
  the move); gameplay_core --lib 1133, content 5 suites, app --lib 140,
  rl_sim plugin_minimal_app/duel_arena/boss_lifecycle green.
- **Residue noted:** `StartingCharacter::DEFAULT_ID = "player"` remains a
  content string in core machinery (the default-wearing seam) — R3.4 candidate,
  not blocking.

### R3.2b — the WORLDS leave the engine ✅ (`1717e90c`; one BLIND bit)
Violation #1's biggest chunk: the four `.ldtk` files move to
`ambition_content/assets/worlds/`, and `ambition_content::worlds` authors +
installs the `WorldManifest` at every sim-entry choke. Core's built-in
default manifest DIES — an uninstalled production world load is a loud
panic; core tests read the game's real worlds via the cross-crate fixture.
- The bevy_ecs_ldtk spine sheds its last named-world knowledge: the three
  per-world asset resources + root markers collapse into
  `LdtkWorldAssets` (one handle per manifest row) + ONE `LdtkWorldRoot`;
  the bundle spawn is a manifest loop. **BLIND:** the hall world gets a
  painted-tile bundle for the first time (it was silently unbundled) —
  hall tiles now render like every other world's.
- New app-registered `game://` asset source (content assets in dev, shipped
  assets/ otherwise); `world_bevy_asset_path` prefers `embedded://` when
  the build embeds (web/Android unchanged). The duplicated embedded-URL
  consts + `SANDBOX_LDTK_ASSET` die — catalog, embedded registry, hot
  reload, and the spine all derive from the SAME rows.
- Path sweep: ldtk_tools defaults, migration scripts, run_headless.sh,
  deploy_to_steamdeck.sh (now rsyncs content assets too), live docs.
- **Gate:** full workspace --all-targets green except the documented
  unified_melee feel-red; static_map configs check; the REAL headless
  binary boots the moved worlds end-to-end.
- `rg 'ambition/worlds' crates/ambition_gameplay_core/src` → zero. The
  engine ships no worlds.

### R3.3 — room mechanics split by kind ✅ (`1b9b34b4`, `a0c1118a`)
- **Duel-arena → `RoomLoaded` consumer:** `features/arena.rs` (duel ids,
  fighter requests, the room constant) moves to
  `ambition_content::duel_arena`. Its room stager is the FIRST real consumer
  of the R3.1c fact — both entry points (room load + the `<<duel>>` yarn
  command, now on the `YarnContentBindings` installer seam instead of the
  engine's built-in command table) write plain `SpawnActorRequest` messages
  the engine's applier already resolves (grudge cross-wiring included).
  `spawn_room_feature_entities` stages NO named content anymore. The
  adjudicated one-frame message delay is not load-bearing: duel_arena 4/4
  green including the reset re-stage.
- **falling_sand → self-gating content plugin (AJ6):** the 1.3k-line
  prototype room + its switch ids + the optional `bevy_falling_sand` dep
  move to `ambition_content::falling_sand`; the plugin self-gates on its
  authored room at runtime. Core's time-domain audit drops its allow-list
  row (the wall-clock-by-design policy comment rides with the file — the
  review's "Res<Time> smell" turned out to be DOCUMENTED cadence policy,
  not a bug; left as-is).
- **Hall:** already pure authored data by construction after R3.2 (hall.ldtk
  + hall.yarn + the catalog all live in content; core keeps only the
  `hall_dialogue_id` schema field — machinery over installed data).

### R3 SESSION LOG — 2026-07-04 evening run (executor: fable)
Wall-clock (from commit stamps, single autonomous session):

| Phase | Landed | Est (doc) | Actual |
|---|---|---|---|
| R3.1a converter registry | `ee48719e` | — | ~30 min |
| R3.1b WorldManifest | `4c3d5717` | — | ~7 min |
| R3.1c RoomLoaded | `2d9ec893` | — | ~6 min |
| R3.2a-i audio registries | `9cb5e72b` | — | ~10 min |
| R3.2a-ii yarn set | `6987ecaf` | — | ~9 min |
| R3.2a-iii catalog+cast | `05952f40` | — | ~16 min |
| R3.2b worlds | `1717e90c` | — | ~23 min |
| R3.3 duel + falling_sand | `1b9b34b4`,`a0c1118a` | — | ~10 min |
| **R3.1–R3.3 total** | 14 commits | ~4–5 sessions | **~82 min** |

The doc estimated R3.1 alone as "the multi-session crux" and R3.2 as 3–4
sessions; the whole arc through R3.3 landed in one evening because the
R1/R2 unification left clean seams and the install-registry pattern was
already proven five times over.

**REMAINING in R3 (next session picks up here):**
- **R3.4 named-residue sweep** — untouched: `features/bosses.rs` id consts,
  npcs.rs bark tables (#4, ~450 lines), boss sheet defaults + enumerated
  arrays (#5), `sync.rs` id→sheet arms (#8), the 9 named boss constructors
  (#9), `ParallaxTheme` (#6), projectile visual kinds, render's
  `pirate_weapon.rs` (#7). Plus new small residue found this run:
  `StartingCharacter::DEFAULT_ID = "player"` in core machinery.
- **R3.5 mount field (AJ3)** — LDtk `mount:` authoring + the 5-step E64 plan.
- **R3.6 profile-key collapse (AJ4)** — ~72 refs / 8 files.
- The R3 exit grep (`rg 'gnu_ton|pca|mockingbird|shark|duel_arena|noether|pirate'`
  in engine crates) is NOT yet clean — that's R3.4's ratchet.
- **Deferred-by-design:** the build.rs sprite-RON bake + backgrounds/boss
  art + the OGG tree (the asset-ROOT flip) — they ride the R4e sprite-sheet
  seam; the Bevy asset root still points at core's assets/ until then.

---

## R3.4 — named-residue sweep (executor: opus, 2026-07-04)

**Prelude fix (`b28406e9`):** the R3.2b landing left `ambition_content/src/worlds.rs`
UNSTAGED — `lib.rs` had `pub mod worlds;` + `plugin.rs`/`content_validation.rs`
called `worlds::install()`, so `main` did not compile on a fresh clone (built
locally only because the untracked file sat in the tree). Staged it; also
removed a stale duplicate `gameplay_core/assets/data/character_catalog.ron`.

Landed the NON-blocked residue (each gated + committed):
- **#4 bark tables (`248eb9cc`)** — the per-character hit/hostile/idle bark
  pools (~200 lines of `key`/`name` substring tables) DELETED from
  `features/npcs.rs`; the catalog `barks` field (populated by R3.2a-iii) is the
  sole voice source, with a single engine-GENERIC default for anonymous actors.
  The placed parrot carries `character_id: "stochastic_parrot"` so its idle
  voice resolves from the catalog. −260 net lines. gameplay_core --lib 1133.
- **#9 boss constructors (`2edaef0f`)** — the nine `pub fn <boss>()` on
  `BossBehaviorProfile` were named content in the production API. Six had zero
  callers (deleted); the two production fallbacks (`generic`/`for_authored_boss`)
  repoint to `from_data("clockwork_warden")`; the three test-used ones move to a
  `#[cfg(test)]` impl. The engine's production API now names no boss.
- **bosses.rs slugs / self-dodge (`8e8f0d3d`)** — `spawn_actors` string-matched
  `GNU_TON_ENCOUNTER_ID` to grant GNU-ton an apple-rain self-dodge; now a
  generic `BossBehaviorProfile.self_dodge: Option<(amp,freq)>` DATA field
  (authored `Some((70.0, 1.6))` for gnu_ton; brain fields renamed
  `apple_rain_dodge_*`→`self_dodge_*`). `GNU_TON_ENCOUNTER_ID` +
  already-dead `GRADIENT_SENTINEL_ENCOUNTER_ID` deleted. Byte-identical.
- **MOCKINGBIRD_ENCOUNTER_ID (`051a32a3`)** — its "generalization plan" pointed
  at `sync_mockingbird_treasure_chest`, which no longer exists (the chest folded
  onto the generic `BossRewardProfile::DropChest`). Deleted from production
  `ids.rs`; the literal is now a test-local fixture. `ids` ships only the
  content-free slugging helper.

Gate after the four content slices: full `cargo test --workspace --all-targets
--features rl_sim --no-fail-fast` — only failure is the documented pre-existing
RED `unified_melee::a_hostile_actor` (unified_melee.rs:117, unchanged).

**REMAINING R3.4 — the residue that's BLOCKED, with the blocker named:**
- **Asset-ROOT-blocked (ride R4e, NOT independently landable):** `ParallaxTheme`
  (#6) — a closed biome enum; string-keying it needs the theme SET + alias table
  to become an installable registry AND the parallax background PNGs to move to
  content (manifest generation iterates `ParallaxTheme::ALL`). A half-version
  that keeps 9 built-in themes in-engine is NOT a clean eviction. `pirate_weapon.rs`
  (#7) + projectile visual kinds (apple/glider/lasersword) similarly reference
  core-side sheet PNGs. **Fold these INTO R4e** (the sprite-sheet + asset-root
  flip), not R3.4.
- **LDtk-authoring-blocked (R3.3-residual / R3.5-adjacent):**
  `HALL_OF_CHARACTERS_AREA = "hall_of_characters"` (update.rs:1281) switches NPC
  barks to the `Hall` pool by matching the room id — the clean fix is a room
  metadata `gallery: bool` flag authored in hall.ldtk (LDtk field + loader
  wiring). Do it WITH the hall→authored-data item (AJ2) via ambition_ldtk_tools.
- **Boss sheet statics (#5) + sync id→sheet arms (#8):** the 6 `BossSheetSpec`
  LazyLock statics mirror `boss_sheets.ron` (pinned byte-identical by
  `boss_sheets_ron_matches_builtin_defaults`); `sync.rs::sprite_target_for_boss`
  + `sprite_render_size_for` are id→target→static matches. Evicting the statics
  = making the installed RON the sole source + a `sprite_target` DATA field
  (same shape as the self-dodge fix). This is entangled with R3.6 (profile
  keys) and R4e (sprite metadata carve) — **best landed as the front half of
  R3.6**, so a new boss is 100% RON.
- **Borderline (documented, low-priority):** `StartingCharacter::DEFAULT_ID =
  "player"` + `PLAYER_CHARACTER_ID`/`PLAYER_FILE_ROOT` — a default-character SEAM
  (engine machinery) hardcoding a content id. Cleaner: content injects the
  default (it knows `PLAYABLE_ROSTER[0]`). Touches several app setup files for
  marginal gain; deferred.

**Exit-grep status:** the named-content grep in engine crates now hits
(a) the asset-blocked references above, (b) test fixtures (allowed), and
(c) the boss-sheet statics — no longer the bark tables, boss constructors, or
the encounter-id consts. R3.4's independent surface is landed; its remainder is
correctly folded into R3.6 / R4e / the hall-data item.

---

## R3.6 — `BossAttackProfile` collapses to a keyed carrier (executor: opus, 2026-07-04)

**AJ4 landed (the profile-half; the sheet-half #5/#8 remains — see below).** The
enum's 11 hardcoded geometry variants (`FloorSlam`, `SideSweep`, …,
`HazardColumn`) + the open `Special(String)` carrier collapse into a **2-variant
keyed enum**: `Strike(String) | Special(String)`. A `Strike`'s key selects its
body-local hitbox rects from the strike-geometry table (built-in default OR the
boss's RON `strike_geometry` override); a `Special`'s key names a content
Technique. So a new geometry strike is a new key + authored rects, and a new
special is a new key + a content system — NEITHER edits this enum.

**Why 2 variants, not a bare `BossAttackProfile(String)` newtype:** the crux the
mapping surfaced — the geometry-vs-special distinction is read by the brain
(`boss_pattern/tick.rs`: `is_special()` routes `special_pressed` vs
`melee_pressed`), which lives in `ambition_characters`, BELOW the geometry table
in `ambition_gameplay_core`. A pure newtype would force "special ⇔ no geometry
entry" — a table lookup the brain can't do without a layering violation. Keeping
`Strike`/`Special` as variants makes the distinction structural + brain-visible
(no table needed), while still collapsing the 11 named geometry variants into
ONE keyed carrier. This is AJ4's "Special stops being special: every profile is
a key," honestly bounded by the layering.

- **The three exhaustive variant matches became key-driven** (match on the
  `move_id` string): `strike_geometry` (the E58 rect table),
  `boss_anim_for_attack_profile` + `boss_animation_key_for_sample`
  (anim_helpers), `boss_animation_keys_for_profile` (behavior). The built-in
  strike vocabulary now lives in ONE place — `BossAttackProfile::BUILTIN_STRIKE_KEYS`
  (ambition_characters) — which `from_move_id` uses to reconstruct a profile
  from a content-free move id (a key in the set ⇒ `Strike`, else `Special`). The
  one render match (`overlays.rs` HazardColumn) resolves via `move_id`.
- **`boss_profiles.ron` reauthored** (scripted, comments preserved): 102 tokens
  `FloorSlam` → `Strike("floor_slam")`, etc.; `Special("…")` unchanged; the
  syntax-doc header rewritten to the keyed form.
- **BYTE-IDENTICAL, pinned:** `strike_geometry_is_byte_identical_to_the_old_hardcoded_match`
  (its golden `reference()` rewritten key-driven, same rects) +
  `move_id_round_trips` + the 4 boss suites + `boss_motion_parity`. Verified:
  characters --lib 253, gameplay_core --lib 1133, content --lib 61 (RON parse),
  boss_lifecycle 4 / boss_contact_iframes 8 / boss_motion_parity 2 /
  boss_possession_specials 1 / duel_arena 4 (rl_sim) — all green [+ full gate].
- **Layering note (documented edge):** a wholly-new CONTENT geometry key (not in
  `BUILTIN_STRIKE_KEYS`, authored only via RON `strike_geometry`) projects as
  `Special` in the READ-MODEL only; its damage still flows through the authored
  `Strike` move, so the edge is cosmetic (the projected telegraph label). Refine
  the projection to consult the RON override map if a game needs custom geometry
  keys visible in the telegraph — not needed by any current boss.

**Sheet-half progress:**
- **#8 sprite_target — LANDED (`ff1cb5f5`).** `sync.rs::sprite_target_for_boss`'s
  hardcoded boss-id→target match (gnu_ton→gnu_ton_boss, mockingbird→…, warden→
  "boss") became a `BossBehaviorProfile.sprite_target: Option<String>` DATA field
  (authored for the 3 divergent bosses; `None` = id-is-target). `generic()`
  resets it to identity. Byte-identical (gameplay_core 1133 incl. the mockingbird
  hurtbox test, content 61, boss suites). sync.rs names no boss.
- **#5 BossSheetSpec statics → R4e.** The 6 `BossSheetSpec` LazyLock statics that
  `sprite_render_size_for` reads (+ `builtin_boss_sheets`/`dedicated_boss_sheets`/
  `all_boss_sprite_filenames` enumerations, pinned byte-identical to
  `boss_sheets.ron` by `boss_sheets_ron_matches_builtin_defaults`) are sprite-sheet
  METADATA. Fully evicting them (engine ships zero sheets) changes
  headless-without-content behavior (static size → boss_size fallback) AND the
  render-side sheet loading + enumerations read them too — this is the R4e
  sprite-sheet carve (character_sprites + boss sprites/attack_geometry →
  `ambition_sprite_sheet`, M7), NOT a clean pre-R4e deletion. **Folded into R4e.**

After the profile-key collapse + #8, a boss's PROFILE + strike RECTS + sprite
TARGET are 100% RON; only the sheet-LAYOUT statics remain, and they ride R4e with
the rest of the sprite-metadata pipeline.

---

## R3.5 mount — CORRECTED execution map (opus, 2026-07-04; AJ3 under-specified)

Investigated the full surface (mechanics are already generic:
`Mountable`/`RidingOn`/`spawn_composite_mount_rider` — note `mount/mod.rs`'s doc
names a stale `spawn_mount_rider_pair`; the real fn is `spawn_composite_mount_rider`
at `spawn_mounts.rs:37`). The ldtk-tools capability EXISTS (`def update-entity
EnemySpawn --add-field mount:String:` + `entity set-field`, run per .ldtk on
BOTH `sandbox.ldtk` and `intro.ldtk`). The engine-side named residue
(`pirate_on_shark`/`pirate_heavy_on_shark`) is ALL `#[cfg(test)]`; production
resolves composition dynamically via `spec.is_composite()` (`enemies/mod.rs:658`,
keyed on `composite_visual.is_some()`, decision at `spawn_actors.rs:927`).

**THE LOAD-BEARING NUANCE AJ3 MISSED (do NOT execute naively):** the fused
`pirate_on_shark` (`character_archetypes.ron:595`) / `pirate_heavy_on_shark`
(`:677`) rows are the SOLE home of the MOUNTED rider's combat identity —
`brain_template: Skirmisher`, `attack_range: 1100`, `ranged: Bolt(...)`,
`held_item: "gun_sword"/"gun_sword_heavy"`, `rider_max_health: Some(4)/Some(6)` —
AND a mount-body-HP override (`max_health: 6/7`; the standalone
`burning_flying_shark` is HP 6, so the HEAVY rides a 7-HP shark). The plain
`pirate_raider` (`:540`) and `pirate_heavy` (`:644`, `attacks_player: false`) are
MELEE-ONLY cove grunts spawned standalone elsewhere. So AJ3's "rider keeps its own
brain" = a naive `plain-rider + mount` compose **silently drops the gun-sword +
1100px orbit-and-fire ranged behavior + both HP pools** — a real enemy-behavior
regression that is NOT headless-verifiable (it's spawned-enemy combat feel).

**Byte-identical decomposition (the correct model):** create DEDICATED mounted-
rider archetypes (`pirate_shark_rider` / `pirate_heavy_shark_rider`) carrying the
fused rows' rider loadout (Skirmisher/Bolt/gun_sword/rider HP as their own
`max_health`); keep `burning_flying_shark` as the mount, and give the heavy its
7-HP shark via a variant archetype OR a mount-HP override on the compose (the
6-vs-7 mismatch is the fiddliest bit — decide if the tougher heavy-shark is
intentional or an accident). Then: `mount: "<mount archetype id>"` +
`brain: "<mounted-rider archetype>"` + `name: "<bare rider name>"` on the 7
spawns; the compose reads the two ids; delete `composite_rider_name` +
`rider_name_suffix`. Parity harness = the composite-spawn tests
(`ecs/spawn/tests.rs:245-390`, `enemies/mod.rs:945-953/1213`) which pin the
mounted rider's resolved spec — they catch a dropped loadout.

**Status: DEFERRED.** This is a content-MODELING slice with a genuine judgment
call (where the mounted loadout + the two HP pools live) and enemy-behavior
regression risk, not the clean byte-identical field-plumbing AJ3 implied. Best
done with Jon's steer on the mounted-loadout model (it also touches feel — the
gun-sword shark-rider is a signature cove enemy). The corrected map above makes
it a ~1-session slice once the model is picked.
