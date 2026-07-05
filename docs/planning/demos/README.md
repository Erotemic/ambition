# The demo suite — doctrine

**Authored by fable, 2026-07-05.** The demos are the engine's executable
acceptance suite ([`../vision.md`](../vision.md) §4–5). Four are written in
stone — [Sanic](sanic.md), [Super Mary-O](super-mary-o.md),
[Super Smash Siblings](super-smash-siblings.md),
[Hollow Lite](hollow-lite.md) — with later matrix tiers gaining docs when
their tier opens. Priorities may shuffle; the designs do not.

## The shape (every demo, no exceptions)

```
demos/
  demo_<name>/
    <name>_content/   — ONE crate: worlds (own .ldtk), rosters/catalog rows,
                        movesets, rules plugin(s), mode/match state, HUD data
    <name>_app/       — the thin shell (~100 lines): foundation plugins +
                        PlatformerEnginePlugins + PlatformerHostPlugins +
                        <Name>ContentPlugin + <Name>RulesPlugin (global)
```

- **Standalone:** depends only on engine crates. `git log --stat` for the
  demo touches ZERO engine crates. Every needed core change files an
  `oracle-violation` in [`../tracks.md`](../tracks.md) and becomes engine
  work executed OUTSIDE the demo's commits.
- **Adversarial discipline:** the demo agent may not "quickly fix" the
  engine. The violation log is the product as much as the demo.
- **Headless-first:** every demo ships scripted reachability/win-path
  tests (complete the level / win a match via `SlotControls` headlessly)
  before any feel pass. Visuals draw blind and ship.

## The scoped game-mode pattern (ambition hosts every demo)

A demo's RULES are a plugin whose systems run under a **mode scope**, not
global app state:

- Engine seam (decomposition Phase D-C): `RoomMetadata.mode:
  Option<String>` + an `in_mode("<name>")` run-condition helper; the demo's
  rules systems attach with `.run_if(in_mode("sanic"))` when hosted, or
  unconditionally (`GlobalMode`) in the standalone app — same systems, two
  activation policies, chosen by the APP, not the rules crate.
- `ambition_app` depends on each `<name>_content`, mounts the demo's zone
  (its .ldtk world merges via the multi-world loader; a LoadingZone door
  from the sandbox), and tags the zone's rooms with the mode. Possess Sanic
  in the Hall, walk into the Sanic wing, and the demo IS running — same
  systems as standalone; only HUD chrome and the surrounding world differ.
- Mode state (score, stocks, timers) lives on mode-scoped entities/
  resources owned by the rules plugin, reset on zone entry (the
  RoomScopedEntity pattern generalizes: ModeScopedEntity).
- **This pattern is the composability forcing-function:** if a demo's
  rules can't scope to a zone, the design leaked global assumptions —
  fix the design, not the pattern.

## Executor notes

Demo work is [opus] by default (the engine tracks it depends on carry
their own grades); art draws blind per the standing rule; each demo doc
lists its engine dependencies — a demo agent finding a dependency unmet
STOPS demo work and files/executes the engine track instead (per its own
grade), never inlines engine changes into demo commits.
