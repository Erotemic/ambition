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

**Phase 1 — Generic transit core (the heart).** Introduce the portal-owned **marker**
`PortalBody` + a `PortalPolicy` component, and ONE aperture/centroid transit system
that queries `(&mut BodyKinematics, With<PortalBody>, &PortalPolicy)` and mutates the
lower-crate body **in place** (no sync copy). Migrate the player + actor paths
(`portal_transit_system`, `portal_transit_actors`) onto this single system; Ambition
adds the marker + policy to the player and actor entities (they already carry
`BodyKinematics`). Drop `BossConfig` from transit (fold its mass/size into
`PortalPolicy`). Identical-sim — same aperture/centroid result, one code path.

**Phase 2 — Portal API decoupling (the 7 leaks).** Carve output → portal-owned
`PortalCarves` resource + an Ambition bridge into `FeatureEcsWorldOverlay`. World →
a generic collision-world seam (trait/query), not `crate::GameWorld`. Fire → a generic
fire-intent (origin+dir+channel) the Ambition input/inventory adapter emits, not
`FirePortalGun`-implies-primary-player-held-gun. Reset → portal exposes a `clear
portals` API; Ambition calls it on room reset (portal stops reading
`ResetRoomFeaturesEvent`). `BodyKinematics` refs → `ambition_engine_core` (lower
crate). Identical-sim.

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
