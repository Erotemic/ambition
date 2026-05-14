# Game modes and data sharing

This is a story/gameplay brainstorm note, not a locked implementation plan.

## Core idea

At the start of a run or campaign branch, the player may be asked whether they want to share their data.

Sharing data has an upside:

- future generations/runs can build on routes, discoveries, ability traces, or world-state improvements,
- generated content can become more personalized and responsive,
- collaborators may learn from the player's actions,
- persistent upgrades or world repairs may become available sooner.

Sharing data also has a cost:

- enemies or hostile institutions may gain access to some of the player's abilities,
- boss patterns may adapt to repeated habits,
- unsafe optimization systems may learn player routes,
- the world may become more efficient but less humane.

This turns the AI/data theme into mechanics instead of only dialogue.

## Candidate modes

```text
Semi-linear metroidvania
  Curated world, stable progression, authored story, deterministic unlocks.

Pure platformer
  Movement challenge rooms with minimal story overhead.

Pure roguelike
  Generated run structure, reset-on-failure arc, strong data-sharing/metaprogression theme.

Hybrid
  Persistent metroidvania hub plus generated excursions whose results feed back into the world.
```

## Design cautions

- Do not add this before the core movement and first vertical slice feel good.
- Do not make data sharing a simple good/evil switch.
- Do not let enemy adaptation become unfair or opaque.
- Always show enough information that the player can form a strategy.
- Keep a non-adaptive mode available for players who want fixed challenge mastery.

## Mechanical hooks

Possible things that can persist between runs:

- discovered room graph fragments,
- theorem/ability research progress,
- NPC trust or collaborator state,
- route heatmaps,
- player combat habits,
- generated boss schedules,
- world repair/corruption levels,
- unlocked challenge rooms.

Possible enemy learning hooks:

- enemies gain one player ability in later runs,
- bosses punish repeated safe spots,
- turrets learn common dash timing,
- conduit drones replay old player traces,
- hostile systems close routes the player overuses.

## Narrative framing

The opt-in/out choice should not be framed as a fake software consent dialog unless intentionally satirical. It can be an in-world decision about memory, inheritance, collaboration, surveillance, or institutional capture.
