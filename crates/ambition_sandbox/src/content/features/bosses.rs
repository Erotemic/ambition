use super::*;

// Boss policy vocabulary (`BossMovementProfile`, `BossPatternStep`,
// `BossPattern`, `BossAttackPattern`, `BossAttackProfile`,
// `step_duration`) moved to `crate::brain::boss_pattern` per the
// "move boss policy out of BossRuntime" migration. Re-exported here
// because `BossBehaviorProfile` and the volumes / construction code
// below still reference them by their old `content::features::bosses`
// path — those references stay legal via the re-export while call
// sites migrate to the brain-module path at their leisure.
pub use crate::brain::boss_pattern::{
    BossAttackPattern, BossAttackProfile, BossMovementProfile, BossPattern, BossPatternStep,
};

// `BossTickOutputs` (previously: `projectile_spawns: Vec<…>`) was
// deleted with Task B of the actor/brain follow-up plan. Apple-rain
// spawning moved to `spawn_gnu_apple_rain_from_special_messages` (an
// EFFECTS-stage consumer driven by `ActorActionMessage::Special`).
// Future boss specials follow the same pattern — one consumer per
// `SpecialActionSpec` variant — instead of accumulating side-channel
// `Vec`s the caller flushes.

/// Encounter id of the gnu_ton boss — derived from
/// `encounter_id_from_name("GNU-ton")`. Centralized so the boss
/// ActionSet wiring (which binds the boss's special slot to
/// `SpecialActionSpec::GnuAppleRain`) can string-match without
/// re-deriving the slug.
pub const GNU_TON_ENCOUNTER_ID: &str = "gnu_ton";

/// Apple-rain tuning consumed by the spawn-time `ActionSet` wiring
/// (spawn.rs binds these into `SpecialActionSpec::GnuAppleRain`).
/// The visual / collision constants (gravity, lifetime, half_extent,
/// spawn-height) live next to the EFFECTS consumer in
/// `content/features/ecs/brain_effects.rs` — the consumer is the
/// only thing that reads them, so they're local there instead of
/// a cross-module knob set.
pub const APPLE_RAIN_INTERVAL: f32 = 0.35;
pub const APPLE_RAIN_SPAWN_SPEED: f32 = 35.0;
pub const APPLE_RAIN_DAMAGE: i32 = 1;
/// Stable id prefix used by the visuals layer to switch the
/// flat-red-rectangle bullet shape to the apple sprite (red body +
/// green leaf + brown stem). Keep in sync with
/// `enemy_projectile::visuals::is_apple_owner`.
pub const GNU_TON_APPLE_OWNER_PREFIX: &str = "gnu_ton_apple";

// Gradient Sentinel encounter id (per `BossEncounterSpec::gradient_sentinel`).
// Audit-engine name `clockwork_warden` resolves to the same boss via
// `BossBehaviorProfile::for_authored_boss`; both ids surface through the
// `BossEncounterRegistry`, but the canonical id used by the brain config
// and EFFECTS consumers is the public name.
pub const GRADIENT_SENTINEL_ENCOUNTER_ID: &str = "gradient_sentinel";

// ===== Gradient Sentinel special-attack tuning =====
//
// Constants kept here (next to the behavior profile that authors the
// schedule) so the EFFECTS consumers and the brain wiring share one
// source. The numeric values are tuned for the
// first_system_boss arena (1280×768) — see the design doc at
// `dev/journals/gradient-sentinel-boss-design-2026-05-25.md`.

/// OverfitVolley: how often (in seconds) the brain samples the
/// player's position during the telegraph window. With 5 samples and
/// 0.30 s spacing the consumer captures ~1.5 s of player travel,
/// covering a player who is reactively zig-zagging.
pub const OVERFIT_VOLLEY_SAMPLE_INTERVAL_S: f32 = 0.30;
/// OverfitVolley: max number of position samples to memorize. Caps the
/// bolt count fired on the strike edge so the player can read the
/// barrage instead of getting blanket-coverage'd.
pub const OVERFIT_VOLLEY_SAMPLE_COUNT: u8 = 5;
/// OverfitVolley: per-bolt projectile speed (px/s). Fast enough that
/// the bolts feel decisive but slow enough to dodge if the player
/// reads the barrage early.
pub const OVERFIT_VOLLEY_SHOT_SPEED: f32 = 360.0;
/// OverfitVolley: per-bolt damage.
pub const OVERFIT_VOLLEY_SHOT_DAMAGE: i32 = 1;

/// MinimaTrap: how long the pit hazard hitbox stays live after the
/// strike edge spawns it. Long enough to be a real area-denial threat,
/// short enough that the player isn't permanently locked out of half
/// the arena.
pub const MINIMA_TRAP_HAZARD_DURATION_S: f32 = 5.0;
/// MinimaTrap: per-tick damage. The standard `apply_hitbox_damage`
/// once-per-strike gate ensures one hit per pit lifetime.
pub const MINIMA_TRAP_DAMAGE: i32 = 2;
/// MinimaTrap: half-extent (x, y) of the pit hitbox.
pub const MINIMA_TRAP_HALF_EXTENT_X: f32 = 56.0;
pub const MINIMA_TRAP_HALF_EXTENT_Y: f32 = 24.0;

/// SaddlePoint: half-extent of each arm along its long axis.
pub const SADDLE_POINT_ARM_LENGTH: f32 = 220.0;
/// SaddlePoint: half-extent of each arm along its short axis.
pub const SADDLE_POINT_ARM_THICKNESS: f32 = 36.0;
/// SaddlePoint: seconds an axis stays active before toggling. The
/// brain's `BossPatternStep::Strike { duration }` governs total
/// strike time; this is just the rotation period.
pub const SADDLE_POINT_AXIS_PERIOD_S: f32 = 1.2;
/// SaddlePoint: per-tick damage.
pub const SADDLE_POINT_DAMAGE: i32 = 2;

/// GradientCascade: number of "slop" minions to spawn at the top of
/// the arena per strike. Kept low so the player can clear before
/// the next attack lands.
pub const GRADIENT_CASCADE_MINION_COUNT: u8 = 2;

/// Design-space y anchor on the shoulder ridge in the regenerated
/// 768×576 GNU-ton sprite (REST_BODY_Y 60 - 62 = -2). Public so the
/// pure volume helpers in `boss_attack_geometry` can read it without
/// duplicating the constant. Must stay in lockstep with
/// `boss_encounter::sprites::GNU_TON_SHEET::feet_anchor_y`.
pub const GNU_TON_ANCHOR_Y: f32 = -2.0;

// `GNU_TON_COLLISION_SCALE`, `GNU_TON_FRAME_HEIGHT`, and
// `gnu_ton_sprite_scale` live in
// `crate::content::features::boss_attack_geometry` next to the
// part-AABB math that consumes them.

/// Live sandbox-side behavior tuning for a boss. This is deliberately separate
/// from `ae::BossEncounterSpec`: the engine spec owns phase progression and HP
/// thresholds, while this profile owns sandbox movement, contact size, damage,
/// and hitbox shapes.
#[derive(Clone, Debug, PartialEq)]
pub struct BossBehaviorProfile {
    pub id: String,
    pub combat_size: Option<ae::Vec2>,
    pub movement: BossMovementProfile,
    /// Optional per-phase movement overrides. `None` means "use
    /// `movement` during this phase." Lets a boss escalate its
    /// movement personality across phases without changing the
    /// profile enum itself.
    pub movement_phase2: Option<BossMovementProfile>,
    pub movement_enrage: Option<BossMovementProfile>,
    /// Multiplier applied to movement speed while an active special
    /// strike is committed. `< 1.0` keeps the boss roughly anchored
    /// so World-space special hitboxes (saddle cross, minima pit)
    /// don't slide out from under the visual telegraph. `1.0` keeps
    /// pre-Gradient-Sentinel behavior.
    pub strike_speed_scale: f32,
    /// Macro state machine tuning — when enabled, the boss runs an
    /// Engage / Approach / Retreat dance on top of the scripted
    /// attack schedule. See [`crate::brain::BossMacroTuning`].
    /// Use `BossMacroTuning::disabled()` for legacy "stand and
    /// fight" behavior.
    pub macro_tuning: crate::brain::BossMacroTuning,
    pub attacks: Vec<BossAttackProfile>,
    pub attack_cooldown: f32,
    pub attack_windup: f32,
    pub attack_active: f32,
    pub attack_damage: i32,
    pub body_damage: i32,
    /// How attack hitboxes are selected. `Cycle` (default for legacy bosses)
    /// rotates through `attacks` using the flat durations above. `Scripted`
    /// runs an authored phase-keyed timeline of telegraph / strike / rest
    /// beats and ignores `attacks` / `attack_cooldown` / `attack_windup` /
    /// `attack_active`.
    pub attack_pattern: BossAttackPattern,
    /// World-space anchor offset (in pixels) from the boss center where
    /// "hand"-class attacks should originate. For body-centered giants
    /// (GNU-ton) the entity transform sits at the scholar on the shoulder,
    /// not the giant's body — without this offset, hand hitboxes would
    /// hover near the scholar instead of where the giant's arms are. Y is
    /// world-space positive-down; leave at `Vec2::ZERO` for ordinary bosses.
    pub attack_origin_offset: ae::Vec2,
}

