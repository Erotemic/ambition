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

## Current implications for agents

- Treat LDtk as the world-composition source of truth; use LDtk tooling rather than hand-editing JSON.
- Author placement in LDtk; instantiate runtime effects/actors *from* the authored data rather than hardcoding positions in code.
- Keep authored data, runtime execution, and presentation separate so each composes — and can be removed — independently.
