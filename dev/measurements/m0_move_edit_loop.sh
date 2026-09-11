#!/usr/bin/env bash
# M0 / fast-iteration I0 — what does ONE MOVE-TIMING EDIT cost, through the
# content file versus through the compiled Rust table?
#
# THE PAIR IS THE MEASUREMENT. A single number for "the content road is fast"
# says nothing on its own: the question is what the SAME class of edit costs on
# the road it replaces, on the same machine, in the same cache state, minutes
# apart.
#
# THE FIRST BUILD IS THE COLD ONE AND IS NOT A SAMPLE. `noop_build` absorbs
# whatever the cache was missing; the three after it are the comparable numbers,
# and `restore_build` is the CONTROL — if undoing the Rust edit does not cost
# about what making it cost, the middle number was noise.
#
# IT MEASURES A BUILD, NOT A LOOP A PERSON LIVES IN. Launching the host is not in
# here, and a running host still has to be restarted to see the change; that is
# what fast-iteration I3's coordinated reload is for.
#
# IT EDITS THE TREE AND PUTS IT BACK. Every `sed` is checked with a `grep` — a
# failed sed and a fast build look identical — and the closing `git diff --stat`
# prints nothing when the restore landed.
#
# Recorded run: dev/measurements/m0_move_edit_loop.log
#
#     bash dev/measurements/m0_move_edit_loop.sh
set -e
cd /home/joncrall/code/ambition
export PATH="$HOME/.cargo/bin:$PATH"
echo "== WARM NO-OP: cargo build -p ambition_app =="
/usr/bin/time -f "noop_build %e s" cargo build -j 4 -q -p ambition_app 2>&1 | tail -2
echo "== EDIT THE CONTENT FILE (a move timing) =="
sed -i '55s/duration_s: 0.19,/duration_s: 0.25,/' game/ambition_content/assets/data/movesets/officer.ron
grep -q "duration_s: 0.25," game/ambition_content/assets/data/movesets/officer.ron || { echo "EDIT DID NOT APPLY"; exit 1; }
/usr/bin/time -f "content_edit_build %e s" cargo build -j 4 -q -p ambition_app 2>&1 | tail -2
sed -i '55s/duration_s: 0.25,/duration_s: 0.19,/' game/ambition_content/assets/data/movesets/officer.ron
echo "== EDIT THE RUST TABLE (the same class of change) =="
sed -i '22s/0.348/0.352/' game/ambition_content/src/officer_moveset.rs
grep -q "FIRE_AT_S: f32 = 0.352" game/ambition_content/src/officer_moveset.rs || { echo "EDIT DID NOT APPLY"; exit 1; }
/usr/bin/time -f "rust_edit_build %e s" cargo build -j 4 -q -p ambition_app 2>&1 | tail -2
sed -i '22s/0.352/0.348/' game/ambition_content/src/officer_moveset.rs
echo "== RESTORE-VERIFY: build is clean again =="
/usr/bin/time -f "restore_build %e s" cargo build -j 4 -q -p ambition_app 2>&1 | tail -2
git diff --stat -- game/ambition_content/assets/data/movesets/officer.ron game/ambition_content/src/officer_moveset.rs
echo "M0 DONE"
