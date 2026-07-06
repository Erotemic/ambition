# Combat model — the full smash stack, as data, for every actor

**Authored by fable, 2026-07-05.** Completes the combat architecture from
"one damage resolver + movesets" to the FULL platform-fighter stack:
knockback scaling on a damage-accumulation axis, directional influence,
smash attacks, cancel/chain tables, and per-move presentation. Everything
here is body-generic (relativity principle), authored as data (RON on
archetype/catalog rows + `MoveSpec`s), and **headless: every rule below
changes simulation outcomes and therefore must be steppable and assertable
without a renderer.** Only cosmetic presentation (what a hit LOOKS like) is
render-side.

What already exists and is NOT redesigned here: one victim-side
`resolve_body_hit` + `HitEvent` transport; hitstun/knockback/hitstop (A2);
`MoveSpec` (proper-time clocks, phase windows, volumes) + prefab registry +
techniques-with-params (track A complete); `CombatVolume` OBB/convex;
authored blade polygons + slash VFX volume tags (`05a32378`); directional
verb resolution; shields/parry; ledge grab; possession/slots.

---

## 1. The damage-accumulation axis (one meter, two death policies)

Smash's percent and Ambition's HP are the SAME quantity read through
different policies:

- **`DamageMeter`** — the accumulated damage a body has taken. It already
  exists implicitly as `Health.max - Health.current`; the slice makes it a
  first-class read (`fn damage_taken(&self) -> f32`) rather than a new
  component, so no state is duplicated.
- **Death policy** (per-game/per-body data, an archetype field):
  - `HpDepleted` — dies when the meter reaches `max` (Ambition today;
    unchanged behavior).
  - `Unbounded` — the meter has no death threshold (`Health.max` acts as a
    display normalizer only); death comes from the WORLD (blast-zone/OOB
    rule — which the engine already has as the hazard/fell-out gate).
    This is smash percent, and it costs one enum.
- **Knockback formula** (the resolver's launch step becomes):

  ```text
  kb = (base + growth * damage_taken * scale_of(victim.weight)) * move.kb_mult
  dir = di_adjust(move.launch_dir_bodyframe -> worldframe, victim_DI)
  hitstun_frames = hitstun_base + hitstun_per_kb * kb
  ```

  `base`, `growth`, `launch_dir` author per `HitVolume`/`MoveSpec` (with
  prefab defaults); `weight` authors per archetype. Today's flat knockback
  is exactly `growth = 0` — the migration is byte-parity by construction,
  then content opts rows into growth.

## 2. Directional influence (DI) — the two-port discipline applied to defense

The victim's CONTROLLER gets a say; the victim's BODY enforces the limits.
At launch resolution, read the victim's `ActorControl.locomotion` (the same
gated input every system reads — hitstun gating does NOT zero DI; DI is
what hitstun input is FOR) and rotate the launch vector toward the held
direction by at most `di_max_angle` (data; smash-like ≈ 18°). Because it
reads `ActorControl`, DI works identically for humans, brains, and RL
policies — a level-9 CPU that DIs correctly falls out of the fighter brain
reading the same seam. v1 is launch-DI only; SDI (hitstun nudges) is a
listed extension, not speculative work.

C4 test: identical hit under rotated gravity with rotated DI input produces
the conjugated trajectory. RL test: optimal DI measurably extends survival
in the headless rig.

## 3. Smash attacks & charge

Composition of two landed pieces plus one rule:

- `simple_charge` prefab (A2) provides hold-to-charge move shells.
- The directional-verb chain (attack_up/attack_down/side) provides the
  smash-input surface; a `smash: true` variant class on the verb map lets a
  row bind strong-directional attacks distinct from tilts (input
  distinction — flick vs. hold — is a resolver knob, authored per game;
  the SSB demo turns it on, Ambition may not).
- Charge scaling: released charge fraction multiplies damage and `kb_mult`
  (`1.0 → smash_charge_mult`, data). Charge state lives on `MovePlayback`
  (a held Startup phase) — no new component.

## 4. Cancels & chains (the animation/ability chain)

`MoveSpec` gains a **cancel table**: per phase-window, a list of
`(condition, into_class)` rows — conditions from {`OnHit`, `OnBlock`,
`OnWhiff`, `Always`}, `into_class` naming move classes (`jump`, `dash`,
`special`, `any_attack`, specific ids). The playback advancer already owns
phase state; the addition is: when a cancel-legal input arrives (from the
SAME buffered-intent path that starts moves today), end the current move at
the cancel point and start the next. Combo/chain design then becomes
AUTHORED DATA (a jab that chains into jab2 on hit; aerials canceling into
jump on hit = jump-cancel), and the fighter brain can read the table as
frame-data. Input buffering already exists for attack/pogo; the buffer
window becomes per-row data.

## 5. Per-move presentation (Jon's report: one generic swing everywhere)

The seam landed in `05a32378` (volume-level `vfx` tags) generalizes: a
`MoveSpec` phase may author **presentation events** — `sfx: <cue-id>`,
`vfx: <effect-id>` — resolved through the SAME content-registered
registries as everything else (typo = startup validation error, per the
AJ1 hook). `simple_melee`/`simple_ranged`/`simple_charge` prefab params
include the cue/effect ids so every authored row can sound/look distinct
with zero code. The sim emits the event facts (`FrameEvents` /
`MoveEventKind`); presentation consumes. This slice also closes jonnotes
item "the specific attack should be tied to a vfx and sfx" and subsumes the
remaining §7.2 vocabulary note.

