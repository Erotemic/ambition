# leafwing clash-scan short-circuit — deferred upstream patch

**Status:** deferred, 2026-07-23. Jon does not want to carry a leafwing fork
now. Re-checked against `leafwing-input-manager` 0.21.0 on 2026-09-03.

## Finding

Upstream `handle_clashes` calls `get_clashes(..)`, which runs the full
O(actions²) `possible_clashes()` pair scan every frame for every `InputMap`, and
only then consults `ClashStrategy`. With `PressAll` and no chords, the scan is a
semantic no-op that still costs 1–3.1% of frame CPU in the gameplay chunks of
`desktop-lifecycle-5` (measured on a map with about 20 actions; the current
bindings have more, so treat that range as a lower bound). The upstream comment
about a "cached set" is wrong: `possible_clashes()` builds a new `Vec` on each
call.

The Ambition side is in place and inert by design:
`tune_clash_strategy_to_bindings` (`crates/ambition_platformer2d_host/src/lib.rs`)
sets `PressAll` for chord-free bindings and `PrioritizeLongest` when a chord is
bound. `cargo test -p ambition_platformer2d_host --features input` pins both
directions.

The patch is `dev/patches/leafwing-0.20-pressall-shortcircuit.patch`: two lines
in `src/clashing_inputs.rs::handle_clashes` that return early on
`ClashStrategy::PressAll`. Despite its filename, it targets 0.21.0. Apply it
with `git apply`; `patch -p1` refuses it.

## Evidence command

```bash
grep -n -A2 'name = "leafwing-input-manager"' Cargo.lock
```

Then read `handle_clashes` and `possible_clashes` in that version's
`src/clashing_inputs.rs` (upstream crate, not this repo; cite-ok), and take a
`timeline-run` capture to measure the
`clash` category in gameplay chunks.

## Owner

None. `tracks.md` "Trigger-based work" holds it.

## Trigger to promote

A leafwing version change, or a measured clash cost that matters to a declared
target profile. When promoted:

1. Fork `leafwing-input-manager` at the lockfile version and apply the patch
   with `git apply`.
2. Add a `[patch.crates-io]` entry in the workspace `Cargo.toml` with the same
   shape and retire discipline as the `bevy_ggrs` entry.
3. Verify with a `timeline-run` capture: the `clash` category drops to about 0.
4. Send the change upstream. That is also the retirement path for the fork.

A newer leafwing release can make this obsolete. Re-measure before you carry a
fork.
