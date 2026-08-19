# Immutable content / transactional construction — remaining work

> **Verified against `fda5db88` (2026-08-19).** Prepared content, structured
> diagnostics, explicit provenance, the construction registry/plan, migrated room
> construction families, removal of the legacy construction exemption,
> rollback-envelope coverage, and the first external consumer slice are
> implemented. The 2,400-line campaign record is archived at
> [`../../archive/planning-superseded/2026-08-13/engine/immutable-content-and-transactional-construction.md`](../../archive/planning-superseded/2026-08-13/engine/immutable-content-and-transactional-construction.md).

## Federation by domain — two lanes as of 2026-08-19

Construction is no longer one enum. A domain that owns a runtime capability owns
the construction vocabulary for it, and the room composes independently typed
**lanes** into one transaction.

| lane | owner crate | optional? | services |
| --- | --- | --- | --- |
| actor | `ambition_platformer2d_actor_monolith::construction` | no | actor context |
| `portal-gun` | `ambition_portal2d::gun_construction` | yes (`portal` feature + `PortalGunPlugin`) | `()` |
| `gravity-zone` | `ambition_platformer2d_shared_tangle::gravity::construction` | no | `()` |

The invariant is unchanged: **plan → preflight → commit → verify → publish**, one
baseline captured for the whole room, every lane verified against it, publication
only after all of them verify. Verification is detection/publication atomicity,
not world-mutation rollback: a failed verification leaves mutations in the world
and withholds publication.

**What each lane owes.** A typed parameter vocabulary that is NOT the room's spec
(the room adapter translates, so the domain never depends upward on
`ambition_platformer2d_world`); a closed exhaustive dispatch; a named
`ConstructionLane`; metadata-only registration into `ConstructionSchemaCatalog`
from its own plugin. **The catalog cannot select a constructor** and must not
become able to — no trait objects, no `Any`/`TypeId`, no string-dispatched
callbacks.

**Why gravity was the second one.** It is deliberately boring: not an actor, no
relation vocabulary, no execution services, a constructor that only lowers a
resolved region and direction into `GravityZone` plus an optional
`OscillatingZone`. It also lands beside those components in the crate that
already owns the construction machinery and `SpawnSessionScopedExt`, so the move
added **zero dependency edges**. And unlike portal-gun it is not feature-gated,
which stops `#[cfg]` from looking like part of the pattern.

### The cost to watch

Adding the second lane touched **eleven separate blocks** of
`RoomFeatureConstructionPlan`: plan field, receipt field, prepare, roster claim,
struct literal, deterministic dump, binding assert, verify, rebuild-one, commit,
committed ids. That number is the honest price of a third lane.

One of the eleven improved on the way through: the cross-lane collision check was
a hand-written pairwise intersection — fine at two lanes, quadratic in lanes — and
is now a fold (`claim_lane_ids`) that claims each lane's ids into the room roster,
so composing a lane and checking it are the same line.

⛔ The pressure does **not** justify type erasure. A universal registry that can
execute a new domain is exactly what this seam exists to avoid; if the eleven
blocks become intolerable, the answer is a better typed composition, not a
`TypeId` map.

### Capability installation vs schema fingerprinting

Investigated 2026-08-19. The portal-gun lane is compiled into room planning by
`#[cfg(feature = "portal")]`; the schema entry that prepared-content
fingerprinting reads is contributed by `PortalGunPlugin` at runtime. These are
two authorities. A composition that compiled `portal` and installed only
`PortalSimulationPlugin` — which that plugin's own doc invites — would fingerprint
a gun-less world while its rooms still built authored gun pickups.

What prevents it today is one line: `PortalSchedulePlugin` installs `PortalPlugin`
(simulation **plus** gun), and it is the only place in the workspace that installs
portal simulation at all. No engine composition can express "portal simulation, no
portal gun", so the divergent state is unreachable and no abstraction was added
for it. A test in `portal_schedule.rs` — compiled under the same feature as the
lane — holds that coincidence in place and goes red the day the line changes,
naming what else must move: a runtime capability token threaded into
`ActorConstructionContext::for_room_construction`, which is a seventh authority
there plus a parameter on the six systems that call it.

## Remaining construction work

- ▢ **Prove real snapshot reconstruction (the old operation 5).** There is still
  no production cross-room snapshot caller exercising source-snapshot selection,
  decode/compatibility rejection before mutation, rollback entity identity and
  remapping, restoration of non-room authoritative state, and atomic commit.
  Room-transition use of `RoomConstructionPlan::apply_to_world` does not by itself
  prove this operation.

- ✔ **Prove possession → transition → carried body end to end (2026-08-19).**
  `carried_item_crosses_rooms::an_item_carried_by_a_possessed_body_survives_the_door_too`
  possesses an actor, has THAT body take custody of the authored item, and walks
  it through a real `Door`. ⭐ the finding is a COUPLING nothing had written
  down: `project_custody_onto_residency` decides whether a held object travels by
  asking whether its HOLDER is room-scoped, and what makes that true of a
  possessed body is `possess_target` promoting it out of room scope — a write
  made to save the BODY that also decides the ITEM. Poison-verified in both
  directions; the possession code now carries the reverse pointer.

- ▢ **Prove corrected-input cancellation and peer-coordinated lifecycle commit
  when external/P2P rollback becomes real.** Local sync testing cannot
  mispredict, so this belongs to the real external-netplay trigger rather than a
  synthetic local ritual.

## Remaining external-consumer proof

`fixtures/external_consumer` already proves an independent workspace can prepare
content, run headlessly, route a shell, stage a character/enemy, and traverse an
in-room transition through the umbrella. The remaining useful proof is:

- ▢ run the visible external consumer on a machine with a display;
- ▢ measure first-room workflow and deliberate-error diagnostics rather than only
  describing them qualitatively;
- ▢ add a queryable readiness/last-failure convenience API if a consumer actually
  benefits from it;
- ▢ exercise construction/content authoring from a second meaningfully different
  consumer before freezing a broad public prefab/content API.

## Exit

Prepared content remains immutable after activation; construction is planned
before mutation and commits atomically; snapshot reconstruction has a real
behavioral proof; public construction APIs are justified by multiple consumers
rather than by the historical migration campaign.
