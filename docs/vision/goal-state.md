# Goal state

Ambition should become a sharp-feeling 2D metroidvania/platformer whose systems are inspectable, data-driven, and expressive enough to support unusual mathematical/world rules without losing moment-to-moment feel.

## Product direction

The first real milestone is a miniature but coherent vertical slice:

- one hub,
- a few connected LDtk-authored areas,
- movement/combat that feels good as raw shapes,
- one meaningful ability unlock,
- one encounter/boss-style challenge,
- dialogue/commercial/story hooks,
- generated or tool-authored audio/visual assets,
- reliable builds on desktop, web, Android/mobile touch, controller, and Steam Deck.

## Technical direction

- Bevy-native and ECS-first.
- LDtk owns world composition.
- Reusable mechanics stay in engine crates.
- Sandbox/game crates adapt authored/generated data into Bevy runtime state.
- Generated assets are reproducible and routed through asset-manager/platform profiles.
- Headless/minimal tests protect movement, collision, data projection, and platform-sensitive seams.

## Design direction

Ambition is allowed to be weird: mathematical spaces, time/control mechanics, generated art/audio, AI agency, enemy learning, and story systems are all welcome. The docs should keep those ambitions grounded by distinguishing:

- **brainstorm:** idea is alive but not binding;
- **vision:** desired direction;
- **concept:** durable vocabulary/invariant;
- **ADR:** accepted architectural decision;
- **system/recipe:** current implementation/procedure;
- **archive:** historical evidence.

## Non-goals right now

- Do not build a huge world before the movement toy is excellent.
- Do not resurrect RON-based world authoring as the primary level workflow.
- Do not add platform-specific hacks that break other active targets.
- Do not make AGENTS.md or current-state docs large context dumps.
