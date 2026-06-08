# Stage 19: Portal mechanic crate + projectile ECS migration + generic transit

**Status:** PROPOSED — awaiting eyeball, then executed phase-by-phase (each
identical-sim gated). On `main`. The goal is an **elegant** reusable mechanic crate,
not a quick win.

## North star
The portal plugin should feel like a **small physics/mechanic plugin with adapters**,
not "Ambition's portal gun ripped out into another crate." It owns the *mechanic*;
Ambition owns the *content + glue* and opts its bodies in with policy markers.

### Ownership boundary (verbatim from the owner)
**Portal crate OWNS:** portals, portal-gun mechanics, placement, transit math, portal
carves, portal **body movement through portals**, and portal events.

**Portal crate does NOT own:** input, inventory, room-reset policy, collision-world
implementation, rendering/audio/VFX, fireball semantics, player abilities,
achievements, and how portal carves alter the collision representation.

## The over-assumptions to remove (today's leaks)
The current API still assumes too much:
1. `FirePortalGun` implies the **primary player** + a held `PortalGun`.
2. `portal_fire_system` takes origin from the **primary player's** `BodyKinematics`.
3. `portal_projectile_step` reads `crate::GameWorld` (concrete collision world).
4. `publish_portal_carves` writes **directly** into `FeatureEcsWorldOverlay`.
5. `portal_transit_system` is **primary-player-specific**.
6. `portal_transit_actors` names feature `BodyKinematics` + `BossConfig`.
7. `clear_portals_on_reset` reads `ResetRoomFeaturesEvent`.

The fix for each: portal core operates on a **generic `PortalBody`** + portal-owned
resources/events; Ambition adapters bridge the concrete (input→fire-intent,
PortalCarves→FeatureEcsWorldOverlay, room-reset→clear API, BodyKinematics↔PortalBody).

## One generic transit algorithm
Today there are THREE transit paths of differing correctness: `portal_transit_system`
(player), `portal_transit_actors` (feature `BodyKinematics`/`BossConfig`), and
`portal_teleport_ground_items` (the weaker `PortalTransitable` path). `PortalTransitable`
is a good adapter *seed* but less correct than the aperture/centroid machine the first
two use. **The crate gets ONE generic transit algorithm** (the aperture/centroid one),
operating on the body + a portal marker; Ambition opts players, enemies, bosses,
**and projectiles** into it. `portal_teleport_ground_items` and the three-path split
are deleted.

### Body seam (DECIDED): lower crate owns the body; portal consumes it via a marker
The **generic kinematic body component lives in a lower crate** —
`ambition_platformer_runtime` already owns the unified `BodyKinematics`
(pos / vel / half-extent / facing, re-exported from `engine_core`). The portal crate
does **not** define or copy a body; it defines a **marker** `PortalBody` (+ a
portal-owned `PortalPolicy` component for the bits transit needs that aren't on the
body — mass/size class, transit-cooldown behavior, whatever `BossConfig` supplied).
The transit system queries `(&mut BodyKinematics, With<PortalBody>, &PortalPolicy)` and
mutates the body **in place** — NO `BodyKinematics ↔ PortalBody` sync copy. Ambition
opts an entity in by *adding the marker* (players/enemies already carry the body;
projectiles get it in Phase 3). Dependency points strictly DOWN (portal →
`platformer_runtime`/`engine_core`); Ambition's whole role is "tag + set policy."
(Phase-3 detail: when projectiles become entities, their kinematic state should BE the
lower-crate body component so the same marker-based transit just works on them.)

**`PortalPolicy` describes HOW a body participates in transit — not WHAT it is**
(DECIDED). No `Player` / `Boss` / `Projectile` / `EnemyFaction` may leak into it; those
are object identities the crate must not know. Policy fields are behavioral/physical:
how the body fits the aperture, whether it re-orients, its transit-cooldown behavior,
etc. Ambition maps its game identities → policy when it tags the entity.

**Orientation (DECIDED):** **velocity rotation is part of transit for almost
everything** — going through a portal pair rotates the velocity vector by the pair
transform; that lives in the core transit and is on by default. The **optional** part
is **actor re-orientation** (rotating the body's facing / up-vector / sprite frame to
the exit aperture) — that's a `PortalPolicy` flag (a player/enemy re-orients; a
free-tumbling fireball or apple just has its velocity rotated and keeps flying).

