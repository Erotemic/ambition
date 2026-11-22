# Interaction opportunities and accepted activation

**State:** I1 landed 2026-11-22 (below, "I1 landed"); I2's first slice
landed the same day (below, "I2 first slice").
Source reviewed at `b91b29d35935`; planning follows `9e5e1e06`.
[The queue](../queue.md) selects the bounded work.

## Shared operation

A contextual interaction has three distinct values: an opportunity offered by
a domain at a read cut, a request to take that opportunity, and the domain's
accepted result. The prompt describes the first. Gameplay owns all three.

The shared operation is selection and consumption of one interaction attempt.
Talking, opening a chest, boarding and using a door keep their own admission,
state and consequences. A new mechanic contributes an opportunity and uses
its existing typed request or prepared behavior method. It should not teach
each other input adapter how to yield to it.

This extends `InteractReach`, `InteractCandidate`, `interact_order` and the
existing domain requests. It does not restore the deleted table for every
combat button, add a universal action transaction, or replace the participant
input road.

## Current evidence

| Owner | Source fact | Missing distinction |
| --- | --- | --- |
| [Interaction proximity](../../../crates/ambition_sim_view/src/affordances/interactable_proximity.rs) | `chosen_in_reach` selects talk/switch/chest candidates. `update_nearest_interactable` adds mount/door precedence, then stores only a presentation variant. | Target occurrence, domain admission and presentation label are different values. |
| [Conversation opening](../../../crates/ambition_conversation/src/opening.rs) | `open_between` refuses a pair held by another seat or a self-conversation with no authored self node. | The proximity query does not read these facts. Reach does not establish an available conversation. |
| [Actor interaction](../../../crates/ambition_platformer2d_actor_monolith/src/body/interact.rs) | Execution can skip a refused conversation and try another candidate; it consumes the buffered gesture only after an accepted interaction. | The first proximity candidate does not establish the action that accepts the press. |
| [Mount commandeering](../../../crates/ambition_mount/src/lib.rs) | A raw `ActorControl.interact_pressed` is suppressed by `InteractPressesClaimed`, which `sim_view` writes. | A view-owned classification participates in gameplay arbitration. |
| [Portal input adapter](../../../game/ambition_content/src/portal/input_adapter.rs) | A raw slot press toggles the generator only when `NearestInteractable` predicts no other interaction. | This caller depends on a later operation's predicted consumption and on a presentation crate. |
| [Door detection](../../../crates/ambition_platformer2d_session/src/rooms/systems.rs) | Doors consult talk reach separately, and preserve a buffered tap when the lifecycle slot refuses admission. | A temporarily blocked selected operation differs from an absent opportunity. |

The existing `self_talk_without_a_self_branch_is_suppressed_without_a_trace`
fixture in the actor interaction owner establishes intentional refusal and an
unspent gesture. Combined with the current proximity inputs, it is a source
witness that proximity classification alone cannot prove activation. This
review did not run the composed prompt or portal scenario.

Current timing also differs. The portal adapter runs in `PlayerSimulation`,
doors in `RoomTransition`, mount commandeering in `CombatSet::Settle`, and
talk/switch/chest execution in `FeatureInteraction`. The proximity system
orders after input commitment and before mount commandeering; that is not a
single common read cut for all later consumers. See the
[phase composition](../../../crates/ambition_platformer2d_runtime/src/schedule.rs)
and each owner's installation. A cached answer tagged only by tick is not
enough to merge these observations.

## Domain-owned opportunities

An opportunity identifies the acting body, target occurrence, domain operation,
read cut, availability and presentation facts. Use current occurrence identity
and prepared definition handles. A display name or `InteractVariant` cannot
identify the target to execute. A stored Entity is only a current lookup result.

The domain supplies its real readiness decision. For conversation, factor the
entry-node and participant-availability decision from `open_between`, then
use it for the opportunity and for opening. The door's
`TalkableBodies::nearest_in_reach` must consume it too: its current geometric
prediction can let an unavailable conversation suppress a door. Keep effects
out of that decision.
Use the same resolved conversation meaning when opening if its inputs remain
valid. A changed read cut requires the owning recheck.

