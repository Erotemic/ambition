# Native scheduling access

This is `bevy_kira_audio` 0.26.0 from crates.io, under its original licenses.
Its upstream source revision is `73ce4e99239698c8fb018ee2e07ff6a0c6cc660b`.
The patch exposes `AudioOutput`, access to its existing manager, the Kira types,
and `AudioInstance::{from_native, resume_at}`. It adds no manager or audio drain.

Ambition uses these controls to admit music voices before a known file end and
resume them on one native clock. Each bounded music group owns a native track
for its exit envelope. Remove this patch when an upstream release
provides the same controls.
