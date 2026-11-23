# Relationship endpoints and accepted transitions

**State:** REL1 and REL2 LANDED 2026-11-23 (receipts in their sections);
REL3 and REL4 PLANNED. Source audit 2026-11-22. Runtime source:
`00bf92c2214274fdf2ecaacdc0abfd19be569a50`. The preceding geometry overlay is
`f533f6c030701e2e52346496cbd72142d495194d`; it changes documentation only.
The counterexamples below follow native value and control paths. They are
not gameplay reproductions or measured performance results.

**Owners:** capture, mount, possession and item domains own their relations.
The existing lifecycle/publication owners coordinate accepted endpoint
changes. [Queue](../queue.md) selects bounded work under
ARCHITECTURE-COMPOSITION and A7. This page supplies the shared contract and
the mount refinement; it does not create a second execution queue.

## Decision

A relation is valid under more conditions than the existence of its two
endpoints. Its owner must admit the pair, distinguish an active relation
from an intent to form one, and preserve the context needed when it ends.
A transfer or reconstruction must decide whether those conditions still
hold before it changes either endpoint's authority or coordinate space.

Reconstruction must preserve the selected future behavior, not only the
current view. If two held objects look identical but their next legal
orphan release places them differently, their retained state is not
equivalent. Name that continuation at its native owner; do not retain every
component merely because some future reader might need it.

The principal correction is to separate **current residency custody** from
**the bodies required by this particular transfer**. `InCustodyOf` remains
a domain projection. It is not a complete answer for every newly selected
departure root. Prepare the actual transfer cohort from those roots and
native relation policy; then admit each affected relation's disposition.

Use existing domain components, native operations and publication receipts.
Share passive endpoint-change facts where two owners need them. Do not add
a general relationship graph, executable registry, entity per relation,
universal identity type or new per-frame world walk.

## Source evidence

| Boundary | Current evidence | Missing distinction |
| --- | --- | --- |
| Capture ingress | [`acquire_captures`](../../../crates/ambition_combat/src/capture/systems.rs) reserves both endpoints; [`lower_body_holds`](../../../crates/ambition_combat/src/extension.rs) calls the same grant but reserves only captors | Shared mutation does not establish shared admission |
| Ride ingress | [`can_commandeer`, `board_reserved_mounts`, `board`](../../../crates/ambition_mount/src/lib.rs) compare positions or admit a pair without common room evidence; [interaction selection](../../../crates/ambition_platformer2d_actor_monolith/src/body/interact/selection.rs) supplies all mounts | Two nearby coordinate values do not establish a common physical space |
| Ride state and end | The same mount file retains `RidingOn` after an undriven rider's death, and deferred dismount removal permits two `RiderDismounted` messages; the [ability adapter](../../../crates/ambition_abilities/src/ride.rs) has local deduplication | Reset association, active ride, end request and accepted end are different facts |
| Departing cohort | [`project_body_custody`](../../../crates/ambition_platformer2d_actor_monolith/src/body_custody.rs) grows mount/limb custody beneath session or possession travelers; [`leave_through_zones`](../../../crates/ambition_platformer2d_actor_monolith/src/body/direction.rs) and [`cross_on_errands`](../../../crates/ambition_platformer2d_actor_monolith/src/body/errand.rs) start from arbitrary bodies | A current projection is not prospective closure under a new root |
| Possession return | Custody records possessed → home; [release](../../../crates/ambition_platformer2d_actor_monolith/src/control/possession.rs) copies the target's position into home without changing room membership | A return body can be a transfer companion without being physically attached |
| Held-item continuation | [`restore_custody_to_checkpoint`](../../../crates/ambition_platformer2d_items/src/pickup/mod.rs) rebuilds a missing held mint at zero; [`return_released_items`](../../../crates/ambition_held_items/src/lib.rs) uses that stored body if the holder is gone | Data not read during a relation can still be required by its exit |
| Handoff identity | [`CustodyHandoff`](../../../crates/ambition_platformer2d_publication/src/custody_handoffs.rs) retains a custodian's bare `SimId`; its drain builds a global `SimId → Entity` map | A definition address does not select a live occurrence or authorize succession |

### Two transfer witnesses expose the same missing operation

