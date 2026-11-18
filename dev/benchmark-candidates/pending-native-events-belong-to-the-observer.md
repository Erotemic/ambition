# Pending native events belong to the observer

A native operation can finish after a script's read cut. Its message then
crosses a snapshot boundary. Clearing messages on rollback can lose that
pending delivery, even when the operation's resulting entity is restored.

Separate the native fact from pending script delivery:

- Capture native identity before source or target removal.
- Keep native messages as temporary observations.
- Put pending script facts on the observing body or room root.
- Bound that queue with the game's rules and report truncation.
- Restore the queue with its observer. Remove it when that observer retires.
- Use the native entity link to select the current observer. A replacement
  with the same authored ID must not receive the old body's facts.
- Store portable semantic identities in the queue. Do not store entity handles
  or local session counters in its checksum.

A single queue in a world resource does not meet these rules by itself. A
room can rewind while other rooms stay live. Observer components follow the
existing room snapshot path and travel with a surviving body. The native
publisher must also respect the world's visibility filters, so replay in one
room does not write into a hidden live room.

The projectile outcome service uses this design. Its loaded fixture creates
shots between read cuts, clears native message transport, takes a baseline,
and checks one delivery through GGRS replay for a controlled body and an NPC:

`game/ambition_app/tests/scripted_projectile_credit.rs`

Native tests also cover source removal, replacement refusal, room/session
isolation, travel, and queue bounds:

`crates/ambition_combat/src/projectile_outcomes/extension/tests.rs`
