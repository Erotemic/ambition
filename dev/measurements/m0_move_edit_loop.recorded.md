# M0 — one move-timing edit, two roads

Recorded 2026-09-11 by `dev/measurements/m0_move_edit_loop.sh` on the development
machine, warm, `-j 4`. The first build is cold-ish and is not a sample; the
last row is the CONTROL.

```
== WARM NO-OP: cargo build -p ambition_app ==
noop_build 179.30 s
== EDIT THE CONTENT FILE (a move timing) ==
content_edit_build 0.61 s
== EDIT THE RUST TABLE (the same class of change) ==
rust_edit_build 6.30 s
== RESTORE-VERIFY: build is clean again ==
restore_build 6.05 s
M0 DONE
```