**Mounted NPC.** A normal room-scoped NPC rider supplies no mount custody
edge unless it is already beneath a recognized traveler root. Native
direction and errand departure nevertheless start at that NPC and traverse
the current custody projection. The rider can enter another live room while
the mount remains in the source. Possessing the same rider changes that
closure, although its ride and physical departure are otherwise the same.
The useful abstraction makes the transfer's actual roots explicit; a late
cross-room check that merely dismounts the NPC would lose intended travel.

Membership alone is also insufficient. Publication places explicit arrivals,
not every custody member. The existing rider/mount/limbs crossing fixture in
[`carried_item_crosses_rooms`](../../../game/ambition_app/tests/carried_item_crosses_rooms.rs)
explicitly pins the mount beneath the rider with `place_body` while crossing.
It proves survival/custody, not a consistent native destination pose. Without
that configuration, the next saddle pin can pull the rider away from its
accepted arrival. Preserve that receipt and add the missing placement proof.

**Possession with a retained source.** Alice possesses an actor in A; Bob
keeps A live; Alice's actor enters B. Custody traversal grows from custodian
to dependent. Starting with the possessed actor cannot discover its home
through the existing actor → home custody edge.
[`apply_world_replacement`](../../../crates/ambition_platformer2d_publication/src/transaction.rs)
moves arrivals plus custody, and leaves other source residents in place
when `keeps_left` is true. The home stays in A. Native release then places
it at the actor's B coordinates while its `InRoomInstance` still says A.

The existing solo foreign-room release test in
[`carried_item_crosses_rooms`](../../../game/ambition_app/tests/carried_item_crosses_rooms.rs)
does not contradict this: retiring the source remaps all surviving source
bodies. The retained-source rule is itself guarded by
`a_room_stays_live_for_another_player_and_not_for_the_subjects_own_bodies`
in [transition commit](../../../crates/ambition_platformer2d_runtime/src/room_transition/commit.rs).
Compose those two supported behaviors in the new witness.

### A hold can outlive the spatial domain that admitted it

Fighter and native module capture check common room at acquisition.
`body_custody` excludes capture with the premise that only versus stages
use it. Native [T-rex](../../../game/ambition_content_modules/src/trex.rs)
and [Hunny](../../../game/ambition_content_modules/src/hunny.rs) holds make
that premise false. Keeping physical attachment separate from custody is
still correct.

A captive home body keeps `DrivingParticipant`. The
[zone transition owner](../../../crates/ambition_platformer2d_session/src/rooms/systems.rs)
can admit its automatic Walk/EdgeExit while the captor carries it into the
zone. Alternatively, a possessed captor can cross without its captive.
If another participant retains the source, both endpoints remain alive in
different rooms. Capture maintenance checks existence/reaction, not common
room. Its pose owner applies the captor's coordinates to the captive;
`BodyPoseAdmission` knows the captive's room, not the source of that point.

After crossing, [room replay](../../../crates/ambition_hosted_world/src/room_rollback.rs)
hides foreign-room entities with `Disabled`. The same foreign captor then
appears absent to capture maintenance, so replay can release a hold that
forward simulation kept. Making foreign entities visible to replay would
retain the invalid dependency. Resolve the relation at the accepted
transfer instead. A native fixture can place a hold beside an automatic
exit; this audit does not claim that a shipped boss room has that layout.

## Native states and the proposed disposition matrix

These are proposed first policies, except where a retained receipt is named.
They do not make all relations share one storage or lifetime.

| Native fact | Same live body changes form/control | Accepted live-room transfer | End, reconstruction or reset |
| --- | --- | --- | --- |
| Active capture | Keep the body endpoint; A2 rebinds the named anchor or releases if removed | Release if endpoints separate; preserve only with an explicitly supported joint transfer | Native release withdraws only its relationship hold; remap live references on supported rollback |
| Active ride | Revalidate mount role/licence when their effective inputs change; controller identity alone is not the ride | Rider-root departure includes its active mount/dependents in one admitted destination configuration; other splits need native dismount | End once; keep an independently stated reset association where policy permits |
| Ride reservation | It remains a pending attempt, not an occupied saddle | Refuse/end when endpoints separate in the first slice; no implied cross-room delivery | AC1 identifies construction; mount admission separately decides whether that result can be boarded |
| Remembered ride reset | A named association can remain without authorizing current riding | It supplies no active travel edge | Revival re-admits the pair; preserve the existing reset customer |
| Possession | Body identity, AI policy and seat assignment stay separate | When the possessed body travels, include its inactive home in membership, without making home a driven arrival | Release still places home at the target, at rest; source retention must not alter that rule |
| Held item | Preserve its own recipe and bind-time material relation | Keep custody; transport or replace its native exit context at the accepted cut | Reconstruction must retain the state used by later orphan release; normal release keeps its existing admitted materialization |

