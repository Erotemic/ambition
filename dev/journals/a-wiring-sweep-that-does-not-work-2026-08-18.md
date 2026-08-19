# A repo-wide "which systems does only a test register?" sweep — REFUSED, with cause

**2026-08-18.** Written down so nobody builds it again, and because the reason it
fails is more useful than the tool would have been.

## Why it looked worth building

The same defect shape landed five times in one day:

```text
a human's Grab press          brain/player.rs never carried the field, and every
                              capture test writes ActorControl directly
the AI Slop's body            the sizing writes a MIRROR; the guard asks the
                              sizing FUNCTION, never a spawned slop
Oiler's .loop cues            the guard checks bursts that already carry an
                              override, never one that should and does not
item custody release          D125 recorded it outright: "its test pins the
                              FUNCTION, not the WIRING"
my own first fix for it       initialized the schedule against a FRESH World,
                              enumerated zero systems, reported "not registered"
```

⇒ so: could a script find the rest? The decidable version is **"a production
function that reaches a schedule only inside a test"** — proven to compose,
never run.

## What it actually did — four iterations, zero real findings

```text
v1  identifiers inside add_systems(..)      192 hits, mostly PROSE from doc
                                            comments ("answered", "composes")
v2  strip comments; require the name to be
    a fn defined in production               66 hits
v3  raise the 6000-char span cap             66 hits, unchanged
v4  stop matching `mod tests` as a substring 60 hits
```

Every candidate checked by hand was a false positive, and each was false for a
DIFFERENT reason:

* `drop_ability_pickup` / `drop_currency_coin` / `drop_health_pickup` — a
  suspicious sibling cluster, and not systems at all: plain helpers called
  directly from `boss_hit.rs` and `actor_hit.rs`.
* `capture_ball_dash_input`, `fire_spark_on_run_press` — registered as
  `ball_dash::capture_ball_dash_input` in `lib.rs`, and STILL listed after two
  fixes aimed at exactly that.
* `catalog`, `clear`, `close`, `default`, `authored`, `construction` — generic
  names colliding with unrelated functions.

## The two reasons it cannot work as written

1. **`add_systems` is not a parseable boundary from the outside.** Registrations
   are module-qualified, nested in tuples with `.chain()`, `.after()`,
   `.run_if()`, spread across `configure_sets`, and sometimes bound to a local
   first. Identifier extraction cannot tell a registration from a mention.
2. **"is this a test?" has no textual answer.** Path, `#[cfg(test)]` and
   `mod tests` all misfire: a demo's `mod tests;` declaration near the top of
   `lib.rs` made every production registration in that file read as a test one.

## What IS worth doing, and already is

⭐ the **targeted** registration assertion, one per system whose behaviour test
builds its own chain: `the_production_plugin_registers_the_custody_release`
builds the real plugin and asks the sim schedule whether the system is in it.
Poison-verified — deleting the production registration leaves the behaviour test
green and fails this one.

⚠ and it needs a **zero floor**: the first draft initialized the schedule against
a fresh `World`, enumerated nothing, and reported a registered system as missing.
Assert the enumeration is non-empty BEFORE asserting membership, or the guard
reports its own breakage as the subject's.

⛔ **the general lesson, which is the part to keep**: a sweep that produces a
plausible-looking list of false positives is worse than no sweep. It costs every
future reader the hand-verification I just did, and it looks like evidence.


## A SECOND sweep, same day, same verdict — and what the difference was

Later the same session I tried the sibling question: **which rollback-registered
type does production never reference?** Rewound state nothing writes is real
waste, and `BodyActionBuffer` had just turned out to be exactly that.

It ranked `LimbRouteState` at **0 production references**. Verified by hand:
`limbs.rs:314` takes `&mut LimbRouteState` in a system and
`spawn_actors.rs:1698` inserts it. Live. The zero was an artefact of the same
class of filter mistake as the first sweep.

⇒ **two lead generators over Rust source, two unreliable answers.** The rule that
falls out is not "don't measure" — it is about WHERE the leads came from:

```text
found by a SWEEP        add_systems orphans      60 candidates, 0 real
                        unreferenced rewound     1 candidate, 0 real
                        types
found by READING a doc  BodyActionBuffer         1 candidate, REAL — the
        that contradicted                        `AxisManeuverState` field doc
        the code                                 named the combat buffer, and
                                                 it has zero writers and zero
                                                 tick callers
```

⭐ every real finding this session came from a DOC that disagreed with the code —
`ActorControl::grab_pressed` ("the human's Grab button and a CPU's decision write
this SAME field"), `_mary_o_v2_svg_poc` ("Every `ControlFrame` field … survives
this translation"), `AxisManeuverState` ("Combat buffers … stay on the shared
`BodyActionBuffer`"), and D125's own ledger note. ⛔ none came from a grep that
counted things.
