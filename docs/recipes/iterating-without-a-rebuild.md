---
status: current
last_verified: 2026-11-10
---

# Iterating without rebuilding the game

A development build can load many authored changes from disk. Use the owning
content or tool pipeline; don't rebuild the engine for an edit that only changes
a content file.

## Authored content packs

`game/ambition_content/assets/pack.ron` declares Ambition's pack sources.
`game/ambition_content/src/content_watch.rs` polls the manifest and the local
source paths it declares every 20 update frames in non-`static_content` builds.
When a source changes, it compiles a candidate pack and asks the active route
to prepare and publish a new generation. A changed manifest also updates the
watch list.

This includes local pack entries such as movesets, encounters, character data,
items, and **`data/npc_lives.ron`**. The NPC-life publisher installs an updated
`LifeBook`; active lives use the new domain on their next planning decision.

**Detection, preparation, and publication are distinct.** A valid file save is
not proof that the current session adopted it. A failed compilation, an
unsupported content change, or an externally owned rollback timeline can
refuse the reload without replacing live content. Check the reload log and the
result in gameplay. Editing content while no route can prepare it may leave a
pending change for the next available route.

**Library-backed files need special care.** The pack compiler resolves a
source's declared `library`, but the current watcher's timestamp checks use
`root.join(path)` for each declaration. That is not the resolved path in an
external content library. Do not rely on edits to, for example, the shared
music/SFX registry being detected automatically. Restart or explicitly
reprepare the content through a supported route, and verify the changed result.

A standalone demo has its own authored pack and composition; don't assume
Ambition's watcher automatically watches every other game's source tree.

## Other edit paths

| Edited material | Development iteration path |
| --- | --- |
| Yarn dialogue | Its own dialogue-project watcher recompiles the project; an open conversation may restart its current node |
| Movement/feel defaults (`platformer_defaults.ron`) | The developer tuning-edit path publishes supported fields; starting abilities may require restarting the session |
| LDtk world | Use the editor, then `F11` to validate/apply, or `F12` to toggle auto-apply; see [LDtk hot reload](../systems/ldtk-hot-reload.md) |
| Generated art/audio | Run the relevant publication or bank-generation tool; writing a registry entry does not regenerate its already packed media |
| Rust gameplay/engine code | Recompile the affected crate/application, unless it is an independently loaded extension module |

## Procedural Rust modules

The development module road compiles a small `.wasm` without relinking the
engine, when the `wasm_modules` runtime capability is enabled. Once the module
build is authorized for the assigned checkout, a watch loop can rebuild it:

```bash
scripts/regen/build_extension_modules.sh --watch
```

Run the host with the resulting module directory:

```bash
AMBITION_EXTENSION_MODULES=target/extension-modules/wasm32-unknown-unknown/release \
    cargo run -p ambition_app
```

For Mary-O's module crate, use:

```bash
scripts/regen/build_extension_modules.sh --watch ambition_demo_mary_o_modules
AMBITION_EXTENSION_MODULES=target/extension-modules/wasm32-unknown-unknown/release/ambition_demo_mary_o_modules.wasm \
    cargo run -p ambition_demo_mary_o_app
```

The script rebuilds on source changes; the host watches the resulting module
file. A failed rebuild leaves the previous good module running. See
[writing a procedural module](writing-a-procedural-module.md) for the authoring
contract. Coordinate builds with other agents in the assigned checkout.

## Checking an edit

Prefer the actual observed gameplay result and the relevant reload diagnostic.
For a malformed pack or missing source, use
[validating a content pack](validating-a-content-pack.md). Static distribution
builds embed content and don't use these runtime file-watch paths. Historical
per-machine build times are not useful predictions for a different checkout.
