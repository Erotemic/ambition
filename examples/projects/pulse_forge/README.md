# Pulse Forge

This project adds a charge-and-release technique with no Rust module.
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

A headless load check also works with no compiler on `PATH`:

```bash
PATH=/nonexistent ./target/debug/ambition_game_bin --project examples/projects/pulse_forge --headless-ticks 30
```

Open the inventory and equip **Pulse Forge**. It occupies the Eye Laser
catalog slot. Hold Projectile for at least 0.2 seconds, then release it.
An emitter stays at the release position and fires four bolts per pulse.
A longer charge gives more pulses, up to five. The emitter ends after its
last pulse.

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
```
