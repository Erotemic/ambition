# Room-replay follow-ups — found while closing tracks §2.5

> **State:** TRIAGE, 2026-07-21. None of these blocked §2.5 and none are fixed.
> Each is small and independently landable. Filed together because they were all
> surfaced by the same work: moving the `RoomReplayRequested` consumer into
> `ambition_runtime` and writing the first proofs that a replay actually replays.

## 1. Sanic clears the act and then dies off the end of it — [sonnet]

`GOAL_X = LEVEL_WIDTH - 400.0` (6000 of 6400), against `ACT_CLEAR_DWELL = 4.0`.
Clearing the act neither brakes the body nor closes the course, so Sanic crosses
the line at speed, coasts past it, runs out of level, and dies — well inside the
results dwell his own card is still counting down.

Also recorded in `dev/journals/code_smells.md` (2026-07-21) with the full
noticing story.

**Why it matters beyond feel:** it makes `act_completion.rs` structurally unable
to assert the act-clear replay. The death respawn returns him to spawn and
rebuilds the room by itself, so position and ring count are identical with and
without a replay consumer installed (verified: 47 → 47 either way). The proof in
`ambition_demo_sanic_app/tests/room_replay.rs` stamps the cleared phase under
controlled setup as a stand-in.

**Fix:** move the goal well inside the runnable extent, or have the clear brake
the body / close the course behind it. Then fold the replay assertion back into
`act_completion.rs` and retire the stand-in.

## 2. A duplicate schedule registration is silent in the demo hosts — [opus]

Registering the same system twice is caught in `ambition_app` only by accident:
`apply_player_reset_input_system` carries a `.before` edge to the replay
consumer, and that edge cannot resolve against a twice-registered system, so
Bevy panics at schedule build. The demo apps have no such edge, and there a
duplicate is a silent double-execution — for an idempotent reset, invisible.

The §2.5 tests pin it with a one-request-one-`ResetRoomFeaturesEvent` count, but
that is per-seam bookkeeping, not a general guard.

**Question for triage:** is there a cheap general shape here — e.g. a startup
assertion that no engine-group system appears twice in the sim schedule — or is
per-seam counting the honest answer? Prefer the Rust/schedule-level answer over
a scanner (standing execution rule). Note this is exactly the hazard class the
§2.5 move creates repeatedly as more consumers migrate from host to engine.

## 3. Emit-observing tests, as a pattern — [opus]

Three separate proofs of the replay beat were green while zero consumers existed
in the process, because each observed a value the EMITTER writes rather than
anything the consumer does:

- `ambition_demo_mary_o/src/lib.rs` `a_settled_tally_rearms_the_level_after_a_dwell`
  — asserts the clock refill written one line before `replay.write(...)`;
- `scripted_level_run.rs` — returns the instant it sees that same refill;
- `act_completion.rs` — stops 0.5s into a 4.0s dwell, before the emit at all.

**Worth a bounded sweep:** other request/consumer seams (`ResetRoomFeaturesEvent`,
`RoomTransitionRequested`, `EncounterCommand`, the effect-intent families track 1
will add) may have the same shape — a test that pins the writer's own bookkeeping
and would not notice a missing reader. This is a specific, checkable instance of
[[a green guardrail proves nothing]]; the general rule is already recorded, the
open work is finding the existing instances.

## 4. Both demos redundantly re-register `RoomReplayRequested` — [sonnet]

`ambition_demo_sanic/src/lib.rs:880` and `ambition_demo_mary_o/src/lib.rs:821`
each call `add_message::<RoomReplayRequested>()`. The engine already registers it
in `SandboxResetSchedulePlugin` (`ambition_actors/src/session/reset/mod.rs:337`),
which both demos get through `PlatformerEnginePlugins`.

Harmless today (Bevy's `add_message` is idempotent), but it reads as though the
demo owns the channel — which is precisely the misreading that let the missing
consumer look fine for so long. Delete both, or keep them only if a demo can
genuinely build without the engine group.
