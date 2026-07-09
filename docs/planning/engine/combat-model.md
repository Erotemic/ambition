# Combat model — the full smash stack, as data, for every actor

**Authored by fable, 2026-07-05; compressed 2026-07-09 (CM1–CM5 + CM7 landed).**
Completes the combat architecture from "one damage resolver + movesets" to the
FULL platform-fighter stack: knockback scaling on a damage-accumulation axis,
directional influence, smash attacks, cancel/chain tables, and per-move
presentation. Everything here is body-generic (relativity principle), authored
as data (RON on archetype/catalog rows + `MoveSpec`s), and **headless: every
rule below changes simulation outcomes and therefore must be steppable and
assertable without a renderer.** Only cosmetic presentation (what a hit LOOKS
like) is render-side.

---

## 1. The damage-accumulation axis (one meter, two death policies)

Smash's percent and Ambition's HP are the SAME quantity read through different
policies:

- **`DamageMeter`** — the accumulated damage a body has taken. It already
  exists implicitly as `Health.max - Health.current`; it is a first-class read
  (`BodyHealth::damage_taken()`), not a new component, so no state is
  duplicated.
- **Death policy** (per-game/per-body data, an archetype field):
  - `HpDepleted` — dies when the meter reaches `max` (Ambition today).
  - `Unbounded` — the meter has no death threshold (`Health.max` acts as a
    display normalizer only); death comes from the WORLD (blast-zone/OOB rule,
    which the engine already has as the hazard/fell-out gate). This is smash
    percent, and it costs one enum.
- **Knockback formula** (the resolver's launch step):

  ```text
  kb = (base + growth * damage_taken * scale_of(victim.weight)) * move.kb_mult
  dir = di_adjust(move.launch_dir_bodyframe -> worldframe, victim_DI)
  hitstun_frames = hitstun_base + hitstun_per_kb * kb
  ```

  `base`, `growth`, `launch_dir` author per `HitVolume`/`MoveSpec` (with prefab
  defaults); `weight` authors per archetype. Flat knockback is exactly
  `growth = 0` — the migration was byte-parity by construction, and content
  opts rows into growth.

## 2. Directional influence (DI) — the two-port discipline applied to defense

The victim's CONTROLLER gets a say; the victim's BODY enforces the limits. At
launch resolution, read the victim's `ActorControl.locomotion` (the same gated
input every system reads — hitstun gating does NOT zero DI; DI is what hitstun
input is FOR) and rotate the launch vector toward the held direction by at most
`di_max_angle` (data; smash-like ≈ 18°). Because it reads `ActorControl`, DI
works identically for humans, brains, and RL policies — a level-9 CPU that DIs
correctly falls out of the fighter brain reading the same seam. v1 is launch-DI
only; SDI (hitstun nudges) is a listed extension, not speculative work.

C4 test: identical hit under rotated gravity with rotated DI input produces the
conjugated trajectory. RL test: optimal DI measurably extends survival in the
headless rig.

## 3. Smash attacks & charge

Composition of two landed pieces plus one rule: the `simple_charge` prefab
provides hold-to-charge move shells; the directional-verb chain provides the
smash-input surface (a `smash` verb family binds strong-directional attacks
distinct from tilts — the flick-vs-hold input distinction is a resolver knob,
authored per game); released charge fraction multiplies damage and `kb_mult`
(`1.0 → smash_charge_mult`, data). Charge state lives on `MovePlayback` (a held
Startup phase) — no new component.

## 4. Cancels & chains (the animation/ability chain)

`MoveSpec` carries a **cancel table**: per phase-window, `(condition,
into_class)` rows — conditions from {`OnHit`, `OnBlock`, `OnWhiff`, `Always`},
`into_class` naming move classes (`jump`, `dash`, `special`, `any_attack`,
specific ids). When a cancel-legal input arrives (from the SAME buffered-intent
path that starts moves), the current move ends at the cancel point and the next
starts. Combo/chain design is therefore AUTHORED DATA (a jab that chains into
jab2 on hit; aerials canceling into jump on hit = jump-cancel), and the fighter
brain reads the table as frame-data.

**As landed:** the sketch's parallel `cancels: Vec<CancelRule>` was replaced by
extending the EXISTING timeline vocabulary — `WindowTag::Cancelable` grew
`condition: CancelCondition` (serde-default `Always`) — because the timeline
already IS the span structure and `frame_data()` already derived `CancelWindow`
from it. One authoring surface, no parallel table. `into` entries share ONE
string namespace: literal move ids, verbs, and `CANCEL_CLASS_NAMES`. `OnBlock`
deliberately waits for CM6's shield-contact fact — **a parseable-but-never-
firing variant is an authoring trap.**

