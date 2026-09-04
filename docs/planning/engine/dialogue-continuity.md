# Dialogue continuity — a conversation is sustained, not modal

> **Verified against `ef2c4bd50` (2026-09-04).** Four claims re-derived from the
> code, not re-read from this page: `crates/ambition_conversation/src/rules.rs:32` is still the `let [a, b] =
> participants.as_slice()` let-else that guards multi-participant;
> `can_hold_station(&AbilitySet, grounded: bool)` is still at
> `ambition_platformer2d_core/src/abilities.rs:177`; the break predicate still
> reads `strict_intersects` on the two bodies' own AABBs (`crates/ambition_conversation/src/rules.rs:52`); and
> `ConversationCutBark { speaker: *b }` still names the second participant
> (`crates/ambition_conversation/src/rules.rs:67`). ⛔ **AND A COMPLAINT THIS HEADER USED TO MAKE IS WITHDRAWN
> (2026-09-04): it said "the planning README's drift list still names this file …
> that entry is not current". THE README ENTRY IS CORRECT.** It sits under
> *"OBSERVED 2026-09-02, over a sweep of eleven planning files"* and its list is
> introduced as *"Drifted, none of them header-bearing **at the time**"* — a
> dated observation with its own tense, not a live status list. ⇒ This page was
> reading dated EVIDENCE as if it were a stale CLAIM, which is the inverse of the
> error the recipe warns about, and it told a reader to distrust an accurate
> entry. Nothing in the README needs changing; this sentence did.

**Jon, 2026-08-06, verbatim:**

> "I think I do want time to not stop when you are in dialog. So if you get hit
> in dialog, dialog needs to be interrupted I think. Or say you are falling and
> you talk to a the flying parrot, if you fall away from them dialog should also
> break, but if both character are capable of flying and hoverying and you stop
> to talk, then both characters should hover so they can have the dialog. A
> broken dialog can have some bark to indicate that it was broken."

This supersedes the shape D4 was recorded with. D4 decided that dialogue stops
claiming to stop the world; this decides what has to exist *because* the world
keeps running.

## The reframe

A modal dialogue needs nothing but a state flag: the world is frozen, so the
conversation cannot be disturbed. Once the world keeps running, a conversation
becomes a **sustained condition between actors that has to keep being possible**,
and three things follow that did not exist before:

1. **It can be broken.** By damage, and by the participants ceasing to be in a
   position to talk.
2. **It can be HELD.** Participants that are capable of holding station do so,
   *so that* the conversation stays possible.
3. **A break is an EVENT with an outward sign** — a bark — because a conversation
   that simply vanishes reads as a bug.

⭐ Note the direction of (2). The hover is not a special case bolted on; it is the
general rule stated from the actors' side. A conversation asks its participants to
maintain a conversational stance, they comply if they can, and the ones that
cannot are carried off by ordinary physics — at which point (1) fires. The parrot
is not a parrot rule.

⚠ **and it is symmetric.** "Both characters should hover" — not "the NPC waits
for the player". This is [relativity over player-centrism](relativity.md) applied
to dialogue: the flying parrot holds station for the same reason the player does,
and a grounded NPC talking to a hovering player is the same situation with the
roles swapped. Any implementation that reads "can the PLAYER still talk" has
already got it wrong.

## What exists to build this on

- **Breaking a dialogue**: `ambition_dialog`'s runtime already has `close()` /
  `pending_close`, drained by a dispatch system. A break is that path with a
  reason attached, not a new teardown.
- **Barks**: real vocabulary already, not a new concept — `tick_npc_idle_barks`,
  `suggested_barks` in the actor RON (`richard_duckling_actor.ron`,
  `paul_diracula_actor.ron`, …), and room-metadata bark pools keyed `Hall` /
  `Idle`. A break bark wants a pool of its own beside those.
- **Station-keeping**: ⛔ **was the gap — CLOSED, see the re-measure below.**
  ⚠ **This bullet is kept as written and corrected here rather than edited,
  because a reader arriving at the top of the page would otherwise meet a ⛔ that
  the block below refutes.** What it said: `can_fly` exists only inside the SMASH
  brain config (`ambition_characters/src/brain/smash.rs`) as a per-fighter tuning
  field, there is no general actor capability answering *"can this body hold its
  position without being carried away"*, and that is the one new authority this
  design needs. ⇒ That authority now exists as
  `can_hold_station(&AbilitySet, grounded: bool)`
  (`ambition_platformer2d_core/src/abilities.rs:177`), general and body-facing
  exactly as this bullet asked.

## The open sub-questions

> **RE-MEASURED against `4a9d73a27` (2026-09-02). THIS DESIGN IS BUILT.** It lives in
> `crates/ambition_conversation` (carved out by `197ec6828`), whose module doc
> states the same authority split this page argued for: `ActiveConversation` is
> the simulation authority and holds/UI are projections rebuilt from it.
>
> ⭐ **AND "THE GAP" IS CLOSED.** This page called `can_hold_station` the one new
> authority the design needed, existing then only as a per-fighter `can_fly`
> field inside the SMASH brain config. It is now
> `can_hold_station(&AbilitySet, grounded: bool)` in
> `ambition_platformer2d_core/src/abilities.rs`, with its own tests — a general
> body-capability predicate, exactly as specified.
>
> The ⚠ below ("measure the other participant's brain before building anything")
> is also resolved: `project_conversation_hold` claims non-talker participants
> with `HeldByConversation` and `ControlHold::Conversation`, and releases them
> when the conversation ends.
>
> Three of the four questions below are now ANSWERED BY THE CODE, marked inline.
> The fourth is still open and is now explicitly guarded rather than merely
> unaddressed.

