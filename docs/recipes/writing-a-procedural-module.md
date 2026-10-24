---
status: current
last_verified: 2026-10-01
---

# Writing a procedural module (no engine build)

A procedural module is a gameplay algorithm with its own state: a boss
technique, or a wielded item's use (the shockwave, beam, volley and meteor). It is written against `ambition_extension_sdk` and the pure
values of the ports it uses, and nothing else. Edit it, rebuild only the
module crate as a `.wasm` file, and the running game picks it up.

Typical cost of a one-constant edit, warm build:

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

With the watcher, save to rebuilt `.wasm` is about half a second (build plus the
watcher's half-second poll). The game's reload poll adds up to 20 frames.

The log says `AMBITION_EXTENSION_MODULES: … provides [...]` at start and
`… changed; reload proposed` / `extension modules reloaded` on a reload.

- A loaded module **explicitly replaces** the linked module of the same key;
  admission records it (`AdmittedExtensions::replaced`).
- A file that does not load or admit is **reported and the running code
  stays**. A file named at start that does not load stops the run.
- A reload that changes a state schema's **fields** carries every live record
  over by field tag (`StateSchema::migrate`): a renamed or moved field keeps its
  value, a field whose kind changed starts from its initial value. A reload that
  changes a schema's **attachment** (body or session) or **save policy** is
  refused; restart for that.
- A reloaded file replaces **every module it gave**, as one set: a module the
  new build no longer exports leaves. Files changed before one publication are
  one reload. A schema no module declares any more takes its records with it.
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
   is a new field. A hot reload that changes the fields carries the live
   records over by tag: a kept field keeps its value (also under a new name),
   a new field or one whose type changed starts at its initial value, a
   removed field is dropped. Moving a record to another store (body ↔
   session) is refused (restart).

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
| `ambition.combat.held_damage_box` v2 | request | `ambition_combat_port::HeldDamageBox` (held while re-submitted each tick, at the submitted centre; a new generation replaces it) |
| `ambition.items.wielded_use` v2 | trigger (phase `wielded_use`) | `ambition_combat_port::Wielder` (selector: the held item id) |
| `ambition.world.spawn_module_entity` v1 | request (phase `wielded_use`) | `ambition_combat_port::ModuleEntitySpawn` (a kind, a place, a lifetime; ask `Wielder::names_spawns` first) |
| `ambition.world.module_entity_tick` v1 | trigger (phase `module_entity_tick`) | `ambition_combat_port::ModuleEntityTick` (selector: the kind; records are the entity's) |
| `ambition.world.end_module_entity` v1 | request (phase `module_entity_tick`) | `ambition_combat_port::EndModuleEntity` (end the entity before its lifetime) |
| `ambition.world.pull_bodies` v1 | request (phase `module_entity_tick`) | `ambition_combat_port::PullBodies` (pull `Enemy`-side bodies toward a point) |
| `ambition.resources.spend_mana` v1 | request, a cost (`wielded_use`) | `ambition_combat_port::SpendMana` (ask `Wielder::can_pay_mana` first; it is a snapshot, so a refused payment refuses every request of the invocation) |
| `ambition.feedback.body_sound` v1 | request | `ambition_combat_port::BodySound` (a cue id, as the body) |
| `ambition.boss.summon` v2 | request | `ambition_boss_special_port::BossSummon` (a boss only; the minion joins its encounter; `on_boss_side` lets the boss's volumes pass it) |
| `ambition.actor.think` v2 | trigger (phase `actor_think`) | `ambition_combat_port::ActorThink` (selector: the `CharacterBrain::Custom` key the placement authors; one call each tick for each live body the player does not drive; its target where it is now; v2: `placement`, the id of the placement that built it) |
| `ambition.body.contact` v1 | trigger (phase `body_contact`) | `ambition_combat_port::BodyContact` (selector: the `CharacterBrain::Custom` key the placement authors; one call each tick for each live body that player bodies of its own live room touch, after the bodies integrated and before the shared contact damage; the body and each toucher, in identity order) |
| `ambition.body.contact_bounce` v1 | request (`body_contact`) | `ambition_combat_port::ContactBounce` (a toucher, by its index, jumps up off the body; with `pogo_cue`, its pogo cue plays) |
| `ambition.body.defeat` v1 | request (`body_contact`) | `ambition_combat_port::BodyDefeat` (the touched body is defeated now: health zero before the contact damage, then removed; no drops) |
| `ambition.actor.fire` v1 | request (`actor_think`) | `ambition_combat_port::ActorFire` (a fire direction in the body's control frame, before the control gate; the body's moveset fires it) |
| `ambition.motion.velocity` v1 | request (`wielded_use`, `actor_think`) | `ambition_combat_port::SetVelocity` (the body's velocity; in `actor_think` the body integrates it this tick) |
| `ambition.boss.conduct` v9 | trigger (phase `boss_conduct`) | `ambition_boss_special_port::BossConduct` (selector: the boss's behaviour id; one call each tick; v2: `holding` and `minions`; v3: `between_phases`; v4: `room`; v5: `health`, `invulnerable` and `target_body`, the tracked body now; v6: `size`, the boss's own; v7: the `swing` of `target_body`, the move the tracked body plays; v8: `awake`; v9: `predecessor`, the boss it wakes after) |
| `ambition.boss.conducted_pose` v2 | request (`boss_conduct`) | `ambition_boss_special_port::ConductedPose` (hold the boss's pose, or release it; the side it faces; v2: no side leaves its facing to whoever drives it) |
| `ambition.presentation.drawn_row` v2 | request (`boss_conduct`) | `ambition_boss_special_port::DrawnRow` (the sheet row a part is drawn with; v2: the part is the boss, the mount it rides or a limb of that mount, and the names are a fallback list) |
| `ambition.boss.mount` v3 | observation (`boss_conduct`) | `ambition_boss_special_port::MountView` (the body the boss rides: its pose, its back, the hall under it, its limbs with their rest, lost health, hold, contact and the bodies their strike hit) |
| `ambition.boss.limb_pose` v1 | request (`boss_conduct`) | `ambition_boss_special_port::LimbPose` (hold a limb of the ridden mount at a pose, or release it to fall straight down) |
| `ambition.boss.limb_bond` v1 | request (`boss_conduct`) | `ambition_boss_special_port::LimbBond` (where the bond of a limb of the ridden mount starts, while re-submitted each tick: a trail that is the module's own; a limb not asked for has its bond from its host) |
| `ambition.boss.limb_opening` v1 | request (`boss_conduct`) | `ambition_boss_special_port::LimbOpening` (a closed limb of the ridden mount refuses damage) |
| `ambition.boss.limb_blow` v1 | request (`boss_conduct`) | `ambition_boss_special_port::LimbBlow` (heal what a limb lost; with `pass`, the boss takes it unless it refuses damage now) |
| `ambition.boss.mount_launch` v1 | request (`boss_conduct`) | `ambition_boss_special_port::MountLaunch` (launch the ridden mount through its launch gateway; `flinchless` for a hop) |
| `ambition.boss.mount_walk` v1 | request (`boss_conduct`) | `ambition_boss_special_port::MountWalk` (walk the ridden mount on the floor to `to_x` at `speed`; a speed of 0 stops it) |
| `ambition.boss.foe_launch` v1 | request (`boss_conduct`) | `ambition_boss_special_port::FoeLaunch` (launch the body the boss targets through its launch gateway: a clash, a recoil) |
| `ambition.world.held_force` v2 | request (`boss_conduct`) | `ambition_combat_port::HeldForce` (a field over a box of the room, held while asked: take a part of each body's weight, push it by a force over its mass, and (v2) pull it toward a gravity well, the same acceleration on any mass) |
| `ambition.boss.wear` v1 | request (`boss_conduct`) | `ambition_boss_special_port::Wear` (put a `worn_gear` row on the boss, its mount or its target, or take it off; see `docs/systems/worn-gear.md`) |
| `ambition.feedback.speech_bubble` v1 | request (`boss_conduct`) | `ambition_combat_port::SpeechBubble` (text in a bubble, in the body's live room) |
| `ambition.combat.riding_hitbox` v1 | request (`boss_conduct`) | `ambition_combat_port::RidingHitbox` (a volume that rides its owner) |
| `ambition.portal.held_pair` v1 | request (`boss_conduct`; a game with the `portal` feature) | `ambition_portal_port::HeldPortalPair` (a linked pair of portals, held while re-submitted each tick at the submitted places; named by its slot; a look index for each end) |
| `ambition.portal.light_line` v2 | request (`boss_conduct`; a game with the `portal` feature) | `ambition_portal_port::LightLine` (a line of a portal's light that is no portal: nothing goes through it and it pushes nothing; drawn while re-submitted each tick; named by its slot; a look index; v2: `on_limb`, the limb whose drawn cut the line is laid on) |
| `ambition.feedback.burst` v1 | request (`boss_conduct`) | `ambition_combat_port::Burst` (particles) |
| `ambition.combat.held_ray` v1 | request (`technique_execution`, `boss_conduct`) | `ambition_combat_port::HeldRay` (a strike along a line, cast through portals, drawn as its named look) |
| `ambition.feedback.lightning` v1 | request (`boss_conduct`) | `ambition_combat_port::Lightning` (a leased bolt that shows and does not hurt) |
| `ambition.feedback.lightning_aura` v1 | request (`boss_conduct`) | `ambition_combat_port::LightningAura` (a leased aura on the asking body) |
| `ambition.feedback.camera_shake` v1 | request (`boss_conduct`) | `ambition_combat_port::CameraShake` (an amplitude; the strongest of a frame wins) |
| `ambition.combat.body_hold` v2 | request (`boss_conduct`) | `ambition_combat_port::BodyHold` (seize a body in a reach, carry it, pummel it, throw it, release it: the engine's capture relation; v2: `hold_at` names the attachment of the owner's body rig that the hold rides, and the engine places it from the rig's pose each tick) |
| `ambition.body.attachments` v1 | observation (`boss_conduct`) | `ambition_combat_port::BodyAttachments` (each named point of the owner's posed body rig, from its position, in its local frame: where a module puts a sound or a spark. A hold does not use it: a hold names the point) |

A thing that outlives the press (a turret) is a module entity: one entry asks
for it in `wielded_use`, a second entry bound to its kind runs each tick it
lives, and keeps its state in a record (the record is scoped to the entity, so
it goes with it). `sentry.rs` is the example; `vortex.rs` keeps its own clock
and ends its entity itself.

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
the module road (a headless fight test, such as `trex_fight.rs`).
