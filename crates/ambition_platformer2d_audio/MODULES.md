# `ambition_platformer2d_audio` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_audio** — The platformer's audio runtime: the Kira install and channels, the acoustic environment, and the music direction ([`music`]) that turns encounter, boss, room, conversation and radio state into a music intent.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`environment`](src/environment.rs) | ECS audio-environment layer. |
| [`music`](src/music/mod.rs) | Ambition-game music adapters over the `ambition_audio` music core. |
| [`plugin`](src/plugin.rs) | The Bevy plugin for the platformer's audio. |

_3 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