None of these block starting; each changes behaviour enough to be worth an
explicit answer rather than a default nobody chose.

- ✔ **What counts as "hit"? ANSWERED — KNOCKBACK, and the prediction below was
  right.** `break_dialogue_on_hit_or_separation` tests
  `recoil_lock_timer > 0.0 || hitstun_timer > 0.0` on `BodyCombat`, not any
  health change, and says why in situ: a poison tick or a chip of environmental
  damage leaves both bodies standing where they were and leaves them talking.
  (Original reasoning: damage alone breaks a conversation when a poison tick
  lands; knockback alone lets a chip hit pass unnoticed. ⭐ knockback is the one
  that matches the stated reason — the *reason* a hit ends a conversation is
  that it moves you.)
- ✔ **What is the break DISTANCE, and who owns it? ANSWERED — BY NONE OF THE
  THREE OPTIONS OFFERED.** The rule is `a_aabb.strict_intersects(b_aabb)`: the
  two bodies' own AABBs must overlap. Not a dialogue-authored range, not a
  per-actor reach, and not a proximity radius — the bodies themselves are the
  reach, and nothing owns a number. ⚠ Worth a deliberate look rather than
  acceptance by default: overlap is TIGHTER than "talking range" as this page
  imagined it, so two characters conversing at arm's length break the moment
  they stop touching. If that reads wrong in play, this is the line to change,
  and it is one predicate.
- ✔ **Who barks? ANSWERED — the second participant, and ONLY for separation.**
  `ConversationCutBark { speaker: b }`, gated on `reason.wants_its_own_bark()`.
  ⭐ The interesting half is the suppression: a conversation broken by a HIT
  emits no cut bark, because `npc_hit_bark_line` already fires on every strike,
  and a second bubble for one event would be worse than none. The continuity
  layer names the speaker; which line, from which pool, with which fallback,
  stays a cast question.
- ✔ **Is station-keeping a suspension or an ABILITY being exercised?**
  **ANSWERED 2026-08-06, from the code rather than from taste.** Exercising the
  real thing — and the tree already does it, so no new mechanism is needed.
  `integrate_flight_clusters` drives a flying body toward
  `local_stick * terminal_speed`, so a flying body given NEUTRAL input decays to
  rest and hovers. Zeroing gravity would have been a lie that shows up the first
  time something reads velocity; this is the body doing what it can do.

  ⭐ **and that collapses the whole hold into one rule: a conversation zeroes its
  participants' movement INTENT.** All three of Jon's cases fall out of it,
  symmetrically, with no per-case branch:
  - a grounded body given no intent stands still — it holds station;
  - a flying body given no intent hovers — it holds station;
  - a falling body with no flight has no intent to zero, keeps falling, leaves
    reach, and the conversation breaks — which is the parrot case, correct by
    omission rather than by a rule about parrots.

  So `can_hold_station` is a PREDICTION of what neutral intent produces, not a
  thing anybody enforces. Nothing has to force `fly_enabled` on and restore it
  afterwards — which matters, because that would be a memo with a restore
  obligation, and a memo is rollback state.

  ✔ **BOTH HALVES ARE TRUE NOW — this row is FINISHED, and the paragraph it
  supersedes is compressed to a line rather than left standing below its own
  answer.** It read *"half of this is already true … what is left is the other
  participant — an NPC whose brain may still be steering it mid-sentence.
  Measure that before building anything."* Measured 2026-09-04:
  `project_conversation_hold` (`ambition_conversation/src/hold.rs:20`) claims the
  other participant with `HeldByConversation` + `ControlHold::Conversation` and
  releases it when the conversation ends — and the release names ONLY
  `ControlHold::Conversation`, so other `ScriptedControl` claims survive.

  ⚠ **AND ONE NAMING TRAP, recorded because it nearly cost a false correction:
  `ActiveConversation::talker()` is NOT the one talking.** Its doc says *"The
  body being talked TO. The hold applies to this one"*, and the initiator is
  `initiator()`. So `let holding = conversation.talker()` in
  `project_conversation_hold` **is** the other participant — but a reader
  checking this row against the code will briefly conclude the opposite. The
  page's claim is right; the identifier reads backwards.
- ▢ **Multi-participant — STILL OPEN, and now explicitly guarded.** Everything
  above is written for two, and the code enforces that rather than degrading:
  `break_dialogue_on_hit_or_separation` destructures participants as `[a, b]`
  and returns early otherwise, so a third actor does not silently get a
  half-applied rule. A third actor joining or leaving is still not addressed and
  should still not be invented yet — but the place it would go is now one
  `else` branch with a comment on it.
  ✔ **Re-verified against the code 2026-09-02 and it holds exactly**:
  `crates/ambition_conversation/src/rules.rs:32` is
  `let [a, b] = participants.as_slice() else { … return; };` — a let-else, so a
  conversation with one or three participants leaves the rule without applying
  half of it. Nothing has quietly generalised it, and the `else` is still the
  one place a third actor would be handled.

## What this does NOT change

`RoomTransition` and `Cutscene` stay globally world-stopping. A room is loading,
or a scripted beat owns the screen; neither is a conversation between actors.
The per-experience opt-in to stop the world for dialogue also stays — Jon's
2026-08-03 ruling made both expressible a requirement, and this decides the
DEFAULT.
