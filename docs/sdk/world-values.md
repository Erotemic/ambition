# World values

A world value is named state that the durable save keeps and that every room
reads. Use one for a consequence that outlives the room that caused it: a
bridge lowered in one room that another room's script draws lowered, a count
of goblins routed across the valley, the stage of a quarrel two towns share.
A script's own [session record](script-state.md) is private to its module and
written from wherever the script runs; a world value is shared by every module
and written only through the world domain.

## Write

```rhai
this.requests.push(request("set_world_value", #{key: "bridge.state", value: "lowered",
    expect: 0, reply: #{request: 1, selector: "my_mod::room/answer"}}));
this.requests.push(request("add_world_count", #{key: "goblins", delta: 1,
    reply: #{request: 2, selector: "my_mod::room/answer"}}));
```

A key is 1 to 48 bytes of `a-z`, `0-9`, `_`, `.` and `-`. A value is any
script value whose encoding fits 1024 bytes. Each write raises the value's
`revision`, from 1. With `expect`, a set applies only while the revision is
still that one; `expect: 0` means "only if nobody has written it yet". A count
is an integer value; an absent count is 0, and two rooms that each add one make
two. Any body entry or `actor_think` entry may write.

The answer is a terminal action result:

| stage | code | meaning |
| --- | --- | --- |
| applied | `world_value_set`, `world_count_added` | written on this tick |
| applied | `world_value_held` | the writer's room rewinds alone; the write waits for its tick to be confirmed (below) |
| rejected | `invalid_world_key`, `world_value_bound` | the key or value is out of bounds |
| rejected | `revision_changed` | `expect` no longer matches |
| rejected | `not_a_count` | `add_world_count` on a value that is not an integer |
| rejected | `world_count_overflow`, `world_value_store_full`, `world_write_queue_full`, `world_change_queue_full` | a bound (256 values, 64 held writes, 64 unheard changes) |

## When a write applies

The world domain applies a write on a tick no room can simulate again. Under
one rollback authority (local play, a GGRS session) that is the writing tick,
and every re-simulation of it writes the same value. On a host where each
room rewinds alone, a write is held with its room and tick: a rewind of that
room takes back the writes it made after its save, and the replay writes them
again; once the confirmed boundary passes the writing tick, the next live tick
applies it. A held write's writer has been told only `world_value_held`; if the
value refuses it when it applies (a changed revision, a full store), it is
dropped with a warning. Read the value, or listen for its change, to learn
what the world holds.

## Read

`world_values` is an `actor_think` observation of every value, in key order:
`this.observe.world_values.values` is a list of `#{key, value, revision}`. A
room entered later, or a session started from the save, reads the values the
save holds.

## Hear a change

An `actor_think` entry with the trigger
`#{port: "world_value_changed", selector: "<key>"}` runs once for each applied
write of that key, at the session, on the next read cut. `this.trigger` is
`#{key, value, revision}`. The session's root owns its records, so give it
`attachment: "session"` schemas. A load is not a change.

## Open and shut an authored gate

A world value is also an authored condition, so anything an author gates on a
condition line can follow it: a `LockWall`'s `gated_by`, a route, a dialogue
branch. `world.value_is <key> <value>` holds while the value is that text,
whole number or truth (`world.value_is bridge.state lowered`);
`world.count_at_least <key> <n>` holds once a count reaches `n` (an absent
count is 0). The gate stays with its owner: the gated wall's writer puts its
solid in the room while the condition does not hold, exactly as for a flag.

A script opens such a wall by writing the value, hears the write's outcome,
and reads the wall's state through `lock_walls`, an `actor_think` observation
of its own room's gated walls: `this.observe.lock_walls.walls` is a list of
`#{id, gated_by, standing}`. A Rhai helper function cannot read `this`, so pass
the list to it. The witness is `scripted_lock_walls`: a wall stands, opens
after one write and stands again after another.

The witnesses are `scripted_world_values`: one room writes, another reads, a
reload keeps them, and a write from a room that rewinds alone applies once,
only when confirmed.