## 5. Per-move presentation

A `MoveSpec` phase may author **presentation events** — `sfx: <cue-id>`, `vfx:
<effect-id>` — resolved through the SAME content-registered registries as
everything else, so a typo fails at the startup validation gate rather than
silently producing no effect. `simple_melee`/`simple_ranged`/`simple_charge`
prefab params include the cue/effect ids, so every authored row can sound and
look distinct with zero code. The sim emits the event facts; presentation
consumes them.

## 6. Grabs, throws, shield-stun (staged vocabulary, SSB-gated)

Not speculative — Super Smash Siblings needs them; they land WITH that demo
under the oracle discipline. Design pinned in §8.

## 7. Match/mode state lives OUTSIDE the engine

Stocks, percent HUD, blast-zone dimensions, respawn invulnerability,
platforms-only stages, victory conditions — ALL demo-content (see
[`../demos/super-smash-siblings.md`](../demos/super-smash-siblings.md)). The
engine's obligations end at: the damage axis (§1), OOB events a mode can
consume as "blast", spawn/respawn primitives, and local-N slot routing
([`netcode.md`](netcode.md) N1). If SSB needs anything else engine-side, that's
an oracle-violation to file, not a quiet edit.

## 8. Design sketches for the UNLANDED slices (pre-solved; do not re-derive)

**CM6 — shield / grab / throw (fable, 2026-07-06 night; opus executes):**

- **Shield is a component + a held verb, resolved INSIDE the one victim-side
  resolver.** `BodyShield { hp, max, regen_per_s, raised, stun_s }` on
  shield-capable bodies (authored per archetype/`ActorTuning`; ABSENT by
  default = byte-parity for all PvE). The raise input is a held `shield` verb
  on the shared control frame — so a brain / RL policy / level-9 CPU shields
  through the SAME seam a human does (relativity principle), and the EXISTING
  bubble-shield visual + `ShieldRingsView` become this component's presentation
  for free. Resolution order: **grab beats shield beats damage** — (1) a Grab
  contact ignores `raised`; (2) `raised && stun_s == 0.0` routes the hit to
  shield HP (× authored `shield_efficiency`), victim takes zero body
  damage/knockback, gains `stun_s = stun_base + stun_per_damage × dmg` (data),
  and optional authored `chip_fraction` leaks to body HP (default 0); (3)
  otherwise today's path, unchanged. Shield BREAK (`hp ≤ 0`): long authored
  `break_stun_s`, shield unusable until regen crosses a re-enable threshold.
  v1 shield volume = the body hurtbox (no spatial shrink; shrink is a later
  presentation + partial-exposure slice, explicitly out of v1).
