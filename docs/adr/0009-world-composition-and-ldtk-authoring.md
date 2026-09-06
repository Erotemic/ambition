# ADR 0009: LDtk is the world-composition authoring source

## Status

Accepted; updated 2026-06-13.

## Context

Ambition needs visual world authoring, editor roundtrip safety, entity metadata, platform packaging, and runtime ECS projection. LDtk is the canonical authoring source for world composition. A large part of the game world — collision, loading zones, room/world layout, hazards, spawners, actors, and initial placement — lives in the LDtk file *by design*; authored placement is a hard requirement, not a convenience.

## Decision

Use LDtk as the canonical authoring source for world/level composition.

Rules:

- Do not hand-edit `sandbox.ldtk` JSON for semantic changes. Use `python -m ambition_ldtk_tools` for authoring/repair/validation/roundtrip workflows.
- Treat LDtk IntGrid/entity semantics as runtime data that feeds Bevy ECS and reusable engine types.
- **LDtk authors *where / which*; the runtime executes *how*.** The level loader is an emitter: at load it instantiates runtime ECS actors/effects from the authored entities (an authored hazard becomes a runtime damage-box at the authored position). Authored placement and runtime-spawned content (e.g. an enemy spawning a hazard mid-fight) converge on the *same* primitives — the only difference is who emits and where, never how.
- Keep authored world data, runtime execution, and presentation **separate and independently removable**. A missing execution or presentation plugin should degrade one feature (authored spikes drawn but harmless; a tell's windup animation gone but the attack still lands), not break world loading.
- Keep the runtime projection testable: validators and tests should catch missing graph links, bad loading zones, collision/category mismatches, and spawn repairs.
- Preserve static/embedded map paths needed by web and Android.

RON is for tuning, save/settings, generated-audio specs, character/boss data, and other non-world data — never world composition.

## Consequences

World docs should point to `docs/systems/ldtk-world-composition.md`, `docs/recipes/ldtk-authoring.md`, `docs/tools/index.md`, and the LDtk-related tests/tools. Docs that still describe RON-based world/level authoring are stale and must be archived or rewritten — log stragglers in `dev/journals/code_smells.md`.

## Refined by ADR 0021: LDtk is PREFERRED, and required where it matters most

This ADR predates the backend-agnostic world IR. ADR 0021 made `ambition_platformer2d_world`
the model and LDtk one backend that converts into it, so a generated / RON /
programmatic source is not a special case. That does **not** demote LDtk. It
remains the preferred way to make a room and the one that gets the investment:
when a level needs a concept the authoring path does not have yet, the answer is
to **add it to LDtk and the tooling**, not to route around them in Rust.

Where each is acceptable (Jon, 2026-07-25):

| Surface | Room authoring |
|---|---|
| Demo crates (`game/ambition_demo_*`) | Rust-constructed `RoomSpec` is **fine**. Mary-O's 1-1/1-2 are, and that is not a violation. |
| The Ambition sandbox | **Prefer LDtk.** Programmatic rooms are tolerated, not encouraged; do not add more. |
| The Ambition game proper (when we get to it) | **LDtk, required.** |

Whichever backend, the IR is not optional: lower authored placement records
rather than opening a parallel spawn channel.

## Current implications for agents

- Treat LDtk as the world-composition source of truth for the sandbox and the game; use LDtk tooling rather than hand-editing JSON. A demo may build the IR in Rust.
- Missing an authoring concept is a reason to EXTEND LDtk + `ambition_ldtk_tools`, not a reason to hand-build the room in code.
- Author placement in the world IR and instantiate runtime effects/actors *from* the authored data rather than through a spawn channel that skips it.
- Keep authored data, runtime execution, and presentation separate so each composes — and can be removed — independently.
