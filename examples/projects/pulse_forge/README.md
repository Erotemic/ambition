# Pulse Forge

The default project runs a charge-and-release technique from Rhai source.
Its character row binds `pulse_forge` to the Projectile slot. Its Rhai
declaration defines the inputs, state schemas, requests, and limits.
Its LDtk world has one drill room, a raised platform, and a patrol body.
The project owns both character rows and its starting-character rule.

Build the host and inspection tool once, from the repository root:

```bash
cargo build -p ambition_app --bin ambition_game_bin
cargo build -p ambition_app_tools --bin extension_inspect
```

Use the built executables for each development run:

```bash
./target/debug/extension_inspect --project examples/projects/pulse_forge
./target/debug/ambition_game_bin --project examples/projects/pulse_forge
```

To select an implementation explicitly, add this field to `project.ron`:

```ron
implementations: [
    (provider: "pulse_forge", module: "emitter", implementation: File("modules/pulse_forge.rhai")),
],
```

`File` adds the selected file to the watched inputs. It can name Rhai source
or a WASM artifact that exports the same module key. `Linked` selects code
compiled into the executable. The stock `ambition_game_bin` does not link
Pulse Forge; the example host below does. If several files export a key,
the selection picks one. With no selection, duplicate replacements refuse.
Each selected implementation must exist. Removing its file keeps the last
good generation. Change the selection and restart the project to use another
implementation; this does not rebuild the host. Selecting linked code requires
a host build that contains it. Edits to the selected file still reload.

A headless load check also works with no compiler on `PATH`:

```bash
PATH=/nonexistent ./target/debug/ambition_game_bin --project examples/projects/pulse_forge --headless-ticks 30
```

The forge keeper has **Pulse Forge** from its character declaration.
Hold Projectile for at least 0.2 seconds, then release it.
An emitter stays at the release position and fires four bolts per pulse.
A longer charge gives more pulses, up to five. The emitter ends after its
last pulse.
Each pulse costs 12 mana. The mana owner accepts or refuses the full release.
If the body cannot pay, it pays nothing and creates no emitter.

The same emitter rule is also in `portable/src/lib.rs`. It uses the public
ports and host records. This source builds as WASM or linked Rust. The three
manifests select one implementation for `pulse_forge::emitter`. They use the
same world, catalogs, and control binding.

Build the selected module without rebuilding the engine:

```bash
scripts/regen/build_extension_modules.sh pulse_forge_modules
mkdir -p examples/projects/pulse_forge/build
cp target/extension-modules/wasm32-unknown-unknown/release/pulse_forge_modules.wasm examples/projects/pulse_forge/build/pulse_forge.wasm
./target/debug/extension_inspect --project examples/projects/pulse_forge/project-rhai.ron --compare examples/projects/pulse_forge/build/pulse_forge.wasm
./target/debug/ambition_game_bin --project examples/projects/pulse_forge/project-wasm.ron
```

For the linked path, build the example host. This is an executable build:

```bash
cargo build -p pulse_forge_host
./target/debug/pulse_forge_host --project examples/projects/pulse_forge/project-linked.ron
PATH=/nonexistent ./target/debug/pulse_forge_host --project examples/projects/pulse_forge/project-linked.ron --headless-ticks 30
./target/debug/pulse_forge_host --project examples/projects/pulse_forge/project-linked.ron --headless-ticks 0 --compare examples/projects/pulse_forge/build/pulse_forge.wasm
```

`--compare` checks the selected linked declaration against the loaded file and
returns status 1 on a mismatch. It does not prove gameplay parity. The Linux
host reports `build=blake3:...` for linked code. This digest covers the running
executable's exact bytes, including its target and build configuration. It is
read once before simulation. Copying the executable keeps that identity;
rebuilding native code changes it even when the crate version stays the same.
The semantic declaration comparison excludes code identity. Browser and Android
hosts report `build=unavailable` until their loaders supply the compiled
artifact's identity. They must not identify native code by the browser process
or Android launcher. This digest is not an artifact signature.

