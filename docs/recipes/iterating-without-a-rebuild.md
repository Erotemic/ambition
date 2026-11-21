---
status: current
last_verified: 2026-11-14
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

## Module code in Rhai (no build at all)

With the `script_modules` runtime capability (on in `ambition_app`), the same
variable may name `.rhai` files. A script is the code of a module the game
already declares: its stem names the module (`sentry.rhai`, or
`ambition.sentry.rhai` when two providers share a key), and the module's
entries, ports, schemas and limits stay the game's declaration. An entry the
script defines (a function with the entry's name and no parameters) runs from
the script; any other entry runs the native code.

```bash
AMBITION_EXTENSION_MODULES=game/ambition_content_modules/rhai/sentry.rhai ./run_game.sh --reuse
```

Edit and save the file while the game runs: the change is proposed and
published through the same mechanical-edit road as a rebuilt `.wasm` file
(a turret deployed after the publication runs the edit). A script that does
not compile stops the run at startup, and leaves the previous code running
when it is a reload; the log names the file, line and column.

A script can also bring a module the game does not have: it defines
`fn declare()`, returning the module's declaration (its provider and key,
its schemas, and each entry's phase, trigger, records and request ports,
ports named as scripts name them), and a function for each entry. The
format and its defaults: `crates/ambition_extension_rhai/src/declare.rs`;
an example, a tally kept on each sentry turret:
`a_module_a_script_declares_runs_with_its_own_record` in
`game/ambition_app/tests/a_script_module_runs_without_a_build.rs`.
A field's kind is a scalar (`bool`, `u32`, `i32`, `u64`, `f32`, `vec2`), a
reference to a body or an item by its identity (`body`, `item`),
`opt <kind>`, or `seq <max length> <kind>`. A module that steers a room's
group together (`group_think`) keeps its record on the room. An example
keeps the flock's members and goal by reference:
`a_room_record_keeps_its_flock_by_reference` in
`game/ambition_app/tests/the_primeval_trail.rs`.

### Room story scripts

The `story_tick` trigger selects a room by its definition id. Each live
instance gets one invocation in `actor_think`, with its own room root as
the scope. Body-attached records on that scope rewind with the room.

A room's setup belongs in `story_began` instead: the same trigger and scope,
invoked once when an instance first runs. A room entered again is a new
instance and begins again, and a rewind across the first tick runs the setup
again with the state it rewound. An entry that runs every tick only to
return after its first one costs a call every tick (about 0.45 ms in a debug
build; the [bridge warden's yard](../../examples/projects/bridge_warden/README.md)
measured it before it moved to `story_began`).

The trigger reports the room's bodies, named LDtk labels, true save flags,
and whether a conversation or cutscene holds the room. Positions use room
pixels, with +Y down. Bodies are sorted by simulation identity. A recent
strike comes from the shared hit resolver; a practice target can report a
strike without losing health.

A script can submit `story_command` with a `line` from the authored
command catalog, or `play_cutscene` with a library `id`. Both requests keep
the invocation's live room. The command catalog checks each line before
dispatch. An unknown command or cutscene is refused and reported in the
log, or, with a `reply`, in an [action result](../sdk/action-results.md);
a cutscene asked with a reply is also answered when it starts and ends.
These ports do not give the script direct ECS access.

`actor.form_up <character> <place> <spacing> <brain>` sends the bodies to
separate points from a named place, in simulation identity order. Spacing
uses horizontal pixels: positive goes right, negative goes left. It uses
the same visits and movement as `actor.walk_to`. The workshop uses it to
keep the arriving raiders apart.

Ambition's optional workshop lesson is
`game/ambition_content/rhai/intro_training.rhai`. Yarn offers the lesson;
Rhai waits for the landing and strike; `data/cutscenes/intro.ron` supplies
the feedback. The landing point is an LDtk label placed from the same
generator values as the block. To edit the script on a built host:

```bash
AMBITION_EXTENSION_MODULES=game/ambition_content/rhai/intro_training.rhai ./run_game.sh --reuse
```

The normal module reload checks still apply. A build embeds the shipped
script for targets that have no source tree.

A **project** names its scripts (and optionally its own content pack, start
room and starting bag, `starting_items: Some([("bomb", 3)])`) in a
`project.ron`, so a run needs no variable:

```bash
./target/debug/ambition_game_bin --project examples/projects/sentry_drill
```

The example and the manifest's fields: `examples/projects/sentry_drill/README.md`.

What a script sees and may do (the invocation as `this`: `tick`, `dt`,
`trigger`, `state.<schema>`, `requests`), and the guarantees the engine is
configured for (exact `f32` arithmetic, no hidden state, an operation
budget), are in `crates/ambition_extension_rhai/src/lib.rs`. The ports a
script may name, with every field, its kind, its default and its meaning,
are in the generated [script port reference](../sdk/script-ports.md); a
request written as `request("projectile", #{ .. })` is checked where it is
made, so a misspelt field or a wrong kind names the script's line.

## Camera policies

The native default requires no script. To edit the policy on a built host, load
the camera-only Rhai example:

```bash
AMBITION_EXTENSION_MODULES=crates/ambition_world_port/examples/ambition.camera.rhai ./run_game.sh --reuse
```

The example declares its own module and view record. It selects exclusive
regions from physical body positions and uses the native composition solver.
Edit `response_hz`, or add `target_y` to its `camera_intent` request, then save.
The running view takes the change without replacing gameplay bodies. This
example supplies an editable policy; it does not implement the native policy's
spatial blends or swept thin gates.

The same default native Rust policy can run as an independent Wasm artifact:

```bash
cargo build -p ambition_world_port --example camera_policy --release --target wasm32-unknown-unknown
AMBITION_EXTENSION_MODULES=target/wasm32-unknown-unknown/release/examples/camera_policy.wasm ./run_game.sh --reuse
```

Both artifacts replace `ambition::camera/follow`. A level can instead select
a different admitted entry with its `camera_policy` field. Keep camera modules
separate from simulation modules to retain camera-only reload. Wasm artifacts
are generated output and stay outside Git.

Camera entries run once per view on the presentation clock. They read
`this.frame` and `this.dt`; `this.tick` is absent. View records are transient
presentation memory, not save flags. A code edit resets those records. A
fault retains the last valid camera intent without committing record writes.
See [camera compositions](../systems/camera-and-visual-profiles.md) for
activation, bounds, transitions, visibility and gameplay-owned observations.

## Checking an edit

Prefer the actual observed gameplay result and the relevant reload diagnostic.
For a malformed pack or missing source, use
[validating a content pack](validating-a-content-pack.md). Static distribution
builds embed content and don't use these runtime file-watch paths. Historical
per-machine build times are not useful predictions for a different checkout.
