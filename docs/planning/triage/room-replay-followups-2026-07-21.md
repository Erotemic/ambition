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

---

# Level-1 acceptance follow-ups — found writing the Mary-O run (2026-07-21)

Same shape as the above: surfaced by one piece of work, none blocking, none
fixed here. Three bugs fell out of the acceptance run; two are fixed in their
own commits (`4e4bd0fd8` the body reset, `cbc6902d2` the vault exit pipe). These
two are not.

## 5. Pit B is not a pit — it opens into the secret vault — [fable/Jon: authoring call]

`vault_bounds()` spans x `800..1248` at y `480..704`. The vault's ceiling is the
level's own ground slab — but pit B is the gap `1088..1184` in exactly that
slab. So the middle pit drops straight into the secret vault.

Consequences, all live:

- falling into pit B is a soft landing in the secret rather than a death, so the
  level's middle hazard is not a hazard;
- jumping while under the shaft launches the player OUT of the vault, which is a
  second undocumented exit;
- `level_1_1`'s own comment says the vault "is reachable ONLY through the pipe:
  it is walled on all four sides, and the ground slab above is its ceiling" —
  that is false as authored;
- a crony that walks into pit B lands in the vault and patrols there. The
  acceptance run currently RELIES on this to exercise the cap's armor, so fixing
  this will need that beat re-pointed at another hit source.

`the_pipe_leads_into_a_sealed_vault_and_back_out` does not catch it: it asserts
the vault is BELOW the slab (a y-ordering) and never that the slab above it is
continuous. Same failure mode as #3 above — the name claims sealed, the
assertions establish something weaker.

**Fix is an authoring call, not a mechanical one:** move the vault out from under
pit B, narrow it, or give it its own lid. Whichever it is, add the assertion the
existing test is missing — no gap in the ceiling slab over the vault's x-span.

## 6. Enemies that fall into a bottomless pit fall forever — [opus]

Cronies patrol without ledge awareness, so they walk into pits A and C and are
never culled. Observed during the acceptance run: three of five cronies at
y = 1966, 2637 and climbing past 8000 while the world is 768 tall, still ticking,
still integrating, never despawned.

Two separate things worth splitting:

- **The leak.** A body below the world bounds should be retired. Nothing does
  that today, so every pit-walker is a permanent entity accumulating velocity.
  This is engine-generic (`ambition_actors`), not Mary-O's.
- **The behavior.** Whether a crony SHOULD walk off a ledge is a design question
  — SMB1 goombas do. But the practical effect here is that level 1-1 empties
  itself of enemies within about eight seconds of load, well before a player
  walking the level reaches them, which is almost certainly not intended.