## The projectile ECS-migration prerequisite
"Make fireballs transit portals" requires projectiles to be **entities** (you can't
add `PortalBody` to a `Vec` element). Today:
- player projectiles live in `PlayerProjectileState.bodies: Vec<InFlightProjectile>`
- enemy/boss projectiles live in `EnemyProjectileState.bodies: Vec<InFlightProjectile>`

This is a *good* migration candidate: the data is already centralized as
`ProjectileBody` + `owner_id`; the work is moving ownership from Vec pools into spawned
entities and splitting the monolithic update loops into ECS systems.

- **Controller state stays on owners:** player charge/cooldown/motion-buffer,
  boss/enemy attack cadence, ability cooldowns.
- **In-flight bodies become entities:** one per fireball / apple / boss bolt /
  reflected shot.
- **Spawn via messages, not Vec mutation:** `try_fire_projectile` writes a
  `SpawnProjectile` message instead of pushing into `state.bodies`.
- **Split `ProjectileBody`** into (a) a *generic kinematic body* (pos, vel,
  half_extent) and (b) *projectile gameplay state* (kind, faction, damage, lifetime,
  gravity, bounces); `resolve_world_collision` operates on those pieces directly.
- **Persistent visuals:** stop the despawn/respawn-every-frame-from-Vec; once
  projectiles are entities, the sprite rides the entity.
- Then: a projectile entity carrying `PortalBody` transits **for free** via the one
  generic algorithm.

---

## Phased execution (each phase = build-green commits, gated by the differential
## harness — `replay_fixture_regression` MUST stay zero-divergence; portal +
## projectile reachability green)

**Phase 1 — Generic transit core (the heart). ✅ DONE (2026-06-08).** Introduce the
portal-owned **marker** `PortalBody` + a `PortalPolicy` component, and ONE
aperture/centroid transit system that queries
`(&mut BodyKinematics, With<PortalBody>, &PortalPolicy)` and mutates the
lower-crate body **in place** (no sync copy). Migrate the player + actor paths
(`portal_transit_system`, `portal_transit_actors`) onto this single system; Ambition
adds the marker + policy to the player and actor entities (they already carry
`BodyKinematics`). Drop `BossConfig` from transit (fold its mass/size into
`PortalPolicy`). Identical-sim — same aperture/centroid result, one code path.

### Phase 1 — Progress / status (2026-06-08)
Landed, identical-sim. What shipped:

