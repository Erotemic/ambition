# Inserting a test between a neighbour's doc block and its body steals that neighbour's `#[test]`

**Tags:** `agent-verification`, `silent-degradation`, `test-integrity`, `rust-attributes`

## The shape

Rust attaches `#[test]` to the next ITEM, and doc comments are attributes too, so
a doc block and its attribute are only bound to a function by ADJACENCY. Insert a
new test in the gap between them and the binding moves:

```text
    /// docs for A                  /// docs for A          <- now describe B
    #[test]                         #[test]                 <- now B's, duplicated
    fn a() { .. }        ==>        /// docs for B
                                    #[test]
                                    fn b() { .. }           <- receives BOTH
                                    ...
                                    fn a() { .. }           <- an ordinary private fn
```

⇒ **The displaced function is dead code wearing a test's name.** It still
compiles. The suite still passes. `cargo test` reports a plausible count, because
one test was added and one silently removed.

⚠ **Both halves of the damage are quiet.** The doubled attribute is a
`duplicated attribute` warning that reads as lint noise; the orphan is
`function ... is never used`, buried among a crate's other unused warnings. ⛔ **A
deleted guard and a stolen guard are indistinguishable from the outside** — and
the guard most at risk is the one directly above where a new test is naturally
appended, i.e. the closest relative of what you just wrote.

⇒ **The check is mechanical and takes ten lines**: walk back from every `fn` over
its doc comments and attributes and count the `#[test]`s. Anything but exactly one
is a bug — **2** means it took a neighbour's, **0** means it lost its own.

⚠ **And the naive walk-back has a blind spot**: a multi-line attribute
(`#[cfg_attr(\n  not(feature = "x"),\n  ignore = "..."\n)]`) between the attribute
and the `fn` has continuation lines starting with neither `#[` nor `//`, so a loop
that stops at "not a doc-or-attribute line" halts before reaching the `#[test]`.
Collapse logical attributes (join from `#[` until the brackets balance) before
walking. Five such sites exist in this repository.

## The 2026-09-04 instance

Two guards in `game/ambition_demo_smash` were dead for a day, both from one
commit (`45e0ceada`) that added a press-enumeration test:

- `the_side_special_is_a_command_grab_and_not_the_standing_grab_renamed` — the
  guard for the exact `lunge_grab` claim being published in the same session;
- `the_grid_holds_the_roster_plus_the_random_cell` — lost its doc the same way.

A sibling session hit the identical shape independently in
`ladder_rig.rs`, where `mirroring_a_bout_swaps_every_per_seat_reading` had been
inert — the guard proving index 0 means the higher rung in BOTH halves of a pair,
which the paired-outcome arithmetic rests on entirely. ⛔ I had told that session
the property "held by construction". It held by luck.

Restored in `7bb880ff3`; the repo-wide sweep found no other instance.

⇒ **What makes this a benchmark candidate**: the task ("add a test pinning this
behaviour") is ordinary, the patch compiles and passes, and the regression is
invisible unless the model checks a property nothing asked it about. The
invariant to preserve is *every `#[test]` binds to exactly one function*.
