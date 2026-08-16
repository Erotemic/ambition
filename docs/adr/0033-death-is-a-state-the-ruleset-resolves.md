# 0033: Death is a state the ruleset resolves, not an event the world resolves

## Status

Accepted (2026-08-09). Decided by Jon in design conversation, from play reports
against Mary-O.

## Context

Jon, from play: *"I've noticed enemies respawning immediately when she dies even
though the animation and music is still playing. That is not correct."*

Measured, on the standalone Mary-O binary
(`death_reset_timing::the_pinned_death_pose_reflags_the_world_reset_every_frame_of_the_beat`):
one fall into a pit re-flags the kernel's world reset on **192 of the 192 frames**
of the 3.2 s death beat. In the hosted app each of those frames is one
`ResetRoomFeaturesEvent { reason: PlayerDeath }`, because `apply_home_reset_policy`
turns every `Some` on `PlayerBodyFrameOutput.reset` into one with no condition but
`gameplay_allowed`. **One death resets the room ~192 times while the music plays**,
and once more when the beat's replay finally lands.

The chain: the death beat pins her at the place she died → that place is outside
the world → the kernel's blast-zone gate is a POSITION TEST and re-fires every
tick → the flag is `Some` for the whole dwell.

### The engine's death handler is a respawn handler

`death_respawn_player` runs, in order: teleport to `world.spawn` → refill mana →
re-anchor safety → reset the sim clock → `anim.reset()` → `combat.reset()` →
`health.reset()` (FULL HP) → i-frames → banner → **then** `ActorDiedMessage`. The
announcement is a receipt for an undo that already happened.
`integrate_home_body` does the same for the kernel-reset road: any
`ResetCause` teleports the body to spawn unconditionally, with no ruleset
consulted.

So `MaryODeathSequence` is six counter-measures, none of them about dying:

| Mechanism | Exists because the engine already… |
|---|---|
| `at` + `constrain_body_pose` each frame | …teleported her to spawn on the death frame |
| `death_anim_timer` re-armed every tick | …called `BodyAnimFacts::reset()` in that teleport |
| `ScriptedControl` | …keeps feeding her the brain's control frame |
| `Invulnerability::SCRIPTED` | …leaves the hurtbox live on a body that has lost |
| `life_spent` | …re-reports the death every frame, because the pin is outside the world |
| `replay_pending` | …has no notion of "the attempt is over, wait for the ruleset" |

### Two roads, one claimable; and two hosts that disagree

- **Hit death** → `death_respawn_player` → teleport + full heal. Suppressible with
  `RulesetOwnsDeath`.
- **Pit / drown / hazard tile** → `integrate_home_body` teleports unconditionally,
  health untouched, `publish_kernel_reset_death` announces separately.
  **`RulesetOwnsDeath` is never consulted on this road** — the most common death in
  a platformer is the one no ruleset can own.

`RulesetOwnsDeath`'s own doc says *"any ruleset with rounds, stocks, lives, or a
training mode needs exactly this"*. It has one adopter (Smash). Mary-O is a lives
ruleset and does not use it — it reimplements the intent with a pin instead.

And `apply_home_reset_policy` lives in `ambition_app` only, so the standalone demo
binaries and the hosted app **disagree about what a pit death does**. That is why
the demo's own acceptance test has been green throughout: it only ever ran the
host without the defect.

## Decision

**Death is a FACT the engine publishes, a STATE the body carries, and a
CONSEQUENCE the game states. The world does nothing on its own.**

1. **The fact is published before anything acts.** `ActorDiedMessage` stays the
   authoritative "this participant's attempt ended" signal, emitted ahead of every
   consequence rather than after an undo.

2. **A dead body is `OutOfPlay`, and the world does not act on it.** While the
   state is held: no control frame, no hurtbox, nothing teleports it, nothing
   heals it, nothing resets its anim, **and the world's reset gates skip it.**
   That last clause already exists on the ACTOR path (`update.rs`, gated on
   `em.health.alive()`, whose comment states the gate re-fires every tick
   otherwise); the player path never got it. Adding it removes all 192 re-flags at
   the root.

   Consequence: *"she dies where she died"* becomes FREE. Nothing moved her, so
   there is nothing to pin — the pose pin, the anim re-arm, `life_spent`,
   `ScriptedControl` and `Invulnerability::SCRIPTED` all delete. A cliff death
   does the classic thing on its own: she falls off the bottom of the screen while
   the music plays, and the pit gate cannot re-fire because a dead body is not a
   participant.

3. **An INTERLUDE is the named window between a death and its consequence.**
   Authored per experience, opened for the participant who died, closed when its
   duration expires or when content closes it (so "press any key" is
   expressible). Content may fill it with anything — Mary-O's music and death row
   today; a camera push-in, a slow-motion, a "you died" card tomorrow. **The
   consequence runs when the interlude closes**, which makes "the level resets only
   after the death animation completes" structural rather than a scheduling
   accident.

   ⛔ **the interlude does NOT freeze the world.** Co-op forbids it: in NSMB, P2 is
   still playing while P1's death animation runs. It is strictly that
   participant's window and holds nothing global. A game that wants a freeze
   claims a time scale; it cannot be the default. This deliberately leaves the
   open "should the world hold still during a death beat" question unanswered.

