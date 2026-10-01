---
status: current
last_verified: 2026-10-01
---

# Writing a procedural module (no engine build)

A procedural module is a gameplay algorithm with its own state: a boss
technique, or a wielded item's use (the shockwave, beam, volley and meteor). It is written against `ambition_extension_sdk` and the pure
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

Or leave a watcher running instead of the second build:

```bash
scripts/build_extension_modules.sh --watch  # rebuilds on every save; a failed build keeps the last good one
```

Measured on the agent machine, 2026-10-01: a one-constant edit to the
shockwave, from save to rebuilt `.wasm`, **0.53 s** (0.30 s of build, the rest
the watcher's half-second poll); the game's reload poll adds up to 20 frames.

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

Start from `game/ambition_content_modules/src/overfit_volley.rs` (its own record) or `eye_beam.rs` (a shared strike rule). A module is:

1. **A descriptor** (`ModuleDescriptor`): its key, its state schemas, and its
   entries. Each entry names its phase, its trigger (port + selector, for a
   boss technique the `Special("<key>")` in `boss_profiles.ron`), the ports it
   reads and submits to, the schemas it writes, a request limit, and its
   `on_idle` policy.
2. **State** as a typed record. `record!` declares the struct and its schema
   together; each field has a stable TAG, and the struct's `Default` is the
   initial record. The host stores it on the body, rolls it back and
   checksums it. Never keep state in a static: each WASM call is a new
   instance.

   ```rust
   record! {
       pub struct Volley = SchemaKey::new(crate::PROVIDER, "overfit_volley.volley", 1);
       1 samples: Vec<[f32; 2]> [max SAMPLE_COUNT],  // a Vec declares its bound
       2 sample_accum: f32,
       3 fired_this_strike: bool,
   }
   // schemas: vec![Volley::schema()], writes: vec![Volley::KEY]
   let mut s = Volley::load(inv)?;   // in the entry
   s.sample_accum += inv.dt();
   s.store(inv)?;
   ```

   Field types: `bool`, `u32`, `i32`, `u64`, `f32`, `[f32; 2]`, `Option<T>`,
   `Vec<T>` with `[max N]`. Keep a field's tag when you rename it; a new tag
   is a new field, and a changed SHAPE refuses a hot reload (restart).

   A record is the BODY's (one for each body the entry runs for). For one
   record that the whole session shares — a tally, a cursor across bodies —
   write `= KEY, per session;` after the key. It lives on the session root
   and ends with the session; an idle body does not reset it.
   `fixtures/extension_fixture_modules` is the example.
3. **An entry function** `fn(&mut Invocation) -> Result<(), Fault>`: read the
   trigger (`inv.trigger::<BossSpecialCast>()`), load and store your records,
   and `inv.submit::<Port>(value)` requests. A fault
   discards everything the call staged.
4. **Registration**: add it to `ambition_content_modules::modules()`.

Shared rules live in `strike.rs` (`once`, `once_numbered`, `locked`, `locked_when`). Declare
`on_idle: IdlePolicy::ResetState` when an idle tick (no press, no telegraph)
means "my strike is over": the host then skips the call and resets the
entry's records. A technique that keeps something across strikes (apple
rain's lane sequence, a summon counter) puts it in a record of its own and
declares `IdlePolicy::ResetStateExcept(vec![THAT_RECORD])`. Avoid
`IdlePolicy::Invoke` unless the entry must act on idle ticks: it is called on
every tick of every body the trigger reaches, and a loaded call costs tens of
microseconds.

A technique that sizes itself by the room reads `BossCaster::room_size`: the
boss's OWN live room. With two live rooms there is no "the" room.

## Ports a module can use today

| port | role | values |
|---|---|---|
| `ambition.boss.special_cast` v4 | trigger | `ambition_boss_special_port::BossCaster` |
| `ambition.projectiles.spawn` v1 | request | `ambition_projectile_spec::ProjectileSpawn` |
| `ambition.combat.damage_box` v1 | request | `ambition_combat_port::DamageBox` (its faction is the owner's) |
| `ambition.combat.held_damage_box` v1 | request | `ambition_combat_port::HeldDamageBox` (held while re-submitted each tick; a new generation replaces it) |
| `ambition.items.wielded_use` v2 | trigger (phase `wielded_use`) | `ambition_combat_port::Wielder` (selector: the held item id) |
| `ambition.world.spawn_module_entity` v1 | request (phase `wielded_use`) | `ambition_combat_port::ModuleEntitySpawn` (a kind, a place, a lifetime; ask `Wielder::names_spawns` first) |
| `ambition.world.module_entity_tick` v1 | trigger (phase `module_entity_tick`) | `ambition_combat_port::ModuleEntityTick` (selector: the kind; records are the entity's) |
| `ambition.resources.spend_mana` v1 | request | `ambition_combat_port::SpendMana` (ask `Wielder::can_pay_mana` first) |
| `ambition.feedback.body_sound` v1 | request | `ambition_combat_port::BodySound` (a cue id, as the body) |
| `ambition.boss.summon` v1 | request | `ambition_boss_special_port::BossSummon` (a boss only; the minion joins its encounter) |

A thing that outlives the press (a turret) is a module entity: one entry asks
for it in `wielded_use`, a second entry bound to its kind runs each tick it
lives, and keeps its state in a record (the record is scoped to the entity, so
it goes with it). `sentry.rs` is the example.

A mechanic that needs another engine fact or action needs a new port: pure
values in a leaf crate (SDK only), an adapter in the owning domain installed
by `ambition_platformer2d_runtime::extension_composition`, and a port card.
That is engine work; every module after it is not.

## Inspecting what runs

```bash
cargo run -p ambition_app_tools --release --bin extension_inspect -- \
    --boss mockingbird --ticks 900 \
    [--modules target/extension-modules/wasm32-unknown-unknown/release] \
    [--try-replace target/extension-modules/wasm32-unknown-unknown/release/ambition_content_modules.wasm]
```

It prints the installed ports, each entry in serial order with its code
(linked or loaded), what it writes and requests, which modules a file
replaced, the generation text the content identity holds, each body's records
by field name, and, with `--try-replace`, whether admission would take a
rebuilt file and why not. A body has a record only after a call changed it
from its initial value.

## Validation

```bash
cargo test -p ambition_content --lib -- module_parity_tests   # each migrated technique vs its old native system, linked and WASM
cargo test -p ambition_app --test app_it -- a_boss_special_runs_on_the_extension_host
```

A NEW technique has no native reference: give it its own behaviour test on
the module road (see `gradient_nova_reference_tests.rs`'s materialization
tests for the shape).