Move candidate collection and selection below `sim_view` into the interaction
owner and the selected native composition. `sim_view` consumes the result to
make labels and readiness. A headless host with the same installed gameplay
domains must select the same action without installing the prompt plugin.
Omitting a gameplay domain can remove its opportunities; omitting its display
cannot change the other domains' arbitration.

Keep the existing reach, facing, live-room and stable-order rules. State the
documented precedence explicitly, including the door/talk distance rule,
switch preference, a rider's door before dismount, and boarding before a door
for a body on foot. Do not infer new priority from plugin registration order,
display text or the ordinal of an extensible enum. Reconcile the current
phase conflicts as part of the actual migration; the preview's stated order
is intent, not proof that all executors currently implement it.

An owner can expose a typed built-in operation or an already-bound behavior
method. Use the installed service composition and existing addressed behavior
contract. Do not add a second executable registry, arbitrary World callbacks,
or one central enum that interprets every interaction's payload.

Follow the [execution specialization contract](execution-specialization-and-work-selection.md):
prepared operation identity and source bindings stay prepared, while the
selected body reads current local availability. Do not enumerate every action
for every actor or rebuild a global opportunity table to serve one Interact.

The [A7 merchant exchange](item-custody-and-accounting.md#a7-exchange--one-rule-for-an-offer-and-its-accepted-exchange)
is a concrete offer customer: its item owner checks the complete exchange
against the selected bag and payer, and acceptance uses the same pure rule
on current state. Money alone does not prove capacity, and general possession
does not prove a sale can remove a stored quantity. Keep those laws in the
item owner. Shared availability does not require moving them into gesture
arbitration or treating a preview as a reservation.

## One attempt, explicit consumption and fallback

The participant input owner creates the buffered gesture as it does today.
Interaction selection binds an attempt to that issuer and acting occurrence.
All competing contextual consumers use that attempt instead of independently
reading a raw edge and predicting which other system will spend it.

Preserve gesture origin and eligibility. The current ordinary buffer also
accepts double-tap/held Up, while portal toggle and mount commandeering read
their respective raw Interact edges. An Up door shortcut must not become a
color toggle or boarding request. Retaining a tap does not turn it into a
fresh edge on a later tick. Carry the originating gesture, its current
edge/buffer status, expiry and suppression policy; each operation keeps its
accepted gesture set. The common attempt correlates claims on the same
gesture. It does not give every operation the same input window.

Selection does not extend that window. Check eligibility when selecting and
again at the operation's declared admission cut. Expiry retires an unaccepted
request or claim without effects. A fresh-edge boarding request can reach its
later owner cut within the same tick's declared window; it cannot wait for a
new tick to become eligible again. An already accepted operation can continue
after the input window closes.

An owner reports one of these meanings through the existing action result
road where it applies; these names are conceptual, not a new global result enum:

| Meaning | Gesture and arbitration |
| --- | --- |
| Candidate declined at this cut | Continue through eligible candidates at this cut under the domain's fallback policy, without effects. |
| Selected but awaiting admission | Retain or release selection according to the stated temporary-refusal policy; do not infer that policy from a boolean failure. |
| Accepted operation | Consume once at that owner's acceptance boundary; retain its normal result/cancellation lifetime. |
| Selected target became invalid | Re-evaluate at the next permitted arbitration cut under the stated fallback policy. |

A missing self-dialogue node is not an accepted talk. A lifecycle slot that
already holds another request is not an accepted door crossing. Preserve the
current conversation path's fallthrough to other talkers and then switches
for declines known at the arbitration cut, including an already occupied
conversation. Later contention follows the selected observation policy below.
Preserve the busy door's unspent tap. The selected I2 policy additionally
reserves that selected door attempt until its next eligible arbitration or
expiry, instead of silently taking a lower-priority action. This reservation
is a proposed change: current door refusal retains the buffer but does not
reserve it against every later consumer.

An admitted room transition can outlive input consumption; room loading and
final publication remain lifecycle-owned. Resource payment and target
exclusivity remain with their domain reducers.

**Selected observation policy:** rank one opportunity snapshot at the declared
arbitration cut, after required control/reach facts and before the first
competing Interact writer. Same-cut declines can select the next candidate.
After a request leaves that cut, retain its selected target/operation and
revalidate it at the owner's consume cut. A higher-priority target that appears
later does not steal the attempt. An invalid target returns the unconsumed
attempt to the next arbitration cut while its gesture policy permits it; do
not run an earlier phase backward or revive a fresh edge. A still-unselected
buffered gesture also waits for that next cut. This makes timing explicit;
it does not preserve every previous executor's separate geometry observation.

For example, two actors can select the same free talker in the early snapshot.
If the first opens the conversation at the later owner cut, the second actor's
refused attempt waits for the next eligible arbitration cut. Current execution
can try another talker or switch immediately. That contention timing change is
intentional under this selected policy; do not claim identical late fallback.

Route the portal's contextual color toggle through this selection without
moving ordinary portal firing or body integration by accident. Preserve the
existing toggle-before-fire rule when both arrive on one tick. A queued
`board` can still refuse, a `DismountRequested` can find no ride, and
`portal_toggle_system` can find an absent or inactive generator. Correlate the
actual owner outcome back to the attempt; queue emission alone cannot spend
it. Reuse existing action replies where present and a small local result
otherwise. Mount pose/custody and automatic walk/edge crossings keep their
owners; automatic crossings and admitted party travel are not synthetic presses.

A prompt is a read projection at its stated cut. It does not reserve a target
against another actor or guarantee that a later request can still commit.
Previewing has no side effects. A script or AI can call the same native
operation with an explicit target; it need not invent a participant or borrow
the primary seat's gesture to obtain the same domain legality.

## Physical touch retains its admitted material map

**Source follow-up at `00bf92c22142`, 2026-11-22; planned.** Landed I1/I2
already give selection, execution revalidation and inspection one owner.
[`InteractReach`](../../../crates/ambition_platformer2d_interaction/src/lib.rs)
still uses the whole root AABB for chest/switch touch, and the root position,
frame and front for facing and preference.

A stationary portal straddler can physically touch a far-side switch that
the root box misses. A switch inside the clipped-away portion of that box
can pass the same query without any body material touching it.
[Present material occupancy](world-geometry-and-spatial-semantics.md#present-material-occupancy)
is the shared native query with water/climb contact. It supplies admitted
physical parts and their maps; interaction retains its own policy.

Return the admitted overlap witness, not only `any(part.touches(target))`.
Facing and preference must compare body and target in one chart. For example,
map the target back through the admitted part's path, then use the body's
declared reference geometry in that chart. A union boolean would lose which
map justified the decision.

Do not keep `front()`'s current use of `reach.half_size().x` as the extent
along the body's side axis. `kinematics.aabb()` is a world-axis envelope.
A physical half-size (10,20) turned a quarter-turn has envelope half-size
(20,10); the current formula advances 20 along its side instead of 10.
Derive the front from canonical/body-frame geometry or the existing native
extent projected onto the actual side direction. Keep the ordinary unrotated
rule. Root recharting with unchanged physical material must not change target
preference; mapped targets and the body's reference use the same units.

When more than one admitted part reaches a target, select its eligible
witness deterministically under the interaction policy, then keep one
candidate for that target/operation. Preserve existing kind precedence and
stable target tie order. Do not let part enumeration give one chest two
attempts or change which operation spends the press.

Start with a physical switch. Preserve strict overlap and the ordinary
unpartitioned box path. Talk reach, door occupancy, boarding and pickup
domains are not implicitly changed by this migration.

The selected witness records why selection admitted the target at its read
cut. It grants no later permission. I2 still revalidates that same target
at the owner's consume cut; a moved target or changed portal map requires
a current query. Refusal keeps the current buffer/selection law. No lower
candidate takes over outside the existing arbitration policy.

First prove far-side touch, clipped-away-root refusal and ordinary touch.
Add one turned/reflected facing case and a competing-target preference case
to show that the map survives beyond overlap. A rigid/similarity rechart keeps
the same selected physical target. A rotated face-only contact with overlapping
bounds retains strict-overlap behavior. Use the existing
[interaction observer](../../../crates/ambition_sim_view/src/interaction_reach_observer.rs)
to show the owner's actual parts, target and refusal; do not add a display
geometry calculation.

Delete root-box touch authority for migrated cut material. Retain current
selection and executor ownership. This is a bounded material-consumer
continuation under PORTAL-SIMILARITY, not a restart of I1/I2 or a new
interaction registry.

## Bounded delivery

### I1 — one conversation decision, one interaction owner

Factor conversation availability and resolved entry selection. Use it in the
prompt candidate, the door's talk-priority query and the actual opening path.
Move the existing interaction
query/selection out of `sim_view`; retain presentation-only labels there.
Do not move conversation, chest or mount rules into the common interaction
crate to achieve this dependency inversion. Their owners provide the facts.

**Delete:** the preview/door's incomplete copies of conversation eligibility
and gameplay adapters' imports of the presentation classification. Preserve
current no-self-node and held-by-another-seat outcomes, the door rule and
the source occurrence selected by each body.

**I1 landed (2026-11-22).**

- **One conversation decision.** `ambition_conversation::conversation_entry`
  answers whether a pair has a conversation a seat can open, and which node
  it enters: not held by another seat, and a self-conversation only with an
  authored `__self` node. It is effect-free. `DialogueDispatch::open_between`
  opens through it, and `ConversationAvailability` (read-only) asks it
  without opening.
- **Talkers are offered only with a conversation.** The monolith's
  `TalkableBodies` (`body/interact.rs`) filters talkers by that decision
  (`talkers`, `nearest_in_reach`), with the speaker and seat rules the
  opening uses (`talk_of`, `input_owner_of`). So the door's talk-priority
  query no longer lets an unavailable conversation keep a press. A
  composition without the dialogue domain offers no talker, as the press
  already found.
- **Selection is gameplay's.** `select_interactions`
  (`body/interact/selection.rs`) writes `InteractSelection` (in
  `ambition_platformer2d_interaction`) for every driven body and the
  controlled subject. It is installed by the runtime's world schedule in
  `PlayerInput`, after `PrimarySlotInputCommit` (`InteractSelectionSet`), and
  registered as a derived rollback resource (schema 526). The precedence is
  stated on that function: what the press reaches, then a rider's door before
  dismount, a body on foot's boarding before the door. `InteractTargets` is
  the one choice of target, which the selection and the reach observer
  (`interaction.reach`) both ask.
- **The prompt projects it.** `sim_view`'s `update_nearest_interactable` now
  only names the selection (`InteractVariant`, a mount by its identity).
  `chosen_in_reach` and the prompt's mount and pilot queries are deleted.
- **Consumers read gameplay, not the prompt.** `claim_interact_presses`
  (feeding `InteractPressesClaimed` for the mount owner) moved to the
  monolith and reads the selection (`claims_the_press`: talk, a switch, a
  chest, a door; boarding and getting off are the mount's own). The portal
  colour toggle yields to any selection, mount operations included, as it
  yielded to any prompt before, and orders after the selection. So both
  work with no prompt plugin installed.

The witness is `a_person_with_nothing_to_say_gives_no_prompt_and_keeps_no_door`
(sim_view): a person who is the body's own character, in a project with no
`__self` node, gives no Talk, and a farther door keeps its press. An
ordinary person in the same place gives Talk and keeps the press from the
door.

Still open, and I2's: the executor (`interact_ecs_actors_and_switches`)
keeps its own fallthrough over candidates and revalidates by the same
rules, not through the selected attempt. The portal toggle still predicts the
interaction's later consumption. Doors keep their own zone selection.

### I2 — route the competing contextual consumers

Use talk/switch/chest, door, mount and the existing portal mode toggle as the
bounded customer set. Give the selected attempt and owner result one path;
remove raw-edge consumption and predicted claim side channels as each caller
migrates. State the read and consume cuts and the same-tick ordering above.
Use current public schedule milestones; no universal command buffer is needed.

**I2 first slice (2026-11-22): the executors and the door act on the
selected attempt.**

- The talk/switch executor (`interact_ecs_actors_and_switches`) executes
  only the body's selected Talk or Activate target, revalidated at its own
  cut (`FeatureInteraction`). Its own candidate ordering is deleted. A
  target refused at that cut leaves the press buffered for the next
  arbitration: a higher target that appeared since does not steal it, and no
  lower one is taken instead. So two actors that selected one free talker
  settle through the conversation owner, and the second waits.
- The chest executor (`open_ecs_chests`) opens only the selected chest,
  revalidated (closed, not falling, not opened this pass, touched). Its own
  `interact_order` is deleted. Witness:
  `a_stale_selection_opens_nothing_and_keeps_the_press`.
- A press-activated door crosses only when the press is selected for `Use`.
  The door's own talk-priority query (`TalkableBodies::nearest_in_reach`)
  is deleted. So a switch, a chest or a boardable mount in reach now also
  keeps a door's press, by the selection's stated precedence; before, only
  a talker did. A busy lifecycle slot still leaves the tap buffered.
- `select_interactions` takes the room specs as `Option`, so a
  composition with no room set still selects (no rooms, no doors) instead of
  skipping the system.

Read and consume cuts: selection in `PlayerInput` after
`PrimarySlotInputCommit`; the portal toggle in `PlayerSimulation`; the door
in `RoomTransition`; boarding in `CombatSet::Settle`; talk, switch and
chest in `FeatureInteraction`. Each consumer reads the same tick's
selection.

**I2 second slice (2026-11-22): the mount acts on the selected attempt.**
`InteractPressesClaimed` is replaced by `ambition_mount::MountPresses`. It is
the selection projected for the mount owner (`project_mount_presses`): each
body selected to `Ride` a mount maps to `Board(mount)`, and each body selected
to `Dismount` maps to `Dismount`. `commandeer_mounts` acts only on those:
- It boards the selected mount once `can_commandeer` still admits it at
  `MountsCommandeered`. That is the legality filter split out of
  `nearest_commandeerable`.
- It dismounts a rider selected to get off.
- It does nothing for a press the selection gave elsewhere, or a selection
  the body's state no longer matches.

The mount's own nearest-mount choice is gone, and so is
`InteractSelection::claims_the_press`. A composition with no interaction
owner selects nothing, so no press boards (the mount lib test now states its
selection). Schema 527.

Still open:

- The portal toggle still reads its raw edge and yields to any selection.
  It owns no selected operation, so nothing correlates its owner's outcome
  (board refused, no generator) back to an attempt.
- No witness yet exercises retirement, possession or room replay of a
  selected attempt.

**Acceptance:** the same installed gameplay composition works without the
prompt plugin; an unavailable self-conversation does not falsely claim the
press; a rider uses a selected door without also dismounting; two actors
contending for one target settle through that target's owner; a busy lifecycle
slot retains a tap; retirement, possession, room replay and same-tick
toggle/fire preserve the selected issuer, target and consume policy.
Up shortcuts and retained taps do not acquire fresh-edge-only operations.

Extend the owning interaction and lifecycle witnesses, including current
multi-seat and mount-crossing coverage. No broad affordance census or new
action framework is selected. The first implementation is complete only when
the old prediction or duplicate decision has a migrated consumer and is removed.
