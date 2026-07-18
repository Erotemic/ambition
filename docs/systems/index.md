# System docs index

System docs describe current cross-crate behavior. They are deliberately concise:
exact file/symbol inventories belong in source, `MODULES.md`, and `.agent/`.
Migration ledgers, dated audits, and future designs belong in `archive/` or
`planning/`, not here.

## Core flows

| Area | Current doc |
|---|---|
| Input, control authority, action schemes, prompts, menus, touch | [`input-control-and-ui.md`](input-control-and-ui.md) |
| Actors, brains, catalog assembly, action execution | [`actors-brains-and-character-content.md`](actors-brains-and-character-content.md) |
| LDtk import, world lowering, room loading/commit | [`ldtk-world-composition.md`](ldtk-world-composition.md) |
| Persistence, settings, quests, progression | [`persistence-settings-and-progression.md`](persistence-settings-and-progression.md) |
| Headless runtime and harness | [`headless-simulation.md`](headless-simulation.md) |
| Testing | [`testing-strategy.md`](testing-strategy.md) |
| Audio and VFX | [`audio-and-vfx.md`](audio-and-vfx.md) |
| Gameplay effect/message seam | [`gameplay-effects.md`](gameplay-effects.md) |
| Trace recording/replay | [`gameplay-trace-recorder.md`](gameplay-trace-recorder.md) |

## Focused current systems

| Area | Current doc |
|---|---|
| Asset manager | [`asset-manager.md`](asset-manager.md) |
| Camera and room visual profiles | [`camera-and-visual-profiles.md`](camera-and-visual-profiles.md) |
| Developer hotkeys | [`developer-hotkeys.md`](developer-hotkeys.md) |
| Display modes | [`display-modes.md`](display-modes.md) |
| LDtk hot reload | [`ldtk-hot-reload.md`](ldtk-hot-reload.md) |
| Collision and secondary physics | [`collision-geometry-and-secondary-physics.md`](collision-geometry-and-secondary-physics.md) |
| Blink / fast-fall | [`blink-and-fastfall.md`](blink-and-fastfall.md), [`blink-motion-policy.md`](blink-motion-policy.md) |
| Portals | [`portals.md`](portals.md) |
| Transition spawn validation | [`transition-spawn-validation.md`](transition-spawn-validation.md) |
| Parallax | [`parallax-backgrounds.md`](parallax-backgrounds.md) |
| Boss profiles/encounters | [`boss-behavior-profiles.md`](boss-behavior-profiles.md), [`boss-encounter-architecture.md`](boss-encounter-architecture.md) |
| Factions | [`factions.md`](factions.md) |
| Developer tools | [`developer-tools.md`](developer-tools.md) |
| Underwater audio | [`audio-underwater.md`](audio-underwater.md) |
| AI generation contract | [`ai-generation-contract.md`](ai-generation-contract.md) |

Before trusting an exact path in a system doc, confirm it with:

```bash
python scripts/agent_query.py "<system>"
python scripts/agent_query.py crate <owner>
```
