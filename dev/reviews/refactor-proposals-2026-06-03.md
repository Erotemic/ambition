# Refactor proposals — 2026-06-03

Grounded in firsthand friction from the 2026-06-03 long run (which added the
inventory/economy/abilities/loot/gravity loop). Every pain below was hit while
building, and every claim is verified against current code. Ordered by
value-per-risk, not by size. None of these *adds* abstraction for its own sake —
they remove genuine duplication or a footgun (per `[[feedback_design_balance]]`:
narrow specific types, no speculative generalization).

Safety net: the catalog round-trip tests, the boss/damage tests, and the headless
content-validation already cover most of the blast radius, and rename/move
refactors are compiler-checked — so the mechanical ones can be ported boldly
(`[[feedback_bias_toward_executing_big_refactors]]`).

---

## Tier 1 — clear wins, low risk, do first (independent, ~1–2h each)

### 1. `items.rs` catalog: 13 parallel per-item `match` functions → one data table
**Pain.** `crate::items::Item` (24 variants) has **7 forward** per-item match
functions — `grid_pos`, `category`, `display_name`, `description`, `held_item_id`,
`legacy_kind`, `dialog_id` — each a 24-arm `match`, plus the reverse `from_*`
lookups. Adding/renaming one item means editing ~7 arms scattered across the file,
and nothing but vigilance stops you forgetting one (the bug I'd expect: a new item
with a `display_name` but no `dialog_id` arm).

