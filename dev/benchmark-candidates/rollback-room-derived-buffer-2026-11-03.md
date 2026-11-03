# A restored room root needs its derived collision buffer

## Failure

A speculative crossing retired a room root. GGRS restored that root before
replay. Its saved components returned, but its `FeatureEcsWorldOverlay` did not.
The overlay is derived state, so the snapshot does not encode it. The normal
overlay builder queries roots that already have an overlay. It could not fill
the missing buffer.

The first failing witness returned to a room that another body kept live. The
source had two overlay carriers before the crossing and one after rollback.

## Ownership rule

`RoomInstanceRoot` requires `FeatureEcsWorldOverlay`. Bevy supplies the empty
buffer when it inserts a root, including during snapshot restoration. The
normal contribution systems then fill it. Activation and publication do not
need separate default constructors for that buffer.

Do not snapshot derived collision arrays to repair a missing buffer. Do not add
a second room constructor for rollback. Put the structural requirement on the
component that owns the buffer.

## Test phase

Snapshot state must match before replay uses it. A derived value can return at
its normal projection phase. `RoomCombatTuning`, for example, is inserted again
in `WorldPrep`, before its readers. Its absence before that phase is not a
snapshot failure.

The witnesses compare snapshot rows before the crossing and registered probes
after the frame. Float and geometry probes compare values; some other probes
compare carrier counts. The witnesses also require the overlay carrier before
its builders run.
They cover a new destination, source retention, an existing destination, and
moving-platform retirement. All four pass in the default and combined-feature
app binaries. This proves the tested room applications; it does not prove the
peer preparation protocol or remove its current hold.

Change: `79826e053`.

## Sources

- `crates/ambition_platformer2d_shared_tangle/src/lifecycle/room_instance.rs`:
  the room root and its required buffer.
- `crates/ambition_platformer2d_runtime/src/world_overlay.rs`: the overlay builder.
- `crates/ambition_platformer2d_runtime/src/combat_rules.rs`: the combat projection.
- `crates/ambition_platformer2d_actor_monolith/src/body/mod.rs`: overlay
  contributions run before actor targeting, the first collision reader.
- `game/ambition_app/tests/rollback_room_transition.rs`: the four replay witnesses.
