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

**CM4 cancel design — AS LANDED (refined at execution, 2026-07-06):**
the sketch's parallel `cancels: Vec<CancelRule>` was replaced by
extending the EXISTING timeline vocabulary — `WindowTag::Cancelable`
grew `condition: CancelCondition` (serde-default `Always`, so existing
rows parse unchanged) — because the timeline already IS the span
structure and CM7's `frame_data()` already derived `CancelWindow` from
it. One authoring surface, no parallel table. `CancelCondition` v1 =
`{Always, OnHit, OnWhiff}`; **`OnBlock` deliberately waits for CM6**
(the shield-contact fact doesn't exist yet — a parseable-but-never-
firing variant is an authoring trap). `into` entries share ONE string
namespace: literal move ids, verbs, and the classes in
`CANCEL_CLASS_NAMES` (`any_attack`/`attack`/`special`/`ranged`/`jump`/
`dash`) — the catalog validator accepts exactly declared ids + that
set. `MovePlayback.landed_hit` is set by the real hit path
(`mark_move_playback_landed_hits` after `apply_hitbox_damage` for
pre-resolved victims; the volume resolver for player-effective
strikes). The trigger seam (`trigger_moveset_moves`) checks
`MoveSpec::cancel_permits(t, landed_hit, names)` on a verb edge during
playback: permitted → tear down live boxes exactly as natural
completion does and start the new move same-frame; `jump`/`dash`
entries END the move on those edges (the locomotion path reading the
same control frame performs the jump/dash — no second dispatcher). No
`Cancelable` window ⇒ today's reject byte-identically (the parity pin,
tested).

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
| CM1 | ✅ LANDED 2026-07-06. `HitVolume.{kb_growth,launch_dir}`, `ActorTuning.{weight,death_policy}` + archetype fields (serde-default → parity), `BodyHealth::damage_taken()`, pure `scaled_knockback()` helper applied victim-side at the moveset-hitbox overlap (the one growth-carrying path), `DeathPolicy::kills_at_max()` gating the actor kill path. `growth=0`/`HpDepleted`/`weight=1.0` defaults are byte-parity; C4 conjugation + scaling + parity tests green. `launch_dir` field authored, consumed by CM2. | done |
| CM2 | ✅ LANDED 2026-07-06. Pure `combat::damage::di_adjust(launch, di_input_local, gravity_dir, max_angle)` rotates the victim's own launch toward its held `ActorControl.locomotion`, bounded by `SandboxFeelTuning.di_max_angle` (default `0.0` → DI off = parity; a fighter mode authors ≈0.31/18°). Reads the SAME gated input every system reads (player + brain + RL), wired via a localized `Option<&ActorControl>` on the two knockback-consumer SYSTEM queries (not the shared cluster views). Threaded through `resolved_body_knockback_velocity`/`apply_body_hit_reaction`/`apply_player_knockback`/`apply_actor_hit`. Tests: inert-at-zero parity, rotate-toward-bounded, cannot-DI-along-launch, C4 conjugation-under-gravity. RL survival-extension assertion deferred to the FB self-play rig (needs the headless ladder). `launch_dir` full directional launch deferred to CM3 (reworks the ±side launch model). | done |
| CM3 | ✅ LANDED 2026-07-06. `MoveSpec.smash_charge_mult` (data, default 1.0 → parity) + `MoveSpec::charge_fraction_at(t)`/`charge_scale_at(t)` (charge state = the move's clock `MovePlayback.t`, no new component); `advance_move_playback` scales the spawned hitbox's damage + knockback by `charge_scale_at(t)`. `simple_charge` prefab exposes the mult param. Smash verb class = MORE VERBS (AJ1): the generic `verbs` map + `directional_verb_chain(base="smash")` already resolve smash verbs distinctly from tilt/`attack` (test proves it); the flick-vs-hold input distinction is per-game (SSB). Tests: charge scale interpolation + parity + no-startup + smash-verb resolution + a runtime charged-hitbox doubling. Partial-charge-on-EARLY-release awaits an `attack_held/released` control signal (input+feel, Jon's domain); the fraction already derives from `t`, so it's a small future add. | done |
| CM4 | ✅ LANDED 2026-07-06 (fable). `WindowTag::Cancelable` grew `condition` (Always/OnHit/OnWhiff; OnBlock waits for CM6's shield fact); ONE cancel namespace (`CANCEL_CLASS_NAMES` + declared ids, validator-enforced); `MovePlayback.landed_hit` set by the real hit path; the trigger seam cancels via `MoveSpec::cancel_permits` — move-into-move replaces same-frame with the natural-completion teardown, jump/dash entries end the move early. Empty timeline = byte-parity reject (tested); 7 new tests incl. the real-hit-path connect fact. `frame_data().cancel_windows` carries conditions (FB2-ready) | done |
| CM5 | ✅ LANDED 2026-07-06 (opus). Per-move presentation is authored, not hardcoded: `MoveEventKind::Vfx { effect }` (entity_catalog) — a timed COSMETIC burst resolved through the content-registered `ambition_vfx::move_vfx_kind` vocabulary (the shared `ExplosionKind` set); `SimpleMeleeParams`/`SimpleChargeParams` gained `swing_sfx: Option<String>` + `swing_vfx: Option<String>` (default `None` → byte-parity; an authored row makes the move sound/look distinct with zero code). Validation: `MoveSpec::presentation_problems(vfx_known)` (oracle injected — entity_catalog stays vfx-free) runs inside `MovePrefabRegistry::expand`, so a typo'd cue/effect fails at the SAME startup gate a bad prefab key hits — never a silent missing effect. Dispatcher emits `VfxMessage::Explosion` at the owner. 3 new tests (authored-vs-parity, typo rejected, dispatch→burst). NOTE: the slash-VFX black-square is a SEPARATE render-side sprite-source quirk (needs a visual run), NOT closed here. | done |
| CM6 | Grab/throw/shield-stun vocabulary | [opus, lands with SSB demo] |
| CM7 | ✅ LANDED 2026-07-06. `MoveSpec::frame_data() -> MoveFrameData { total_s, startup_s, active_spans, recovery_s, cancel_windows, reach }` — a PURE derivation from `windows`+`duration_s` (no storage), in `ambition_entity_catalog` so brain + boss validators reach it with no upward dep. Startup = first Active start; recovery = duration − last Active end; reach = farthest body-local `+x` extent over Active volumes; cancel windows from `WindowTag::Cancelable` (CM4's richer `CancelRule` folds into the same `CancelWindow` shape when it lands). Tests: full derivation + hitless-move. Consumers (FB2 option scorer, boss validator) wire it when they land. | done |

Exit: a headless test drives two archetypes through hit → DI → knockback →
cancel-chain → KO-by-blast-zone entirely via `SlotControls`, and the same
data renders the ambition robot's unchanged HP combat (`growth=0`,
`HpDepleted`) byte-identically.
