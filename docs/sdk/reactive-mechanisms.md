# Reactive local mechanisms

The `switch_lab` room has a toll lever and a destructible barrier. LDtk defines
both placements, the coin pickup, the barrier shape and its return policy.
[The shipped script](../../game/ambition_content/rhai/switch_lab.rhai) defines
the connection and the price. A source edit can change the behavior without a
host build. The room's older persistence lever keeps its own behavior.

The toll lever uses the native `Activate` action. The interaction owner checks
reach, facing, the acting body and target preference. Each accepted press
updates the target's canonical `ActivationState`. The record contains a revision
and the latest acting occurrence. This action does not write a save switch.

`object_activated` is a typed notification. A behavior subscription names its
source reference and an existing method. The shared inbox delivers the method
in a later `actor_think` cut. Host commit acknowledges the notification.
`object_activation` inspects the native evidence on the current subject.
The example treats notifications as wakeups and also reconciles the latest
activation twice per active simulation second. It can combine several presses into one request.
It does not promise one transaction for every press under queue overflow.

## Find authored objects

`resolve_authored_objects` takes `placements` and a normal `reply`. It uses the
caller's current live room, that room's source definition and native
`SpawnOrigin` records. It returns one `subjects` row per requested placement,
in request order. Each row has the placement key and an exact `object` address.
The lookup request is all-or-nothing. It reports absent, ambiguous and unpublished
placements with distinct refusal codes. It does not select an object by its
position, display name or current capabilities.

`ObjectBehaviorLimits.max_authored_lookups` sets the batch bound. The default
is 64. Placement keys must be nonempty and at most 256 bytes. The registry is a
read-cut projection of native provenance. Travel or retirement changes the next
lookup. A reference already returned continues to name the same occurrence.

## Spend a destructible object

`destructible` inspects the current subject's health and consumed state.
`consume_self` asks that subject's native destructible owner to spend it and
apply inventory terms to an exact `recipient` body in the same live room and
rollback authority. A behavior cannot use this request to spend another prop.
The request uses the same native transaction as movement-contact consumption.
It preflights lifecycle and services, applies the exchange and destructible
state together, and uses the existing collision, art and return paths.

Binding parameters are constructor data. A script reload keeps them. Edit the
method or rebuild the room to change a constructor seed.

The barrier method requests `consume_self` with a coin cost. Its native result
completes the original object call. Insufficient funds, an already consumed
barrier, an inactive body or a foreign recipient refuses without a charge.
Competing calls receive explicit outcomes. A successful callback does not apply
the exchange again. The barrier can also be broken through normal combat.

This is a destructible toll mechanism. A reusable locking door with independent
lock and open states remains a separate native capability. Direct foreign-island
calls refuse. Cross-island consequences still require a committed world domain.


The integration check uses real Interact input, native collision publication,
refused and applied exchanges, duplicate-charge prevention and GGRS replay.
A second fixed-host run sets notification capacity to zero, reconciles the
native revision and changes the method through a watched Rhai file. The same
barrier occurrence remains live. Caller-owned sync-test timelines refuse live
mechanical edits; the edit check uses the fixed host.
