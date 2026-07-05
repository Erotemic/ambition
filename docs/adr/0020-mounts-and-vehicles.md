# 0020: Mounts and vehicles — two linked actors, control-deferral, independent hurtboxes

**Status:** Accepted.

**Decided and designed by:** Jon Crall (2026-07-05). This is the *lead design
decision* for the mount/vehicle subsystem. The model below is Jon's, captured
verbatim where he gave exact design language. **Do not deviate from it unless the
deviation is raised as an explicit challenge and Jon accepts the challenge as a
modification.** A later change that supersedes any part of this ADR must say so
here (update or supersede — never leave contradictory guidance elsewhere, per
`docs/adr/README.md`).

Recorded by: opus (executor), transcribing Jon's decision during the R3.5 mount
design conversation of the 2026-07-04 fable review.

## Context

Mounts entered the codebase as a single **fused archetype row**
(`pirate_on_shark` / `pirate_heavy_on_shark` in `character_archetypes.ron`) whose
`composite_visual` block welded a rider and a mount into one authored spawn. That
row was the *sole* home of the mounted rider's combat identity (Skirmisher brain,
`ranged: Bolt(1100)`, `held_item: gun_sword`, both HP pools) — so it was not
data another game could reuse, and it hard-coded the "X on Shark" composition.

The fable review's R3.5 originally scoped this as a byte-identical decomposition
(evict the fused row into a `mount:` string field). Jon reframed it: the fused
row is the wrong model, and the right model is a **general, extensible,
pluggable mount/vehicle system** that "simulates the way it works in the real
world so it has maximum ability for a new game to have it work in a simpler way."
This ADR records that model.

## Decision (Jon's words)

> "The mount should grant the rider different abilities and potentially disable
> some abilities. The mount and mounted should have separate health pools. The
> mount by itself is an actor with some tag that says it can be mounted. The
> rider is an actor with a tag that says it can pilot a certain class of mounts
> (maybe a shark rider cannot pilot a mech). If the mount is not mounted it
> behaves as a normal actor with its own brain. when mounted that brain defers
> it's controls to the rider, but this doesn't have to grant full control (e.g. a
> wild horse might disobey the riders input in a complex or simple way). The
> default mount brain while being ridden should be to grant total control. Note
> this also models vehicles. It simulates the way it works in the real world so
> it has maximum ability for a new game to have it work in a simpler way. Note
> the mount has its own health and may or may not block damage that would
> otherwise hurt the rider. If the mount dies by default the rider is fine and
> just dismounts, but there should be the ability to hit the rider if the mount
> dies or explodes. e.g. if a mech dies and explodes the rider should suffer
> enough damage to die too."

On authoring (boarding scope for the first slice):

> "defer the ability to mount or board right now, but the authored pairs needs to
> be some state in ldtk that indicates the two actors as linked, so they are
> authored as two entities where the mount action has already happened. this
> probably means a lot of the dynamic mounting work needs to be done, but not the
> actual ability to mount. that can happen later."

## The model

### 1. A mount is an actor; a rider is an actor; the link is a relationship

Both the mount and the rider are ordinary actors — each with its own brain, its
own body, its own **health pool**, and its own **hurtbox**. Neither is a special
kind of entity. Two data tags express capability:

- **`Mountable { class }`** on the mount actor — "this actor can be ridden," and
  which *class* of mount it is (e.g. `shark`, `mech`, `horse`).
- **`CanPilot { classes }`** on the rider actor — which mount classes this actor
  is allowed to pilot. A shark-rider cannot board a mech. Piloting is a
  compatibility check, not a hard-coded pairing.

The riding relationship itself is the existing `RidingOn` / `MountSlot`
component link. Nothing about the *pairing* is baked into either archetype — the
same rider can (subject to class) ride any compatible mount, and an unmounted
mount is just a normal actor.

### 2. Unmounted, the mount runs its own brain

An unridden mount behaves as a normal actor driven by its own brain (a wild shark
swims; an idle mech stands). This is the default; it costs nothing.

### 3. Mounted, the mount defers control to the rider — through a grant

When ridden, the mount's brain **defers its controls to the rider**. This
deferral is *not necessarily total*: a `ControlGrant` on the mount mediates how
much of the rider's intent actually drives the mount ("a wild horse might disobey
the rider's input in a complex or simple way"). **The default is
`ControlGrant::Total`** — the rider drives fully. Partial/disobedient grants
(skittish horse, unstable mech) are future variants; see Scope below.

This is the same mechanism for **vehicles**: a mech, a car, a ship is a mount
whose class the pilot can pilot and whose control grant is (typically) Total.

Locomotion is structurally the **mount's**: the mount is the physics body and the
rider is welded to an offset on it, so the rider's own locomotion does not apply
while mounted — the rider's *intent* drives the *mount's* movement through the
grant. The rider keeps its own non-locomotion kit (weapons, aim) by default; the
mount **grants** additional abilities and may **disable** some rider abilities
while mounted. For the current shark content the grant/disable sets are empty —
the rider keeps its gun and the mount contributes flight by being a flying body —
so these are optional data, present as a seam, filled when content needs them.