impl BossBehaviorProfile {
    /// Clockwork Warden / Gradient Sentinel — polished multi-phase
    /// Scripted boss.
    ///
    /// See `dev/journals/gradient-sentinel-boss-design-2026-05-25.md`
    /// for the full design (theme, arena geometry, attack vocab,
    /// per-phase tempo). At a glance:
    ///
    /// - **Phase 1 (~16 s loop)** — fundamentals: FloorSlam,
    ///   GradientLane (vertical column), OverfitVolley
    ///   (position-sampling bolt barrage), SideSweep. Slow,
    ///   readable, generous Rest beats.
    /// - **Transition (3 s)** — pure Rest while the music swaps.
    /// - **Phase 2 (~22 s loop)** — hazards + minions add to the
    ///   vocabulary: MinimaTrap (pit + puppy_slug spawn), SaddlePoint
    ///   (rotating cross hazard), GradientCascade (small_lurker adds).
    ///   Returning OverfitVolley and FullBodyPulse keep the player
    ///   honest.
    /// - **Enrage (~10 s loop)** — desperate: faster telegraphs,
    ///   tighter combos of MinimaTrap → OverfitVolley → SaddlePoint
    ///   → GradientLane.
    pub fn clockwork_warden() -> Self {
        Self {
            id: "clockwork_warden".into(),
            // Tightened combat size to roughly match the visible
            // sprite body: the clockwork_warden sheet has
            // `body_pixel_bbox: 106×83 px` inside a 128×128 frame.
            // The boss spawns at 64×80 world px (LDtk BossSpawn
            // size), so the visible body is ~106/128 × 64 = 53 wide
            // and ~83/128 × 80 = 52 tall. Without this, the
            // collision/damage AABB stretched to the full 64×80
            // sprite cell and the player took hits at empty-air
            // sprite edges. Tracked sprite metadata-driven
            // alignment is the follow-up — for now we hardcode the
            // body extent.
            combat_size: Some(ae::Vec2::new(54.0, 56.0)),
            movement: BossMovementProfile::AnchorSway {
                x_radius: 130.0,
                y_bob: 18.0,
                x_frequency: 0.72,
                y_frequency: 1.10,
                chase_scale: 0.18,
                chase_limit: 70.0,
                speed: 220.0,
            },
            // Phase 2 swaps to a wide AirSwoop — the boss reads as
            // breaking its anchor and starting to fly across the
            // arena. Bigger x/y radius, aggressive chase.
            movement_phase2: Some(BossMovementProfile::AirSwoop {
                x_radius: 360.0,
                y_radius: 110.0,
                x_frequency: 0.95,
                y_frequency: 0.72,
                chase_scale: 0.35,
                chase_limit: 220.0,
                speed: 320.0,
            }),
            // Enrage takes the AirSwoop and pushes speed + chase
            // harder so the boss visibly *commits* to chasing the
            // player in the last quarter HP.
            movement_enrage: Some(BossMovementProfile::AirSwoop {
                x_radius: 380.0,
                y_radius: 140.0,
                x_frequency: 1.35,
                y_frequency: 0.95,
                chase_scale: 0.55,
                chase_limit: 280.0,
                speed: 420.0,
            }),
            // Hold roughly steady while a special strike is live —
            // the saddle cross / pit / cascade hitboxes are
            // World-anchored at the boss pos and shouldn't drift
            // mid-strike.
            strike_speed_scale: 0.20,
            // Chase / engage / retreat dance.
            //
            // Engage distance ~200 px is "middle range": the player
            // can read attacks but isn't pinned. too_close (110 px)
            // triggers Retreat to avoid cornering the player into
            // the wall. too_far (480 px) triggers Approach so a
            // player who runs to the corner of the 1280-wide arena
            // gets chased rather than ignored. engage_max=9 s
            // creates a periodic "preparing something" retreat
            // beat even when distance is fine — the player learns
            // to chase the boss to maintain pressure.
            macro_tuning: crate::brain::BossMacroTuning {
                too_close_distance: 110.0,
                too_far_distance: 480.0,
                engage_distance: 220.0,
                approach_duration_s: 3.2,
                retreat_duration_s: 2.4,
                engage_max_duration_s: 9.0,
                approach_speed_scale: 1.50,
                retreat_speed_scale: 0.80,
                retreat_distance: 280.0,
            },
            // Legacy `attacks` is unused for Scripted bosses, but kept
            // populated with the full attack vocabulary for
            // diagnostics so `boss inspect`-style tooling can list
            // what the boss is capable of without parsing the
            // Scripted schedule.
            attacks: vec![
                BossAttackProfile::FloorSlam,
                BossAttackProfile::SideSweep,
                BossAttackProfile::FullBodyPulse,
                BossAttackProfile::GradientLane,
                BossAttackProfile::OverfitVolley,
                BossAttackProfile::MinimaTrap,
                BossAttackProfile::SaddlePoint,
                BossAttackProfile::GradientCascade,
            ],
            attack_cooldown: BOSS_ATTACK_COOLDOWN,
            attack_windup: 0.52,
            attack_active: 0.32,
            attack_damage: 2,
            body_damage: 1,
            attack_pattern: BossAttackPattern::Scripted {
                intro: BossPattern {
                    // Single show-of-force beat to anchor the tone:
                    // a clean FloorSlam telegraph + strike with a
                    // long settle, no rest after — the encounter
                    // driver fades into Phase 1 from here.
                    steps: vec![
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::FloorSlam,
                            duration: 1.4,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::FloorSlam,
                            duration: 0.4,
                        },
                        BossPatternStep::Rest { duration: 1.2 },
                    ],
                },
                phase1: BossPattern {
                    steps: vec![
                        // Beat 1: FloorSlam — familiar ground-pound.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::FloorSlam,
                            duration: 1.2,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::FloorSlam,
                            duration: 0.4,
                        },
                        BossPatternStep::Rest { duration: 1.4 },
                        // Beat 2: GradientLane — vertical hazard
                        // column that follows the boss. Player jumps
                        // over or moves laterally.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GradientLane,
                            duration: 1.4,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GradientLane,
                            duration: 1.0,
                        },
                        BossPatternStep::Rest { duration: 1.0 },
                        // Beat 3: OverfitVolley — markers track player
                        // through the telegraph, bolts fire at strike.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::OverfitVolley,
                            duration: 1.4,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::OverfitVolley,
                            duration: 0.30,
                        },
                        BossPatternStep::Rest { duration: 1.5 },
                        // Beat 4: SideSweep — classic two-arm sweep.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::SideSweep,
                            duration: 0.9,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::SideSweep,
                            duration: 0.4,
                        },
                        // Long breather closes the loop.
                        BossPatternStep::Rest { duration: 2.0 },
                    ],
                },
                transition: BossPattern {
                    // Pure 3 s rest so the music swap has space.
                    steps: vec![BossPatternStep::Rest { duration: 3.0 }],
                },
                phase2: BossPattern {
                    steps: vec![
                        // Beat 1: MinimaTrap — pit forms at player pos,
                        // puppy_slug spawns. Forces the player to
                        // reposition.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::MinimaTrap,
                            duration: 1.0,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::MinimaTrap,
                            duration: 0.6,
                        },
                        BossPatternStep::Rest { duration: 1.4 },
                        // Beat 2: SaddlePoint — rotating cross hazard.
                        // Long strike window so the rotation matters.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::SaddlePoint,
                            duration: 1.4,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::SaddlePoint,
                            duration: 4.8,
                        },
                        BossPatternStep::Rest { duration: 1.2 },
                        // Beat 3: GradientCascade — 2 small_lurker
                        // minions descend from top of arena.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GradientCascade,
                            duration: 1.2,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GradientCascade,
                            duration: 0.4,
                        },
                        BossPatternStep::Rest { duration: 2.4 },
                        // Beat 4: OverfitVolley returns, faster.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::OverfitVolley,
                            duration: 1.2,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::OverfitVolley,
                            duration: 0.30,
                        },
                        BossPatternStep::Rest { duration: 1.4 },
                        // Beat 5: FullBodyPulse — close-range pulse.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::FullBodyPulse,
                            duration: 1.1,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::FullBodyPulse,
                            duration: 0.5,
                        },
                        BossPatternStep::Rest { duration: 1.0 },
                    ],
                },
                enrage: BossPattern {
                    steps: vec![
                        // Tight MinimaTrap → OverfitVolley combo.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::MinimaTrap,
                            duration: 0.7,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::MinimaTrap,
                            duration: 0.5,
                        },
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::OverfitVolley,
                            duration: 0.7,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::OverfitVolley,
                            duration: 0.3,
                        },
                        BossPatternStep::Rest { duration: 0.6 },
                        // Faster SaddlePoint — shorter total + tighter
                        // axis-period via per-spec tuning lives in
                        // the consumer (current consumer uses one
                        // shared `axis_period_s`; enrage variant is
                        // exposed via a smaller `duration` field on
                        // the strike so total exposure is shorter).
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::SaddlePoint,
                            duration: 1.0,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::SaddlePoint,
                            duration: 3.0,
                        },
                        // GradientLane closer for the final punish.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GradientLane,
                            duration: 0.7,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GradientLane,
                            duration: 0.8,
                        },
                        BossPatternStep::Rest { duration: 1.2 },
                    ],
                },
            },
            attack_origin_offset: ae::Vec2::ZERO,
        }
    }

    pub fn mockingbird() -> Self {
        Self {
            id: "mockingbird".into(),
            combat_size: Some(ae::Vec2::new(500.0, 185.0)),
            movement: BossMovementProfile::AirSwoop {
                x_radius: 250.0,
                y_radius: 62.0,
                x_frequency: 0.56,
                y_frequency: 1.35,
                chase_scale: 0.08,
                chase_limit: 95.0,
                speed: 320.0,
            },
            movement_phase2: None,
            movement_enrage: None,
            strike_speed_scale: 1.0,
            macro_tuning: crate::brain::BossMacroTuning::disabled(),
            attacks: vec![
                BossAttackProfile::WingSweep,
                BossAttackProfile::DiveLane,
                BossAttackProfile::Broadside,
            ],
            attack_cooldown: 1.05,
            attack_windup: 0.44,
            attack_active: 0.28,
            attack_damage: 2,
            body_damage: 1,
            attack_pattern: BossAttackPattern::Cycle,
            attack_origin_offset: ae::Vec2::ZERO,
        }
    }

    /// GNU-ton: stationary giant with wide-ranging hand attacks.
    ///
    /// Unique pacing among bosses: scripted timeline with explicit *rest*
    /// beats between strikes so the player can read each windup, react,
    /// and learn the sequence. The other bosses (clockwork_warden,
    /// mockingbird) keep the fast `Cycle` rhythm — the contrast itself is
    /// the design intent. GNU-ton should feel like a slow, deliberate
    /// monolith; the other bosses feel like dueling opponents.
    ///
    /// Phase pacing (longer than other bosses by design):
    /// - Intro: single show-of-force slam (no rest after) to set tone
    /// - Phase 1: ~9s — slam → rest → sweep → rest → slam → long rest
    /// - Transition: ~3s pure rest (player gets a breath)
    /// - Phase 2: ~12s — adds head-descent windows where the head is
    ///   exposed and vulnerable, framed by long rests so the player
    ///   can punish during the descent and then reset
    /// - Enrage: ~8s — shockwave + double slam, shorter rests
    pub fn gnu_ton() -> Self {
        Self {
            id: "gnu_ton".into(),
            // The sprite is huge, but the boss entity itself is anchored to
            // the shoulder ridge under the scholar. GNU-ton's damaging and
            // vulnerable regions are generated from named sprite parts, so
            // this combat size is only the movement/placeholder envelope.
            combat_size: Some(ae::Vec2::new(220.0, 220.0)),
            movement: BossMovementProfile::StationaryGiant {
                sway_amplitude: 6.0,
                sway_frequency: 0.28,
                speed: 40.0,
            },
            movement_phase2: None,
            movement_enrage: None,
            strike_speed_scale: 1.0,
            macro_tuning: crate::brain::BossMacroTuning::disabled(),
            // Legacy `attacks` is unused for Scripted bosses — keep it for
            // diagnostics so `boss inspect` style tooling can still list
            // the attack vocabulary.
            attacks: vec![
                BossAttackProfile::GnuHandSlam,
                BossAttackProfile::GnuHandSweep,
                BossAttackProfile::GnuHeadDescent,
                BossAttackProfile::GnuShockwave,
                BossAttackProfile::GnuAppleRain,
            ],
            attack_cooldown: 0.0,
            attack_windup: 0.0,
            attack_active: 0.0,
            attack_damage: 2,
            body_damage: 0, // no contact damage from the offscreen body
            attack_pattern: BossAttackPattern::Scripted {
                intro: BossPattern {
                    steps: vec![
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 1.6,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.55,
                        },
                        BossPatternStep::Rest { duration: 1.4 },
                    ],
                },
                phase1: BossPattern {
                    steps: vec![
                        // Hand slam from above, long telegraph so the player
                        // sees the arms rise.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 1.6,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.55,
                        },
                        BossPatternStep::Rest { duration: 1.2 },
                        // Side sweep — a totally different motion / hitbox shape.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 1.4,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 0.50,
                        },
                        BossPatternStep::Rest { duration: 1.0 },
                        // Apple rain: the scholar gestures up and apples
                        // fall around the player. Strike window is long
                        // enough to drop ~4 apples at the chosen interval.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuAppleRain,
                            duration: 1.0,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuAppleRain,
                            duration: 2.2,
                        },
                        BossPatternStep::Rest { duration: 1.0 },
                        // Repeat the slam to reward memorizers.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 1.6,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.55,
                        },
                        // Long breather closes the cycle.
                        BossPatternStep::Rest { duration: 1.8 },
                    ],
                },
                transition: BossPattern {
                    steps: vec![BossPatternStep::Rest { duration: 3.0 }],
                },
                phase2: BossPattern {
                    steps: vec![
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 1.4,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.55,
                        },
                        BossPatternStep::Rest { duration: 1.0 },
                        // Long head-descent: head is the vulnerable target;
                        // duration matches the score so the music's
                        // "harpsichord exposure" beat lands in this window.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHeadDescent,
                            duration: 1.8,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHeadDescent,
                            duration: 1.4,
                        },
                        BossPatternStep::Rest { duration: 1.4 },
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 1.4,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 0.50,
                        },
                        BossPatternStep::Rest { duration: 2.0 },
                    ],
                },
                enrage: BossPattern {
                    steps: vec![
                        // Faster pace, no head exposure: it's punishing.
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.90,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSlam,
                            duration: 0.45,
                        },
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 0.90,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuHandSweep,
                            duration: 0.45,
                        },
                        BossPatternStep::Rest { duration: 0.6 },
                        BossPatternStep::Telegraph {
                            profile: BossAttackProfile::GnuShockwave,
                            duration: 1.10,
                        },
                        BossPatternStep::Strike {
                            profile: BossAttackProfile::GnuShockwave,
                            duration: 0.70,
                        },
                        BossPatternStep::Rest { duration: 1.2 },
                    ],
                },
            },
            attack_origin_offset: ae::Vec2::ZERO,
        }
    }

    pub fn generic(id: impl Into<String>) -> Self {
        let mut profile = Self::clockwork_warden();
        profile.id = id.into();
        profile
    }

    pub fn for_authored_boss(id_or_name: &str) -> Self {
        let key = crate::boss_encounter::encounter_id_from_name(id_or_name);
        match key.as_str() {
            "mockingbird" => Self::mockingbird(),
            "clockwork_warden" | "gradient_sentinel" => Self::clockwork_warden(),
            "gnu_ton" => Self::gnu_ton(),
            other => Self::generic(other),
        }
    }
}

