# Shared content library

The Ambition games select this library through their content pack's `libraries`
list. A game can select zero or more libraries. The engine does not require
this library. A library can supply art, animation, geometry, audio, and supported
script artifacts. The library is part of this repository, not a submodule.

`library.json` declares a stable ID, roots by role, a semantic asset catalog,
publication and provenance roots, producer policy, and static byte selection.
Import paths are relative to the content pack root. The current packs import
`../../ambition_content_library/library.json`.

`authored/` contains tracked data and recipes. `generated/` contains local
runtime and review products, including every quality tier and the immutable
publication store. Generated assets are ignored by Git. Do not commit them.
Renderer source files remain in their authoring tool repositories.

From the repository root:

```bash
scripts/setup/generated_content.sh
./scripts/regen/sprites.sh --plan --json
./scripts/regen/sprites.sh --check --json
./scripts/regen/sprites.sh --audit --json
./scripts/regen/sprites.sh --target hand_saint
```

Sprite commands accept `--library <manifest>`. `AMBITION_CONTENT_LIBRARY`
selects the manifest for other generation tools. A plan reads source definitions
without generated products. Exact packed page membership can require rendering;
use `--resolve-layout` to compute it without publication.

Use `content://ambition_shared/<logical-path>` for a qualified runtime reference.
A short path must have one owner. Two libraries can have the same logical path
when their references retain the library ID. Duplicate semantic role IDs are
refused. Game sources do not silently override library files.

Native builds read the selected files without a Rust rebuild. Static builds
embed selected metadata and bytes from the same manifest. Accepted reloads
advance files and metadata together. A session retains its admitted mechanical
generation. See [sprite publication](../../docs/tools/sprite-publication.md).

The producer list selects fonts, backgrounds, sprites (including VFX), quality
variants, music, and SFX. Developer setup runs that list. A missing toolchain or
required product fails setup. Use `--skip-assets` on developer setup to omit
content generation.

Loose runtime products live under `generated/runtime/full/`. Reduced products
live under `generated/runtime/half/`, `quarter/`, and `potato/`. Sprite bundles
also name the runtime role and quality tier inside each immutable bundle.
Review products and provenance live outside runtime roots.

A pack can read a schema source from a selected library:

```ron
(path: "audio/sfx_registry.ron", library: Some("ambition_shared"),
 schema: "sfx_registry", version: 1)
```

`authored/runtime/data/room_themes.json` defines alternate theme states and
ambient mote styles. Background image roles remain in `authored/catalog.json`.
A new theme key needs no engine enum entry. Room metadata must select its theme
through `visual_profile.parallax_theme`.