4. **A level reset is a question about the ROSTER, not a death consequence.**

   ```rust
   app.declare_death_rules(
       DeathRulesScope::Mode(MARY_O_MODE),          // the rooms these govern
       DeathRules::replay_level_after(3.2),
   );
   // == DeathRules { interlude: 3.2, level_reset: WhenNoParticipantRemains }
   ```

   ⚠ **AMENDED 2026-08-16: a declaration names the ROOMS it governs.** This was
   `app.insert_resource(DeathRules::…)` — a bare global — and three games in the
   shipped host each inserted one at plugin-build time, so the last plugin the
   shell composed governed the whole binary. Mary-O is composed after Sanic, so
   every Smash match ran under her three-second level replay in an arena that
   wants `Never`. `DeathRulesScope` is the same three answers a
   `<Demo>RulesPlugin` already gives when it gates its systems (`in_mode` /
   `in_base_mode` / ungated): `Mode(..)`, `UntaggedRooms`, `EveryRoom`. Rooms no
   game claimed read the default rather than a stranger's rules.

   `LevelReset`: `Never` (arenas, versus, multiplayer that never resets) ·
   `WhenNoParticipantRemains`.

   ⭐ **NSMB and single-player Mary-O are the SAME value.** The level resets when
   a player dies and every other player is already dead; with a roster of one that
   fires on that one death. Hanging the reset off the individual death is
   player-centric and invisible in 1P, which is exactly why it has to be written
   down. Single player is the 1-element case, not the base case.

   Opting in is one constructor, and the default is NOT to reset the level.

   ⚠ **there is deliberately no per-participant FATE axis yet** (bubble-revive,
   respawn at a checkpoint, wait for a teammate). Nothing in the workspace has
   two participants in one level, so the axis is unanswerable until something
   does. Jon: *"we could build a new on death policy if we ever want something
   more elaborate."* Grow it then.

   ⛔ **and default-off has a cost that must be paid by AUTHORING, not by a
   safer default.** A game that states nothing gets `Never`, and with the reflex
   respawn deleted its bodies fall out of the world forever. That is exactly how
   this landed: Sanic said nothing and `the_pit_still_swallows_him_at_any_ring_count`
   went red with `sent_home: false`. Every host states its rules — Ambition and
   Sanic `replay_level_after(0.0)`, Mary-O `replay_level_after(DEATH_DWELL)`.

5. **Deletions, not additions.** `apply_home_reset_policy` is deleted — its two
   jobs become nothing, because the body is dead and the consequence is authored.
   `death_respawn_player` is deleted, and its two death arms collapse into ONE
   that voices the death and publishes the fact. The unconditional teleport in
   `integrate_home_body` is deleted. The respawn happens through the one shared
   road (`reset_sandbox`, reached by `RoomReplayRequested`), which inherits the
   health restore and the post-respawn i-frames — because a body put back at
   spawn still holding the zero HP that killed it is a corpse at the start of the
   level, not a respawn.

## Two things this cost, both worth writing down

**The interlude CLOSES a frame later than it opens, and the split is a rollback
requirement.** `RoomReplayRequested` is `clear_message_on_rollback`, so a request
written late in one frame and consumed early in the next is wiped by a rewind
across that boundary — the resimulated branch never resets the level and the two
runs diverge (measured: a GGRS sync-test checksum mismatch on a route that was
green). Opening reads `ActorDiedMessage` and must therefore run in the frame that
message is written; closing writes `RoomReplayRequested` and must therefore run
in the frame it is consumed, immediately before `RoomReplayApplied`. Everything
closing reads is snapshot state, so the request is re-derivable.

**`DeathInterlude` carries a `consequence_pending` DEBT and is not removed when
it closes.** "Has the consequence already run for this death" has to be
answerable from snapshot state; a window deleted at close takes the answer with
it. Mary-O's beat carried exactly this field, for exactly this reason, and said
so in its doc — it was right, and only its owner was wrong. Dropping it on the
way through cost a desync and an hour.

**Naming.** `ambition_characters::actor::DeathPolicy { HpDepleted, Unbounded }`
already answers *"does a full damage meter kill this body"* — a different
question. The new vocabulary is `DeathRules` and must not be folded into it.

## Current implications for agents

- Never respawn, heal, teleport or reset a body as part of resolving its death.
  Publish the fact; the authored `DeathRules` owns everything after.
- **A game MUST state `DeathRules`**, beside its other rules, **and name the
  rooms they govern** (`App::declare_death_rules`). The default is `Never`,
  which is correct for a versus stage and leaves anything with a level falling
  out of the world. ⛔ never `insert_resource` them: the type is not the key,
  the SCOPE is, and a second game declaring the same scope panics at build.
- Never hang a level reset off an individual death. Ask the roster.
- `RulesetOwnsDeath` is subsumed: the ruleset owns the consequence by
  construction now. Do not grow a second suppression marker beside it.
- Per-participant lives are a known follow-on: `MaryOLevelState.lives` is on the
  level owner, which is the same single-player assumption in a second place. It
  moves when Mary-O actually goes 2P; do not bake the level-owned counter deeper
  in the meantime.