/// Resolve a boss's *canonical encounter id* from its authored
/// LDtk name + parsed brain payload.
///
/// The room author may set the display name to something flavorful
/// like "System Boss" while the brain points at the canonical
/// boss kind via `PhaseScript:clockwork_warden`. Without this
/// helper the encounter pipeline derives the id from the display
/// name only — `encounter_id_from_name("System Boss")` =
/// `"system_boss"` — and falls back to a generic boss profile
/// (empty music tracks, default behavior). Use this helper any
/// time you need the boss kind for behavior / profile / music
/// lookup; prefer `boss.behavior.id` when you already have a live
/// `BossRuntime`.
///
/// Resolution order:
/// 1. `BossBrain::PhaseScript { script_id }` with non-empty
///    `script_id` — the brain explicitly names the boss kind.
/// 2. `BossBrain::Custom(label)` with a non-empty label — same
///    intent, weaker contract.
/// 3. `encounter_id_from_name(authored_name)` — legacy fallback.
pub fn canonical_boss_id_from(name: &str, brain: &ae::BossBrain) -> String {
    match brain {
        ae::BossBrain::PhaseScript { script_id } if !script_id.is_empty() => script_id.clone(),
        ae::BossBrain::Custom(label) if !label.is_empty() => {
            crate::boss_encounter::encounter_id_from_name(label)
        }
        _ => crate::boss_encounter::encounter_id_from_name(name),
    }
}

