# Archive

Archived docs are historical evidence. They are not current authority and should not be the first retrieval target for agents.

Use the archive only when investigating why a decision happened or recovering details from a landed/abandoned migration. If an archived note still affects current work, promote the durable rule into `docs/concepts/`, `docs/systems/`, `docs/recipes/`, `docs/planning/`, or an ADR.

## Layout

| Area | Meaning |
|---|---|
| `historical-roadmaps/` | Landed or abandoned implementation roadmaps kept when they still explain history. |
| `music-labs/` | Old generated-music transition and balance experiments. |
| `old-system-notes/` | Pre-KB system notes kept for historical context. |
| `port_notes/` | Historical porting notes. |
| `retired/` | Retired systems. |
| `superseded-adrs/` | ADRs replaced by newer ADRs. |
| `superseded-migrations/` | Migration notes kept only when no compact historical summary exists. |
| `superseded-systems/` | System docs superseded by current `docs/systems/` entries. |

Old session-state dumps, overlay readmes, autonomous handoff prompts, and completed migration plans are compacted or pruned because they are high-noise retrieval targets. Use git history for full originals.

Current docs start at `../README.md`.
