# Room replay ownership

A hosted room replay runs the simulation for one live room instance. It must
keep every other room at the live host boundary. Use
`begin_room_replay(world, room)` and `end_room_replay(world)` from the shared
lifecycle module. The hosted rollback owner also hides the other rooms' entities
and restores the host input tables after replay.

| State | Owner and replay rule |
| --- | --- |
| Room entities and components | `RoomSnapshotRows` saves and restores the selected room. Stable identities connect restored entities to their references. |
| Room-owned resources | Writers use `RoomReplay::allows(owner)` or an equivalent domain set condition. A hidden owner is suspended, not absent. Ownership reads can include `Disabled` entities. |
| Writes to shared world stores | The store records the room and tick, and registers its retractor with `RoomWriteRetractors`. Only the replayed room's speculative writes are retracted. |
| Simulation messages | Tick-local work uses the ordinary Bevy channel with `ReplayMessageReader`. Replay gets a private buffer and reader cursor. The live buffer, message ids, and live reader cursors stay at the host boundary. |
| Work across ticks | Keep authoritative work in room state or a stamped host/narrative input ledger. A transient simulation message is not a rollback record. |
| Narrative input ledgers | Release work only while the owning conversation's room runs. Presentation produces new inputs on the normal host frame. |
| World-owned operations | Run on the normal host tick. Room entry discovery and world cutscene conditions do not run against a partially hidden world. |
| Guest input and saved history | A replay changes only the selected rooms' saved ticks. Hidden rooms keep their history and input prediction. Remove inactive-room history on the normal host tick. |

A channel declaration registers both the session-activation clear and the room
replay boundary. A whole-session rollback can still clear all declared channels.
A room replay must preserve pending work for other rooms. Restoring buffers
alone is insufficient: a reader that advanced during replay can skip or repeat
live messages. `ReplayMessageReader` keeps the two cursor positions separate.

Conversation and cutscene simulation work use the active authority's room.
Cutscene requests for other rooms stay queued. Their runtime, wait answer,
control holds, narrative ledger, and room-change log stay outside the replay.
The current API still has one active conversation and one active cutscene per
world. This scope rule preserves that serial policy; it does not provide
simultaneous independent narrative sessions.

The production acceptance tests are in
[`a_guests_room_rewinds_on_the_host.rs`](../../game/ambition_app/tests/a_guests_room_rewinds_on_the_host.rs).
They compare two guest rooms with late inputs against on-time input, preserve a
pending heal through successive rewinds, and compare another room's active or
queued cutscene and conversation against a normal run. The message primitive
also checks two readers at different live positions.