**Proposal.** A single `const ITEM_META: [ItemMeta; ITEM_COUNT]` (or `LazyLock`)
where `ItemMeta { display_name, description, category, held_item_id: Option<&'static str>, dialog_id, legacy_kind, grid_pos }`, indexed by `self.index()`. Forward
functions become `ITEM_META[self.index()].field`; the `from_*` reverse functions
keep iterating the table. Adding an item = **one row + the enum variant** (the
array length `== ITEM_COUNT` is compiler-enforced, so the table can't be partial).

**Benefit.** One place per item; impossible to forget an arm; you can read all 24
items' metadata at a glance. Slightly *less* code to compile.
**Risk.** Low — mechanical, and `items::tests` already round-trips every mapping.

### 2. Disambiguate the two `ActorFaction` enums (rename, don't merge)
**Pain.** Two enums named `ActorFaction`, both reachable:
- `actor.rs:26` — `Player/Enemy/Neutral/Environment`, the **damage matrix**
  (`can_damage(self, target)`).
- `content/features/components.rs:364` — `Player/Enemy/Npc/Boss/Neutral`, a
  `#[derive(Component)]` **actor-side tag** (`is_player_side`/`is_hostile_side`).

Same name, different variants, different roles, both in scope. This run I had to
stop and figure out *which* one had `is_player_side` vs `can_damage`, and a test
asserted the wrong one's method.

**Proposal.** They encode genuinely different things — **don't unify**, rename for
clarity. `actor::ActorFaction` → `DamageTeam` (or `HitTeam`); leave
`features::ActorFaction` as the actor-side tag. Compiler-checked rename.

**Benefit.** The "which ActorFaction?" stall disappears; the damage-matrix vs
ECS-side-tag split becomes legible from the type name.
**Risk.** Low — a mechanical rename the compiler verifies. Scope is the uses of
the renamed enum (the `actor` one is the smaller/less-used side).

### 3. `commands` belongs in `FeatureHitWriters`, not threaded by hand
**Pain.** `damage.rs::apply_actor_hit` and `apply_boss_hit` take
`&mut FeatureHitWriters` (a `#[derive(SystemParam)]` bundle of message writers) but
**not** `Commands`. To spawn loot this run I threaded `commands: &mut Commands`
through *both* helper signatures and *both* call sites (3 edit points) — twice
(coins, then ability/weapon drops). The bundle exists precisely so helpers don't
grow a long param list; `Commands` was just left out of it.

**Proposal.** Add `pub commands: Commands<'w, 's>` to `FeatureHitWriters` (it's a
`SystemParam`, so it composes). Helpers that already take `writers` can then
`writers.commands.spawn(...)` with no extra threading.

**Benefit.** The next "spawn something on a hit" change is a one-liner, not a
signature surgery across call sites.
**Risk.** Low — additive to a SystemParam bundle.

### 4. Collapse the 12 `spawn_debug_*_once` systems into one data-driven spawner
**Pain.** 12 near-identical `spawn_debug_*_once` systems across 9 files (blink,
grapple, mark_recall, bomb, puppy_slug_gun, gravity_grenade, shrine, portal, …),
each "spawn a `GroundItem` near the player on the first frame," plus 12 separate
plugin registrations. I added/extended several this run by copy-paste.

**Proposal.** One `spawn_debug_items_once` system iterating a
`const DEBUG_SPAWNS: &[DebugSpawn { held_item_id, offset, room: Option<&str> }]`.
Bespoke non-held-item debug spawns (gravity switch/zone, shrine) can stay or join
a sibling table.
**Benefit.** Deletes ~12 functions + 12 registrations; new debug item = one row.
**Risk.** Low. (These are debug-only conveniences, so even a regression is cheap.)

---

## Tier 2 — genuine duplication worth a focused pass (~2–4h)

### 5. Held-item *use-behavior* as data, not hardcoded id-chains + per-item modules
**Pain.** A "use-on-attack" held item today needs **~8 sites**: a bespoke
`*_system`, a `spawn_debug_*_once`, a `|| held.spec.id == X_ID` clause in
`item_pickup::throw_held_item_system`'s `use_on_attack`, a `HELD_ITEMS` entry, an
`items.rs::held_item_id` arm, a `pub mod` in lib.rs, and a plugins.rs registration.
I walked this 6× this run (blink, grapple, mark_recall, puppy_slug_gun, bomb,
gravity_grenade) — the `use_on_attack` chain is now 4 `||`s and growing, and the
fireball path is *another* hardcoded `held.spec.id == FIREBALL_ID` (item_pickup.rs).

**Proposal.** Give `HeldItemSpec` a small **`use_behavior`** field — the narrow
vocabulary the existing TODO ("Pick-up / throw held items", design-surface bullet)
already names: `KeepOnUse` (axe), `ThrowOnUse` (javelin/bomb/grenade),
`UseSystem(id)` (blink/grapple/summon — a bespoke verb keyed by id). Then
`throw_held_item_system` reads `spec.use_behavior` instead of an id-chain, and the
bespoke verbs dispatch through one lookup. **Narrow, not a generic plugin system.**

**Benefit.** Per-item edit sites drop from ~8 to ~2–3 (a `HELD_ITEMS` row + the
verb fn, if bespoke). The throw vs use vs explode decision lives on the spec, where
it's authored, not in a growing `||` chain.
**Risk.** Medium — touches the throw/pickup path (well covered by `item_pickup`
tests). Lands the "design surface" the TODO already flagged as the real goal.

### 6. Split the god-modules to shrink the incremental rebuild unit (compile time)
**Pain.** `[[feedback_compile_time]]` (the sandbox build is ~10 min; design for
fast incrementals). The biggest files force a full-file recompile on any edit:
`ledge_grab.rs` 2438, `brain_effects.rs` 2256, `boss_pattern.rs` 2169,
`portal.rs` 1913, `boss_attack_geometry.rs` 1590, `bosses.rs` 1375,
`action_set.rs` 1265, `damage.rs` 1244, `world_flow.rs` 1175.
**Proposal.** Split by concern behind a `mod` dir (e.g. `brain_effects.rs`'s six
`spawn_*_from_special_messages` → one file each; `portal.rs` → fire/teleport/
visual/gravity-switch). Pure code-move, public paths preserved via re-exports.
**Benefit.** Editing one boss attack recompiles ~300 lines, not 2256.
**Risk.** Low (mechanical move; no behavior change), but touch-many — do it once
per file deliberately.

---

## Tier 3 — bigger / already tracked (don't re-discover)

- **The six `spawn_*_from_special_messages` → one generic `apply_special_effects` +
  `SpecialActionSpec: Deserialize`.** The single biggest dedup *and* the unblock
  for player-wields-boss-attacks — already the **⭐ elevated** *Effect-primitive
  ability/attack vocabulary* item in TODO.md. Subsumes #5's bespoke-verb dispatch
  on the actor side.
- **Dual inventory store.** `inventory::PlayerInventory` (3-kind bag) vs
  `items::OwnedItems` (24-item catalog) — already TODO "Collapse the dual store".
  Make `PlayerInventory` a view of `OwnedItems`, or delete it.

## Suggested order
4 → 1 → 2 → 3 (the independent, compiler-checked, ~1h wins), then 5 (held-item
behavior), then 6 (file splits, one at a time), leaving the Tier-3 architectural
ones for a dedicated run with the parity harness.
