# Events refactor plan — compact archive

**Status:** superseded. Gameplay effects now use focused Bevy messages/events rather than the old broad bus shape.

Durable lesson: prefer typed messages with narrow consumers over global catch-all resources. When a behavior crosses subsystem boundaries, name the semantic event and keep payload ownership clear.

Current references:

- `docs/concepts/sim-presentation-seam.md`
- `docs/systems/gameplay-trace-recorder.md`
- `crates/ambition_sandbox/src/dev/trace/`

Use git history for the original roadmap.
