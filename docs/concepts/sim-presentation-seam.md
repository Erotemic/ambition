---
id: sim-presentation-seam
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
implemented_by:
  - crates/ambition_runtime
  - crates/ambition_sim_view
  - crates/ambition_render
  - crates/ambition_vfx
  - crates/ambition_audio
  - crates/ambition_host
related_adrs:
  - docs/adr/0012-sim-presentation-split-and-events-refactor.md
  - docs/adr/0028-dialogue-presentation-is-provider-selected.md
---

# Simulation / presentation seam

Simulation owns authoritative facts and outcome-changing transitions.
Presentation consumes stable read models and typed effect intent, then performs
renderer/audio/UI side effects.

## Stable flow

```text
simulation owner system
    -> authoritative components/resources
    -> typed facts/messages
    -> ambition_sim_view or domain read model
    -> render/audio/UI adapter
```

Immutable authored world data may be read directly when doing so does not hide
observer-dependent or mutable truth.

## Invariants

- Headless composition can run every outcome-changing system without cameras,
  sprites, audio devices, windows, or menus.
- Presentation never mutates simulation to make an effect convenient.
- Effect messages identify semantic events, not renderer implementation details.
- Read models are derived/rebuildable and are not competing persistence truth.
- Provider-owned presentation plugs into public engine observation/effect seams.
- Dialogue runtime/view facts are reusable; the concrete overlay tree is selected
  by exactly one game/provider presenter plugin (ADR 0028), not a universal skin.
- Audio/VFX may be absent or degraded without changing authoritative outcomes.
- Simulation ordering lives in runtime/domain schedule sets; presentation timing
  must not become an implicit gameplay dependency.

## Placement test

A field belongs in simulation when changing it could alter collision, actions,
damage, inventory, progression, AI decisions, timing, or replay. A field belongs
in presentation when it only changes how already-resolved facts are displayed or
heard.

## Validation

```bash
./run_tests.sh -p ambition_sim_view
./run_tests.sh -p ambition_render
./run_tests.sh -k plugin_minimal_app
cargo run -p ambition_app --bin headless -- 120
```
