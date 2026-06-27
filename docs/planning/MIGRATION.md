# Migration record (old `docs/planning/` → this rewrite)

How every doc in the old tree was resolved. All rows are cleared — the old directory
was deleted and this one renamed to `docs/planning/`. Kept as the provenance record of
the consolidation.

## Engine

| Old doc | → Destination |
|---|---|
| `fighter-capability-and-motor-unification.md` | `engine/unified-actors.md` (primary source) |
| `npc-enemy-unification.md` | `engine/unified-actors.md` (relational hostility, no-NPC, in-place provoke) |
| `non-player-centric-actor-unification.md` | `engine/unified-actors.md` (shared spine / floating mover / emergent platform riding) |
| `locomotion-velocity-target-split.md` | `engine/unified-actors.md` (the motion vocabulary) |
| `universal-brain-interface.md` | `engine/unified-actors.md` (two-port, brain backends) |
| `restructuring-blueprint.md` | `engine/architecture.md` (crate layering, the keystone collapse, the reusability oracle, plugin shape) |
| `boss-entity-local-refactor.md` | `engine/boss-system.md` |
| `sprite-renderer-refactor.md` | `engine/sprite-renderer.md` |
| *(new)* | `engine/headless-verification.md` |

## Game

| Old doc | → Destination |
|---|---|
| `story-gameplay-progression-draft.md` | `game/vision.md` |
| `gameplay-idea-index.md` | `game/vision.md` (the kept backlog; faction bloat dropped) |
| `perfect-cellular-automaton-encounter.md` | `game/bosses.md` |
| `character-barks-and-hall-dialogue.md` | `game/characters.md` |
| `hall-of-characters-and-character-catalog.md` | `game/characters.md` (merged) |

## Cross-cutting

| Old doc | → Destination |
|---|---|
| `index.md` | `README.md` (replaced) |
| *(new)* | `roadmap.md` |

## Dropped (history / stale) and folded review items

- `monolith-next-batch.md`, `unified_tabbed_menu.md` — **dropped** (completed work, history).
- `tech-debt-log.md` — **dropped** (in-flight bug/content TODOs, no durable rules; git history holds the closed items).
- `android-power.md` — **dropped** (placeholder; recipes live in the platform docs).
- `rename-sweep-pain-points.md` — **dropped** (post-mortem of a finished refactor).
- `path-forward.md` — **dropped**, its priorities folded into `roadmap.md` (standing practices).
- `other-repo-related-work-survey.md` — **dropped**, its dependency rubric folded into `roadmap.md` (evaluate ecosystem crates before custom; candidate list).
- `refactor-candidates.md` — **dropped**, the live "delete compat re-exports" rule is already the architecture stance ("delete, don't bridge").

## Drop rule applied throughout

Carried the live decisions and the crystal-clear intent; cut the dated progress logs,
the slice-by-slice `S0/S1…` status, the already-shipped narration, and any wording
contradicting `README.md`. A planning doc is a plan, not a changelog.
