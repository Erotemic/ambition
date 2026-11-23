# Combat reception and injury ownership

**State:** PARTLY IMPLEMENTED, with planned follow-ups, 2026-11-22.
Reception source rechecked at `34b4ab0b7f4f5a950a52e6031423e76b5e75f6ac`;
the strike-origin follow-up below reads `00bf92c22142`.
CR1's first publication slice and the strike-origin follow-up are done
(2026-11-23). Its actual-reaction finalization, the reaction-data
refinement below and CR2 remain planned.
No runtime or performance result is claimed by this review.
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
| [`BodyHitResolution`](../../../crates/ambition_damage/src/lib.rs), [`BodyHitReporter::connect`](../../../crates/ambition_damage/src/reception.rs) | Health/defense decisions and native connect publication remain separate operations within the reporter contract |
| [`apply_actor_hit`](../../../crates/ambition_platformer2d_actor_monolith/src/body/damage/actor_hit.rs) | Actor reception is reported before reaction; a second call later publishes connect or block |
| [`apply_boss_hit`, `apply_entity_boss_damage`](../../../crates/ambition_platformer2d_actor_monolith/src/body/damage/boss_hit.rs) | The completed first slice now reports damaging boss connect with actual amount and zero hitlag |
| [`mark_move_playback_resolved_hits`](../../../crates/ambition_combat/src/moveset/mod.rs) | `connected_hit` reads `ResolvedBodyHit`; `landed_hit` is a different overlap fact |
| [`mount_view`, `pass_limb_blows`](../../../crates/ambition_boss_encounter/src/conduct.rs) | GNU's limb damage is reconstructed from missing HP, healed, and directly subtracted from the scholar |
| [GNU conductor](../../../game/ambition_content_modules/src/gnu_ton/conductor.rs) | `lost_health` drives a one-blow opening rule and separate clash behavior |

The completed BOSS-CHARACTER work unified definition, identity, construction
and observations. It did not establish a single final hit publication. Keep
that receipt; this is a different boundary.

## CR1 — Finalize one native hit outcome

### Failure and customer

The missing boss and lethal home-body connect publications were repaired
by the first slice below. Do not reopen them. The remaining boundary is
that receivers still decide separately when to publish reception, connect
and reaction. ACT1 needs the actual interruption decision, and the authored
reaction refinement below needs to settle a hit's intended reaction at the
same native cut. `OnHit`/`Connected` consumers and impact feedback remain
customers of one honest final result.

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

Preserve the completed move-attributed boss witness in the
[native damage tests](../../../crates/ambition_platformer2d_actor_monolith/src/body/damage/tests.rs).
The remaining finalization work must keep one reception and one connect
with the same attacker, move occurrence and resolved amount. Complete the
phase-immunity and lethal home-body controls where still absent. Use the
existing guard/wallet and zero-damage cases when changing their finalization;
do not rerun the closed missing-connect investigation.

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

### Immediate consumer: action interruption

