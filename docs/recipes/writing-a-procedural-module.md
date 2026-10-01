---
status: current
last_verified: 2026-10-01
---

# Writing a procedural module (no engine build)

A procedural module is a gameplay algorithm with its own state: a boss
technique today. It is written against `ambition_extension_sdk` and the pure
values of the ports it uses, and nothing else. Edit it, rebuild only the
module crate as a `.wasm` file, and the running game picks it up.

Measured on the agent machine, 2026-10-01, for a one-constant edit:

| road | edit → playable |
|---|---|
| a technique in `game/ambition_content` (engine build) | 7.05 s to relink the app, then a restart |
| the same technique as a module | 0.34 s to rebuild the `.wasm`, hot-reloaded into the running game |

Design and status: `docs/planning/engine/fast-iteration-implementation.md` (I4,
and the loaded road after it).

## The loop

```bash
scripts/build_extension_modules.sh          # builds target/extension-modules/.../ambition_content_modules.wasm
AMBITION_EXTENSION_MODULES=target/extension-modules/wasm32-unknown-unknown/release \
    cargo run -p ambition_app               # once; leave it running
# edit game/ambition_content_modules/src/<technique>.rs
scripts/build_extension_modules.sh          # the game reloads the file within ~20 frames
```

The log says `AMBITION_EXTENSION_MODULES: … provides [...]` at start and
`… changed; reload proposed` / `extension modules reloaded` on a reload.

- A loaded module **explicitly replaces** the linked module of the same key;
  admission records it (`AdmittedExtensions::replaced`).
- A file that does not load or admit is **reported and the running code
  stays**. A file named at start that does not load stops the run.
- A reload that changes a state schema's **shape** is refused (records already
  live under the old shape). Restart for that.
- A reload is a mechanical edit (`Q120`): a local rollback timeline is rebased
  for it; a timeline this process did not start refuses it.

## Writing one

Start from `game/ambition_content_modules/src/eye_beam.rs`. A module is:

1. **A descriptor** (`ModuleDescriptor`): its key, its state schemas, and its
   entries. Each entry names its phase, its trigger (port + selector, for a
   boss technique the `Special("<key>")` in `boss_profiles.ron`), the ports it
   reads and submits to, the schemas it writes, a request limit, and its
   `on_idle` policy.
2. **State** as schema records (`StateSchema`, fields with stable TAGS). The
   host stores them on the body, rolls them back and checksums them. Never
   keep state in a static: each WASM call is a new instance.
3. **An entry function** `fn(&mut Invocation) -> Result<(), Fault>`: read the
   trigger (`inv.trigger::<BossSpecialCast>()`), change your records
   (`inv.state(&KEY)`), and `inv.submit::<Port>(value)` requests. A fault
   discards everything the call staged.
4. **Registration**: add it to `ambition_content_modules::modules()`.

Shared rules live in `strike.rs` (`once`, `once_numbered`, `locked`, `locked_when`). Declare
`on_idle: IdlePolicy::ResetState` when an idle tick (no press, no telegraph)
means "my strike is over": the host then skips the call. It resets ALL the
entry's records, so a technique that keeps something across strikes (apple
rain's lane sequence) declares `IdlePolicy::Invoke` and resets the rest
itself.

A technique that sizes itself by the room reads `BossCaster::room_size`: the
boss's OWN live room. With two live rooms there is no "the" room.

## Ports a module can use today

| port | role | values |
|---|---|---|
| `ambition.boss.special_cast` v4 | trigger | `ambition_boss_special_port::BossCaster` |
| `ambition.projectiles.spawn` v1 | request | `ambition_projectile_spec::ProjectileSpawn` |
| `ambition.combat.damage_box` v1 | request | `ambition_combat_port::DamageBox` (its faction is the owner's) |
| `ambition.combat.held_damage_box` v1 | request | `ambition_combat_port::HeldDamageBox` (held while re-submitted each tick; a new generation replaces it) |
| `ambition.boss.summon` v1 | request | `ambition_boss_special_port::BossSummon` (a boss only; the minion joins its encounter) |

A mechanic that needs another engine fact or action needs a new port: pure
values in a leaf crate (SDK only), an adapter in the owning domain installed
by `ambition_platformer2d_runtime::extension_composition`, and a port card.
That is engine work; every module after it is not.

## Validation

```bash
cargo test -p ambition_content --lib -- module_parity_tests   # each migrated technique vs its old native system, linked and WASM
cargo test -p ambition_app --test app_it -- a_boss_special_runs_on_the_extension_host
```

A NEW technique has no native reference: give it its own behaviour test on
the module road (see `gradient_nova_reference_tests.rs`'s materialization
tests for the shape).
