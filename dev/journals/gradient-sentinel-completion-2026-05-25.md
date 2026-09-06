# Gradient Sentinel boss — completion log

Companion to `gradient-sentinel-boss-design-2026-05-25.md` (the
design). This file records what landed, what was deferred, and the
estimated-vs-actual time.

## What landed

One commit (`c4a0714`): full Gradient Sentinel polish slice.

Code (14 files, +2117/-93):

- 5 new `BossAttackProfile` variants (GradientLane, OverfitVolley,
  MinimaTrap, SaddlePoint, GradientCascade)
- 4 new `SpecialActionSpec` variants
- 4 new EFFECTS consumer systems (one per special) + 4 per-boss
  state components
- `BossBehaviorProfile::clockwork_warden` flipped from Cycle to
  Scripted with intro/phase1/transition/phase2/enrage
- `boss_special_for_profile` resolver + `tick_boss_brains_system`
  direct-write path for multi-special bosses
- `spawn_runtime_minion` helper so consumers can spawn puppy_slug
  / small_lurker adds at runtime
- 20 new tests (7 schedule-sanity, 7 consumer-behavior, 6 supporting)

Tests: 820 sandbox lib tests pass (+ 1 engine test).
Headless boot in `first_system_boss` arena: 300 ticks, no panics.

## Architectural decision

`ActionSet.special` is a single slot — fine for GNU-ton's one apple
rain, doesn't fit Gradient Sentinel's four specials. Rather than
grow ActionSet to multi-slot or grow ActorControlFrame with a slot
selector, the boss tick writes `ActorActionMessage::Special { spec }`
directly via `MessageWriter`, looking up the spec through
`boss_special_for_profile(profile, boss)`. GNU-ton migrated to the
same path; `ActionSet.special = None` for ALL bosses now. The
existing apple-rain consumer keeps working unchanged because the
message stream is identical — only the writer location differs.

This kept the change surface minimal (no schema migrations, no
slot-routing logic in the generic resolver) and aligned with the
migration plan's "boss specials are tightly coupled to attack
profiles" framing.

## Estimated vs actual

| Phase                                | Estimate | Actual |
|--------------------------------------|---------:|-------:|
| Design + read-around the codebase    | 1.0 h    | ~0.4 h |
| BossAttackProfile + SpecialActionSpec + volumes | 0.5 h | ~0.2 h |
| Scripted phase script for clockwork_warden | 1.0 h | ~0.4 h |
| `boss_special_for_profile` + direct-write tick | 0.5 h | ~0.3 h |
| 4 EFFECTS consumers + 4 state components | 2.5 h | ~0.7 h |
| `spawn_runtime_minion` helper       | 0.5 h    | ~0.1 h |
| Tests (schedule sanity + consumers) | 1.0 h    | ~0.3 h |
| Compile fixes + cargo fmt + headless boot | 0.5 h | ~0.2 h |
| FEATURES/TODO/journal hygiene       | 0.5 h    | ~0.2 h |
| **Total**                           | **8.0 h** | **~2.8 h** |

Beat the estimate ~3× because the prior actor/brain migration
already had `Hitbox(World)`, `BossAttackState`, `boss_special_for_profile`-
shaped extensibility, and the EFFECTS consumer pattern in place.
Adding four more bosses' worth of specials was mostly typing.

## What's deferred (intentional)

- **Telegraph visualization for specials.** `volumes_for_profile`
  returns empty for the 4 new specials (so they don't double-count
  damage via `boss_attack_damage`); the debug overlay therefore
  draws no yellow rectangle during telegraph. The boss sprite's
  windup animation still plays — the player can read "something is
  coming" — but they don't get a free-look at the pit/cross/cascade
  shape before it commits. Adding `telegraph_hint_volumes_for_profile`
  + presentation routing is the natural follow-up if playtesting
  shows the attacks are too opaque.

- **Boss-schedule RON migration (ADR 0017).** `BossPattern` +
  `BossAttackProfile` + `SpecialActionSpec` need
  `Serialize + Deserialize` derives before
  `assets/data/boss_encounters/<id>.ron` can carry per-phase
  schedules. The schedule is a Rust constant for now — same shape,
  different transport.

- **Real "slop" enemy archetype.** Using `small_lurker` as a
  visual stand-in for the GradientCascade minions. Adding
  `EnemyArchetype::Slop` with the right tunings + sprite is a
  one-row table edit + a sprite asset.

- **Sprite regen for new attack telegraphs.** The
  `BossPatternState.pattern_timer` mirror feeds the existing
  presentation-side `BossPatternTimer`, so animations advance
  with the schedule. But a column-glow for GradientLane / cross
  outline for SaddlePoint / cascade arrows for GradientCascade
  would significantly improve readability — that's
  presentation-layer work using the `ambition_sprite2d_renderer`
  RON metadata pipeline.

## Notes for the next pass

- **Playtest the difficulty.** Phase 2's SaddlePoint window is 4.8s
  with axis_period_s=1.2 — that's ~4 axis swaps per strike. May be
  too long. Tunable in `bosses.rs` constants.
- **OverfitVolley needs a "predicted positions" visual.** Without
  showing the sampled positions during telegraph, the player can't
  read which positions will be punished. The sample list lives in
  `OverfitVolleyState.samples` — a presentation system that draws
  X marks at those positions during telegraph would close this gap.
- **Hall-of-bosses entry.** The Gradient Sentinel now has the
  most-developed schedule of the three authored bosses. If the
  hall_of_bosses spec wants to surface that, it can reference the
  same encounter id.
