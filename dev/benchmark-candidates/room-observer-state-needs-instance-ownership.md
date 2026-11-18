# Room observer state needs instance ownership

A room root owns script records, pending event delivery, action history, and
creation counters. Changing only its live-room stamp does not retire this
state. A new visit can then skip initialization or use facts from an old visit.
Clearing only script records leaves the same defect in other attached state.

Each new room instance needs a new root. The root identity includes the native
room ordinal. Its creation counter can start at zero without reusing the
namespace of a body that traveled from an earlier room. A crossing that joins
an existing live room retains that root and its state. A body that travels
retains its own records and counter. Session records span room visits.

Use the normal rollback registration to restore a retired root and its state.
Required components restore derived buffers before their normal builders run.
Do not copy pending observer state from a retired root into its replacement.

The native publication test
`a_new_room_retires_its_old_observer_and_creation_namespace` checks retirement,
carried identity, counter separation, and the required collision buffer.
The loaded `scripted_room_state_restarts_on_reentry_and_replays_with_its_root`
test checks per-room initialization, native actor creation replies, session
records, a replayed crossing, and re-entry through a native door.

The replay fixture must retire its forced-crossing hook after its witness.
A later native teleport can rebase the timeline. A hook tied to a frame number
would then run the old crossing again on a different journey.