## 6. Grabs, throws, shield-stun (staged vocabulary, SSB-gated)

Not speculative — Super Smash Siblings needs them; they land WITH that demo
under the oracle discipline:

- **Grab** = a `HitVolume` with `mode: Grab`: on connect, establishes a
  short `Grappled { holder }` state (a ControlGrant-shaped authority
  reduction — the mount/possession seam reused, NOT a new control path).
- **Throw** = a move whose active phase releases the grapple with an
  authored launch (goes through the §1 formula).
- **Shield-stun/pushback**: the existing shield gets the frame-advantage
  fields (`shieldstun_per_damage`, pushback along contact tangent).

## 7. Match/mode state lives OUTSIDE the engine

Stocks, percent HUD, blast-zone dimensions, respawn invulnerability,
platforms-only stages, victory conditions — ALL demo-content (see
[`../demos/super-smash-siblings.md`](../demos/super-smash-siblings.md)).
The engine's obligations end at: the damage axis (§1), OOB events a mode
can consume as "blast", spawn/respawn primitives, and local-N slot routing
([`netcode.md`](netcode.md) N1). If SSB needs anything else engine-side,
that's an oracle-violation to file, not a quiet edit.

## 8. Design sketches (pre-solved; executors do not re-derive these)

**CM1 field placement** (grounded in the live types): `HitVolume`
(in [the authoring spine], `ambition_entity_catalog`) already carries `damage` + `knockback` (the
flat magnitude). Add beside them, all `#[serde(default)]` so every
existing RON row is untouched:

```rust
pub struct HitVolume {
    // …existing…
    pub kb_growth: f32,                 // default 0.0 == today, parity by construction
    pub launch_dir: Option<(f32, f32)>, // body-local (+x facing, +y gravity-down);
                                        // None = today's facing+contact derivation
}
// archetype row (features/enemies schema):  weight: f32 = 1.0
// archetype row:  death_policy: DeathPolicy = HpDepleted   (enum { HpDepleted, Unbounded })
```

The resolver (`resolve_body_hit`) computes
`kb = knockback + kb_growth * victim.damage_taken() / weight`, then the
DI adjust (CM2), then existing knockback application unchanged.
`damage_taken()` is a method on the existing health cluster — no new
component, no parallel meter.

**CM4 cancel algorithm** (grounded in `MoveSpec`/`MovePlayback`,
`combat/moveset.rs`): add to `MoveSpec`:

```rust
pub struct CancelRule {
    pub window: (f32, f32),          // proper-time span, like MoveWindow
    pub condition: CancelCondition,  // OnHit | OnBlock | OnWhiff | Always
    pub into: CancelClass,           // Jump | Dash | Special | AnyAttack | Move(String)
}
// MoveSpec gains: #[serde(default)] pub cancels: Vec<CancelRule>,
// MovePlayback gains: pub landed_hit: bool,  // set by the hitbox-connect path
```

Algorithm, inside the existing `trigger_moveset_moves` (the ONE
entry point that starts moves — the cancel check is a pre-step, not a
new system): when a verb edge arrives while `MovePlayback` exists,
instead of today's reject: find the first `CancelRule` whose window
contains `t`, whose condition matches (`landed_hit` / shield-contact
fact / neither), and whose `into` class contains the requested
verb/move; if found, remove the playback (despawning live boxes exactly
as natural completion does — reuse that teardown path, do not
duplicate it) and start the new move same-frame. No rule matched →
today's behavior byte-identically (empty `cancels` == the status quo,
which is the parity pin). Jump/dash cancels route the verb back to the
normal locomotion/ability path after the removal — one early-return,
not a second dispatcher.

**CM7 frame-data table**: a pure derivation, no storage —
`fn frame_data(spec: &MoveSpec) -> MoveFrameData { startup, active
spans, recovery, cancels, volumes' reach }` computed from
`windows`/`cancels`; the brain and the boss validator both call it.
Reach = max over volumes of body-local x-extent (the manifest-resolved
polygon's AABB when `vfx`-tagged volumes override — reuse the §7.1
resolution).

## 9. Slices

| # | Slice | Grade |
|---|---|---|
| CM1 | Knockback scaling fields + weight + death-policy enum; `growth=0` parity pins; C4 rig | [opus, fable-specced — §1 is the spec] |
| CM2 | Launch DI off `ActorControl` + `di_max_angle` data + C4/RL tests | [opus, fable-specced — §2] |
| CM3 | Smash/charge release scaling + verb-map smash class | [opus] |
| CM4 | Cancel tables on `MoveSpec` + buffered-intent cancel path + frame-data read API | [fable-specced; the advancer edit wants care] |
| CM5 | Per-move sfx/vfx presentation events + prefab params + validation | [opus] — also closes jonnotes per-attack-vfx/sfx |
| CM6 | Grab/throw/shield-stun vocabulary | [opus, lands with SSB demo] |
| CM7 | Frame-data introspection: derive per-move startup/active/recovery/cancel windows as a queryable table (consumed by the fighter brain + boss validators) | [opus] |

Exit: a headless test drives two archetypes through hit → DI → knockback →
cancel-chain → KO-by-blast-zone entirely via `SlotControls`, and the same
data renders the ambition robot's unchanged HP combat (`growth=0`,
`HpDepleted`) byte-identically.
