# Dialogue continuity — a conversation is sustained, not modal

**Jon, 2026-08-06, verbatim:**

> "I think I do want time to not stop when you are in dialog. So if you get hit
> in dialog, dialog needs to be interrupted I think. Or say you are falling and
> you talk to a the flying parrot, if you fall away from them dialog should also
> break, but if both character are capable of flying and hoverying and you stop
> to talk, then both characters should hover so they can have the dialog. A
> broken dialog can have some bark to indicate that it was broken."

This supersedes the shape D4 was recorded with. D4 decided that dialogue stops
claiming to stop the world; this decides what has to exist because the world
keeps running.

## The reframe

A modal dialogue needs nothing but a state flag: the world is frozen, so the
conversation cannot be disturbed. Once the world keeps running, a conversation
becomes a **sustained condition between actors that has to keep being possible**,
and three things follow that did not exist before:

1. **It can be broken.** By damage, and by the participants ceasing to be in a
   position to talk.
2. **It can be held.** Participants that are capable of holding station do so,
   *so that* the conversation stays possible.
3. **A break is an event with an outward sign** — a bark — because a conversation
   that simply vanishes reads as a bug.

Note the direction of (2). The hover is not a special case bolted on; it is the
general rule stated from the actors' side. A conversation asks its participants to
maintain a conversational stance, they comply if they can, and the ones that
cannot are carried off by ordinary physics — at which point (1) fires. The parrot
is not a parrot rule.

**It is symmetric.** "Both characters should hover" — not "the NPC waits
for the player". This is [relativity over player-centrism](relativity.md) applied
to dialogue: the flying parrot holds station for the same reason the player does,
and a grounded NPC talking to a hovering player is the same situation with the
roles swapped. Any implementation that reads "can the PLAYER still talk" has
already got it wrong.

## Implementation (checked 2026-09-17)

The design is built in `crates/ambition_conversation`. `ActiveConversation` is
the simulation authority; holds and UI are projections of it.

- **Break on a hit: knockback, not damage.** `break_dialogue_on_hit_or_separation`
  tests `recoil_lock_timer > 0.0 || hitstun_timer > 0.0` on `BodyCombat`. A
  poison tick or chip damage does not move the bodies, so it does not end the
  conversation.
- **Break on separation: the two bodies' own AABBs must overlap**
  (`strict_intersects`). No authored range or radius owns a number. This is
  tighter than a normal talking range; if it feels wrong in play, change this
  one predicate.
- **Bark on a break:** the second participant speaks a
  `ConversationCutBark`, only for separation (`reason.wants_its_own_bark()`). A
  hit break emits no cut bark, because `npc_hit_bark_line` already fires. The
  line and pool are a cast question.
- **Holding station:** a conversation zeroes its participants' movement intent.
  A grounded body then stands still, a flying body hovers
  (`integrate_flight_clusters` decays to rest under neutral input), and a
  falling body with no flight keeps falling and breaks the conversation. No
  per-case rule exists. `can_hold_station(&AbilitySet, grounded: bool)`
  (`ambition_platformer2d_core/src/abilities.rs`) predicts the result; nothing
  enforces it. Do not force `fly_enabled` on and restore it later: that is a
  memo, and a memo is rollback state.
- **The other participant:** `project_conversation_hold` claims it with
  `HeldByConversation` and `ControlHold::Conversation`, and the release removes
  only that claim. Note: `ActiveConversation::talker()` is the body being talked
  to, not the one talking; the initiator is `initiator()`.

## Open

- ▢ **More than two participants.** The rules are written for two. The only
  participant destructure, `let [a, b] = participants.as_slice() else { return; }`
  in `crates/ambition_conversation/src/rules.rs`, returns early for any other
  count, so a third actor does not get a half-applied rule. Do not design the
  multi-participant case until a customer needs it.

## What this does not change

`RoomTransition` and `Cutscene` stay globally world-stopping. A room is loading,
or a scripted beat owns the screen; neither is a conversation between actors.
The per-experience opt-in to stop the world for dialogue also stays — Jon's
2026-08-03 ruling made both expressible a requirement, and this decides the
default.

## Ending rules for cutscenes and dialogue

- **A cutscene has one ending.** Skip and completion both call `end_cutscene`
  (`crates/ambition_platformer2d_actor_monolith/src/cutscene.rs`), which sets
  the script's `seen_flag` and clears the runtime and presentation. If the flag
  is not set, `should_play` runs the cutscene again on each visit. A script with
  no `seen_flag` writes nothing. Tests guard both cases.
- **A dialogue close has one description.** The three close roads
  (`DialogState::close`, the `pending_close` drain in `bridge.rs`, and
  `confirm_or_advance`'s runner-finished branch) all call
  `DialogState::clear_conversation_presentation`. Each road keeps its own runner
  request. `every_field_is_cleared_by_a_close_or_rewritten_by_the_next_start`
  (`crates/ambition_dialog/src/runtime.rs`) destructures `DialogState`
  exhaustively, so a new field does not compile until it is classified. The six
  fields that survive a close are all rewritten by `DialogState::start`.
