# Archived: 0002-engine-may-be-bevy-native.md

Superseded by `docs/adr/0002-engine-must-be-bevy-native.md`.

Original path: `docs/adr/0002-engine-may-be-bevy-native.md`

---

# ADR 0002: The engine may be Bevy-native

## Status

Accepted.

## Context

Earlier notes tried to keep `ambition_engine` backend-neutral. That became too restrictive once the project adopted Bevy 0.18, Bevy math types, Leafwing input boundaries, inspector tooling, Bevy states, and Bevy-friendly state machines.

## Decision

`ambition_engine` is a reusable mechanics crate, not a fully backend-neutral library. It may depend on Bevy and Bevy-adjacent crates when they provide battle-tested primitives or make reusable mechanics clearer.

The sandbox/story crates should still own presentation, composition, RON content, visual style, debug UI, and app-shell concerns.

## Consequences

Engine code can use Bevy/glam math directly and can expose Bevy-friendly components/plugins. The important boundary is no longer “no Bevy”; it is “engine owns reusable mechanics, sandbox owns presentation and experiments.”
