# `ambition_content_schemas` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_content_schemas** — The schemas Ambition's own pack names beside the engine's.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`flag_chains`](src/flag_chains.rs) | The `flag_chains` schema: save flags that set other save flags, as data in Ambition's pack (`assets/data/flag_chains.ron`). |
| [`rules`](src/rules.rs) | The `ambition_rules` schema: Ambition's own tuning, as data in its pack (`assets/data/ambition_rules.ron`). |
| [`text`](src/text.rs) | The lines Ambition shows, as data in its pack (`data/ambition_text.ron`): its launcher row and the words of its dialogue box. |
| [`yarn_dialogue`](src/yarn_dialogue.rs) | The `yarn_dialogue` schema: each Yarn file of the game's dialogue is a source of the pack (`(path: "dialogue/sandbox/intro.yarn", schema: "yarn_dialogue", version: 1)`). |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