`modules/sentinel.rhai` remains a Rhai module beside the emitter in all three
modes. It observes the room's bodies tagged `pulse_forge.sentinels`. Its
patrol record keeps an origin, elapsed time, direction, and turn count. It
requests steering through the normal body control owner, and turns at a
distance or time limit. The LDtk room places this body as `forge_sentinel`.

Run the promotion fixture:

```bash
cargo test -p pulse_forge_host --test promotion -- --test-threads=1
```

The fixture runs 280 input frames with each backend, then repeats them with
GGRS rollback. The first release creates an emitter. A second release has
insufficient mana and is refused by the same domain owner in all modes.
The fixture compares initial and populated records, ordered requests with
body and occurrence identities, cost admission, entity creation and retirement,
projectile motion, patrol steering and motion, turn state, and mana. Traces
go to `target/pulse-forge-parity`.
The negative case changes only Rhai's bolt speed. Its structured differences
go to `target/pulse-forge-parity-negative/diff.json`.

Each run settles at the same simulation tick before the input sequence.
The trace clock starts at the first fixture input tick after rebasing the
rollback session. Field values are compared
exactly in this fixture: charge and cooldown use `f32` addition, subtraction,
and a cap; bolt directions are cardinal and their speed is constant. The
70-tick hold reaches the cap before either release. No float tolerance can
change its pulse count. This result does not assert float parity for other
algorithms or platforms.

The `worlds` field in `project.ron` selects the directory of LDtk files.
The pack's `content/worlds.ron` names those files and the entry room. The
project starts in that room unless `entry_room` or `--start-room` overrides
it. The app uses this same declaration for loading, drawing, and reload.
Project worlds have no fallback to the host's embedded worlds.

Edit `room.json`, then build the world with the existing Python tools:

```bash
python examples/projects/pulse_forge/author_world.py
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor examples/projects/pulse_forge/worlds/drill.ldtk
```

These commands need no Cargo build. You can also edit `worlds/drill.ldtk`
with LDtk. While the game runs, use **Apply Reload** in the developer
controls, or enable its automatic world reload. A rejected world reload
keeps the live room. World reload uses the existing session publication
boundary and resets local rollback history. Changes to the world file list,
world directory, or entry room take effect at the next start.

Edit `modules/pulse_forge.rhai` while the game runs. For example, change
`BOLT_SPEED` from `240.0` to `360.0`. The next pulse uses the new speed.
No Cargo command is needed. A syntax error keeps the last good module set
active. Other edits, additions, and removals from that poll wait with it.
Repair the failed file to retry those changes together.
An invocation fault commits no state writes or requests from that
invocation.

```bash
sed -i 's/const BOLT_SPEED = 240.0;/const BOLT_SPEED = 360.0;/' examples/projects/pulse_forge/modules/pulse_forge.rhai
./target/debug/extension_inspect --project examples/projects/pulse_forge --try-replace examples/projects/pulse_forge/modules/pulse_forge.rhai
```

The manifest watches the `modules` directory. Add a `.rhai` file with its
own `fn declare()` to add a module. Remove it to remove that module and its
records. To suspend the emitter module, move its file out of the directory:

```bash
mv examples/projects/pulse_forge/modules/pulse_forge.rhai examples/projects/pulse_forge/modules/pulse_forge.disabled
mv examples/projects/pulse_forge/modules/pulse_forge.disabled examples/projects/pulse_forge/modules/pulse_forge.rhai
```

Wait for the reload between these two commands. Removing the module leaves
existing emitters to their world-owned lifetime. Adding it starts its records
from their declared initial values.

`content/characters.ron` owns the characters, technique binding, and patrol
tag. `content/rules.ron` selects the forge keeper as the starting character.
The pack has explicit empty boss and cutscene catalogs. The content manifest
still reads shared tuning, item catalogs, and an asset library from this
repository. This example is not yet a separate game distribution. Its world,
characters, technique, module, entity kind, and state keys are project files.
The technique has no inventory item binding.