- **The shield FACT (what CM4's `OnBlock` waits for):** the hit path already
  marks `MovePlayback.landed_hit`; the same mark pass sets a new `landed_block`
  when the victim's shield absorbed it, and `HitEvent` grows `outcome:
  HitOutcome { Hit (serde-default), Blocked }`. `CancelCondition::OnBlock` then
  reads `landed_block` exactly as `OnHit` reads `landed_hit` — one namespace,
  one mark pass.
- **Grab = a MoveSpec volume with a contact VERB, and holding = the mount
  vocabulary reused.** `HitVolume` grows `on_contact: ContactVerb { Damage
  (serde-default), Grab }`. A Grab contact establishes a hold: the victim body
  receives a temporary **`ControlGrant(Total)` to the holder** — the SAME
  authority transfer ADR 0020 mounts use (do NOT mint a parallel grabbed-state
  machine; a grab IS a hostile brief mount). Victim pose follows the holder's
  authored hold anchor (body-local offset, gravity-frame). Escape = mash:
  accumulated victim input activity (any verb edges on its slot frame) shortens
  the authored `hold_s`; brains mash through the same seam.
- **Throws are moves in the `throw` verb family** — `directional_verb_chain(base
  = "throw")` resolves throw_up/forward/back/down while holding. A throw's hit
  applies DIRECTLY to the held victim (the hold is the contact — no volume
  overlap test), releasing the grant and feeding damage + `launch_dir` + growth
  + DI through the UNCHANGED resolver chain. Pummel (attack verb while holding)
  is an optional v1.5 flag: small fixed damage, extends `hold_s` slightly.
- **Parity + tests:** every new field serde-defaults to absent/off; the CM exit
  test grows: two archetypes → one shields a hit (stun, no knockback, OnBlock
  cancel fires), one gets grabbed + thrown (grant applied/released, launch
  through DI) — all via `SlotControls` headlessly. C4 conjugation applies to
  throws like any launch.

**A3 — equipment→params (fable, 2026-07-06 night; the card the M-track was
missing; [the stuff kit] + this doc share it):**

- **The model:** worn equipment contributes (a) NUMERIC modifiers that merge
  into move/body params at the moment a value is RESOLVED, and (b) BEHAVIORAL
  grants that are ordinary components/prefab rows — never a third mechanism.
- **(a) Numeric modifiers.** `EquipmentRow` (items RON) gains `modifiers:
  Vec<ParamModifier>` where `ParamModifier { param: String /* the
  EffectRef/prefab param namespace the catalog already validates */, op:
  Add(f32) | Mul(f32), scope: Move(String) | Verb(String) | Body }`.
  Resolution: ONE pure helper `resolved_param(base, worn_equipment, param_key,
  scope) -> f32` called at TRIGGER-RESOLVE time (where the prefab expansion /
  move trigger already reads its params) and at the few body-param read points
  (max HP, `BodyBaseSize` scale). **Never bake modified values into stored
  state** — resolution is a read-time fold (ordering: all Adds, then all Muls;
  document IN the helper).
- **(b) Behavioral grants.** An equipment row may name a `grants:` list —
  moveset prefab rows (the flower-analog grants a `simple_ranged` row into the
  wearer's verb map) and/or components (the mushroom-analog raises
  `BodyBaseSize`). Grant application/removal rides equip/unequip through the
  EXISTING wear/roster seams; no new lifecycle.
- **Armor-instead-of-HP (the on-hit equipment policy).** An equipment row may
  declare `on_hit: ConsumeAsArmor { downgrade_to: Option<RowId> }`: inside the
  ONE victim-side resolver (before body damage), a worn armor row consumes the
  hit — equipment is removed/downgraded, victim takes zero HP damage, gains the
  normal brief i-frames. Default absent = parity. (Mary-O: big→small is
  `downgrade_to: None` on the mushroom row.)
- **Exit test:** headless — equip mushroom-analog (size + armor: one hit
  downgrades, second hit damages HP), equip flower-analog (verb map gains the
  ranged move; unequip removes it), a `Mul` modifier visibly scales one
  authored param at trigger-resolve.

## 9. Slices

| # | Slice | Grade |
|---|---|---|
| CM1 | ✅ LANDED. `HitVolume.{kb_growth,launch_dir}`, `ActorTuning.{weight,death_policy}`, `BodyHealth::damage_taken()`, pure `scaled_knockback()` applied victim-side at the moveset-hitbox overlap (the ONE growth-carrying path), `DeathPolicy::kills_at_max()` gating the kill path. `launch_dir` is direction-only, victim-gravity-frame, and preserves the feel-tuned launch SPEED — an authored angle can never out-throw the default. | done |
| CM2 | ✅ LANDED. Pure `di_adjust(launch, di_input_local, gravity_dir, max_angle)`; `di_max_angle` defaults `0.0` (off = parity; a fighter mode authors ≈0.31/18°). Wired via a localized `Option<&ActorControl>` on the two knockback-consumer SYSTEM queries, not the shared cluster views. **Turning DI on for a fighter is a feel number Jon sets.** | done |
| CM3 | ✅ LANDED. `MoveSpec.smash_charge_mult` + `charge_scale_at(t)` — charge state IS the move's clock, no new component. **Partial-charge-on-EARLY-release awaits an `attack_held`/`attack_released` control signal** (input + feel, Jon's domain); the fraction already derives from `t`. | done |
| CM4 | ✅ LANDED. See §4 "As landed". Empty timeline = byte-parity reject (tested). `frame_data().cancel_windows` carries conditions (FB2-ready). | done |
| CM5 | ✅ LANDED. See §5. `MoveSpec::presentation_problems(vfx_known)` (oracle injected — `entity_catalog` stays vfx-free) runs inside `MovePrefabRegistry::expand`. **NOTE: the slash-VFX black square is a SEPARATE render-side sprite-source quirk (needs a visual run), NOT closed here.** | done |
| CM6 | Grab/throw/shield-stun — **design PINNED §8** | [opus, lands with SSB demo] |
| CM7 | ✅ LANDED. `MoveSpec::frame_data() -> MoveFrameData { total_s, startup_s, active_spans, recovery_s, cancel_windows, reach }` — a PURE derivation, no storage, in `ambition_entity_catalog` so brain + boss validators reach it with no upward dep. Consumers (FB2 option scorer, boss validator) wire it when they land. | done |

Exit: a headless test drives two archetypes through hit → DI → knockback →
cancel-chain → KO-by-blast-zone entirely via `SlotControls`, and the same data
renders the ambition robot's unchanged HP combat (`growth=0`, `HpDepleted`)
byte-identically.
