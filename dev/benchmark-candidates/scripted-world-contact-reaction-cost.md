# Scripted world contact reaction cost

The loaded fixture in
[`scripted_body_contacts.rs`](../../game/ambition_app/tests/scripted_body_contacts.rs)
uses real movement contacts to start native actor creation. A controlled body
and the Creator each strike a separate solid breakable from below. The script
reads the solver role and placement identity. It requests a sandbag once and
records the native creation reply. Both bodies then settle on the floor.

Run the fixture with:

```bash
cargo test -p ambition_app --test app_it -- \
  native_world_contacts_start_scripted_creation_for_both_controllers_and_replay --nocapture
```

A local cached debug run on 2026-11-18 used the fixed 60 Hz harness with GGRS
check distance 4 and maximum prediction window 12. The contact entry made 470 calls. Its
mean loaded-backend cost was 180.4 microseconds per call. Mean host invocation
cost was 219.1 microseconds per call. The creation-reply entry made 12 calls,
with means of 73.2 and 95.4 microseconds respectively.

These counters include replay calls. The fixture verifies exactly one live
created actor per caller. Twelve reply invocations do not mean twelve actors.
Host cost includes state access, VM work, validation, and commit. It excludes
native contact collection, actor construction, physics, and rendering. These
numbers are a local observation, not a runtime budget or a release benchmark.

World-contact projection adds no collision query. It copies at most the
room's `max_world_contacts` rows from the current native movement output.
Body-to-body contact collection keeps its existing physical-geometry work.
Use native services for collision resolution and object transactions. A
script can select and combine the resulting facts without another solver.
