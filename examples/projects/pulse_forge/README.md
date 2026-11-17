# Pulse Forge

The default project runs a charge-and-release technique from Rhai source.
Its equipment row binds `pulse_forge` to the Projectile slot. Its Rhai
declaration defines the inputs, state schemas, requests, and limits.
Its LDtk world has one drill room, a raised platform, and an infinite sandbag.

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

Open the inventory and equip **Pulse Forge**. It occupies the Eye Laser
catalog slot. Hold Projectile for at least 0.2 seconds, then release it.
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
```

`modules/sentinel.rhai` remains a Rhai module beside the emitter in all three
modes. It declares a stateful custom brain. Its authored body placement is
not installed in this drill yet.

Run the promotion fixture:

```bash
cargo test -p pulse_forge_host --test promotion -- --test-threads=1
```

The fixture runs 280 input frames with each backend, then repeats them with
GGRS rollback. The first release creates an emitter. A second release has
insufficient mana and is refused by the same domain owner in all modes.
The fixture compares initial and populated records, ordered requests with
body and occurrence identities, cost admission, entity creation and retirement,
projectile motion, and mana. Traces go to `target/pulse-forge-parity`.
The negative case changes only Rhai's bolt speed. Its structured differences
go to `target/pulse-forge-parity-negative/diff.json`.

The clock starts at the first fixture input tick. This removes the different
setup tick used to start a fresh rollback session. Field values are compared
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

`content/items.ron` owns the equipment binding and its display text. The
content manifest reads the host's other content files and asset library by
relative path. The example thus needs this repository's content library.
Its world and mechanic are project files. It does not supply a separate
game distribution. The fixed inventory catalog still requires the existing
`eyelaser` slot id. The new technique, module, entity kind, and state keys
are declared in project files.

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