Retirement of a live representation, terminal death, resettable defeat and
durable demotion are distinct operations. Do not infer their policy from
`despawn`. Mount custody is currently `SessionOnly`; there is no durable
mount graph to reconstruct. `cross_on_errands` refuses dormant travel with
a nontrivial cohort. Preserve that limit until the destination road can
restore the required native facts. Direction's terminal/dormant exit uses
its own stated disposition; it must not silently leave an occupied saddle.

## REL1 — capture owns admission at every ingress

Make fighter and native hold acquisition share capture's structural law:
each body participates in at most one active capture, in either role. Check
committed occupancy and endpoints already admitted in the current drain
before cancellation, budget changes or relation insertion.

The current native drain can grant two captors the same victim before
deferred inserts become visible. It can also let an already captured B
seize C, creating A → B → C. Pose/throw queries exclude captured captors;
C then has no eligible pose owner. Calling `begin_capture` in both paths
does not prevent either state.

Retain different entry policies. A jaw hold need not inherit fighter
grounded/defense rules; fighter mutual-grab parry remains deliberate policy.
Preserve the actual schedule cuts: native holds drain at `BOSS_CONDUCT`,
fighter acquisition settles later. Share admission at each cut against
settled state and local reservations. Do not delay both into a new global
bus. Use explicit deterministic proposal priority where requests compete;
Bevy query order must not select the winner.

**Remove:** the native adapter's captor-only `seized` approximation and
duplicate structural admission. Keep its authored reach/anchor policy.

**Proof:** at the existing native extension fixture, submit two captors for
one victim and attempt B → C while A holds B. Vary delivery order under the
same declared priority. No refused grant performs side effects. Single
native carry and fighter mutual parry are controls.

**Landed 2026-11-23 (`cb705720a`).** `capture::CaptureOccupancy` is the one
structural law: committed holds at both ends, then both ends of each edge a
cut admits. `sort_by_capture_priority` is the declared priority (captor
`SimId`, then victim). The native seize resolves its proposals against
settled state and admits them in that priority; the captor-only `seized`
list is gone. The fighter grab asks the same law, so a body that holds
somebody is no longer caught. Witness:
`a_seize_admits_one_capture_per_body_in_its_declared_priority`
(`ambition_combat` extension tests); the fighter parry and matching tests
are the controls.

## REL2 — distinguish an active ride and admit its transitions once

Expose one mount-owned active-ride view for steering, pinning, observations,
interaction and travel. It must require the accepted current pair and its
occupied saddle, not just `RidingOn`. Start with a typed query over native
state. Separate the retained reset association only as needed to give each
fact one writer and to avoid states no reader can interpret.

The existing `a_dead_rider_frees_the_saddle_and_its_revival_takes_it_back`
guard in [mount pair tests](../../../crates/ambition_platformer2d_actor_monolith/src/body/mount_pair_tests.rs)
requires that reset behavior without `MountedBrainCache`. Preserve it and
GNU's finale lookup. Neither a dead rider's remembered mount nor a pending
reservation is an active ride. Make reset eligibility a stated native
policy instead of letting a current `DrivingParticipant` query silently
stand for relation origin or persistence.

Use one native board eligibility law for manual boarding, reservation
arrival and rearm. It includes current endpoint liveness, mount role,
licence, common execution scope, saddle occupancy and reservation ownership.
Selection adds reach/preference; final board revalidates at the write cut.
Staged construction still wires and verifies its own local endpoints; it
does not need to call live `board` before its candidate is publishable.
Use the existing room/session contract, including an explicit roomless
composition case. Two absent room stamps do not prove common session.

