# `ambition_persistence` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_persistence** — Saved game, quest, and settings shapes for Ambition.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`host`](src/host/mod.rs) | Host-facing settings vocabulary. |
| [`quest`](src/quest/mod.rs) | Quest data types and progression rules. |
| [`save`](src/save.rs) | Sandbox save game I/O + autosave. |
| [`save_data`](src/save_data.rs) | Pure save-game data shapes (`SandboxSaveData`, `PersistedEncounter`, `PersistedSwitch`, ability/quest flags) — the vocabulary the save format is built from. |
| [`settings`](src/settings/mod.rs) | User-facing persisted settings data. |

_5 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
