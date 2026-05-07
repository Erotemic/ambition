# Music director compile fix v2

Fixes the first compile error from `ambition_music_director_overlay_v1`:
`MusicCueSpec` gained a `layers` field but `first_goblin_tune_v2_spec()` did not
populate it. This overlay replaces `crates/ambition_sandbox/src/music.rs` with
the same generic director code plus the missing `layers,` initializer.

It also removes an unnecessary `mut` from the `drive_music_director` system
parameter.