End a ride once before announcing its terminal fact. Interact bail and
lease expiry can currently both read the old `RidingOn` while removal is
deferred. `apply_dismount_requests` emits twice. Resolve causes under a
stated deterministic priority and admit one end for that current pair.
A native mutation or small per-drain accepted set is sufficient; remove
the ability adapter's `asked` deduplication once the native owner supplies
the promise. Preserve the other owners' pose/control claims.

**Proof:** a foreign same-coordinate mount is neither offered nor boarded;
a reservation cannot board after its rider changes rooms; same-room exact
target boarding still works. Bail plus expiry yields one end fact. A dead
NPC's reset association reports no active ride, then revival re-admits it
when the saddle is available. Keep the existing commandeer and possession
death receipts. No demonstrated same-pair leave/reboard ABA case requires
a new relation occurrence token in this slice.

**Landed 2026-11-23 (`66e7d2386`, `e49d9dbd5`).**

- `lifecycle::ExecutionScope` states one physical space: a live room of a
  session, or the session in a composition with no live room. With two live
  rooms, two bodies of unknown room share none.
- `ambition_mount::BoardCase::refusal` is the one board law (liveness,
  role, licence, other ride, saddle, reservation, one scope). The selection
  adds reach (`nearest_commandeerable`), `board` asks it again at its write
  cut, a reservation that finds its rider apart ends with `RideRefused`, and
  the rearm ends a record the law refuses.
- `active_mount` / `ActiveRides`: a `RidingOn` whose mount's saddle names
  the rider back. Steering, the saddle pin, the `ambition.body.ride`
  observation, interaction selection, custody travel and the zone walls
  read it; boss conduct still reads the association. The reset association
  stays `RidingOn` without a separate component: it has one writer, and the
  active view gives each reader one meaning.
- `apply_dismount_requests` ends each ride once per tick, named by
  `DismountReason::precedence`; the ability adapter's `asked` list is gone.

Witnesses: `a_mount_in_another_live_room_is_neither_offered_nor_boarded`,
`a_reservation_does_not_board_a_rider_in_another_room`,
`two_unknown_rooms_are_not_one_room`,
`a_bail_and_an_expired_lease_on_one_tick_end_the_ride_once` (mount lib) and
the active-ride assertions in
`a_dead_rider_frees_the_saddle_and_its_revival_takes_it_back`.
Open: the saddle pin's own `PoseRefused` end is still a second writer of
`RiderDismounted`; it is safe because a dissolve empties the saddle at once
and the pin reads only an active ride. Form/control revalidation of an
active ride's licence is not done here.

## REL3 — prepare a transfer cohort and native relation dispositions

The transfer plan must distinguish moving membership, arrival placement,
endpoint retirement/succession and native relation effects. A possession
home needs membership movement, not another arrival at the doorway.
A mount may follow a rider although the saddle poses the rider on the mount.
Neither fact can be inferred by reversing every attachment edge.
Active companions need a consistent destination configuration, not just a
new room stamp. The native mount owner uses A1 to place the mount so its
seat and the arriving rider's soles agree; dependent owners prepare their
own poses. Preserve their material, velocity and discontinuity policies.
Admit the whole required configuration before any member changes. This is
native pair/assembly preparation, not a general constraint solver. Keep the
possession home's membership-only case explicit.

1. Start with the actual selected roots, including an NPC departure. Ask
   native owners for required companions and active relations touching the
   affected endpoints. Reuse the bounded closure operation where suitable.
   Include the inactive possession home when its target crosses. Do not add
   a reverse custody edge that cycles with possessed → home.
2. Prepare the complete cohort and its native preserve/release/refuse
   decisions before source-room retention and history footprint are fixed.
   Preserve ordinary local behavior for unaffected endpoints. The first
   capture policy releases a split hold on successful transfer; it adds no
   automatic captive custody. This audit does not add a blanket ban on a
   captured body's existing automatic exit.
3. Revalidate the prepared endpoint facts at the final accepted operation
   cut. An accepted intent slot or a ready asset is not publication success.
   Relation/control changes during preparation invalidate the relevant
   evidence. Refresh before final admission or refuse the stale operation;
   do not discover a refusal after retirement has started.
   `apply_world_replacement` still checks arrivals, target/sinks, mint and
   source-root validity after entry into `promote_and_finalize`. Withdrawal
   must follow those last checks, or a shared non-mutating preflight factored
   from them, as well as the new relation evidence. The earlier promotion or
   restore-consequence seam is not sufficient. No later ordinary refusal
   may leave the old world with its relations already withdrawn.
