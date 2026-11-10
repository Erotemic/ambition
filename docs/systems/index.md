# System docs index

These guides explain the current contracts between subsystems. Source owns exact
symbols and implementation details; planning documents track future work and
reviews record investigations. Preserve useful historical records without
presenting their findings as current architecture.

## Core flows

| Area | Guide |
| --- | --- |
| Input and control authority | [Input, control and UI](input-control-and-ui.md) |
| Characters and brains | [Actors and character content](actors-brains-and-character-content.md) |
| World and room loading | [LDtk world composition](ldtk-world-composition.md) |
| Save data and progression | [Persistence](persistence-settings-and-progression.md) |
| Headless runtime | [Headless simulation](headless-simulation.md) |
| Gameplay messages | [Gameplay effects](gameplay-effects.md) |
| Audio and visual effects | [Audio and VFX](audio-and-vfx.md) |
| Traces | [Gameplay trace recorder](gameplay-trace-recorder.md) |
| Testing | [Testing strategy](testing-strategy.md) |

## Presentation and tooling

| Area | Guide |
| --- | --- |
| Asset catalog and publication | [Asset manager](asset-manager.md) |
| Camera zones and room visual metadata | [Camera and visual profiles](camera-and-visual-profiles.md) |
| Observer reference frames | [Camera reference frames](camera-reference-frames.md) |
| Scenery | [Parallax and terrain skins](parallax-backgrounds.md), [Room looks](room-looks.md) |
| Display and rendering | [Display modes](display-modes.md) |
| Music and sound | [Music diagnostics](music-playback-diagnostics.md), [Underwater audio](audio-underwater.md) |
| Developer controls | [Hotkeys](developer-hotkeys.md), [Developer tools](developer-tools.md) |
| LDtk iteration | [Hot reload](ldtk-hot-reload.md) |

## Gameplay and mechanics

| Area | Guide |
| --- | --- |
| Collision and secondary physics | [Collision](collision-geometry-and-secondary-physics.md) |
| Blink and fast-fall | [Blink](blink-and-fastfall.md), [Motion policy](blink-motion-policy.md) |
| Portals | [Portals](portals.md) |
| Safe arrivals | [Transition validation](transition-spawn-validation.md) |
| NPC navigation | [NPC navigation](npc-navigation.md) |
| Bosses | [Boss profiles](boss-behavior-profiles.md), [Encounter architecture](boss-encounter-architecture.md) |
| Wearables | [Worn gear](worn-gear.md) |
| Factions | [Factions](factions.md) |

To locate current implementation symbols and test ownership, use
`python scripts/agent_query.py "<system>"`. Do not treat an archived plan or
review as a substitute for checking the current implementation.
