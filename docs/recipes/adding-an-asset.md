---
status: current
last_verified: 2026-07-19
related_docs:
  - docs/concepts/content-and-provider-boundaries.md
  - docs/recipes/adding-a-character.md
---

# Add a new asset file

Answers the question every content task asks and no code comment answers in one
place: *I have a new png / ogg / ron — which directory does it go in, what path
string loads it, and will git keep it?*

## The two asset roots

There are two, and picking the wrong one means `asset_server.load` silently
resolves nothing.

| Root | Directory (dev checkout) | Load path | Holds |
| --- | --- | --- | --- |
| Default (no scheme) | `crates/ambition_actors/assets/` | `"sprites/foo.png"` | Shared/generated engine assets: sprite sheets, fonts, audio |
| `game://` | `game/ambition_content/assets/` | `"game://worlds/foo.ldtk"` | Provider-owned named content: worlds, dialogue, data, per-game art |

The default root is the `AssetPlugin { file_path }` the app sets from
`desktop_asset_root()` → `actors_desktop_asset_root()`
(`crates/ambition_asset_manager/src/sandbox_assets/mod.rs:337`). The `game://`
source is registered before `DefaultPlugins` in
`game/ambition_app/src/app/cli.rs`, built by `game_asset_source_builder()`.

**`game://` falls back to the default root on a miss.** Its reader tries the
content crate first, then the shared generated tree
(`ProviderGameAssetReader`, `cli.rs:52`). That is why LDtk worlds under
`game://` can name relative `sprites/...` paths that only exist in
`ambition_actors/assets` — no copying, no misleading `Path not found`.

Both roots collapse to a plain `assets` directory when `BEVY_ASSET_ROOT` is set
and in shipped builds, so never hardcode an absolute path.

Rule of thumb: **Ambition's own named content → `game://`. Anything the engine
or another game could reuse → default root.**

## Will git keep it?

Usually **no**, and that is intended. The root `.gitignore` excludes binaries by
pattern — `*.png`, `*.ogg`, `*.wav`, `*.mid`, `*.flac`, `*.mp3` — plus whole
payload directories (`assets/concept_art`, `assets/backgrounds`,
`assets/vanity_card`, `crates/ambition_actors/assets/fonts/`, …).

So a new png needs **no gitignore edit**; it is already ignored. Confirm rather
than assume:

```bash
git check-ignore -v <path>          # prints the rule, or exits 1 if trackable
```

Payloads are hydrated out of band and are PRESENT on Jon's disk; some are
IPFS-pinned with a tracked `.ipfs` sidecar beside them. Git-ignored is not
missing — `ls` before concluding otherwise, and never add fetch machinery to a
feature.

### Commit the description, ignore the payload

When a feature needs to know *what* files should exist, put that in a committed
text manifest next to the ignored payload. Two things fall out:

- the code composing the feature works on any checkout;
- a checkout without the payload can name precisely what is absent
  (`missing frame 3 of 9`) instead of rendering nothing.

Worked example: `game/ambition_content/assets/data/vanity_card.ron` (committed)
describes `game/ambition_content/assets/vanity_card/*.png` (ignored), consumed
by `game/ambition_content/src/vanity_card.rs`.

## Generated assets

If a tool produces the asset, the tool is the source of truth and must run on a
fresh clone from committed inputs. Emit into the asset root the consumer loads
from, and keep authored inputs (configs, targets) tracked. Existing generators:
`tools/ambition_sprite2d_renderer`, `tools/ambition_music_renderer`,
`tools/vanity_card_prep`.

## Loading it

```rust
// Default root
let handle: Handle<Image> = asset_server.load("sprites/foo.png");
// Provider content
let world: Handle<LdtkProject> = asset_server.load("game://worlds/foo.ldtk");
```

Do **not** route an asset through `AssetLocation::IpfsGateway` because its
directory happens to be IPFS-tracked. `bevy_asset_path()` returns `None` for
that variant (`crates/ambition_asset_manager/src/location.rs:109`) and the
`IpfsGatewayPlaceholder` profile is not selected by any build or CLI flag, so
the asset would never load. IPFS is distribution, not runtime resolution.

## Degrade when it is absent

Another checkout may legitimately lack the payload, so a missing file must not
take down the feature or change its timing. Detect with the established pattern:

```rust
asset_server
    .get_load_state(&handle)
    .is_some_and(|state| state.is_failed())
```

Prior art: `game/ambition_app/src/app/startup_loading.rs:435`,
`game/ambition_app/src/app/world_flow/room_transition_assets.rs:328`, and the
per-frame notice in `crates/ambition_game_shell/src/basic_presentation.rs`.

## Validate

```bash
git check-ignore -v <path>                  # payload ignored, manifest not
cargo check -p ambition_app                 # the compile gate
cargo test -p ambition_app --test app_it -- <module>
```

If a Python tool writes a RON manifest a Rust crate reads, add a Rust parse test
over the committed file — Python RON writers drift looser than the `ron` crate.
