# The demo suite — doctrine

**Authored by fable, 2026-07-05.** The demos are the engine's executable
acceptance suite ([`../vision.md`](../vision.md) §4–5). Four are written in
stone — [Sanic](sanic.md), [Super Mary-O](super-mary-o.md),
[Super Smash Siblings](super-smash-siblings.md),
[Hollow Lite](hollow-lite.md) — with later matrix tiers gaining docs when
their tier opens. **Parody names are policy** (Q28, Jon 2026-07-06): every
demo name, character, and asset is a parody-original — homage in grammar,
never a copy. Each demo doc carries a **Consumes (by role) / Owns** section:
"consumes" lists engine crates by their [role handles]
([`../engine/architecture.md`](../engine/architecture.md) §2); "owns" is what
the demo builds for itself. If work appears that fits neither list, stop —
it's either an oracle-violation (engine work, file it in tracks.md) or scope
drift. Priorities may shuffle; the designs do not.

## The shape (every demo, no exceptions)

```
game/
  ambition_demo_<name>/      — ONE content crate: worlds (own .ldtk), rosters/catalog rows,
                               movesets, rules plugin(s), mode/match state, HUD data
  ambition_demo_<name>_app/  — the thin shell (~100 lines): foundation plugins +
                               PlatformerEnginePlugins + PlatformerHostPlugins +
                               <Name>DemoContentPlugin + <Name>RulesPlugin (global)
```

**The executable reference:** `crates/ambition_host/tests/
demo_shell_smoke.rs` (E5 step 6) IS this shape, live and gate-enforced —
foundation + engine group + host group + a fixture content plugin that
(1) installs its one-character catalog RON, (2) inserts its
`RoomSet`/`RoomGeometry`/`ActiveRoomMetadata`, and (3) runs the engine's
`session::setup::simulation_world` in a Startup system labeled
`SimulationSetupSet` (which spawns the player box the host's input attach
finds). Start every `ambition_demo_<name>_app` by copying that fixture.

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
- `ambition_app` depends on each `ambition_demo_<name>` content crate, mounts the demo's zone
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
