# Dialogue continuity — a conversation is sustained, not modal

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
- **Station-keeping**: ⛔ **the gap.** `can_fly` exists only inside the SMASH
  brain config (`ambition_characters/src/brain/smash/mod.rs`) as a per-fighter
  tuning field. There is no general actor capability answering *"can this body
  hold its position without being carried away"*, and that is the one new
  authority this design needs.

## The open sub-questions

None of these block starting; each changes behaviour enough to be worth an
explicit answer rather than a default nobody chose.

- **What counts as "hit"?** Any damage, or any hitstun/knockback? Damage alone
  breaks a conversation when a poison tick lands; knockback alone lets a chip
  hit pass unnoticed. ⭐ knockback is the one that matches the stated reason —
  the *reason* a hit ends a conversation is that it moves you.
- **What is the break DISTANCE, and who owns it?** A dialogue-authored range, a
  per-actor reach, or the same proximity that started the conversation? The
  cheapest honest answer is the third: you stay in talking range or you stop
  talking.
- **Who barks — the interrupted party, the one who broke it, or both?** An NPC
  barking when the player falls away is the case Jon described; a player-side
  bark is a different feature.
- **Is station-keeping a suspension or an ABILITY being exercised?** Holding a
  hovering body still by zeroing its gravity is a lie that will show up the first
  time something else reads its velocity. Exercising the hover the body already
  has is the honest version, and it is also what makes "capable of" a real query
  rather than a flag.
- **Multi-participant.** Everything above is written for two. A third actor
  joining or leaving is not addressed and should not be invented yet.

## What this does NOT change

`RoomTransition` and `Cutscene` stay globally world-stopping. A room is loading,
or a scripted beat owns the screen; neither is a conversation between actors.
The per-experience opt-in to stop the world for dialogue also stays — Jon's
2026-08-03 ruling made both expressible a requirement, and this decides the
DEFAULT.