[ACT1](body-action-execution.md#act1--accept-and-interrupt-actions-at-the-owning-cut)
uses the remaining actual-reaction part of CR1. The result must say whether
the resolved reaction interrupts the victim's move. Damage or connect alone
cannot decide this for armor, windboxes or boss policy. Make that small
result unconditional gameplay data; the current causal diagnostic outcome
is not present as such in every build. Capture the victim's affected move
occurrence before a deferred consumer could see a replacement.

This does not reopen reporter-owned connect/block or depend on CR2 injury
routing. Complete only the outcome needed by the real action/reaction
crossing, at the established hit-resolution cut.

### Carry attack origin through strike creation and contact

**State:** DONE 2026-11-23.

- **Construction:** `spawn_body_strike` requires a `StrikeOrigin` (`Move(occurrence)` or
  `Independent`) and installs `AttackerMoveInstance` for a move-owned
  strike.
- **Lowerer:** `lower_body_strikes` inherits the submitted request's
  host-set `occurrence` (`StrikeOrigin::of_occurrence`).
- **Early contact:** `LandedBodyHit` carries `attacker_move_instance`, and
  `mark_move_playback_landed_hits` marks and stales only the use it names
  (`verdict_belongs_to`, the rule the resolved connect already used).
- **Unchanged:** on-hit techniques stay per volume, and the early overlap
  cut and its first-contact staleness keep their slot.

A temporary probe of the consumer was then removed. An independent landing
and an old use's landing marked nothing and staled nothing; this use's
landing marked once and staled once. The spawned strike carried
`AttackerMoveInstance(42)` for `Move(42)` and none for `Independent`. The
ground-slam trace through the Smash module was not run: Smash does not
boot on this VM (no music registry).

The audit as written:

**Planned 2026-11-22**, static audit.
This closes a native construction and early-contact gap before reception.
It can ship independently of the remaining receiver finalizer, ACT1/ACT2
and CR2. It preserves the completed reporter-owned connect/block slice.

[Q101](../maintainer-decisions.md) already requires explicit move occurrence
for contact credit. An independent effect has no move credit; an old
occurrence cannot credit the move now playing. The completed A12 receipt
in [the queue](../queue.md) states that rule. The current source below does
not satisfy it at every contact path. This audit cannot date the gap and
does not reopen reflection identity or the whole completed packet.

**Where origin is lost.**

| Boundary | Source fact |
| --- | --- |
| [Ground-slam timeline](../../../game/ambition_content/assets/data/movesets/pugnacious_polygon.ron) | `polygon_brawler_ground_slam` emits `smash.riposte_strike` at 0.24 seconds |
| [Move event dispatch](../../../crates/ambition_combat/src/moveset/mod.rs), [special invocation](../../../crates/ambition_combat/src/extension/special.rs) | The event's move occurrence reaches the submitted body-strike request's `occurrence` |
| [`lower_body_strikes`](../../../crates/ambition_combat/src/extension/special.rs), [`spawn_body_strike`](../../../crates/ambition_combat/src/strike.rs) | The lowerer drops that occurrence; the spawn helper has no origin argument and cannot install its move credit |
| [Hitbox contact](../../../crates/ambition_combat/src/hitbox/mod.rs) | `HitEvent` reads optional `AttackerMoveInstance`, but `LandedBodyHit` carries no move occurrence at all |
| [Playback consumers](../../../crates/ambition_combat/src/moveset/mod.rs) | Accepted connect/block requires the exact occurrence; early overlap marks and stales whichever move is current on the attacker |

The normal authored-volume path is a positive control:
`advance_move_playback` installs `AttackerMoveInstance(pb.instance)` at
birth. The projectile adapter also preserves the submitted occurrence.
The defect is at a specific native operation, not in the ability of the
extension host to retain origin. Its entry declaration already exposes
trigger, phase, allowed ports and request bound together.

**Required construction input.** Give combat strike creation a small,
combat-owned origin value: source body plus an explicit move-credit choice.
Use the existing occurrence identity. Require that value at construction,
with named move-originating and independent forms if useful. The native
adapter must inherit the submitted occurrence for a move-owned strike, or
state an intentional independent choice. An optional decoration on the
returned entity would still let the caller omit a required fact.

Do not recover origin from the owner's current playback. Do not change all
combat requests into a universal command object. `DamageBoxRequest` and
`ProjectileSpawnRequest` already show domain construction with carried
context; reuse their discipline and the existing native materialization
path.

Carry the occurrence through early `LandedBodyHit` as well as accepted
connect/block. Any consumer that changes move-local contact or staleness
must compare the originating occurrence with that playback. Independent
and old-occurrence contacts can still cause their normal physical effects,
damage and feedback, but cannot mark or stale a replacement move.

Keep the early overlap cut and its first-contact staleness rule. Do not
delay it until health reception, retract it on a block, merge it with
accepted connect, or change current actor/home-body reception timing.
Those facts answer different questions. This correction makes each fact
complete enough for its own consumer.

**Attribution is separate from lifetime.** A lingering strike can retain
the move that caused it after that move ends. Its origin does not make it
part of `MovePlayback::live_boxes`, cancel it on interruption, or replace
its authored `HitboxLifetime`. Spatial anchoring and elapsed-time ownership
also keep their current rules. ACT2 owns time changes.

**Smallest useful proof.** Trace the existing ground-slam secondary strike
through the real module adapter. The ordinary fists can independently
confirm the move and hide the loss, so only the shock should reach the
victim. The [authored reach check](../../../game/ambition_content/src/pugnacious_polygon_moveset.rs)
already identifies that wider shock. Account for the victim's hurtbox
when choosing the separation. Verify matching occurrence credit, an
independent strike while another move plays, and an old strike contacting
after replacement. Preserve the normal authored-volume control and the
early overlap versus later reception cuts.

This is a source-derived failure route, not a demonstrated cancel failure
of the shipped slam; its shown definition has no `TechniqueFlow`.
No gameplay, build or test ran in this audit. The correction carries a
small value already available at emission and adds an occurrence check.
It needs no history search, lifetime registry or general causal graph;
no speedup or binary-size change is claimed.

### Bind a hit's reaction to its accepted reception

**State:** PLANNED CR1 refinement from the deeper review. This is shared
engine work, independent of CR2 and of the later Smash polish pass.

The current authored status path starts from contact, not the resolved hit:

| Boundary | Current fact |
| --- | --- |
| [`apply_hitbox_damage`](../../../crates/ambition_combat/src/hitbox/mod.rs) | Emits `HitEvent` and `LandedBodyHit` before the receiver's shield, ordinary i-frame and armor decisions |
| [`dispatch_landed_hit_effects`](../../../crates/ambition_combat/src/on_hit.rs) | Emits the authored effect for that contact without the later reception result |
| [`queue_strikes`](../../../crates/ambition_combat/src/extension/reaction.rs) | Invokes `StruckPort` with victim, contact, striker and params, but no accepted-hit or reaction identity |
| `lower_burials` / `lower_crumples` in the same adapter | Remove the body's current staged launch and stun, then install the status if the body is grounded |
| [Home-body receiver](../../../crates/ambition_damage/src/lib.rs) | Drains stored hits next step; the ordinary reaction can clear a status installed on the earlier contact step |

The normal target-volume publication does not remove ordinary damage
i-frames or active shields. A contacted body can therefore receive one of
these statuses after native reception ignored or blocked the hit. The home
receiver also marks later events in its batch ignored; their contact effects
still have no such result to consult. Preserve that receiver's arbitration
when fixing effect delivery. Do not make every event accepted as a side effect.

The burial/crumple operation names a body, not the reaction it claims to
replace. Another accepted hit can have changed that body's launch before
the callback runs. A later mutation of the current launch cannot establish
which hit it replaced. Rechecking current invulnerability is also wrong:
the accepted hit can itself have charged it.

**Selected design:** put the four simple on-hit reaction choices in prepared
hit data and resolve them inside ordinary native reception. The shipped
`smash.paralyze`, `smash.bury`, `smash.freeze` and `smash.crumple` modules
only forward an authored duration to an engine operation. They do not need
a later procedural invocation to express that data. Keep the reusable
choice with the existing hit/reaction vocabulary; content selects it without
an engine branch on a Smash key or character name.

Keep reaction intent beside the optional launch, not inside it. The current
reaction function returns early when no knockback exists or armor absorbs
it. That must not silently discard an independently authored status.
Evaluate the native defense/armor policy for that intent explicitly; the
absence of a launch is not permission to bypass armor or proof that no
reaction was requested.

The native receiver performs these steps for one admitted hit:

1. Resolve its actual surface/defense decision using the existing owner.
   Ignored and blocked hits do not install these body reactions. Retain
   armor interaction and protected-body policy explicitly. Applied damage
   greater than zero is not the eligibility test: zero-damage reactions and
   accepted damage without flinch both exist.
2. Choose the final ordinary or modified reaction using that receiver's
   stance and capabilities. Burial/crumple require its grounded snapshot;
   an airborne target keeps its ordinary reaction. Paralysis holds motion
   and delays stun/launch; encasement blocks control while motion continues.
   Preserve the existing distinct meanings and mash policies.
3. End prior statuses that this accepted reaction ends, then install this
   hit's selected status and launch together. Decide what actually interrupts
   the victim. A later hit sees the committed result in the existing reception
   order; an earlier callback cannot overwrite that later hit's launch.
4. Finalize reception, credit and actual reaction once, then let ACT1 and
   the other existing consumers act on that result. Keep accepted reciprocal
   contacts and the current actor/home resolution cuts.

Do not apply ordinary mutation and then repair the body from a generic
after-hit callback. Preparation validates the typed reaction and duration.
The ordinary payload remains the direct fast path. A module that constructs
a strike may select the same data before contact; no inline VM call, dynamic
reaction registry or general effect graph is required. Keep body reaction
timers with their current native owner and ACT2's declared clocks.

Distinguish this reaction data from real aftermath. Pogo is a physical
contact response and must not start requiring HP loss. A mark, reward or
other procedural effect can depend on accepted reception or another explicit
native outcome. Carry the required outcome and captured effect context to
that invocation; do not join unrelated messages by victim alone or guess
acceptance from absence. Copy the needed context before a hitbox retires.
Use the existing pending hit carrier when a receiver crosses a step. Do not
add a second persistent event history.

For the touched effects, make the trigger condition explicit at their
existing prepared binding. Do not move every `on_hit` effect wholesale from
contact to damage, and do not create a universal predicate language.
Ordinary standalone status operations can remain when their semantics are
an independently admitted spell or grant. They must not claim to replace a
specific hit's reaction through an unqualified body write. Remove the four
static forwarding paths after their content migrates; keep procedural
capabilities on their semantic merits.

**Proof:** run one authored status hit through actual native reception with
an ordinary target, an ignored/blocked control and the supported armor
control. Repeat the accepted case on the home-body route; the selected
status must follow its own hit's reaction there too. Include a second hit
that replaces the first reaction, and an ignored second event that must
install nothing. Reuse existing native/module fixtures. The current
paralysis fixture's linked/loaded module variants do not test the two native
receivers; directly writing `OnHitEffectMessage` skips this failure boundary.
Retain a contact-only pogo control. No full roster matrix is required.

This removes redundant parameter decoding and module dispatch for these
static reactions, but no speedup has been measured. Select it with CR1's
actual-result work so ACT1 consumes the final reaction, not an intermediate
launch that a later callback is expected to undo.

**Separate native-capture follow-up:** `lower_struck_seizes` clears launch
and stun before its `CaptureAttemptRequested` has succeeded. An accepted
hit is not an accepted capture. Commit that replacement only with native
capture acquisition, retaining the original reaction on refusal. This is
fallible relation work under [native action completion](native-action-completion.md),
not a fifth duration status or a prerequisite for migrating the four simple
reaction choices.

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
