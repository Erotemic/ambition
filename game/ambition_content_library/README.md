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

## Missing generated content

Missing generated content does not stop the game. On a fresh checkout,
`generated/` is absent until `scripts/setup/generated_content.sh` completes.
The game then runs with these results:

* An absent `published.json` is an empty selection. Each missing sheet shows
  as a placeholder.
* A pack source that a producer makes (the music registry) and that is absent
  is an `ungenerated-source` warning on the prepared pack. The pack compiles
  without it, and the game plays no music.
* A missing SFX bank is a logged load error. The bank cues are silent.
* The library prints ONE warning per process. The warning names the missing
  families, the store, and the query command.

An absent AUTHORED file (a file under `authored/`, or a file of the pack's own
tree) is an authoring bug. It stays an error. A build that embeds its content
(`static_content`) also stays strict.

To ask what is missing or stale, without a build and with no toolchain:

```bash
python3 scripts/content_status.py                   # each family and each declared source
python3 scripts/content_status.py --family music --json
python3 scripts/content_status.py --require-complete   # exit 1 when something is missing or stale
```

The script exits 0 unless `--require-complete` is given. It reads
`library.json`, the selection, each selected bundle manifest, and the library
sources of each game pack. Stale means that an input that the bundle recipe
records changed after the bundle was accepted. The exact producer check is
`python3 scripts/generate_library_content.py --check`, which needs the
renderer toolchains.

## Commands

From the repository root:

```bash
python3 scripts/content_status.py
scripts/setup/generated_content.sh
./scripts/regen/sprites.sh --plan --json
./scripts/regen/sprites.sh --check --json
python3 scripts/generate_library_content.py --check
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
live under `generated/runtime/half/`, `quarter/`, and `potato/`. All generated bundles
name the runtime role and quality tier inside each immutable bundle.
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