Scripts use the host's [described ports](../../../docs/sdk/script-ports.md).
Script imports are disabled. Project loading uses the native filesystem;
this example does not supply a packaged or browser project loader.

The script uses only simulation time and host-owned records. The body and
each emitter have separate state. The normal body, control, spawn, faction,
and projectile paths apply to this technique. A rollback restores those
records. A live reload uses the existing mechanical-edit admission boundary;
it does not replace code inside an active rollback history.

Focused verification:

```bash
cargo test -p ambition_combat_port --lib
cargo test -p ambition_app --test app_it -- a_project_authors_a_technique
cargo test -p pulse_forge_host --test promotion -- --test-threads=1
```

The project uses these domain interfaces. Both languages use the same
SDK description; Rhai uses the installed names in the first column.

| Interface | Domain owner | Rust SDK access |
| --- | --- | --- |
| `technique_use` | `ambition_abilities::extension` | `TechniqueUsePort` trigger |
| `module_entity_tick` | `ambition_abilities::module_entity` | `ModuleEntityTickPort` trigger |
| `spawn_module_entity` / `end_module_entity` | `ambition_abilities::module_entity` | `SpawnModuleEntityPort` / `EndModuleEntityPort` requests |
| `spend_mana` | `ambition_abilities::extension` | `SpendManaPort` request; settlement accepts or refuses the release |
| `projectile` | `ambition_projectile_combat` | `ProjectileSpawnPort` request |
| `group_think` | `ambition_abilities::group` | `GroupThinkPort` trigger; ordered room members and items |
| `steer` | `ambition_abilities::group` | `SteerPort` request; control authority checks room membership and drive ownership |

The Rust types are exported by `ambition_combat_port` and
`ambition_projectile_spec`. Their `ScriptPort` descriptions generate the
[Rhai reference](../../../docs/sdk/script-ports.md). Scripts read trigger
values through `this.trigger`, records through `this.state`, and send
requests through `this.requests`. Portable Rust uses the typed invocation
and output methods in `portable/src/lib.rs`. Neither path writes raw ECS
state. The host owns physics, damage, resources, identity, and rollback.

The LDtk owner creates the placed actor from `NpcSpawn`. The character
catalog owner resolves its body grants and techniques. World and character
loading use the same paths as the host's other authored content. The project
supplies no Rust actor constructor or brain descriptor.

An optional recipe keeps a counter on an authored body. Copy it into the
watched directory:

```bash
cp examples/projects/pulse_forge/recipes/progress.rhai examples/projects/pulse_forge/modules/progress.rhai
```

Add this field to the `forge_sentinel` row in `content/characters.ron`:

```ron
techniques: [(id: ("pulse_forge"), slot: Projectile, gate: Technique("pulse_forge"))],
```

`idle: "invoke"` calls the counter while the body owns the technique, even
when it does not press the control. The counter uses a durable body record.
That save road keeps records for authored and provider-staged bodies under
their stable identity. It does not keep records for bodies with no durable
provenance, such as the current home avatar. Transient emitter and patrol
records stay out of the save. A different kind on an existing field tag, or a
change of attachment or save policy, is refused against live and saved schemas.
The last good code and records remain usable. An unloaded module's saved rows
stay in the save. Adding that module again must honor those rows. A fresh
headless start with an incompatible saved schema refuses session activation
and reports the schema key and changed field or policy. It does not rewrite the
save file. Use a new schema key for new state; this is not an arbitrary state
migration API.

```bash
cargo test -p pulse_forge_host --test project_state -- --test-threads=1
```

This fixture also changes a script and its character tuning together. Invalid
content holds both changes. Repairing that content publishes both. Each
recorded invocation sees matching catalog, body, and script values. The old
backend is released after publication. This test keeps the durable counter
active during publication. The trace, saved RON, save/load observations, and
saved-schema refusals go to `target/procedural-project-state`.
