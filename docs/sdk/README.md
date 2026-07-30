# Ambition SDK

**Building a game on this engine? Start here — and you should not need to open
anything under `crates/`.**

That is the acceptance test, not a courtesy. ADR 0031 makes the blind-agent run
one of two mechanical gates on the public API: *can an agent implement a
character, a room and a mechanic with only `docs/sdk/` and `ambition::prelude`
in context, never opening a file under `crates/`?* The recorded result includes
**which engine file it had to open first**, because that field names the next
leak. If you had to open one, that is a bug in this directory.

## Status: slice A, in progress

This SDK is being built one leak at a time by
[the API 1.0 campaign](../planning/engine/api-1.0-campaign.md). Being honest
about what is not here yet is part of the method — a doc that implies coverage
it lacks sends a reader into `crates/` with no warning.

| Area | Status |
|---|---|
| Host composition — standing up a game, visible and headless | **IMPLEMENTED** — `ambition::app`, designed in [api-prototype.md](api-prototype.md) |
| Declaring content — characters, rooms, packs | not started (slice B) |
| Capabilities and rollback schema | not started (slice C) |
| Revising content at runtime | not started (slice D) |

## Before any of that: your `Cargo.toml`

**This engine does not currently compile for you without a patch table, and
nothing tells you so.** Found by the 2026-07-30 blind run, which hit it before
it could ask a single API question:

```toml
[dependencies]
ambition = { path = "../path/to/ambition/crates/ambition" }

# ⚠ REQUIRED. Ambition builds against a fork of bevy_ggrs (a backported
# `GgrsFrameTiming` accessor). Cargo patch tables do NOT cross a workspace
# boundary, so you must repeat this one yourself — copy the current value from
# the engine's workspace-root Cargo.toml.
[patch.crates-io]
bevy_ggrs = { git = "https://github.com/Erotemic/bevy_ggrs", rev = "4d2eff2a89f00c127e17fd26dd3f25d3a1113fa2" }
```

Without it a fresh lockfile resolves `bevy_ggrs` from crates.io and the build
dies in `ambition_runtime` with `cannot find type GgrsFrameTiming in crate
bevy_ggrs` — an error with no visible connection to a patch table you have never
seen.

You do **not** need to declare `bevy` yourself for ordinary use: `ambition`
re-exports it (`ambition::bevy`). You *do* need it in your own manifest if you
`#[derive(Component)]` or `#[derive(Resource)]`, because Bevy's derive macros
resolve `::bevy_ecs` through the consuming crate's manifest and a re-export does
not satisfy that.

This is a real defect, not a documentation quirk: an engine another game can be
built on has to be an engine another game can *link*. It is recorded as the
top-ranked finding in
`docs/planning/engine/slice-evidence/blind-agent-runs/2026-07-30-slice-a-baseline.json`.

## Standing a game up

Four lines:

```rust
use ambition::app::prelude::*;

fn main() {
    PlatformerApp::windowed("My Game")
        .mount(MyModule::default())
        .run();
}
```

`PlatformerApp::headless()` is the same game with no display, where one
`App::update` is exactly one simulation tick. Both faces mount the same module
and install the same engine in the same order.

They are not yet fully interchangeable, though — see *Known gaps* below. The
visible face also prepares art, which currently requires content a minimal
module does not have.

For everything a row above says is not started, the engine is still composed by
hand. The worked example of both — the declarative host composition and the
hand-composed remainder — is `fixtures/external_consumer`, a complete tiny game
built from outside the workspace through the `ambition` umbrella alone.

## The compatibility promise

A game depends on **`ambition`** and nothing else from this workspace (plus
`bevy`, because derive macros resolve `::bevy_ecs` through the consumer's own
manifest).

The promise is made at that surface and nowhere else. Inner `ambition_*` crates
stay independently usable by engine developers and carry **no stability
promise** — if your imports name one, you are depending on our implementation
topology and we will move it.

That is enforced, not asked for: `scripts/check_absence_contracts.py` carries a
module allowlist over consumer code, with a frozen baseline that may only
shrink. Run it to see what the reference consumer still names and how often:

```bash
python3 scripts/check_absence_contracts.py
python3 scripts/check_absence_contracts.py --allowlist-open-count
```

Every module in that output is a leak this SDK has not closed yet. Eighteen at
the start of slice A.

## Known gaps, as of slice A

Measured by the 2026-07-30 blind run, not guessed. Listed here because a
document that hides its gaps sends readers into `crates/` with no warning, and
"which engine file did it open first" is what this SDK is scored on.

* **A minimal module cannot reach a WINDOWED host yet.** The visible face
  installs `PlatformerAssetsPlugin`, which requires a `CharacterCatalog` — so a
  module that boots headless can still panic windowed. Content is slice B, and
  until it lands there is no supported empty-content story. `CharacterCatalogPlugin`
  takes a raw RON string with no `Default`; the empty value is
  `(brain_presets: {}, action_set_presets: {}, characters: {})`.
* **There is no supported way to ask "did my game actually start?"** Four of the
  eight ordering rules the engine now owns used to fail silently, and while
  `try_build` refuses the ones it can see, there is no active-route or
  active-session read-model to smoke-test against. The blind run fell back to
  counting entities.
* **`docs/sdk/api-prototype.md` §3's example is illustrative, not runnable** —
  its identifiers are the fixture's. The runnable reference is
  `fixtures/external_consumer`.

Fixed since that run, and now covered by tests: a declared route that no mounted
capability registers is refused by `try_build` with the registered routes named
(it used to build clean and run empty), and `ModuleDraft::capability` no longer
requires `Clone` (which had excluded the engine's own plugins).
