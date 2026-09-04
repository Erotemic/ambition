# Controlled-character actor kernel

**State:** NARROW / OPEN — the first decision-authority convergence is complete.
This is no longer the Engine 1.0 P0 gate.

## Goal

⚠ **This page defines what the residual kernel CONTAINS. It does not define when
the engine is modular.** Arriving at the small kernel described here satisfies
internal modularity and leaves the second criterion open: whether a major
capability is independently installable with explicit lower-level dependencies
and no unrelated siblings. See
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md),
"Two goals, and the second is not implied by the first".

A human-controlled body, AI-controlled body, possessed body and future remote
participant should use the same actor-local simulation path. Distinctions such
as home avatar, driving participant, input seat, camera/view subject and
presentation focus remain legitimate when they have different semantics; they
must not become alternate movement/combat authorities.

## Landed convergence

The major player-centric decision split that motivated this program has largely
landed:

- actor observation/decision phases are explicit enough to separate observation,
  targeting and pre-decision maintenance from later mutation;
- the old combat-slot arbitration anchored on `PrimaryPlayer` was deleted after
  proving no production reader consumed it;
- controlled and autonomous bodies consume increasingly shared actor/body
  decision contracts;
- several seat-zero/primary-player special roads have been removed or reduced to
  legitimate presentation/home-avatar fallback semantics;
- `CollisionWorld` is the shared collision observation authority instead of each
  large system independently recomposing room/platform/overlay geometry.

Do not reopen deleted player-vs-actor mirrors merely because old campaign history
mentions them.

## Residual kernel target

The residual actor kernel should own only behavior that genuinely needs tight
body-local integration, approximately:

```text
body state and actor-local lifecycle
    + accepted intent / control projection
    + movement/contact integration
    + core reaction/action application seams
    + narrow observation/decision interfaces
```

It should not be the composition root for persistence, rooms/session lifecycle,
boss/encounter orchestration, item/projectile domains, named content,
presentation/UI/audio, developer tools or host/platform policy.

## Current work

### K1 — finish the remaining body-integration fork only if it is still real

The previous campaign narrowed the structural fork to the remaining home-body /
generic-body integration difference. Re-measure HEAD before changing it. Merge
paths only when they express the same body semantic; do not erase legitimate
home-avatar presentation or shell policy for naming symmetry.

### K2 — per-driven-body item/projectile control — DONE (fold landed 2026-09-02)

Both leaks this section named are closed:

- held ranged shots no longer attribute through slot-zero. A held bolt carries
  `ProjectileOwner(firer)` — the rollback-registered, entity-remapped component
  the ECS projectile road already uses — so the hit is credited to whoever
  fired it instead of `Query<Entity, PrimaryPlayerOnly>`;
- nine held-item abilities (`ranged/{volley, meteor, beam, vortex}`,
  `thrown/puppy_slug_gun`, `traversal/{grapple, dive, blink, mark_recall}`) loop
  `DrivenBodies` instead of resolving one `ControlledSubject`.

⛔⛔ CONVERTING A SINGLE-SUBJECT SYSTEM INTO A LOOP IS NOT MECHANICAL. Two
classes of defect come with it, and both were live here:

- **a per-body exit written as `return` ends the SYSTEM.** Seat zero is idle on
  most ticks, so the first seat's early exit silences every seat after it —
  which is what `fire_held_ranged_system` was already fixed for once.
- **a per-tick BUDGET read from a query cannot see this tick's `Commands`
  spawns.** The puppy-slug summon cap was counted that way, so N seats firing on
  one tick each read the same pre-tick count and every one of them summoned. A
  budget shared across the loop has to be tallied inside it.

✔ **The fold landed 2026-09-02.** A press on a held gun-sword or fireball is
an `ActionRequest::Ranged` (`fire_held_ranged_system`, `items/pickup/mod.rs`)
consumed by the same spawner every brain's ranged action uses; the parallel
`HeldProjectile` simulation — its own world collision, range gate, splash and <!-- cite-ok: records the deleted parallel simulation -->
rollback row (`item.held_projectile`) — is deleted, and the fireball's burst is
`splash_half_extent` on `ProjectileGameplay` (schema v150). The two facts the
old path decided by code, not authoring — no recoil for the hand, the side
muzzle — are recorded as decisions 40–41 in `awaiting-maintainer-decision.md`;
the fold preserves the shipped feel until they are ruled. Guards:
`ambition_held_items::tests` (the request — ⚠ this said `items::pickup::tests`
until the pickup carve of 2026-09-03 moved it out of the kernel) and
`game/ambition_app/tests/hand_fired_held_shot.rs` (the projectile, in the
shipped composition; recoil proven red at −380 px/s).

The press-gated WORLD verbs followed (D-CONTROL-INTERACT): `open_ecs_chests`,
`interact_ecs_actors_and_switches`, `heal_save_shrine_system` and
`regen_player_mana`. Two more lessons came out of that half:

- **a `return` that is right at ONE scope is wrong at another.** The switch
  loop's "once we flip one we stop" is correct per body and ends the SYSTEM as
  written; dialogue's identical-looking `return` is CORRECT, because a
  conversation is a global mode flip and two bodies cannot both open one. The
  question is never "loop or not" — it is what the exit is the exit FROM.
- **an N-body verb can still carry a 1-body fact.** A shrine heals every resting
  body and writes ONE checkpoint. Which body owns that fact is a real question
  (D-SHRINE-CHECKPOINT-OWNER: the comment and the code have long disagreed), and
  a multi-seat conversion is not the place to decide it — so it picks the first
  body in the rewind-stable order and says so.

Still NOT converged, deliberately: the portal gun, whose `FirePortalGun` gesture
carries no seat for a resolver to key off — that is a change to the GESTURE, not
the resolver (D-PORTAL-GESTURE-SEAT). Presentation readers — the portal eye, the
drawn gun, control prompts, the camera — stay singular because a view has one
viewpoint (K3).

### K3 — preserve separate identities where they mean different things

Do not collapse these merely to reduce type count:

- gameplay participant / network peer;
- input seat/device assignment;
- currently driven body;
- home/avatar body;
- camera/view subject;
- presentation focus.

A bug is one concept accidentally used as another concept's authority, not the
existence of several concepts.

### K4 — let actor-monolith carves follow ownership

The residual-kernel definition should guide
[`actor-monolith-decomposition.md`](actor-monolith-decomposition.md). A carve is
valuable when it removes an unrelated authority/dependency from the actor kernel,
not when it merely makes this file's implementation smaller.

## Acceptance pressure

- zero-human-controlled-body headless simulation;
- two or more independently driven bodies with correct item/projectile ownership;
- possession/body switching without a second movement/combat path;
- local and future remote participants controlling ordinary bodies;
- persistent NPCs using the same body/control/navigation seams;
- home-avatar presentation remaining correct without becoming simulation
  authority for every body.

## Open design questions — deliberately unresolved

- What is the smallest stable observation/decision input without a giant context
  bag?
- Which remaining home-body integration differences are genuine semantics versus
  historical duplication?
- Where should provider-specific body abilities attach so multiple driven bodies
  can consume them without enlarging the generic actor action taxonomy?
