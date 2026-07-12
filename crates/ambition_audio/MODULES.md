# `ambition_audio` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_audio** — Content-free audio data/runtime layer.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`bank_asset`](src/bank_asset.rs) | Bevy `Asset` + `AssetLoader` for the packed SFX bank. |
| [`catalog`](src/catalog.rs) | App-local authored-audio catalogs contributed by experience providers. |
| [`library`](src/library.rs) | Authored-audio playback library: typed SFX cue table, lazily-loaded pre-rendered music tracks, the music/SFX Kira channels, and the track-switch/radio/default-start helpers. |
| [`mix`](src/mix.rs) | Host-supplied mix levels. |
| [`music`](src/music/mod.rs) | Adaptive music core: cue catalog, layered Kira channels, the director (simple + adaptive cue playback), and its tuning. |
| [`render`](src/render.rs) | SFX-bank byte → Kira asset adapter and lazy handle cache. |
| [`spec`](src/spec.rs) | Audio data schema: the authored (RON) shapes for procedural SFX and pre-rendered music. |
| [`web_unlock`](src/web_unlock.rs) | Browser AudioContext unlock detection + ECS readiness flag. |

_8 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
