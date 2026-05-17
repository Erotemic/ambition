# System docs index

System docs describe current subsystem behavior. Superseded migrations and one-off patch notes belong in `docs/archive/`.

## Core architecture

| Area | Doc |
|---|---|
| Engine/module structure | [`engine-architecture.md`](engine-architecture.md), [`code-structure.md`](code-structure.md), [`core-and-bevy-boundary.md`](core-and-bevy-boundary.md) |
| Sim/presentation and messages | [`gameplay-effects.md`](gameplay-effects.md), [`gameplay-trace-recorder.md`](gameplay-trace-recorder.md), [`headless-simulation.md`](headless-simulation.md) |
| Testing | [`testing-strategy.md`](testing-strategy.md) |
| Dev tools | [`developer-tools.md`](developer-tools.md) |

## Gameplay and world

| Area | Doc |
|---|---|
| LDtk/world composition | [`ldtk-world-composition.md`](ldtk-world-composition.md), [`ldtk-hot-reload.md`](ldtk-hot-reload.md), [`transition-spawn-validation.md`](transition-spawn-validation.md) |
| Movement/collision | [`blink-and-fastfall.md`](blink-and-fastfall.md), [`blink-motion-policy.md`](blink-motion-policy.md), [`moving-platforms.md`](moving-platforms.md), [`enemy-collision.md`](enemy-collision.md) |
| Abilities/combat/actors | [`ability-system.md`](ability-system.md), [`ability-subset.md`](ability-subset.md), [`factions.md`](factions.md), [`boss-behavior-profiles.md`](boss-behavior-profiles.md), [`boss-encounter-architecture.md`](boss-encounter-architecture.md) |
| UI/input/settings | [`input-model.md`](input-model.md), [`mobile-touch-controls.md`](mobile-touch-controls.md), [`menu-navigation.md`](menu-navigation.md), [`game-mode-pause.md`](game-mode-pause.md), [`settings-system.md`](settings-system.md), [`pause-menu-settings.md`](pause-menu-settings.md), [`save-and-settings.md`](save-and-settings.md) |
| Presentation/assets/audio | [`asset-manager.md`](asset-manager.md), [`parallax-backgrounds.md`](parallax-backgrounds.md), [`camera-and-visual-profiles.md`](camera-and-visual-profiles.md), [`audio-particles.md`](audio-particles.md), [`audio-underwater.md`](audio-underwater.md), [`display-modes.md`](display-modes.md) |
| Experimental/future foundations | [`avian2d-physics-foundation.md`](avian2d-physics-foundation.md), [`parry2d-geometry.md`](parry2d-geometry.md), [`time-reference-platform.md`](time-reference-platform.md), [`two-clock-simulation.md`](two-clock-simulation.md), [`progression-systems.md`](progression-systems.md), [`ai-generation-contract.md`](ai-generation-contract.md) |

If a doc reads like "migration plan", "patch skeleton", or "landed roadmap", archive it instead of listing it here.