4. Inside that accepted cut, execute native end effects while required source
   context still exists. Make deferred relation writes visible before any
   destructive endpoint change or new-room reader. Apply membership and
   arrival by their separate policies, then publish accepted outcomes.
   Refused, held or cancelled operations cause no relation changes of their
   own. Ordinary native progression can still end a relation while a
   transfer waits; the later preflight must observe that change.
5. Use this native disposition road for the existing publication crossing
   and direct live-room direction/errand crossing. Their authorization and
   persistence policies stay distinct. Do not repair only the participant
   publication road or defer cleanup to a later `RoomLoaded` observer.
   Cover direction's existing direct terminal/dormant despawn arms too:
   settle native endpoint effects before a permitted retirement. Preserve
   the errand refusal when a dormant destination cannot reconstruct the
   complete cohort; do not turn `SessionOnly` rides into durable records.

Keep dependencies directed. Publication already composes combat, mount and
item owners. Possession remains with control: the composition supplies its
typed companion/evidence through the transfer plan. Do not add a production
publication → actor-monolith dependency to inspect its private resource.
Choose the smallest existing composition seam that can revalidate this
evidence at the same cut; a cached companion list without that check is
insufficient. If the private binding type prevents this, move only the
passive binding/read contract to an existing lower control-data owner that
both sides can use. Keep its one authority, remapping and rollback contract;
do not add a shadow copy for publication. No string-keyed callbacks or
executable service registry.