### 4. Rider-agnostic: the player can pilot through the same path

The rider is **any brain**. An NPC rider and a player rider use the *same* mount
path — a player piloting a vehicle is the identical mechanism as an AI riding a
shark, reached through the existing single control seam (`ControlledSubject` /
`Brain::Player(slot)`, ADR-adjacent to the control-seam design). The model must
never assume the rider is an NPC. Player-piloting is wired in the first slice.

### 5. Two health pools; damage by normal hitbox↔hurtbox; opt-in death splash

The mount and rider each have their own health and their own hurtbox. There is
**no special shielding logic**: a hitbox damages whichever hurtbox it overlaps,
exactly like any other actor. If the rider's hurtbox is exposed (the shark rider's
is), it is hittable; if a mount's body geometry covers the rider, the mount takes
those hits simply because that is what the hitbox overlaps — emergent, not a coded
shield. (Jon: "if a hitbox hits its hurtbox is a target then yes, otherwise no.")

Mount death does **not** by default harm the rider: "If the mount dies by default
the rider is fine and just dismounts." A per-mount **`death_impact`** hook is
opt-in data for the cases that should hurt the rider — "if a mech dies and
explodes the rider should suffer enough damage to die too." Default `death_impact`
is none (clean dismount).

On dismount the rider unwelds, restores its own gravity, and reverts to its own
brain/kit. The runtime already derives the dismounted brain from the rider's
durable combat kit + live held item ("the item is the authority"), so a
gun-carrying rider keeps its weapon after the mount dies and an unarmed one falls
to a melee brute.

## Authoring: two linked LDtk entities (the mount action pre-applied)

Authored mount+rider pairs are **two separate LDtk entities** (one mount, one
rider), linked by an LDtk **entity-reference field** so the world file records
that "the mount action has already happened." The rider entity carries a
`mounted_on` EntityRef pointing at its mount entity (rider→mount, matching
`RidingOn`); the loader resolves the ref into a `RidingOn`/`MountSlot` link at
spawn and welds the pair.

This replaces the fused `composite_visual` archetype row: no single spawn encodes
both actors. It requires (and this ADR authorizes) the loader to parse an LDtk
EntityRef field, and the `ambition_ldtk_tools` to author such a link — this is the
"lot of the dynamic mounting work" Jon noted must be done now even though the
in-game **board action is deferred**.

## Scope of the first slice

**In scope now:**
- Two-actor model with `Mountable{class}` / `CanPilot{classes}` tags and the
  `RidingOn`/`MountSlot` link resolved from an authored LDtk entity-ref.
- `ControlGrant` seam on the mount, **`Total` implemented only**.
- Rider-agnostic path with the **player-piloting** hookup wired.
- Independent hurtboxes/health via the normal hitbox path; opt-in `death_impact`.
- Retirement of the fused `pirate_on_shark` / `composite_visual` archetype model;
  the shark riders become a plain rider archetype + a plain shark mount + a link.

**Deferred (later features, seams reserved):**
- The in-game **board / dismount action** (walk up to a mount and pilot it). The
  authored "already mounted" state is done now; the interaction is not.
- **Partial/disobedient control grants** (skittish/unstable). The `ControlGrant`
  seam exists; only `Total` ships until a content use case lands.
- **Ability grant/disable** payloads on mounts — the seam exists; the sets are
  empty until content needs a mount that adds or suppresses rider abilities.

## Consequences

- Mounts/vehicles become **pure content**: a new game adds a drivable thing by
  adding a `Mountable` actor + a `CanPilot` rider + an authored link, editing no
  engine code — satisfying the engine-for-other-games oracle.
- The `pirate_on_shark` fused archetype, `composite_visual`, `composite_rider_name`
  and the `rider_name_suffix` strip are removed; the two shark-rider variants
  become a rider archetype (its own gun/HP) + the standard `burning_flying_shark`
  (HP 6 for both riders — the old 7-HP heavy-shark was an accident, per Jon, and
  is dropped).
- Behavior is not preserved bit-for-bit (pre-release; elegance-first). The parity
  guard is the composite-spawn test suite, retargeted to assert the *linked-pair*
  resolves the same rider loadout and two HP pools.

## References

- Supersedes the fused-composite portion of ADR 0018 (enemy cluster variation)
  only insofar as composite spawns are concerned; per-actor jitter still applies
  to both the rider and the mount at their brain-spawn sites.
- Relates to ADR 0016 (actor unification — a mount and a rider are both actors),
  ADR 0009/0017 (LDtk owns space/links; RON owns tuning; Rust owns behavior),
  ADR 0012 (sim/presentation split — the welded visual pair is presentation).