- **Seam (`portal/transit.rs`).** `PortalBody` — a unit marker component. `PortalPolicy
  { reorient: bool, carry_velocity: bool }` — behavioral only, never names
  Player/Boss/Projectile. `reorient` = flip `facing` on a same-wall turn-around;
  `carry_velocity` = write the rotated exit velocity (`false` = old boss
  no-velocity path). Velocity rotation itself stays core/default (it's in
  `transit_step`'s `vel` output).
- **ONE core system `portal_transit`.** Replaces BOTH `portal_transit_system` (player)
  and `portal_transit_actors` (actors) with a single query
  `(Entity, &mut BodyKinematics, &PortalPolicy, Option<&mut PortalTransit>,
  Option<&mut ActorRoll>, Option<&PortalTransitCooldown>), With<PortalBody>`. One
  `&mut BodyKinematics` → no self-conflict. Uses `platformer_runtime::body::BodyKinematics`
  (dropped the `crate::features::BodyKinematics`/`BossConfig` names from transit). Keeps
  ENTER/EXIT sfx in-system (sfx decoupling is Phase 2). Emits a NEW
  `PortalBodyTransited { body, enter_normal, exit_normal, facing_flip, exit_pos }`
  on Transfer; no longer emits `BodyTeleported` from core.
- **Ambition tagging adapter** (`ambition_content/portal/transit_body_adapter.rs::ensure_portal_bodies`):
  identity → policy, run `.before(portal_transit)` in `PortalSet::Transit`. player
  (`PlayerEntity`+`PrimaryPlayer`) → `{reorient:true, carry_velocity:true}`; boss
  (`BossConfig`) → `{reorient:false, carry_velocity:false}`; other actors → `{reorient:false,
  carry_velocity:true}`. Idempotent ensure-system (`Without<PortalBody>`), so the SET of
  transiting bodies is identical to before (player + all actors). `portal_teleport_ground_items`
  left untouched (Phase 4).
- **Ambition player-input adapter** (`…::portal_player_input_adapter`), `.after(portal_transit)`:
  reads `PortalBodyTransited` and FOR THE PLAYER reproduces today's bits — emits `BodyTeleported`
  (trace), inserts `PortalEmission`, and (iff `facing_flip` && held intent > eps) inserts
  `PortalInputWarp`. `PlayerMovementIntent`/`PortalEmission`/`PortalInputWarp` are INPUT and are
  no longer referenced by core. **No fallback needed** — Step D extraction caused zero replay
  divergence.
- **Verify:** `cargo build -p ambition_sandbox` clean; `--lib` 1428 passed; `architecture_boundaries`,
  `scripted_gameplay`, `replay_fixture_regression` (ZERO divergence), `portal_bridge_reachability`,
  `portal_lab_usable` all green.

**Phase 2 — Portal API decoupling (the 7 leaks). ✅ DONE (2026-06-08).**

### Phase 2 — Progress / status (2026-06-08)
Landed as five isolated, identical-sim commits (one per seam). Each: zero replay
divergence; `architecture_boundaries` + `scripted_gameplay` + `portal_bridge_reachability`
+ `portal_lab_usable` + `--lib` (1428) all green.

- **Seam 1 — carve output (CLEAN).** `publish_portal_carves` now writes a
  portal-owned `PortalCarves { holes: Vec<ae::Aabb> }` resource instead of
  `FeatureEcsWorldOverlay.portal_carves`. New Ambition bridge
  `ambition_content/portal/carve_adapter.rs::bridge_portal_carves` copies it into
  the overlay each frame in `PortalSet::Carves` after `publish_portal_carves`
  (before `CoreSimulation` consumes the overlay), publish order preserved. Portal
  core no longer names `FeatureEcsWorldOverlay`. Commit `eea53902`.
- **Seam 2 — world seam (CLEAN).** The `Res<GameWorld>`-reading shot stepper moved
  to `ambition_content/portal/shot_adapter.rs::portal_projectile_step`. Portal core
  keeps a pure `step_portal_shot(&PortalShot, &PortalShotWorld<impl SolidWorldQuery>, dt)
  -> PortalShotStep`. **`SolidWorldQuery` wiring:** `GameWorld(pub ae::World)` and
  `ae::World: SolidWorldQuery` (Stage 16 adapter) — the adapter passes `&world.0`,
  no new impl needed. **`is_portal_placeable(hit, normal) -> bool`** hook added in
  `portal/shot.rs`, defaults `true` (no-op; future LDtk no-portal flag is a data
  change). Adapter runs `.after(portal_fire_system)`, preserving toggle→fire→step.
  Commit `eeb89273`.
- **Seam 3 — fire intent (CLEAN).** Core `portal_fire_system` consumes a generic
  `PortalFireIntent { origin, dir, channel }`; new resolver
  `ambition_content/portal/fire_adapter.rs::resolve_portal_fire_intent` maps the
  `FirePortalGun` gesture → intent (origin from the primary player's body, dir from
  the aim, channel from the held gun, gun-active gating), in `PortalSet::InputAdapter`
  after the input adapter / before `WeaponAndProjectiles`. Core fire path no longer
  names player/gun/inventory. Commit `43d30de5`.
- **Seam 4 — reset (CLEAN).** Core `clear_portals_on_reset` consumes a portal-owned
  `ClearPortals` message; new bridge
  `ambition_content/portal/reset_adapter.rs::bridge_room_reset_to_clear_portals`
  emits it from `ResetRoomFeaturesEvent` in `PortalSet::RoomReset` before
  `clear_portals_on_reset`. Core no longer names `ResetRoomFeaturesEvent`. Commit
  `f07f2e81`.
- **Seam 5 — body refs (CLEAN).** Repointed the last `crate::player::BodyKinematics`
  in core (`presentation.rs`) to `ambition_platformer_runtime::body::BodyKinematics`
  (`transit.rs` already did in Phase 1). Commit `411b5414`.

**Tightened guard:** new `architecture_boundaries_portal_core_does_not_name_host_world_or_reset`
asserts portal core (non-test, non-`presentation.rs`) names none of
`crate::features` / `crate::GameWorld` / `Res<GameWorld>` / `FeatureEcsWorldOverlay`
/ `ResetRoomFeaturesEvent` / `crate::input::ControlFrame`.

**Residue (deferred, noted per the don't-force rule):**
- `crate::player` still appears in core `gun.rs` (color-toggle + dev-toggle query
  the primary player's `PortalGun`) and `transit.rs` (`suppress_ledge_grab_during_transit`,
  `warp_portal_input` query the primary player). These are OUTSIDE Phase 2's five
  seams (toggle / ledge-grab suppression / input-warp), so the new guard does NOT
  assert `crate::player` freedom yet; decouple in a later phase.
- `ambition_sfx` stays in core (`portal_transit`, `portal_fire_system`,
  `portal_projectile_step` emit `SfxMessage`) — sfx decoupling is explicitly Phase 5.
- `presentation.rs` (render-gated) still reads `Res<GameWorld>` + `crate::player`
  markers — Phase 5 / render territory; excluded from the goal check.

#### Original Phase-2 leak list (for reference):
- **Carve output** → portal-owned `PortalCarves` resource + an Ambition bridge into
  `FeatureEcsWorldOverlay` (portal owns the carve geometry; Ambition owns how carves
  alter its collision representation).
- **World seam (DECIDED): use the existing `ambition_platformer_runtime::SolidWorldQuery`** —
  do NOT invent a new trait. The shot/raycast core becomes a **pure helper** over
  `SolidWorldQuery` (+ aperture math); Ambition owns the Bevy adapter system that reads
  `Res<GameWorld>` and calls the helper. So `portal_shot_step`/`portal_projectile_step`
  hold no `Res<GameWorld>` — the crate stays ECS-light pure logic, the sandbox wires it.
  - **Solid ≠ portal-placeable (DECIDED):** the world seam distinguishes "blocks
    movement/raycast" from "accepts a portal." A surface can stop a body/ray yet reject
    a portal. **Default: every solid surface accepts portals.** A future LDtk tile will
    mark some surfaces non-portal-accepting — its exact representation is **deferred
    until we have a concrete example** of a solid-but-no-portal surface. For now, model
    the seam as `is_portal_placeable(surface) -> bool` defaulting to `true` (a no-op
    hook), so adding the LDtk flag later is a data change, not an API change.
- **Fire** → a generic fire-intent (origin + dir + channel) the Ambition input/inventory
  adapter emits, not `FirePortalGun`-implies-primary-player-held-gun.
- **Reset** → portal exposes a `clear portals` API; Ambition calls it on room reset
  (portal stops reading `ResetRoomFeaturesEvent`).
- **`BodyKinematics` refs** → `ambition_engine_core`/`ambition_platformer_runtime`
  (lower crate). Identical-sim throughout.

**Phase 3 — Projectile ECS migration (the big enabling refactor).** Split
`ProjectileBody` → kinematic body + projectile gameplay; in-flight bodies → entities;
`SpawnProjectile` message replaces Vec pushes; `try_fire_projectile` emits it;
`resolve_world_collision` operates on the split pieces; persistent entity visuals.
Controller state stays on owners. This is the largest, riskiest phase — gate hard on
`replay_fixture_regression` (bit-identical projectile behavior) + `scripted_gameplay`.
Likely several commits (split types → spawn-via-message → entity step systems →
visuals → delete Vec pools).

**Phase 4 — Projectiles transit portals (the demo).** Tag projectile entities with
`PortalBody` + the transit policy → they use the one generic algorithm. Delete
`portal_teleport_ground_items` (ground items become generic `PortalBody` opt-ins too).
Add a reachability test: a fireball fired into portal A emerges from portal B with the
mapped velocity.

**Phase 5 — Extract `crates/ambition_mechanics_portal`.** Now portal core is generic
(markers + seams + adapters) → move it (transit math, placement, lifecycle, carve,
pieces, gun mechanics, shot, messages, types, schedule). Presentation stays
render-gated in sandbox; the content adapters stay in `ambition_content::portal`.
Facade `crate::portal` re-exports → zero inbound churn. Boundary guards: the crate
depends only on `ambition_engine_core`/`ambition_platformer_runtime` + Bevy, never on
`ambition_sandbox`/content/features.

## Principles
- **Elegant, not quick.** Prefer the generic aperture algorithm over the weaker
  ground-item path even where the quick patch would do.
- **Differential harness is the safety net** ([[feedback_bias_toward_executing_big_refactors]]):
  each phase proves bit-identical sim before commit; portal is the flagship.
- **Adapters live in Ambition,** the mechanic lives in the crate. If a phase can't
  reach the clean seam, land the decoupling-in-place + note the residue (don't force).

## Open sequencing note
Phase 1+2 (portal generic + decoupled) and Phase 3 (projectile ECS) are largely
independent; Phase 4 needs both. Recommended order is 1 → 2 → 3 → 4 → 5 so the generic
transit exists before projectiles plug into it ("almost free"). Phase 3 can be pulled
earlier if preferred — it stands alone as a valuable migration.