/// Boss-side resolver for `Special`-flavored `BossAttackProfile`s.
///
/// The Gradient Sentinel carries multiple distinct specials
/// (OverfitVolley, MinimaTrap, SaddlePoint, GradientCascade) — more
/// than the single `ActionSet::special` slot can express. Rather
/// than grow `ActionSet` or `ActorControlFrame` for one boss, the
/// `tick_boss_brains_system` calls this function when the brain
/// commits to a special-flavored profile and writes the resulting
/// `ActorActionMessage::Special { spec }` directly via
/// `MessageWriter`. The boss's `ActionSet.special` is set to `None`
/// for multi-special bosses so the generic resolver doesn't fire a
/// duplicate.
///
/// `None` means the profile doesn't have a registered special spec
/// — the consumer should treat that as a no-op (defensive against
/// schedule edits that introduce a profile before the spec wiring
/// lands).
pub fn boss_special_for_profile(
    profile: &crate::brain::BossAttackProfile,
    boss: &BossRuntime,
) -> Option<crate::brain::SpecialActionSpec> {
    use crate::brain::{BossAttackProfile, SpecialActionSpec};
    match profile {
        BossAttackProfile::GnuAppleRain => Some(SpecialActionSpec::GnuAppleRain {
            interval_s: APPLE_RAIN_INTERVAL,
            spawn_speed: APPLE_RAIN_SPAWN_SPEED,
            damage: APPLE_RAIN_DAMAGE,
        }),
        BossAttackProfile::OverfitVolley => Some(SpecialActionSpec::OverfitVolley {
            sample_interval_s: OVERFIT_VOLLEY_SAMPLE_INTERVAL_S,
            sample_count: OVERFIT_VOLLEY_SAMPLE_COUNT,
            shot_speed: OVERFIT_VOLLEY_SHOT_SPEED,
            damage: OVERFIT_VOLLEY_SHOT_DAMAGE,
        }),
        BossAttackProfile::MinimaTrap => Some(SpecialActionSpec::MinimaTrap {
            hazard_duration_s: MINIMA_TRAP_HAZARD_DURATION_S,
            damage: MINIMA_TRAP_DAMAGE,
            half_extent_x: MINIMA_TRAP_HALF_EXTENT_X,
            half_extent_y: MINIMA_TRAP_HALF_EXTENT_Y,
            spawn_minion: true,
        }),
        BossAttackProfile::SaddlePoint => Some(SpecialActionSpec::SaddlePoint {
            arm_length: SADDLE_POINT_ARM_LENGTH,
            arm_thickness: SADDLE_POINT_ARM_THICKNESS,
            axis_period_s: SADDLE_POINT_AXIS_PERIOD_S,
            damage: SADDLE_POINT_DAMAGE,
        }),
        BossAttackProfile::GradientCascade => Some(SpecialActionSpec::GradientCascade {
            minion_count: GRADIENT_CASCADE_MINION_COUNT,
        }),
        // Ordinary melee profiles never route through this resolver
        // (they damage via `boss_attack_damage` reading `BossAttackState`
        // directly). The `_` arm keeps this function the single
        // source of truth for *which* special spec each profile maps to.
        _ => {
            let _ = boss; // future per-boss tuning may read it
            None
        }
    }
}

#[cfg(test)]
mod canonical_boss_id_tests {
    use super::*;

    /// PhaseScript brain wins over display name. The user-reported
    /// bug: BossSpawn named "System Boss" in `first_system_boss`
    /// derived encounter_id "system_boss" (no profile, no music).
    /// With `canonical_boss_id_from` reading the brain's
    /// `PhaseScript:clockwork_warden` it resolves to the
    /// authored profile and the boss fight gets its violin music.
    #[test]
    fn phase_script_brain_wins_over_display_name() {
        let id = canonical_boss_id_from(
            "System Boss",
            &ae::BossBrain::PhaseScript {
                script_id: "clockwork_warden".to_string(),
            },
        );
        assert_eq!(id, "clockwork_warden");
    }

    /// Empty PhaseScript falls back to the display name.
    #[test]
    fn empty_phase_script_falls_back_to_name() {
        let id = canonical_boss_id_from(
            "System Boss",
            &ae::BossBrain::PhaseScript {
                script_id: String::new(),
            },
        );
        assert_eq!(id, "system_boss");
    }

    /// Custom brain with a non-empty label is treated like a name
    /// (gets normalized to an encounter_id slug).
    #[test]
    fn custom_brain_label_becomes_encounter_id_slug() {
        let id = canonical_boss_id_from(
            "Display",
            &ae::BossBrain::Custom("Clockwork Warden".to_string()),
        );
        assert_eq!(id, "clockwork_warden");
    }

    /// Dormant brain falls back to the display name.
    #[test]
    fn dormant_brain_falls_back_to_name() {
        let id = canonical_boss_id_from("Clockwork Warden", &ae::BossBrain::Dormant);
        assert_eq!(id, "clockwork_warden");
    }

    /// BossRuntime constructed with a "System Boss" name + PhaseScript
    /// brain ends up with the clockwork_warden behavior — the runtime
    /// resolves the canonical id before reading
    /// `BossBehaviorProfile::for_authored_boss`. Without this fix the
    /// runtime would carry a generic placeholder behavior.
    #[test]
    fn boss_runtime_uses_phase_script_for_behavior_lookup() {
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(40.0, 50.0));
        let boss = BossRuntime::new(
            "boss_under_test",
            "System Boss",
            aabb,
            ae::BossBrain::PhaseScript {
                script_id: "clockwork_warden".to_string(),
            },
        );
        assert_eq!(boss.behavior.id, "clockwork_warden");
        // Sanity: the Gradient Sentinel macro tuning is non-trivial
        // (chase/retreat thresholds non-zero), which the generic
        // boss profile doesn't set.
        assert!(
            boss.behavior.macro_tuning.is_enabled(),
            "clockwork_warden behavior should carry macro tuning",
        );
    }
}

#[cfg(test)]
mod boss_special_resolver_tests {
    use super::*;

    fn gnu_ton_runtime_fixture() -> BossRuntime {
        let aabb = ae::Aabb::new(ae::Vec2::new(500.0, 400.0), ae::Vec2::new(110.0, 110.0));
        let mut runtime = BossRuntime::new("boss_gnu_ton", "GNU-ton", aabb, ae::BossBrain::Dormant);
        runtime.behavior = BossBehaviorProfile::gnu_ton();
        runtime
    }

    fn gradient_sentinel_runtime_fixture() -> BossRuntime {
        let aabb = ae::Aabb::new(ae::Vec2::new(640.0, 696.0), ae::Vec2::new(64.0, 80.0));
        let mut runtime = BossRuntime::new(
            "boss_gradient_sentinel",
            "Gradient Sentinel",
            aabb,
            ae::BossBrain::Dormant,
        );
        runtime.behavior = BossBehaviorProfile::clockwork_warden();
        runtime
    }

    /// Every special-flavored profile must map to a Some(spec) — otherwise
    /// the boss tick will emit no Special message for that beat and the
    /// schedule silently degrades. Pin the mapping so future schedule
    /// edits can't introduce a profile without its consumer wiring.
    #[test]
    fn every_special_profile_resolves_to_a_spec_for_gradient_sentinel() {
        use crate::brain::BossAttackProfile;
        let boss = gradient_sentinel_runtime_fixture();
        for profile in [
            BossAttackProfile::OverfitVolley,
            BossAttackProfile::MinimaTrap,
            BossAttackProfile::SaddlePoint,
            BossAttackProfile::GradientCascade,
        ] {
            assert!(
                boss_special_for_profile(&profile, &boss).is_some(),
                "{profile:?} must resolve to a spec for Gradient Sentinel",
            );
        }
    }

    /// GNU-ton's apple rain still resolves through the new path so
    /// the consumer (`spawn_gnu_apple_rain_from_special_messages`)
    /// keeps receiving messages after the migration.
    #[test]
    fn gnu_apple_rain_profile_resolves_to_apple_rain_spec_for_gnu_ton() {
        use crate::brain::{BossAttackProfile, SpecialActionSpec};
        let boss = gnu_ton_runtime_fixture();
        match boss_special_for_profile(&BossAttackProfile::GnuAppleRain, &boss) {
            Some(SpecialActionSpec::GnuAppleRain {
                interval_s,
                spawn_speed,
                damage,
            }) => {
                assert!((interval_s - APPLE_RAIN_INTERVAL).abs() < f32::EPSILON);
                assert!((spawn_speed - APPLE_RAIN_SPAWN_SPEED).abs() < f32::EPSILON);
                assert_eq!(damage, APPLE_RAIN_DAMAGE);
            }
            other => panic!("expected GnuAppleRain spec, got {other:?}"),
        }
    }

