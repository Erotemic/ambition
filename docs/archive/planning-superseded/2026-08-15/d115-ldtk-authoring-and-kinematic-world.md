# D115 — Ambition-first LDtk authoring + moving-platform architecture (discharged 2026-08-15)

**Role:** EVIDENCE. ⛔ not authority. The live row is the compact rest row in
`docs/planning/queue-72h-2026-08-08.md`; current design lives in
[`../../../planning/engine/ldtk-authoring-and-world-tools.md`](../../../planning/engine/ldtk-authoring-and-world-tools.md)
and [`../../../planning/engine/kinematic-world-objects.md`](../../../planning/engine/kinematic-world-objects.md).

K2–K6 all closed. This file preserves the execution detail those slices produced.

---

- ⏸ **D115 — Ambition-first LDtk authoring + moving-platform architecture. RESTING: K2–K6 all closed; reopen only for a real kinematic customer.**

Design: [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md)
and [`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).
Prefer typed/native LDtk references and explicit validated motion semantics over
string linkage and optional-field precedence; preserve current ride/ledge/portal
behaviour while authoritative kinematic state moves to a coherent owner.

✔ **K2, K3 and K4 are CLOSED (2026-08-14).** Typed path references, the ownership
carve, and contact completeness are done; the discharged case files are archived
at [`../archive/planning-superseded/2026-08-14/d115-kinematic-world-k2-k3-k4.md`](../archive/planning-superseded/2026-08-14/d115-kinematic-world-k2-k3-k4.md).

⭐ **the three lessons that outlived those slices, because they will re-appear:**

1. **All three asked for something that already existed.** The typed resolver, the
   construction→presentation seam and the explicit dynamic-geometry query were all
   present; every slice became ADOPTION plus a deletion. ⇒ grep for the capability
   before designing a mechanism for one customer.
2. **The bugs were AGREEMENT bugs, not missing features.** Four sites decided which
   path spellings resolve, as three rules — so shipped content resolved in the
   sweep, resolved in the validator, and silently found nothing at runtime. And
   ledge carry was a FORK: only the home body could ride a moving platform's ledge,
   while `ledge_grab` is unmarked kernel state, so an enemy could latch, be left
   behind, and be dragged through a wall.
3. ⛔⛔ **AND K3's "no consumer composes a collision world by hand now" IS
   FALSIFIED (2026-08-14).** A reachability census found `CollisionWorld::carves_only()`
   has **ZERO callers**, `ambition_projectiles` carries a **parallel**
   `ProjectileCollisionWorld` that composes geometry itself, and **three production
   sites bypass `CollisionWorld` entirely** to call `world_with_sandbox_solids` /
   `world_with_portal_carves` directly — `features/ecs/actors/update.rs`,
   `avatar/body_integration.rs`, and `items/pickup/mod.rs`, **the last of which is
   literally what `carves_only()` exists for.** ⭐ this is the adopters lesson
   biting the row that taught it: a capability can be built, correct, and
   half-adopted, and *"no non-adopters"* is a claim that decays the moment someone
   writes new code. ⚠ also worth knowing: `CollisionWorld` has **no query methods
   at all** — it is a `SystemParam` with four accessors returning
   `Option<Cow<'_, ae::World>>`; the hypothetical-position queries
   (`body_overlaps_any`, `first_body_sweep`, `supporting_block`) live on
   `ae::World`.
4. ⛔ **crush CONSEQUENCE is a refused wire; crush DETECTION is not** — and the
   distinction was sharpened 2026-08-14, so do not read the first half as a ban on
   the second. Every resolve site is gated on Jon's no-artificial-pushout rule,
   with a test asserting non-ejection, so **the kernel must not eject a body or
   invent a displacement**. But it may and should REPORT that no legal position
   exists (D126.1). ⭐ **the reusable mechanics layer reports what physically
   happened; Ambition policy decides what it means** — damage, death, reset,
   forced displacement and immunity are all policy and none belongs in the kernel.

⛔ **and do NOT read K2 as closing the broader LDtk path question** (sharpened
2026-08-14). `Ref<KinematicPathId>` gives typed RUNTIME and VALIDATION resolution
**around a string** — which is what fixed the live defect — but it is not yet the
native LDtk `EntityRef`-style authored relationship the L2 plan envisions. ⭐
**runtime agreement is fixed; agent-native authored relationship QUALITY is still
open**, and that is K5's real content rather than cosmetics.

⭐⭐ **K5's MEASUREMENT REDREW THE MAP (2026-08-14), and the headline is that L2's
own named first proof has NO CONTENT.** Read against all six authored `.ldtk`
worlds:

- **`EntityRef` is native, shipping, and already END-TO-END** —
  `EnemySpawn.mounted_on` / `BossSpawn.mounted_on` are `__type: "EntityRef"` with
  **8 authored instances, all resolving**, plus Python `set-field` / `allowed_refs`
  / validation and Rust `field_entity_ref`. ⇒ **the native road is BUILT and has
  one customer.** Nobody needs to invent it.
- **the dominant authored relationship is `LoadingZone.target_room` /
  `target_zone` — 302 instances.** Then `Portal.link` 14, `BossSpawn.brain
  "PhaseScript:"` 11, `MaryOPipe.link` 4, `Switch.target_encounter` 2,
  `EnemySpawn.brain "Patrol:"` 2.
- ⛔ **`MovingPlatform → KinematicPath`, the "first proof" this plan names, has
  ZERO authored instances.** `path_id` is declared on four entity types and
  authored on **none**, in any world; `patrol_path_id` is declared by **no** entity
  definition at all. The whole corpus holds **two** `KinematicPath` entities, both
  referenced through `EnemySpawn.brain = "Patrol:<id>"` — a relationship hidden
  inside an unrelated string field. ⇒ **there was no string road to migrate; there
  was a string road to DELETE.**

⇒ this row once read *"so K5's real target is `LoadingZone` (302 instances), not
paths."* ⛔ **corrected by the maintainer's reviewer (2026-08-15): a big number is
not a proof target.** `LoadingZone`'s 302 conventional references are a separate
design problem — cross-world room targeting — and adopting native refs there
would be volume, not evidence.

✔ **K5's native-reference proof LANDED 2026-08-15, on the two live
`EnemySpawn brain="Patrol:"` references** — ⛔ **not** on `LoadingZone`'s 302,
which is a separate design problem and would have been volume rather than
evidence. `EnemySpawn` gained a native `path_ref` `EntityRef` resolved through
**the same call that mints the target's own id**, so no second spelling authority
exists. Measured: `Patrol:` 1→0 in both worlds and absent from every shipped
world.

⭐ **two refs beat 302 because they could be DELETED**: `parse_enemy_brain`'s
`Patrol:` branch · the `path_id` override (zero shipped instances authored it) ·
`validate_patrol_brain_paths` **and its two tests, 347 lines**, its subject having
moved into the type system · a spec file · `Patrol:` from `BUILT_IN_PREFIXES`.
Design detail: [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md).

⚠ **one inert residue, with its gate:** `EnemySpawn.path_id`'s LDtk `fieldDef`
still sits on 184 instances holding `null`. Removing a field def needs
`def upsert-entity`, which is manifest-driven, and only `mary_o` ships a
`*.entities.json`. ⇒ gate: *author an entity manifest for the content worlds, or
add a loss-checked `--remove-field`.*

✔ **and the deeper defect under K2 is now gone, by deletion rather than by
bridge.** K2 fixed the symptom with an alias; K5 found the CAUSE: **two id-minting
rules**. `kinematic_path_lookup_id` slugged a name with a private rule that
collapsed `_path_` away, while `name_slug` — which every resolver's alias set is
generated from — did not. Sandbox's path authors no `id` and is named
`enemy patrol path A`, so conversion minted `enemy_patrol_a` while the placement
said `enemy_patrol_path_a`. ⭐ **nothing ever referenced the compacted spelling; it
existed only to be bridged, and that bridge was implemented three times and wrong
twice while two validators stayed green and the patroller stood still for
months.** `compact_path_name` is DELETED, the legacy `patrol_path_id` authored
spelling is DELETED (zero defs, zero instances), and the shipped path now resolves
**by identity rather than by alias**.

⭐ **the diagnostic surface is `room relationships [--level] [--detail] [--out]`**,
and its design restraint is the point: it reports authored FIELDS and **never
re-derives a resolution verdict** — that would be a fourth authority for the exact
rule that drifted three times — so it names the owner instead. The one verdict it
does own is LDtk's own referential integrity: a dangling `EntityRef` is reported
broken, naming entity and level. ⚠ **its output asymmetry IS the argument for
native refs**: they are discovered from the schema with no list, while string
conventions need a hand-kept table that shrinks as relationships migrate.

⛔⛔ **AND DO NOT MIGRATE `LoadingZone` MERELY BECAUSE IT IS THE BIGGEST**
(maintainer, 2026-08-14). Its room+zone relationship is a **separate design
problem**; 302 references is a reason to design it carefully, not a reason to do
it first. ⭐ **the right native-reference proof is the two live `EnemySpawn`
`brain = "Patrol:<id>"` relationships** — they are the only authored instances of
that relationship in the whole corpus, and the reference is **hidden inside an
unrelated string field**, which is the entire argument for native refs stated in
one example.

⚠ **and the new `room relationships` diagnostic has a bug found the same day**: a
prefix convention reports `KinematicPathSpec::matches_id` as the resolver owner
even for `BossSpawn`'s `PhaseScript:` references, which that resolver has nothing
to do with. ⇒ **resolver ownership belongs IN THE CONVENTION DATA**, so adding a
convention cannot silently inherit somebody else's resolver — a report whose whole
value is naming the right owner is currently lying about it.

✔ **that gate is MET — all of it landed 2026-08-15** (`path_ref` on the
`EnemySpawn` def, both authored instances rewritten, `parse_enemy_brain`'s
`Patrol:` branch and the `path_id` override deleted). ⚠ one inert residue
remains, recorded above: `EnemySpawn.path_id`'s LDtk `fieldDef`, whose removal
needs an entity manifest for the content worlds.

✔ **K6 is CLOSED 2026-08-15, on the evidence rather than on an adoption: there IS
a second customer, it is the DOOR, it has been shipping for months, and it is not
kinematic.** A census of every production dynamic-world-geometry producer (in the
plan, with the table) says the shortage is of KIND, not of instances:
**`MovingPlatformState` is the only writer of a non-zero `Block::velocity`
anywhere in the repo**, and every other dynamic-geometry feature — encounter lock
walls, intro flag-gated lock walls, falling sand/liquid, breakables, world-pogo
targets, portal and climbable carves — toggles EXISTENCE at a fixed place, with
`velocity: ZERO` and no rollback state at all. ⛔ **so K6 must not be re-opened to
go find a customer; re-open it when content arrives.**

⭐ **each named candidate, one line.** *Moving door/wall* — REJECTED and it is the
one with real content (3 authored `LockWall` — one encounter-sealed, two
save-flag-sealed, by two independent contributors — and the LDtk entity is inert
until a game wires the condition, which is an AUTHORING gap and not a kinematic
one): it APPEARS, it does not SLIDE, so adopting the mover
would buy a motion driver for no motion, promote a per-frame *derived* resource
into serde snapshot state, and delete nothing. *Conveyor-like solid* — REJECTED
for want of content: the word occurs twice at HEAD and both are already
something else (a run of anchored `VerticalLoop` lifts; a `ForceZone` updraft that
accelerates bodies, not geometry). *Smash stage platform* — REJECTED as invented:
`smash_stage()` is one static block and the demo's own doc argues the stage is
four numbers. *Falling sand* — the real second customer the plan never named, and
it must STAY separate: it is a FIELD not a BODY, a per-tile lattice re-derived
each frame from a particle grid, so tiles have no cross-frame identity and
`last_delta`/`previous_aabb`/portal-host/ledge-carry are meaningless for it.

⛔⛔ **the falsifier K6 leaves behind, recorded so nobody adds a `bool` instead:
`Block::velocity` MEANS TWO THINGS AT ONCE and only a belt can tell them apart.**
For a platform the per-frame DISPLACEMENT and the DRAG imparted to a rider are the
same vector, so one field carries both — the sweep reads it as
`Contact::surface_velocity`, `ledge_carry_for_frame` selects a carrier by
`velocity != ZERO` **and recovers the previous pose as
`aabb.translated(-velocity)`**, and `MovingPlatformState::previous_aabb` is that
line's world-side sibling. A belt has displacement ZERO and drag non-zero, so
authored as `Block { velocity: drag }` today it would be picked as a ledge carrier
(dragging a body merely hanging off its lip) and handed a previous pose it never
occupied. ⇒ **split `velocity` into `displacement` and `surface_drag` before any
new `BlockKind` or authoring field** — same mixed-frame hazard as the one-way item
below, found from the other end.

⭐⭐ **and the census SPLIT the thing this row kept calling "the kinematic
representation" into two, only one of which lacks a second customer.** The motion
DRIVER (`KinematicPath` / `PathMotion`) already has three consumers — moving
platforms, damage volumes (`HazardFeature::new_with_paths` resolves a `path_id`
into a `PathMotion` and `update_ecs_hazards` moves and RESIZES the volume every
frame), and enemy patrol brains — ⛔ **and its only AUTHORED customer is a brain**:
2 `KinematicPath` entities in the whole corpus, both reached from
`EnemySpawn.path_ref`, with no `MovingPlatform` and no `DamageVolume` authoring a
path anywhere. The driver does not need a second customer; it needs CONTENT for
the two geometry consumers it already has. What has exactly one consumer is the
moving-SOLID contact contract — `Block::velocity` and everything derived from it
(rider carry, ledge carry, previous pose, portal host velocity). ⇒ **K6 was only
ever asking about the second one, and the answer is no.**

⚠ **also measured, because a pipeline diagram could be read to assume it:
`WorldDelta` DOES NOT EXIST in code** — only the reserved `GeoSource::Delta`
variant and aspirational doc text. Nothing anywhere mutates a `Block::aabb` or a
`SurfaceChain`'s points after room construction; the runtime substitute is the
immutable authored base plus per-frame recomposition. ⚠ and one solid's SURFACE
does move with `velocity: ZERO` — `SettledSandLedger::blocks()` sizes each dense
tile's block by fill, so a growing pile's top face rises every frame. Not a mixed
frame and not a bug (it is a differently sized block at the same tile, not one
block moving), but it is the sharpest form of the finding: **geometry can move
without being kinematic.**

⭐ **and what two materially different uses DO prove is the COMPOSITION seam, not
the mover:** `FeatureEcsWorldOverlay` + `CollisionWorld`'s four named views carry
a moving transform, three kinds of existence gate, a particle field and two
subtractions, with no producer editing the authored base. That is the abstraction
K6 validated; the kinematic representation keeps exactly one customer.

⇒ **what remains: K5** authoring polish (visible path and semantic diagnostics in
LDtk and the tooling).

⇒ **K4 INHERITED ONE MORE ITEM from D126 when that row closed (2026-08-14): a
moving platform cannot be authored ONE-WAY**, because `as_collision_block`
hardcodes `BlinkWall{Soft}` on blink grounds — a blink concern deciding a contact
policy. ⛔ **it is not a `bool` away.** `one_way_landing_from_previous_feet`
compares the body's PREVIOUS feet coordinate against the block's CURRENT
anti-gravity face: sound for static geometry, a **MIXED FRAME** for geometry that
moves, so a rising elevator would steal a landing off a stale feet line and a
descending one would refuse a legitimate landing. `MovingPlatformState` already
carries `previous_aabb()` for exactly this hazard, and **that question must be
answered before the field exists.** ⚠ cost if taken: a field on
`MovingPlatformSpec` (5-arg positional `new`, 4 call sites) and on
`MovingPlatformState` (5 constructors), which is serde-derived **rollback snapshot
state** ⇒ a schema re-baseline, plus a new LDtk `field_bool` and entity fieldDef;
`MovingPlatformState` is referenced from **8 crates**.

⚠ the resolver's block-order dependence that this row once pointed at is CLOSED
(D126.1) — feasible and infeasible contacts are now separated, and the fix that
looked obvious was rejected because it would have been deterministically wrong.