The [construction verifier](construction-and-reconstitution.md#verification)
already proves endpoints minted within one receipt. Keep that proof.
This new obligation concerns retained live relations touching a move,
replacement or retirement; it is not a reopen of A10 demolition.
[ISL-3](island-rollback-and-compact-peer-input.md#isl-3a--retain-the-transfer-footprint-and-its-membership-cut)
must consume the accepted cohort and all affected relation owners when
forming the history cut. Do not teach replay to read hidden foreign bodies.

**Proof:** use a room-scoped NPC rider with a limbed mount for a live-to-live
departure. Check arrival and seat/soles agreement immediately and after the
next native pin, without test-side mount repositioning. Then use the
retained-source possession case above. Add the native
capture/automatic-exit case with source retention, successful separation
and late refusal: stage/hold a valid operation, invalidate an arrival or
relation fact before final commit, and keep the old capture, control and
membership unchanged. An intent-slot rejection does not prove that cut.
Check membership before another ordinary tick can repair it. Include one
direct retirement/dormant-refusal witness for the existing NPC exit arms.
Replay the accepted destination state and compare with forward
simulation. Existing solo and possessed rider/mount/limbs crossings remain
controls. These are discriminating native witnesses, not a combinatorial
relationship harness or a demand to test every controller permutation.

## REL4 — retain exit context and authorize endpoint succession

**A7 and IR2/IR3 own this packet.** A held item's currently drawn pose is
derived from its hand; its stored body is also the current orphan fallback.
Those are different roles. A reconstruction that is indistinguishable
while held can still behave differently on the next legal release.

Name and retain the fallback placement with its coordinate meaning at the
item owner. Keep the intentional same-room return-where-taken receipt.
Do not update a second live pose every tick. Capture the selected exit
context at checkpoint/durable cuts and restore it even when the mint's
entity was absent. A dummy zero position is not that context.

**Proposed cross-room default:** at an accepted transfer, replace the old
fallback with an item-owned admitted placement in the destination, selected
from that transfer's arrival/binding context. From that cut it is the new
fallback until the next acquisition, accepted transfer or explicit fallback
policy change. Form, control or anchor rebinding alone does not replace it.
This preserves same-room behavior
and gives the previously unqualified foreign-room case an explicit rule.
It does not mean copy A's pickup numbers into B or sample the hand every
frame. Prepare any required map/material admission before final acceptance;
a refusal preserves the prior fallback and custody. Returning to the original
room would be a different authored policy with a dormant-placement customer,
not an implicit consequence of retaining an old coordinate.

Prepare with the destination holder pose/material and destination geometry.
The current [`ItemReleaseWorld::prepare_release`](../../../crates/ambition_held_items/src/release.rs)
reads the holder's current room, so calling it before movement would still
admit against the source. If storage reuses the item's body/material as the
fallback, rebase `HeldMaterialBasis` with it atomically; changing only stored
material applies the holder's scale delta twice. A separate retained exit
placement can leave that material pair unchanged. Choose one native
representation, not both as independently mutable authorities.

Retain placement and its coordinate meaning, not an evergreen permission
to materialize. Geometry can change before the holder ends.
[`PreparedBodyPose::commit`](../../../crates/ambition_platformer2d_core/src/movement/authority.rs)
requires unchanged admission inputs. At orphan exit, prepare against the
current context or revalidate equivalent evidence. Do not serialize and
later replay a `PreparedItemRelease`. If admission fails after the holder
has gone, retain an explicit item-owned pending placement for that fallback
until its owner can admit it. It is neither a world body nor a continuing
relation to the absent holder. Use the existing retention/continuity owner
and retry at its relevant admission boundary, without a hand-pose mirror.

Keep recipe, current pose, relation exit context and serialized export
separate. Use the native continuity owner's placement representation; do
not create a second persistence schema or use an ephemeral live-room ordinal
as a durable place. Preserve `HeldMaterialBasis` and ordinary release's
current material/admission policy. This audit proves a placement loss; it
does not justify choosing a new release speed or changing throw behavior.

The handoff side has a separate scope defect. Two live instances may have
the same authored body `SimId`; native room opening already proves this in
`a_publication_that_opens_a_live_room_leaves_the_room_it_leaves_whole`
([stage tests](../../../crates/ambition_platformer2d_publication/src/stage.rs)).
A global `BTreeMap<SimId, Entity>` drops one holder, and a same-spec hand in
the wrong room passes the current drain comparison.

Retain the exact old holder occurrence and let the accepted publication's
explicit reconstruction/succession authorize any new holder. Keeping only
the old `Entity` fails reconstruction; keeping only its `LiveBodyId` can
fail legitimate room succession. Compose existing scoped identity with
the actual receipt instead of guessing a successor by definition. Audit
the same bare-ID lookup in checkpoint custody restoration against its
selected restore population. A durable custodian address must resolve within
that admitted population, not all current rooms.

Keep handoffs on their publication. A refusal discards only that operation's
effects. Use the handoff's item occurrence when establishing which binding
ends; spec equality alone cannot prove occurrence ownership. No current
same-spec equip-swap ABA interleaving was proved, so do not introduce a
general hand generation solely for that hypothetical.

**Proof:** extend the existing missing-mint checkpoint reconstruction prefix
in [death restore tests](../../../game/ambition_app/tests/death_restores_the_checkpoint.rs)
with native holder removal and a nonzero fallback; compare with the ordinary
orphan control. The existing app test stops after reconstruction and is not
already this witness. Also transfer a held item between differently placed
room charts before orphan release. For handoff, place same-ID holders in
two live instances with the same spec, affect one publication and assert
the other hand is unchanged. A legitimate reconstructed successor and a
refused publication are controls. Preserve IR1's canonical-extent receipt.

## Cost, removal and limits

Admission needs current native endpoint reads, small local reservations and
explicit priority. Boundary work visits the affected cohort and its active
relations. Empty capture/ride paths stay empty; ordinary pose alignment
keeps A1's direct math. No persistent all-body graph, new all-world tick
scan, retained callback object or generic type family is needed.

The expected savings are removal of duplicate arbitration, adapter end
deduplication, wrong-scope recovery and repeated interpretation. They are
not measured frame-time or binary-size gains. If relation counts make the
boundary traversal expensive, measure that customer before adding an index
or data-structure package. Existing small vectors/sets and closure helpers
are enough to establish the semantics first.

Preserve A1/A2, I1/I2 target binding, AC1's construction receipt slice, AC2,
IR1, native rollback `MapEntities`, and independent pose/control claims.
Live `Entity` references are appropriate within these relations' current
lifetimes; the audit found no need to replace all of them with durable IDs.
Form replacement is not automatically endpoint retirement. Do not reopen
portal geometry, introduce exact reference physics on ordinary frames, or
promote Smash-only work through this shared Ambition correction.