    /// Ordinary melee-style profiles return None — they don't go
    /// through the Special path; their damage routes via
    /// `boss_attack_damage` reading `BossAttackState` directly.
    #[test]
    fn ordinary_profiles_resolve_to_none() {
        use crate::brain::BossAttackProfile;
        let boss = gradient_sentinel_runtime_fixture();
        for profile in [
            BossAttackProfile::FloorSlam,
            BossAttackProfile::SideSweep,
            BossAttackProfile::FullBodyPulse,
            BossAttackProfile::GradientLane,
        ] {
            assert!(
                boss_special_for_profile(&profile, &boss).is_none(),
                "{profile:?} should not have a Special spec",
            );
        }
    }
}

// `step_duration` moved to `crate::brain::boss_pattern`.

/// Live boss state owned by the simulation: body, HP, alive flag,
/// encounter-phase mirror, and a few cosmetic-timer scalars.
/// **Attack policy and attack execution state live elsewhere:** the
/// brain layer's `BossPatternState` owns the cursor / clocks and the
/// `BossAttackState` component owns the live telegraph/active
/// profile. `BossRuntime` carries body fields only.
/// Snapshot of the sprite generator's `body_metrics` for a boss,
/// captured once at sprite-registry lookup time so per-tick
/// damage/hurtbox math doesn't re-query the SheetRegistry resource.
///
/// `body_pixel_bbox` is the single overall body bbox (legacy /
/// single-piece bosses). `body_pixel_parts` is the multi-rect
/// representation for disjointed-piece bosses (head + body + arms).
/// Either one or both may be populated; the consumer picks parts
/// when present and falls back to bbox otherwise.
///
/// `frame_width` / `frame_height` are the sprite-frame dimensions
/// (e.g. 128×128 for clockwork_warden) used to scale pixel-space
/// coordinates into world-space via the boss's render size.
///
/// `sprite_render_size` is the world-space extent of the rendered
/// sprite quad — i.e. `BossSheetSpec::render_size(boss.size)`. The
/// hurtbox / hitbox math uses this (NOT `boss.size`) as the world
/// scale so the cyan / red / yellow boxes line up with the visible
/// sprite. Without this distinction, the boss spawns at LDtk size
/// (e.g. 128×160) but renders 1.6× bigger (~256×256), and the boxes
/// end up half the size of the visible body.
#[derive(Clone, Debug, Default)]
pub struct BossSpriteMetrics {
    pub frame_width: u32,
    pub frame_height: u32,
    pub body_pixel_bbox: Option<crate::presentation::character_sprites::registry::PixelRect>,
    pub body_pixel_parts: Vec<crate::presentation::character_sprites::registry::NamedPixelRect>,
    /// World-space extent of the rendered sprite quad. Equal to
    /// `BossSheetSpec::render_size(boss.size)` at derivation time.
    /// Falls back to `(boss.size, boss.size)` when the sprite spec
    /// isn't known (test fixtures); consumers treat zero as
    /// "no render size yet, use ctx.size".
    pub sprite_render_size: ae::Vec2,
    /// Per-animation `{hurtbox, hitbox}` data keyed by animation
    /// name (matches the spritesheet rows: `"rest"`,
    /// `"floor_slam"`, `"side_sweep"`, …). The renderer fills
    /// `hurtbox` from each animation's union alpha-bbox; the
    /// adapter declares `hitbox` rects for attack animations.
    /// Consumers (`damageable_volumes`, `volumes_for_profile`)
    /// look up by current animation name to scale hurtboxes /
    /// hitboxes with the on-screen sprite pose.
    pub animations: std::collections::HashMap<
        String,
        crate::presentation::character_sprites::registry::AnimationMetrics,
    >,
}

impl BossSpriteMetrics {
    /// True iff this snapshot carries at least one rectangle the
    /// derivation can use.
    pub fn has_body(&self) -> bool {
        !self.body_pixel_parts.is_empty() || self.body_pixel_bbox.is_some()
    }

    /// Per-animation hurtbox lookup. Used by `damageable_volumes`
    /// to size the hurtbox to the *currently-playing* animation
    /// (so attack frames with extended arms get a wider hurtbox
    /// than the rest pose). Returns `None` if the animation has
    /// no per-animation override; the caller falls back to
    /// `body_pixel_parts` / `body_pixel_bbox`.
    pub fn hurtbox_for_animation(
        &self,
        animation: &str,
    ) -> Option<&crate::presentation::character_sprites::registry::AnimationBox> {
        self.animations.get(animation)?.hurtbox.as_ref()
    }

    /// Per-animation hitbox lookup. Used by `volumes_for_profile`
    /// to read the sprite-author-declared damage geometry for an
    /// attack animation (so a side-sweep's hitbox covers both
    /// extended arms, not the generic bounding rect). Returns
    /// `None` if the animation has no authored hitbox; the
    /// caller falls back to its hardcoded volume math.
    pub fn hitbox_for_animation(
        &self,
        animation: &str,
    ) -> Option<&crate::presentation::character_sprites::registry::AnimationBox> {
        self.animations.get(animation)?.hitbox.as_ref()
    }
}

/// Map a [`crate::brain::BossAttackProfile`] to the boss sprite's
/// animation name. Used by `volumes_for_profile` /
/// `damageable_volumes` to look up per-animation hit + hurt box
/// data in [`BossSpriteMetrics::animations`]. Returns `None` for
/// profiles whose sheet isn't the AI-Slop-Zeta clockwork
/// (mockingbird / gnu_ton); the consumer falls back to its
/// hardcoded math in that case.
pub fn boss_animation_for_profile(
    profile: &crate::brain::BossAttackProfile,
) -> Option<&'static str> {
    use crate::brain::BossAttackProfile;
    match profile {
        BossAttackProfile::FloorSlam => Some("floor_slam"),
        BossAttackProfile::SideSweep => Some("side_sweep"),
        BossAttackProfile::FullBodyPulse => Some("spike_halo"),
        BossAttackProfile::GradientLane => Some("dash_echo"),
        // Gradient Sentinel specials don't have a dedicated row
        // in the AI-Slop-Zeta sheet; route them to `spike_halo`
        // (closest visual: a ring of damage around the boss) so
        // the player still sees an anim cue during the strike.
        BossAttackProfile::OverfitVolley
        | BossAttackProfile::MinimaTrap
        | BossAttackProfile::SaddlePoint
        | BossAttackProfile::GradientCascade => Some("spike_halo"),
        // Other-boss profiles (mockingbird / gnu_ton) aren't part
        // of the clockwork sheet; return None so the consumer
        // falls back to hardcoded math if they accidentally land
        // on a clockwork-sheet boss.
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct BossRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub spawn: ae::Vec2,
    pub size: ae::Vec2,
    pub health: ae::Health,
    pub brain: ae::BossBrain,
    pub behavior: BossBehaviorProfile,
    pub alive: bool,
    pub hit_flash: f32,
    /// Active encounter phase. Forwarded by `sync_boss_encounter_phase`
    /// from `BossEncounterRegistry`. `Dormant` until the encounter
    /// wakes up. The brain reads this via `BossPatternContext`;
    /// pattern selection happens in the brain, not here.
    pub encounter_phase: ae::BossEncounterPhase,
    /// Sprite-driven body metrics — populated by the
    /// `derive_boss_sprite_metrics` system after the SheetRegistry
    /// has loaded. `None` for bosses whose sprite has no
    /// `body_metrics` entry (the derivation system leaves them
    /// alone), and the legacy `combat_size` path applies.
    pub sprite_metrics: Option<BossSpriteMetrics>,
}

