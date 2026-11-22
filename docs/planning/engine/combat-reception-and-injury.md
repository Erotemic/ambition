# Combat reception and injury ownership

**State:** PLANNED, 2026-11-22. Source reviewed at production commit
`f412189796254f280e9f5018b685314b6994060c`, with planning overlays through
`d5b1e9d28e8ec3d19a1f7adf532ac614b8f63a99`. No runtime or performance
result is claimed.
**Selection:** [ARCHITECTURE-COMPOSITION](../queue.md#architecture-composition--link-owned-facts-and-preserve-causal-boundaries).
**Contracts:** [combat model](combat-model.md),
[projectile contact](projectile-contact-protocol.md), and
[authored technique admission](authored-technique-admission.md).

## Decision

A strike has several results. Preserve each result at its owner, then publish
the related facts together. Do not infer one result from another.

| Fact | Question it answers | Why another fact is insufficient |
| --- | --- | --- |
| Contact | Which physical surface did the strike reach? | Overlap does not establish accepted damage or a move confirm |
| Surface reception | Did that surface ignore, block or accept the strike? | A shield block and a protected impact can both change no HP |
| Health reception | Which body accepted which injury or defense cost? | The struck body can be a fist whose injury belongs to its rider |
| Physical reaction | What launch, hitlag or other reaction was actually applied? | A boss can lose HP with no launch; a windbox can move a body with no HP loss |
| Move and damage credit | Which move occurrence connected, and what change may be credited? | Surface impact and recipient injury must not produce two confirms for one routed strike |

The first packet unifies result publication for existing receivers. The
second uses that result for GNU's existing fist-to-scholar injury relation.
Neither packet changes contact geometry or requires a general damage graph.

## Current evidence

| Source | Established fact |
| --- | --- |
| [`BodyHitReporter`, `BodyHitResolved`](../../../crates/ambition_damage/src/reception.rs) | Native reception retains source and victim identities before retirement |
| [`BodyHitResolution`, `publish_resolved_hit`](../../../crates/ambition_damage/src/lib.rs) | Health/defense decisions and native connect publication are separate operations |
| [`apply_actor_hit`](../../../crates/ambition_platformer2d_actor_monolith/src/body/damage/actor_hit.rs) | Actor reception is reported before reaction; a second call later publishes connect or block |
| [`apply_boss_hit`, `apply_entity_boss_damage`](../../../crates/ambition_platformer2d_actor_monolith/src/body/damage/boss_hit.rs) | Boss damage publishes reception, but does not publish the native connect used by moves |
| [`mark_move_playback_resolved_hits`](../../../crates/ambition_combat/src/moveset/mod.rs) | `connected_hit` reads `ResolvedBodyHit`; `landed_hit` is a different overlap fact |
| [`mount_view`, `pass_limb_blows`](../../../crates/ambition_boss_encounter/src/conduct.rs) | GNU's limb damage is reconstructed from missing HP, healed, and directly subtracted from the scholar |
| [GNU conductor](../../../game/ambition_content_modules/src/gnu_ton/conductor.rs) | `lost_health` drives a one-blow opening rule and separate clash behavior |

The completed BOSS-CHARACTER work unified definition, identity, construction
and observations. It did not establish a single final hit publication. Keep
that receipt; this is a different boundary.

## CR1 — Finalize one native hit outcome

### Failure and customer

A vulnerable boss can lose HP and publish a native `Damaged` reception,
while the attacking move's `connected_hit` remains false. The boss road
never calls `publish_resolved_hit`. The common feature aftermath marks only
`landed_hit`, and the normal actor receiver excludes `BossConfig`, so it
cannot supply the missing result.

The home-body lethal-damage arm in `ambition_damage` also skips native
connect publication after reporting `Damaged`. This is the same boundary
error: an accepted injury does not stop being a hit when the receiver takes
the death branch instead of the ordinary reaction branch.

This reaches authored gameplay. The
[Performer](../../../game/ambition_content/assets/data/movesets/performer.ron)
uses `OnHit`; the
[Goblin](../../../game/ambition_content/assets/data/movesets/goblin.ron)
uses `Connected`. An attack can thus damage a boss, stop being a whiff, and
still fail its connected condition. The same native stream feeds attacker
recoil and damage-credit consumers. This conclusion follows from the
production writers and readers; it has not been reproduced in a running host.

### Shared operation

Extend the existing damage reporter with one finalization operation. Capture
source, target, room, contact and move occurrence while they are valid. Retain
that context while the receiver resolves health and reaction. Finalization
takes the actual decisions and produces the existing typed projections:
native reception, move connect/block, and the reaction facts those consumers
need. It must not require each receiver to remember independent publication
calls with repeated identity and damage fields.

Keep reception and reaction separate fields. `BodyHitResolution` alone is
not enough to determine the complete result. A training body, a peaceful
protected body, a guarded hull, armor, a wallet shield and a windbox have
different rules. Do not use `damage > 0` or `resolution != Ignored` as the
whole connect rule. State the resolved surface decision and actual reaction
explicitly, then let the common finalizer derive its projections.

Preserve these cases:

- A damaging boss hit confirms once with the applied amount, even with no
  victim launch or hitlag. Zero reaction is an explicit valid result.
- An accepted lethal strike reports its actual injury and move credit once,
  before lifecycle handling loses the context. Keep its existing death
  behavior and explicit absence of an ordinary victim reaction. This corrects
  the missing home-body lethal connect; it does not invent extra hitlag.
- A guard block publishes block, not connect. A not-yet-resolved hit is
  neither a block nor a miss.
- Phase-invulnerable rejection does not become a connect because geometry
  overlapped. Protected local feedback does not by itself prove connect.
- A zero-damage hit that actually has a permitted body reaction keeps its
  existing connect policy. Armor and wallet rules retain their own decisions.
- Match freeze reads the hitlag the reaction actually earned. Do not move
  publication earlier and accidentally report zero for ordinary actors.
- Move credit retains `attacker_move_instance`; a late result cannot confirm
  the attacker's replacement move. Source identity remains available after
  source retirement.

For a direct hit, surface and health recipient are the same occurrence.
Represent that as the ordinary case. The result can later carry a distinct
health recipient for CR2. Do not route every ordinary hit through an object
graph or duplicate its complete context in two persistent journals.

Capture identity before lifecycle effects. Finalize after the required
reaction decision, then let existing owners run their consequences. The
current actor and home-body roads have different resolution cuts; retain
those cuts in this packet. Keep early overlap/stale accounting, immediate
projectile interception, later damage reception and body retirement distinct.
This is not a new event ordering service or a universal death handler.

### Ownership and removal

`ambition_damage` owns finalization and its result vocabulary. Its reporter
already captures identities and it already depends on `ambition_combat`,
which owns move-confirm and hitlag consumers. The actor integration supplies
native receiver policy and calls the shared operation. Pure portable
reception values remain in their current port owner.

Remove caller-managed duplicate publication of `BodyHitResolved`,
`ResolvedBodyHit` and `BlockedBodyHit`. The typed streams can remain as
projections if their consumers need different read cuts. Remove repeated
construction of attacker, victim, source, amount and move fields. Preserve
feature hit deduplication and early overlap markers: they answer different
questions and are not redundant events.

### Proof

Extend the [native damage tests](../../../crates/ambition_platformer2d_actor_monolith/src/body/damage/tests.rs)
with one move-attributed hit on a constructed vulnerable boss. Run the
actual move-confirm consumer. Require HP loss, one accepted reception and
one connect with the same attacker, move occurrence and resolved amount.
Repeat with phase immunity and require no connect. Retain the existing guard
and wallet-shield cases as controls, one zero-damage reaction case, and a
lethal home-body strike which reports its injury without inventing a launch.

No broad new matrix is required. The risk is inconsistent outcome publication,
not all combat tuning. Report publication and lookup counts if cost is
measured; this packet makes no frame-time or binary-size promise.

### CR1 first slice done (2026-11-22)

`BodyHitReporter` owns the move outcome beside the reception record:
`connect(body, hit, resolution, hitlag)` and `block(body, hit)` build
`ResolvedBodyHit` and `BlockedBodyHit` from the hit the road resolved, and the
`resolved`/`blocked` writer fields and the free `publish_resolved_hit` /
`publish_blocked_hit` helpers are gone from both roads. A damaging boss hit
now connects once with the applied amount and zero hitlag; an accepted lethal
home-body strike connects once with zero hitlag before the death arm, with no
launch invented. A guarded boss, an invulnerable phase and a wallet keep their
own decisions (no connect). Witness: `a_damaging_boss_hit_confirms_the_move_that_landed_it`
(attacker, move occurrence 7, amount equal to the HP lost; a guarded boss as
control).

**Open in CR1:** a single finalization taking surface decision and actual
reaction (each road still chooses when to call `connect`); the lethal
home-body unit witness; the phase-immunity control on a constructed boss.

## CR2 — Route limb injury without an HP mailbox

### Failure and customer

`MountLimb.lost_health` is `max - current`. The conductor sees only that
scalar and sends `LimbBlow { slot, pass }`. The native adapter recomputes
the deficit, heals the limb, checks phase immunity, and calls the scholar's
`health.damage` and phase `kill` itself.

The original strike is no longer available. One hit, several hits, a health
cost or damage followed by a partial heal can produce the same deficit.
The scholar's injury has no retained striker, move or contact. It bypasses
the ordinary boss health floor and wallet shield. For example, at 3 HP with
a floor of 2, four missing fist HP can kill the scholar through the forwarding
adapter; the ordinary boss receiver would admit only one HP of damage.
Floor and wallet are supported composition cases, not defenses found on the
shipped GNU encounter.

The authored rule is established: [one blow through an open fist](../game/bosses.md).
The giant remains a mount; encounter membership must not add damageability
to it. Independent limb health remains valid for other authored creatures.

### One optional relation

Use a held native relation for a currently open limb. It binds:

- the exact struck limb occurrence;
- the exact scholar occurrence that receives injury;
- the authored opening occurrence and its remaining one-blow allowance;
- an explicit cap on injury from one accepted blow.

The conductor declares the relation before Combat Resolve. Native reception
reads it only when a hit reaches that limb. Ordinary bodies retain their
direct path with no relation traversal or extra population pass.

Keep the current request authority: the portable declaration names a limb
slot, opening and cap under the invoking scholar. Native binding follows
`submitted.scope` through `RidingOn` and `LimbRig` to its exact limb. A pair
of object references in the same authority does not grant permission to
redirect arbitrary bodies' injury. Retract the active route when that riding
or rig relation changes.

The opening is not the limb's permanent identity and not its offensive
hitbox rearm generation. In phase 1, use the relevant new punch/kick or
hammer opening. In phase 2, use each fist's new stuck interval. Holding the
same opening across ticks must not refill an allowance that native reception
already spent. Retraction removes the relation; a new occurrence can grant
a new allowance. The allowance and held relation follow existing rollback
ownership and room retirement. Both endpoints must belong to the same
supported simulation authority; this packet adds no cross-island injury.

Keep this first relation in the existing boss/limb domain. The portable
declaration belongs in `ambition_boss_special_port`; the held relation and
opening state belong in `ambition_boss_encounter`. These owners already
describe the conducted limbs. The damage integration consumes the relation.
Do not add a generic redirect callback, recursive graph or new crate.

### Ordered resolution

1. The normal geometry and contact owners select the real fist. Projectile
   interception still spends contact once; it does not retarget flight at
   the scholar.
2. Apply the fist's surface eligibility and response rules. Keep its real
   impact and contact point. A later health refusal cannot erase this fact.
3. Check the held opening and resolve the exact injury recipient. Apply the
   per-blow cap, then the recipient's phase/floor and supported health/defense
   policy. Do not test the scholar's hull geometry or hull guard again.
4. An ignored health decision leaves the injury allowance unspent. An
   accepted injury or accepted armor/wallet defense spends it once. Commit
   this with the native result, not with a script's prediction from HP.
5. Finalize one CR1 outcome with the real surface, recipient, surface
   reaction and recipient change. Emit one move confirm for the surface
   operation and one damage credit from the accepted recipient amount.
   Deliver the consumption fact to the conductor at its next read cut.

The scholar's `guarded` flag protects their own hull. GNU deliberately allows
fist punishment while that hull is guarded. Share the boss health-policy
operation below that gate, not `apply_boss_hit` as a whole. Move the current
`apply_entity_boss_damage` implementation into `ambition_damage`, which
already depends on the boss encounter types. Both direct and routed injury
call it. The conductor must stop mutating HP or running its own kill path;
it does not gain a dependency on the actor monolith or damage adapter.

Do not run the ordinary mutating limb damage path and then undo it. Split
surface eligibility/reaction from recipient health mutation where this
customer needs that split. Preserve limb hit-repeat policy and guard effects
without spending or healing fictitious limb HP. Do not apply the same
incoming damage modifiers twice at two receivers.

One routed outcome needs both recipient identities in its read model.
For example, attacker recoil uses the contacted fist's location; damage
credit uses the scholar's accepted injury. Merely changing the old event's
`victim` to the scholar breaks the first consumer. Emitting a second ordinary
hit for the scholar duplicates confirm and credit. Adapt the small set of
consumers to the named facts instead.

| Consumer | Projection from the one routed outcome |
| --- | --- |
| Move playback | One connect/block decision with the original attacker and move occurrence |
| Attacker recoil and impact freeze | Struck surface, captured contact and actual surface reaction; recoil keeps its damaging-impact eligibility even if the recipient refuses injury; retain the surface's room/rule selection |
| Damage-taken reward, such as Limit | Health recipient and the amount that recipient accepted |
| Damage-dealt reward | Original credited attacker and that same accepted amount, once; retain the self-damage rule against the health recipient |
| Portable `hit_receptions` | Named surface and health-recipient decisions, with observer roles; a matching observer gets one row even if it has several roles |
| Conductor | Opening consumption and separate clash acceptance, with the contact/opening time |

The current portable value has one `victim` and `incoming` flag. Extend and
version that contract when CR2 lands. A limb observer must be able to see its
surface reception and a scholar observer their injury decision. An outgoing
attacker or room observer sees one operation, not a duplicate row for each
role. A direct hit has equal surface and health recipient and still produces
one row. Preserve existing per-handler budgets, order and truncation. This
is a bounded projection of the native result, not another event dispatcher.

The conductor's next-cut state must not depend on that truncatable stream.
Retain the accepted clash and injury-consumption facts in the native opening
state and expose them through `MountView`. Include opening identity, impact
time/contact and recipient decision. Transient `BodyHitResolved` messages
clear on rewind; they cannot own a pending next-cut obligation. Keep one
bounded opening state per declaring source occurrence and limb slot, with
its last opening serial and spent markers. Route withdrawal stops new use
but does not erase those markers or refill the same opening. A new opening
replaces the old state; source/limb retirement removes it. No general event
history is needed for this consumer.

### Explicit gameplay policy

**Cap:** preserve the full-health fist's present upper bound as an authored
per-blow injury cap. The current hands and hoof use a 42-HP character
definition in the
[character catalog](../../../game/ambition_content/assets/data/character_catalog.ron).
Migrate that value into the injury policy; do not read a mutable limb HP pool
to obtain it at impact. The cap can then change independently of whether a
limb has its own health. This deliberately stops aggregating unrelated HP
changes and removes the effect of pre-damaging the fist as a transfer queue.
It does not silently allow an uncapped high-damage strike.

**Timing:** select same-cut recipient resolution for this first packet. The
current conductor reads the last Combat step's deficit in WorldPrep, so this
moves scholar damage one tick earlier. Its phase progression and next
conductor's `last_health`/reflex decision can change at that boundary. Check
that edge in the real encounter. If an authored delay is required, retain
the accepted blow with its provenance as bounded rollback state; missing HP
must not return as the delayed-work representation.

**Clash versus injury:** phase-1 clash currently sets `clashed_at`, recoil,
freeze and feedback even if the scholar is invulnerable. Keep a distinct
surface-clash acceptance fact. A refused scholar injury can leave the injury
allowance unspent while the clash episode still ends. Do not derive clash
from `injury_spent`. Record the contact/opening time needed by limb recovery;
do not substitute the next module invocation time for the impact time.

Clash and forwarding require a positive injury proposal admitted by the
fist's surface gates. A zero-damage windbox can keep its ordinary connect
without starting a clash or forwarding injury. Keep this damaging-impact
eligibility separate from the scholar's accepted amount: otherwise phase
refusal would also erase the fist's existing attacker recoil. The first
accepted clash sets a native per-opening latch immediately, even if scholar
injury is refused. Later hits in the same Resolve cut cannot clash again
while they wait for the conductor to set `clashed_at`. The retained clash
fact drives that later authored episode state.

These are proposed implementation choices, not new maintainer rulings. They
preserve the known one-blow and guarded-hull design and name the changes to
provisional HP-mailbox behavior. Agents can implement them without waiting
for a general damage-policy redesign.

### Removal and proof

Delete `lost_health` as a routed-hit notification, `LimbBlow`'s heal/pass
protocol, and `pass_limb_blows`' duplicate phase/flash/kill implementation.
The conductor reads native surface and opening outcomes for its state and
feedback. Delete its duplicate prediction of injury success and repeated
health-deficit checks. Keep its distinct authored clash episode state.

Replace the [test that manually subtracts fist HP](../../../game/ambition_content/src/bosses/conduct_limb_tests.rs)
with a real move or projectile against an open fist. Require one contacted
surface, the correct scholar recipient, retained source/move credit,
unchanged fist HP, and one confirm. A second strike in that opening cannot
forward another injury; a new opening can. Repeated holds of one opening do
not re-arm it. Check the cap, a phase refusal, and a floor or wallet defense
through the common recipient operation. Preserve clash on phase refusal and
check one phase-edge hit under the proposed timing. Rewind across a spent
opening and replay it through the existing populated fixture.

This removes health debit/heal repair, repeated deficit reconstruction and
module-side injury inference. Measure the extra relation/outcome work as well
as those removals. No speedup is established by source review alone.

## Delivery and scope

CR1 ships first and is useful without CR2. CR2 is one shipped encounter
customer of the split between a hit surface and its health recipient. Add no
arbitrary fan-out, recursive redirects, shared-health graph or global damage
registry. A later customer must establish any further law it needs.

Death remains with the existing lifecycle owners. In particular, current
`DefeatCommitted` publication is not uniform across boss and home-body paths;
CR1 does not claim to complete all defeat subscriptions. That limit is not a
reason to replace the whole lifecycle system in this packet.
