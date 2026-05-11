# Content validation refactor: error-mode counts

During the content-graph validation patch, validation exposed two failures before
the new checks could run:

1. **Module/test import-closure miss — hit #6 in this refactor series.**
   `pause_menu/tests.rs` referenced `RADIO_VISIBLE_ROWS` and `MAX_ROWS` after the
   radio menu patch, but the extracted test module did not import those model
   constants explicitly. This is the same recurring class as earlier missing
   `Vec2`, `AabbExt`, Bevy `App`/`Time`/`Update`, and facade visibility misses.

2. **Owned loop context moved into a collected tuple — new class in this series.**
   `content_validation.rs` computed an `area: String` once per LDtk level, then
   pushed it into a `links` vector inside the nested loading-zone loop. Moving the
   string on the first loading zone made the next iteration unable to reuse it for
   duplicate-zone tracking and subsequent link tuples. The minimal fix is to push
   `area.clone()` into the collected tuple while keeping the per-level `area`
   available for the rest of the loop.

Pattern update: pre-handoff review should include both import/visibility closure
for extracted modules and ownership closure for new validators that collect owned
context inside nested loops.