impl BossRuntime {
    pub(crate) fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: ae::BossBrain,
    ) -> Self {
        let name = name.into();
        // Behavior lookup prefers the brain's `PhaseScript:` id
        // over the LDtk display name. A room whose BossSpawn is
        // named "System Boss" but whose brain is
        // `PhaseScript:clockwork_warden` should still resolve to
        // the clockwork_warden / Gradient Sentinel profile — not
        // a generic placeholder.
        let canonical_id = canonical_boss_id_from(&name, &brain);
        Self {
            id: id.into(),
            pos: aabb.center(),
            spawn: aabb.center(),
            size: aabb.half_size() * 2.0,
            health: ae::Health::new(18),
            behavior: BossBehaviorProfile::for_authored_boss(&canonical_id),
            sprite_metrics: None,
            name,
            brain,
            alive: true,
            hit_flash: 0.0,
            encounter_phase: ae::BossEncounterPhase::Dormant,
        }
    }

    /// `target_pos` is populated from the boss entity's `ActorTarget`
    /// component by `select_actor_targets` (OVERNIGHT-TODO #17.8).
    /// The boss movement profile reads it for anchor-sway / air-swoop
    /// chase math; scripted patterns (`StationaryGiant`) ignore it.
    /// Integrate the boss's body using the brain-emitted `desired_vel`
    /// from `ActorControl`. **Integration only** — the brain
    /// (`tick_boss_brains_system` → `boss_pattern::tick_boss_pattern`)
    /// owns the policy decision and writes `ActorControl` upstream;
    /// this method only translates that desired velocity into a
    /// collision-resolved position change.
    ///
    /// `BossRuntime::update` (the old policy + integration combo)
    /// was deleted by the "move boss policy out of BossRuntime"
    /// migration. The runtime no longer ticks the scripted cursor
    /// or chooses Telegraph/Strike/Rest; it just integrates the
    /// velocity the brain produced.
    pub fn integrate_body(&mut self, world: &ae::World, desired_vel: ae::Vec2, dt: f32) {
        if !self.alive || dt <= 0.0 {
            return;
        }
        // Bosses float (gravity = 0, max_fall_speed = 0). Multi-part
        // bosses like GNU-ton expose a `combat_size` distinct from
        // the sprite `size`; that's the size we collide against.
        let mut body = ae::KinematicBody {
            pos: self.pos,
            vel: desired_vel,
            size: self.combat_size(),
            on_ground: false,
            facing: 1.0,
        };
        ae::step_kinematic(
            &mut body,
            world,
            ae::KinematicTuning {
                gravity: 0.0,
                max_fall_speed: 0.0,
            },
            ae::KinematicInputs {
                drop_through: false,
            },
            dt,
        );
        self.pos = body.pos;
        self.hit_flash = (self.hit_flash - dt).max(0.0);
    }

    // `tick_runtime_clocks`, `tick_apple_rain`, `update_scripted_attacks`,
    // `update_cycle_attacks`, `pattern_timer`, `movement_timer`,
    // `attack_windup_timer`, `attack_timer`, `attack_cooldown`,
    // `active_strike_profile`, `telegraph_profile` all moved out of
    // `BossRuntime` and into the brain layer:
    //
    // * Cursor / clocks / pattern-step decision live in
    //   `crate::brain::boss_pattern::{BossPatternCfg, BossPatternState,
    //   tick_boss_pattern}` (brain).
    // * Live telegraph/active profile + remaining time live on the
    //   `BossAttackState` component (still in brain).
    // * Volume math is pure functions in
    //   `crate::content::features::boss_attack_geometry`
    //   (`active_attack_volumes`, `telegraph_volumes`,
    //   `damageable_volumes`, `volumes_for_profile`, `body_damage_aabb`).
    // * Boss → player damage is the pure `boss_attack_damage` helper
    //   in the same module; `update_ecs_bosses` calls it from a
    //   `BossVolumeContext` built off `BossRuntime` + `BossAttackState`.
    //
    // Anything that needs to look at "what attack is live right now?"
    // queries `BossAttackState` directly, not `BossRuntime`.

    pub fn is_mockingbird(&self) -> bool {
        self.behavior.id == "mockingbird" || self.name.eq_ignore_ascii_case("mockingbird")
    }

    pub fn is_gnu_ton(&self) -> bool {
        self.behavior.id == "gnu_ton"
            || self.name.eq_ignore_ascii_case("gnu_ton")
            || self.name.eq_ignore_ascii_case("gnu-ton")
    }

    pub fn render_size(&self) -> ae::Vec2 {
        self.size
    }

    /// World-space anchor for a combat-banter speech bubble. For GNU-ton the
    /// scholar sits on the right shoulder — offset slightly right and not as
    /// high as the body top so the bubble appears near the character, not
    /// floating above the beast's head.
    pub fn bark_anchor(&self) -> ae::Vec2 {
        if self.is_gnu_ton() {
            let half_h = self.combat_size().y * 0.5;
            ae::Vec2::new(self.pos.x + 38.0, self.pos.y - half_h * 0.55 - 18.0)
        } else {
            let half_h = self.combat_size().y * 0.5;
            ae::Vec2::new(self.pos.x, self.pos.y - half_h - 20.0)
        }
    }

    pub fn apply_behavior_profile(&mut self, behavior: BossBehaviorProfile) {
        self.behavior = behavior;
    }

    pub fn combat_size(&self) -> ae::Vec2 {
        self.behavior.combat_size.unwrap_or(self.size)
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.combat_size() * 0.5)
    }

    // All attack-volume / telegraph-volume / damageable-volume /
    // player_damage / cycle_pattern_volumes / volumes_for /
    // gnu_ton_part_aabb / body_damage_aabb methods moved out of
    // `BossRuntime`. They are now pure functions in
    // `crate::content::features::boss_attack_geometry` that take a
    // `BossVolumeContext` (built from `&BossRuntime` + `&BossAttackState`)
    // and read the brain's `BossAttackState` instead of mirror fields.
    //
    // If you need "the boss's live hitbox volumes right now" call
    // `features::active_attack_volumes(&BossVolumeContext::from_runtime(boss, attack_state))`.
}

#[cfg(test)]
mod scripted_pattern_tests {
    use super::*;
    use ambition_engine as ae;

    fn gnu_ton_runtime() -> BossRuntime {
        let behavior = BossBehaviorProfile::gnu_ton();
        let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(220.0, 220.0));
        let pos = ae::Vec2::new(500.0, 400.0);
        let aabb = ae::Aabb::new(pos, combat_size * 0.5);
        let mut runtime = BossRuntime::new("boss_gnu_ton", "GNU-ton", aabb, ae::BossBrain::Dormant);
        runtime.behavior = behavior;
        runtime.encounter_phase = ae::BossEncounterPhase::Phase1;
        runtime
    }

    #[test]
    fn gnu_ton_pattern_includes_explicit_rest_beats_in_every_phase() {
        let BossAttackPattern::Scripted {
            phase1,
            transition,
            phase2,
            enrage,
            ..
        } = BossBehaviorProfile::gnu_ton().attack_pattern
        else {
            panic!("gnu_ton must use a Scripted attack pattern");
        };
        for (label, pattern) in [
            ("phase1", &phase1),
            ("transition", &transition),
            ("phase2", &phase2),
            ("enrage", &enrage),
        ] {
            let has_rest = pattern
                .steps
                .iter()
                .any(|step| matches!(step, BossPatternStep::Rest { .. }));
            assert!(
                has_rest,
                "{label} pattern must include at least one Rest beat so the \
                 player has breathing room — got steps {:?}",
                pattern.steps
            );
        }
    }

    #[test]
    fn gnu_ton_phase1_is_materially_longer_than_other_bosses() {
        let gnu_phase1 = match BossBehaviorProfile::gnu_ton().attack_pattern {
            BossAttackPattern::Scripted { phase1, .. } => phase1.total_duration(),
            _ => unreachable!(),
        };
        let warden = BossBehaviorProfile::clockwork_warden();
        let warden_cycle = warden.attack_windup + warden.attack_active + warden.attack_cooldown;
        assert!(
            gnu_phase1 > warden_cycle * 3.0,
            "gnu_ton phase1 ({gnu_phase1}s) should be much slower than the \
             clockwork warden cycle ({warden_cycle}s) — design intent is a \
             deliberate, memorizable rhythm"
        );
    }

    // `gnu_ton_scripted_advance_cycles_telegraph_strike_rest` deleted:
    // the cursor-through-steps invariant moved to
    // `brain::boss_pattern::tests::{boss_pattern_telegraph_step_updates_telegraph_profile_state,
    // boss_pattern_strike_step_emits_melee_intent,
    // boss_pattern_resets_cursor_on_phase_change}`. The runtime no
    // longer ticks the cursor (the brain does), so polling
    // `boss.update(...)` and reading `boss.telegraph_profile` is no
    // longer a meaningful exercise — those mirror fields are written
    // by the boss tick system, not advanced by the runtime.

    #[test]
    fn gnu_ton_hand_slam_anchors_to_drawn_hands() {
        // GNU-ton's transform sits on the shoulder ridge. Hand-slam
        // hitboxes should land *below* the shoulder (positive y) and on
        // opposite sides of it (one to the left, one to the right), no
        // matter how the sprite is resized. Earlier revisions pinned
        // these to absolute world-pixel thresholds (>400, >300) tuned to
        // a 384-tall frame; bumping the source PNG to 768×576 silently
        // broke the test even though the visual / hitbox correspondence
        // stayed correct. Stick to invariants instead of magic numbers.
        let boss = gnu_ton_runtime();
        let slam = crate::features::volumes_for_profile(
            &BossAttackProfile::GnuHandSlam,
            boss.pos,
            boss.size,
            boss.combat_size(),
            &boss.behavior,
            true,
        );
        assert_eq!(slam.len(), 2);
        let (left, right) = if slam[0].center().x < slam[1].center().x {
            (&slam[0], &slam[1])
        } else {
            (&slam[1], &slam[0])
        };
        assert!(left.center().x < boss.pos.x, "{slam:?}");
        assert!(right.center().x > boss.pos.x, "{slam:?}");
        assert!(left.center().y > boss.pos.y, "{slam:?}");
        assert!(right.center().y > boss.pos.y, "{slam:?}");
    }

    #[test]
    fn gnu_ton_body_contact_does_not_damage_player() {
        // `body_damage: 0` on the gnu_ton behavior is the authored
        // statement "no contact damage from the offscreen body". A prior
        // revision still dealt 1 damage because `player_damage` used
        // `body_damage.max(1)` after the intersect test. Now guarded by
        // the `body_damage > 0` check inside `boss_attack_damage`.
        // Concrete repro: a player AABB identical to the boss body
        // AABB with no active strike must produce no event.
        let boss = gnu_ton_runtime();
        let attack_state = crate::brain::BossAttackState::default();
        let ctx = crate::features::BossVolumeContext::from_runtime(&boss, &attack_state);
        let player_body = crate::features::body_damage_aabb(boss.pos, boss.combat_size());
        assert!(
            crate::features::boss_attack_damage(&ctx, player_body).is_none(),
            "gnu_ton must not deal contact damage when body_damage = 0"
        );
    }

    // `gnu_ton_scripted_patterns_skip_non_attacking_phases` deleted:
    // the "Dormant / Stagger / Death emit neutral intent + clear
    // attack-state mirror" invariant moved to
    // `brain::boss_pattern::tests::boss_pattern_brain_emits_neutral_in_non_attacking_phase`.
    // The runtime no longer chooses the pattern step, so polling
    // `boss.update(...)` and reading the mirror fields is no longer
    // the right exercise — the brain owns the gate.

    // The `gnu_ton_apple_rain_strike_emits_falling_apple_spawns`,
    // `gnu_ton_apple_rain_spawns_avoid_self_aabb`,
    // `gnu_ton_apple_rain_spawns_cover_full_arena_width`, and
    // `gnu_ton_apple_rain_resets_accumulator_when_strike_ends` tests
    // were deleted with Task B of the actor/brain follow-up plan.
    // They tested `BossRuntime::tick_apple_rain` directly, which no
    // longer exists. The same invariants (downward gravity, owner
    // prefix, self-aabb dodge, full-width coverage, reset-on-leave)
    // are now exercised in
    // `content/features/ecs/brain_effects.rs::tests` against the
    // EFFECTS consumer `spawn_gnu_apple_rain_from_special_messages`.

    #[test]
    fn gnu_ton_apple_rain_volumes_are_empty_so_contact_does_not_double_count() {
        // The strike's damage path goes through enemy projectiles, not
        // a stationary boss AABB. `volumes_for_profile(GnuAppleRain, …)`
        // must return an empty list so the regular contact-damage
        // check in `boss_attack_damage` doesn't ALSO hit the player
        // at the boss's position while apples are in flight.
        let boss = gnu_ton_runtime();
        assert!(
            crate::features::volumes_for_profile(
                &BossAttackProfile::GnuAppleRain,
                boss.pos,
                boss.size,
                boss.combat_size(),
                &boss.behavior,
                true,
            )
            .is_empty(),
            "apple-rain volumes must be empty — damage routes through projectiles"
        );
    }

    #[test]
    fn gnu_ton_head_is_always_damageable_but_descent_brings_it_lower() {
        // The head is always a valid hit target — the older "only
        // damageable during head_descent strike" rule made the boss
        // permanently invulnerable in Phase1 (no descent beat) and
        // therefore unkillable. Now the head is always hittable; the
        // descent window (signaled by `BossAttackState.active_profile
        // == GnuHeadDescent`) just moves it down to player level so
        // the player doesn't have to climb. Both states must produce
        // exactly one head AABB.
        let boss = gnu_ton_runtime();
        let mut attack_state = crate::brain::BossAttackState::default();
        let rest_head = crate::features::damageable_volumes(
            &crate::features::BossVolumeContext::from_runtime(&boss, &attack_state),
        );
        assert_eq!(
            rest_head.len(),
            1,
            "head must always be a damageable target"
        );
        let rest_y = rest_head[0].center().y;
        // Rest head sits ABOVE the shoulder anchor (player must climb).
        assert!(
            rest_y < boss.pos.y,
            "rest head should be above the shoulder anchor, got y={rest_y} vs pos.y={}",
            boss.pos.y
        );

        attack_state.active_profile = Some(BossAttackProfile::GnuHeadDescent);
        let descent_head = crate::features::damageable_volumes(
            &crate::features::BossVolumeContext::from_runtime(&boss, &attack_state),
        );
        assert_eq!(descent_head.len(), 1);
        let descent_y = descent_head[0].center().y;
        // Descended head sits BELOW the shoulder anchor (at player level).
        assert!(
            descent_y > boss.pos.y,
            "descent head should be below the shoulder anchor"
        );
        // And materially lower than the rest position — that's the
        // whole point of the vulnerability window.
        assert!(
            descent_y > rest_y + 50.0,
            "descent must drop the head meaningfully (got rest_y={rest_y}, descent_y={descent_y})"
        );
    }

    // -------------------------------------------------------------
    // Gradient Sentinel (clockwork_warden) — Scripted schedule sanity
    // -------------------------------------------------------------
    //
    // The Gradient Sentinel boss flipped from `Cycle` to `Scripted`
    // with 4 phases (intro/phase1/transition/phase2/enrage). These
    // tests pin design invariants so future schedule edits can't
    // silently drop the rest-beat windows the player needs, drop a
    // special profile so the EFFECTS consumer never fires, or
    // accidentally make the encounter too short to learn.

    #[test]
    fn gradient_sentinel_uses_scripted_pattern() {
        let behavior = BossBehaviorProfile::clockwork_warden();
        match behavior.attack_pattern {
            BossAttackPattern::Scripted { .. } => {}
            BossAttackPattern::Cycle => {
                panic!("Gradient Sentinel should use Scripted, not Cycle");
            }
        }
    }

    #[test]
    fn gradient_sentinel_every_phase_includes_rest_beats() {
        let BossAttackPattern::Scripted {
            intro,
            phase1,
            transition,
            phase2,
            enrage,
        } = BossBehaviorProfile::clockwork_warden().attack_pattern
        else {
            panic!("expected Scripted attack pattern");
        };
        for (label, pattern) in [
            ("intro", &intro),
            ("phase1", &phase1),
            ("transition", &transition),
            ("phase2", &phase2),
            ("enrage", &enrage),
        ] {
            let has_rest = pattern
                .steps
                .iter()
                .any(|s| matches!(s, BossPatternStep::Rest { .. }));
            assert!(
                has_rest,
                "{label} pattern must include at least one Rest beat — got {:?}",
                pattern.steps
            );
        }
    }

    /// Phase 1 should teach the player the GradientLane + OverfitVolley
    /// profiles (the new fundamentals) before phase 2 layers in
    /// hazards + minions. Without this, the player wouldn't see
    /// these attacks until phase 2 and the difficulty curve would
    /// spike sharply.
    #[test]
    fn gradient_sentinel_phase1_includes_gradient_lane_and_overfit_volley() {
        use crate::brain::BossAttackProfile;
        let BossAttackPattern::Scripted { phase1, .. } =
            BossBehaviorProfile::clockwork_warden().attack_pattern
        else {
            panic!("expected Scripted");
        };
        let profiles: Vec<_> = phase1
            .steps
            .iter()
            .filter_map(|s| match s {
                BossPatternStep::Telegraph { profile, .. }
                | BossPatternStep::Strike { profile, .. } => Some(profile.clone()),
                _ => None,
            })
            .collect();
        assert!(
            profiles.contains(&BossAttackProfile::GradientLane),
            "phase1 must include GradientLane — got {profiles:?}"
        );
        assert!(
            profiles.contains(&BossAttackProfile::OverfitVolley),
            "phase1 must include OverfitVolley — got {profiles:?}"
        );
    }

    /// Phase 2 introduces the hazard + minion specials. These are
    /// the "advanced" attacks; if phase 2 doesn't include them, the
    /// encounter degenerates into "phase 1 forever, but slightly
    /// faster", which defeats the design.
    #[test]
    fn gradient_sentinel_phase2_includes_all_advanced_specials() {
        use crate::brain::BossAttackProfile;
        let BossAttackPattern::Scripted { phase2, .. } =
            BossBehaviorProfile::clockwork_warden().attack_pattern
        else {
            panic!("expected Scripted");
        };
        let profiles: Vec<_> = phase2
            .steps
            .iter()
            .filter_map(|s| match s {
                BossPatternStep::Telegraph { profile, .. }
                | BossPatternStep::Strike { profile, .. } => Some(profile.clone()),
                _ => None,
            })
            .collect();
        for required in [
            BossAttackProfile::MinimaTrap,
            BossAttackProfile::SaddlePoint,
            BossAttackProfile::GradientCascade,
        ] {
            assert!(
                profiles.contains(&required),
                "phase2 must include {required:?} — got {profiles:?}"
            );
        }
    }

    /// Every Strike profile in the schedule that `is_special()` must
    /// have a registered SpecialActionSpec via
    /// `boss_special_for_profile`. Otherwise the boss tick emits no
    /// Special message for that beat and the strike silently does
    /// nothing — the worst kind of design bug because the telegraph
    /// still plays.
    #[test]
    fn gradient_sentinel_every_special_strike_has_a_registered_spec() {
        let behavior = BossBehaviorProfile::clockwork_warden();
        let BossAttackPattern::Scripted {
            phase1,
            phase2,
            enrage,
            ..
        } = behavior.attack_pattern.clone()
        else {
            panic!("expected Scripted");
        };
        let aabb = ae::Aabb::new(ae::Vec2::new(640.0, 696.0), ae::Vec2::new(64.0, 80.0));
        let mut boss = BossRuntime::new(
            "boss_gradient_sentinel",
            "Gradient Sentinel",
            aabb,
            ae::BossBrain::Dormant,
        );
        boss.behavior = behavior;
        for (label, pattern) in [
            ("phase1", &phase1),
            ("phase2", &phase2),
            ("enrage", &enrage),
        ] {
            for step in &pattern.steps {
                if let BossPatternStep::Strike { profile, .. } = step {
                    if profile.is_special() {
                        assert!(
                            boss_special_for_profile(profile, &boss).is_some(),
                            "{label} strike of {profile:?} has no registered \
                             SpecialActionSpec — boss_special_for_profile must \
                             return Some so tick_boss_brains_system can emit \
                             the Special message",
                        );
                    }
                }
            }
        }
    }

    /// Every Telegraph step must be immediately followed by a Strike
    /// step for the SAME profile. Otherwise the player sees a windup
    /// for an attack that never fires (or fires a different one),
    /// which breaks the "telegraph teaches the strike shape" contract.
    #[test]
    fn gradient_sentinel_telegraph_steps_are_paired_with_matching_strike() {
        let BossAttackPattern::Scripted {
            intro,
            phase1,
            phase2,
            enrage,
            ..
        } = BossBehaviorProfile::clockwork_warden().attack_pattern
        else {
            panic!("expected Scripted");
        };
        for (label, pattern) in [
            ("intro", &intro),
            ("phase1", &phase1),
            ("phase2", &phase2),
            ("enrage", &enrage),
        ] {
            let mut iter = pattern.steps.iter().peekable();
            while let Some(step) = iter.next() {
                if let BossPatternStep::Telegraph { profile, .. } = step {
                    let next = iter.peek().unwrap_or_else(|| {
                        panic!("{label} ends on a Telegraph without a matching Strike")
                    });
                    match next {
                        BossPatternStep::Strike {
                            profile: strike_profile,
                            ..
                        } => {
                            assert_eq!(
                                profile, strike_profile,
                                "{label} Telegraph({profile:?}) must be followed by \
                                 Strike({profile:?}), got Strike({strike_profile:?})",
                            );
                        }
                        other => panic!(
                            "{label} Telegraph({profile:?}) must be followed by a \
                             Strike — got {other:?}",
                        ),
                    }
                }
            }
        }
    }

    /// Phase 1 should be appreciably longer than the legacy
    /// Cycle-mode loop so the player has enough time to learn the
    /// schedule. Lower bound is intentionally loose — tighter
    /// numerical checks belong in the design doc, not the test.
    #[test]
    fn gradient_sentinel_phase1_loop_is_substantial() {
        let BossAttackPattern::Scripted { phase1, .. } =
            BossBehaviorProfile::clockwork_warden().attack_pattern
        else {
            panic!("expected Scripted");
        };
        let total = phase1.total_duration();
        assert!(
            total >= 12.0,
            "phase1 loop should be at least 12s for memorability, got {total}s",
        );
        assert!(
            total <= 30.0,
            "phase1 loop shouldn't exceed 30s or each cycle drags, got {total}s",
        );
    }

    /// Bosses used to write `self.pos` via a bespoke per-axis sweep
    /// against `boss_space_is_free`. With the brain→sim seam they
    /// run through the SAME `step_kinematic` primitive every other
    /// actor uses — so a wall placed in the chase path blocks them
    /// at the wall instead of relying on a parallel-but-different
    /// collision code path. This guards against future regressions
    /// where someone reintroduces a position-space write.
    #[test]
    fn boss_motion_respects_world_collision_against_a_wall() {
        let combat_size = ae::Vec2::new(80.0, 80.0);
        let spawn = ae::Vec2::new(200.0, 400.0);
        let aabb = ae::Aabb::new(spawn, combat_size * 0.5);
        let mut boss = BossRuntime::new(
            "test_warden",
            "Clockwork Warden",
            aabb,
            ae::BossBrain::Dormant,
        );
        boss.behavior = BossBehaviorProfile::clockwork_warden();
        boss.encounter_phase = ae::BossEncounterPhase::Phase1;
        // World: a wall at x=400 blocks any rightward chase past it.
        let world = ae::World::new(
            String::from("boss_collision_test"),
            ae::Vec2::new(1200.0, 800.0),
            ae::Vec2::new(100.0, 100.0),
            vec![
                ae::Block::solid(
                    String::from("floor"),
                    ae::Vec2::new(0.0, 760.0),
                    ae::Vec2::new(1200.0, 40.0),
                ),
                ae::Block::solid(
                    String::from("wall"),
                    ae::Vec2::new(400.0, 200.0),
                    ae::Vec2::new(40.0, 500.0),
                ),
            ],
        );
        // Place the player far to the right of the wall so the
        // AnchorSway profile pulls the boss as far right as its
        // chase_limit allows.
        let player_pos = ae::Vec2::new(1000.0, 400.0);
        // Build the brain cfg + state directly — the runtime no
        // longer ticks scripted attacks, so we drive
        // `tick_boss_pattern` ourselves and hand the resulting
        // `desired_vel` to `integrate_body`. This mirrors what
        // `tick_boss_brains_system` + `update_ecs_bosses` do in the
        // real schedule.
        use crate::brain::{
            tick_boss_pattern, BossAttackState, BossPatternCfg, BossPatternContext,
            BossPatternState,
        };
        let mut cfg = BossPatternCfg::neutral_test();
        cfg.aggressiveness = 1.0;
        cfg.pattern = boss.behavior.attack_pattern.clone();
        cfg.movement = boss.behavior.movement.clone();
        cfg.spawn = boss.spawn;
        cfg.combat_size = boss.combat_size();
        cfg.cycle_attack_windup = boss.behavior.attack_windup.max(0.01);
        cfg.cycle_attack_active = boss
            .behavior
            .attack_active
            .max(FeatureCombatTuning::default().boss_attack_active)
            .max(0.01);
        cfg.cycle_attack_cooldown = boss.behavior.attack_cooldown.max(0.05);
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let dt = 1.0 / 60.0;
        for _ in 0..600 {
            let mut frame = ae::ActorControlFrame::neutral();
            tick_boss_pattern(
                &cfg,
                &mut state,
                &BossPatternContext {
                    encounter_phase: boss.encounter_phase,
                    actor_pos: boss.pos,
                    target_pos: player_pos,
                    world_size: world.size,
                    dt,
                },
                &mut frame,
                &mut attack_state,
            );
            boss.integrate_body(&world, frame.desired_vel, dt);
        }
        let boss_right_edge = boss.pos.x + boss.combat_size().x * 0.5;
        let wall_left_edge = 400.0;
        assert!(
            boss_right_edge <= wall_left_edge + 0.5,
            "boss clipped into wall at pos {:?} (right edge {}); wall left edge {}",
            boss.pos,
            boss_right_edge,
            wall_left_edge,
        );
    }
}
