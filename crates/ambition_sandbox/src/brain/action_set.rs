//! `ActionSet` — per-entity capability.
//!
//! A brain emits abstract intent into [`crate::actor_control::ActorControlFrame`]
//! (`melee_pressed = true`, `fire = Some(dir)`). The actor's
//! `ActionSet` translates that intent into a concrete effect
//! (`spawn a Swipe hitbox`, `launch a Rock projectile`). Two actors
//! can share the same brain template and look completely different
//! because their ActionSets resolve differently.
//!
//! The same data structure works for players, NPCs, enemies, and
//! bosses. A player possessing a goblin keeps the goblin's
//! ActionSet — pressing Attack still resolves to "leap" because that
//! is the goblin's `melee_attack` spec.
//!
//! Telegraphs aren't a separate concept; each attack spec owns its
//! full windup → active → recover animation timing.

use crate::engine_core as ae;
use bevy::ecs::component::Component;

/// Per-entity capability set. Resolves abstract brain intent
/// (control-frame fields) into concrete effect requests
/// ([`ActionRequest`]) that the EFFECTS-stage spawn systems consume.
///
/// Construct via [`ActionSet::peaceful`] for a "no attacks" baseline
/// and override only the slots that exist for this actor.
#[derive(Component, Clone, Debug, Default)]
pub struct ActionSet {
    /// What `frame.melee_pressed = true` resolves to. `None` means
    /// the actor has no melee at all (peaceful patroller, puppy slug,
    /// etc.); the brain may still set `melee_pressed = true` but the
    /// EFFECTS stage spawns nothing.
    pub melee: Option<MeleeActionSpec>,
    /// What `frame.fire = Some(dir)` resolves to. `None` = no ranged
    /// capability.
    pub ranged: Option<RangedActionSpec>,
    /// How locomotion looks. Walk is the conservative default; brain
    /// templates that emit `desired_vel` get their motion drawn via
    /// this style.
    pub move_style: MoveStyleSpec,
    /// What `frame.special_pressed = true` resolves to. Per-entity
    /// signature move — boss spotlight pattern, player-only ability,
    /// etc.
    pub special: Option<SpecialActionSpec>,
}

impl ActionSet {
    /// "I don't attack" baseline. Used for peaceful NPCs, puppy
    /// slugs, and other passive actors.
    pub fn peaceful() -> Self {
        Self::default()
    }

    /// True iff this ActionSet has at least one offensive capability
    /// (melee or ranged). Daytime HUD / faction logic uses this to
    /// distinguish "passive observer" actors from "could attack
    /// if asked" actors without re-checking three Option<>s.
    #[allow(dead_code, reason = "diagnostic + daytime-consumer helper")]
    pub fn can_attack(&self) -> bool {
        self.melee.is_some() || self.ranged.is_some()
    }
}

/// Authored item carried by an actor. Held items are gameplay capabilities,
/// not just visuals: they can grant melee and/or ranged actions to the
/// actor's `ActionSet`. The item id is intentionally data-authored so future
/// item rows (axe, sword, thrown bomb, bow, etc.) can be added to RON without
/// adding a Rust enum variant for each item.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct HeldItemSpec {
    /// Stable authored id used by visuals / projectile routing / future drops.
    pub id: String,
    /// Optional melee verb granted by the held item.
    #[serde(default)]
    pub melee: Option<MeleeActionSpec>,
    /// Optional ranged verb granted by the held item.
    #[serde(default)]
    pub ranged: Option<RangedActionSpec>,
}

impl HeldItemSpec {
    /// Overlay the item's abilities on top of an archetype action set. The
    /// item wins because weapons are the thing the actor is actually holding;
    /// archetype rows remain useful for body-contact and fallback tuning.
    pub fn apply_to_action_set(&self, actions: &mut ActionSet) {
        if let Some(melee) = self.melee {
            actions.melee = Some(melee);
        }
        if let Some(ranged) = self.ranged {
            actions.ranged = Some(ranged);
        }
    }

    pub fn grants_ranged(&self) -> bool {
        self.ranged.is_some()
    }
}

/// Registry of authored held items, keyed by stable id.
///
/// Archetypes (and future drop tables / pickups) reference an item by id —
/// `held_item: Some("gun_sword")` — instead of embedding the full spec, so a
/// weapon is defined in exactly one place and can be shared. New weapons are
/// added here (or, later, an item RON the loader merges in) rather than
/// duplicated per archetype. The schema is deliberately the current
/// id/melee/ranged shape; richer fields (muzzle offset, ammo, projectile arc)
/// land when the item pass that needs them does.
static HELD_ITEMS: std::sync::LazyLock<std::collections::HashMap<&'static str, HeldItemSpec>> =
    std::sync::LazyLock::new(|| {
        let mut items = std::collections::HashMap::new();
        items.insert(
            "gun_sword",
            HeldItemSpec {
                id: "gun_sword".into(),
                melee: None,
                ranged: Some(RangedActionSpec::Bolt {
                    speed: 500.0,
                    damage: 2,
                }),
            },
        );
        items.insert(
            "gun_sword_heavy",
            HeldItemSpec {
                id: "gun_sword_heavy".into(),
                melee: None,
                ranged: Some(RangedActionSpec::Bolt {
                    speed: 500.0,
                    damage: 3,
                }),
            },
        );
        // The puppy-slug gun has no melee/ranged verb of its own — `Attack` is
        // intercepted by `puppy_slug_gun::fire_puppy_slug_gun_system`, which
        // summons a player-allied puppy slug instead.
        items.insert(
            "puppy_slug_gun",
            HeldItemSpec {
                id: "puppy_slug_gun".into(),
                melee: None,
                ranged: None,
            },
        );
        // The bomb is a pure throwable (no melee/ranged verb): a plain Attack
        // throws it, and `bomb::tick_bomb_fuses` detonates it on a fuse.
        items.insert(
            "bomb",
            HeldItemSpec {
                id: "bomb".into(),
                melee: None,
                ranged: None,
            },
        );
        // The Mark/Recall ability has no melee/ranged verb either — its plain
        // `Attack` is intercepted by `mark_recall::mark_recall_system` (drop a
        // teleport mark) and `Blink` recalls to it. Like the puppy-slug gun it
        // opts out of throw-on-attack via `throw_held_item_system`.
        items.insert(
            "mark_recall",
            HeldItemSpec {
                id: "mark_recall".into(),
                melee: None,
                ranged: None,
            },
        );
        items
    });

/// Resolve a held-item id to its authored spec, or `None` for an unknown id.
pub fn held_item_by_id(id: &str) -> Option<HeldItemSpec> {
    HELD_ITEMS.get(id).cloned()
}

/// Concrete melee actions an actor can perform. Each variant carries
/// its **own** animation timing (windup → active → recover) — there
/// is no separate `TelegraphSpec`.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[allow(
    dead_code,
    reason = "spec variants surface to per-actor EFFECTS consumers"
)]
pub enum MeleeActionSpec {
    /// Generic short swing. Used by Striker / standard goblin melee.
    Swipe(SwipeSpec),
    /// Heavy lunging step + strike. Used by Brute / large mob melee.
    Lunge(LungeSpec),
    /// Pounce + slam. Used by FastFall and the puppy-slug aerial dive
    /// (when applicable). Today no actor uses this; reserved for
    /// future Wanderer-with-aggression archetypes.
    Slam(SlamSpec),
    /// Jaw-snap bite. Used by puppy slug aggressive variants and
    /// sharks if/when they get melee.
    Bite(BiteSpec),
    /// Light reactive punch — used by Sandbag when struck.
    PunchWeak(PunchSpec),
}

/// Concrete ranged actions an actor can perform.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub enum RangedActionSpec {
    /// Throws a rock-shaped projectile (used by skirmishers /
    /// peaceful-turned-hostile NPCs).
    Rock { speed: f32, damage: i32 },
    /// Fires an arrow. Slower windup than Rock, more damage.
    Arrow { speed: f32, damage: i32 },
    /// Fires a pistol shot (used by pirate skirmishers).
    Pistol { speed: f32, damage: i32 },
    /// Fires a magical bolt (used by bosses).
    Bolt { speed: f32, damage: i32 },
}

impl RangedActionSpec {
    /// Effective launch speed. The brain emits `frame.fire =
    /// Some(dir)`; the EFFECTS stage reads this to set the
    /// projectile speed.
    pub fn speed(self) -> f32 {
        match self {
            Self::Rock { speed, .. }
            | Self::Arrow { speed, .. }
            | Self::Pistol { speed, .. }
            | Self::Bolt { speed, .. } => speed,
        }
    }

    /// Damage on hit.
    pub fn damage(self) -> i32 {
        match self {
            Self::Rock { damage, .. }
            | Self::Arrow { damage, .. }
            | Self::Pistol { damage, .. }
            | Self::Bolt { damage, .. } => damage,
        }
    }
}

impl MeleeActionSpec {
    /// Total swing duration (windup + active + recover) in seconds.
    /// Cooldown systems / animation pickers use this to gate the
    /// "can swing again" question.
    #[allow(dead_code, reason = "diagnostic helper for EFFECTS consumers")]
    pub fn total_duration_s(self) -> f32 {
        match self {
            Self::Swipe(s) => s.windup_s + s.active_s + s.recover_s,
            Self::Lunge(s) => s.windup_s + s.active_s + s.recover_s,
            Self::Slam(s) => s.windup_s + s.active_s + s.recover_s,
            Self::Bite(s) => s.windup_s + s.active_s + s.recover_s,
            Self::PunchWeak(s) => s.windup_s + s.active_s + s.recover_s,
        }
    }

    /// Damage dealt on a clean hit.
    #[allow(dead_code, reason = "diagnostic helper for EFFECTS consumers")]
    pub fn damage(self) -> i32 {
        match self {
            Self::Swipe(s) => s.damage,
            Self::Lunge(s) => s.damage,
            Self::Slam(s) => s.damage,
            Self::Bite(s) => s.damage,
            Self::PunchWeak(s) => s.damage,
        }
    }

    /// Reach (hitbox forward extent) in px from the actor's anchor.
    #[allow(dead_code, reason = "diagnostic helper for EFFECTS consumers")]
    pub fn reach_px(self) -> f32 {
        match self {
            Self::Swipe(s) => s.reach_px,
            Self::Lunge(s) => s.reach_px,
            Self::Slam(s) => s.reach_px,
            Self::Bite(s) => s.reach_px,
            Self::PunchWeak(s) => s.reach_px,
        }
    }
}

/// How an actor's locomotion looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize)]
pub enum MoveStyleSpec {
    /// Two-legged walk (default for humanoids).
    #[default]
    Walk,
    /// Heavy slow walk — used by Brute.
    WalkHeavy,
    /// Hop forward in arcs (used by FastFall).
    Hop,
    /// Strafing sideways motion (used by Skirmisher).
    Strafe,
    /// Crawls along surfaces (used by puppy slug). The actor's
    /// `surface_normal` rotates the rendered motion.
    Slither,
    /// Floats / hovers (used by aerial bosses, sharks).
    Float,
}

/// Per-entity signature move. The contents vary widely between
/// actors — keep the enum small and add variants only when a real
/// consumer lands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpecialActionSpec {
    /// Player-only: deploys the bubble shield. Wired through the
    /// existing bubble-shield pipeline when Chunk 4 hooks the player
    /// brain.
    BubbleShield,
    /// Boss-only: triggers a phase-specific spotlight attack. The
    /// content of "spotlight" is resolved by the boss encounter
    /// driver.
    BossSpotlight,
    /// GNU-ton boss: rain of apples falling from the ceiling across
    /// the arena. The boss runtime tags `frame.special_pressed`
    /// every tick the apple-rain strike window is active; the
    /// `spawn_gnu_apple_rain_from_special_messages` consumer
    /// accumulates per-boss spawn cadence and emits the
    /// `EnemyProjectileSpawn`s. Replaces the legacy
    /// `BossRuntime::tick_apple_rain` self-state spawn loop.
    GnuAppleRain {
        /// Seconds between apple spawns while the strike is active.
        interval_s: f32,
        /// Downward initial velocity given to each apple at spawn.
        spawn_speed: f32,
        /// Per-apple damage.
        damage: i32,
    },
    /// Gradient Sentinel boss: position-sampling bolt barrage. The
    /// brain emits `special_pressed` every tick the
    /// `BossAttackProfile::OverfitVolley` window is active; the
    /// `spawn_overfit_volley_from_special_messages` consumer samples
    /// player positions during the telegraph (via the live
    /// `BossAttackState`) and fires `sample_count` bolts at the
    /// memorized positions on the first strike tick.
    OverfitVolley {
        /// Seconds between position samples during the telegraph
        /// window. Sample count caps at `sample_count`.
        sample_interval_s: f32,
        /// Max number of player-position samples to memorize per
        /// strike. More samples = more bolts.
        sample_count: u8,
        /// Per-bolt projectile speed (px/s).
        shot_speed: f32,
        /// Per-bolt damage.
        damage: i32,
    },
    /// Smirking Behemoth boss: a short bubble-laser line emitted
    /// from the eye toward the player's approximate telegraphed
    /// position. This is separate from OverfitVolley so the cut-rope
    /// boss can fire a single readable beam instead of the Gradient
    /// Sentinel's slow memorized barrage.
    EyeBeam {
        /// Per-box projectile speed (px/s).
        shot_speed: f32,
        /// Per-box damage.
        damage: i32,
        /// Number of boxes spawned along the initial beam line.
        box_count: u8,
        /// Pixel spacing between beam boxes at spawn.
        box_spacing: f32,
        /// Beam box half-size.
        half_extent_x: f32,
        half_extent_y: f32,
        /// Lifetime for each beam box.
        lifetime_s: f32,
    },
    /// Gradient Sentinel boss: local-minimum pit trap. The strike
    /// edge spawns a World-anchored hazard hitbox at the player's
    /// current position; the hitbox persists for `hazard_duration_s`
    /// seconds. If `spawn_minion` is true the consumer also spawns a
    /// puppy_slug (pacifist crawler) from inside the pit.
    MinimaTrap {
        /// Seconds the hazard hitbox stays live after spawn.
        hazard_duration_s: f32,
        /// Per-tick damage the pit deals while the player overlaps
        /// it (the standard `apply_hitbox_damage` once-per-strike
        /// gate still applies, so the player takes at most one hit
        /// per pit lifetime).
        damage: i32,
        /// Half-extent of the pit hitbox (px).
        half_extent_x: f32,
        half_extent_y: f32,
        /// Spawn a puppy_slug crawler from inside the pit on the
        /// strike edge.
        spawn_minion: bool,
    },
    /// Gradient Sentinel boss: rotating cross hazard around the
    /// boss. Two World-anchored hitboxes (horizontal arm + vertical
    /// arm); the consumer toggles which arm is "live" every
    /// `axis_period_s` seconds. Player stands on the inactive axis
    /// and reads the swap. Total strike duration is governed by the
    /// brain's `BossPatternStep::Strike { duration }`.
    SaddlePoint {
        /// Half-extent of each arm along its long axis (px).
        arm_length: f32,
        /// Half-extent of each arm along its short axis (px).
        arm_thickness: f32,
        /// Seconds an axis stays live before toggling.
        axis_period_s: f32,
        /// Per-tick damage. Same once-per-strike gate as MinimaTrap.
        damage: i32,
    },
    /// Gradient Sentinel boss: spawn `minion_count` "slop" minions
    /// (small_lurker stand-in) at the top of the arena that descend
    /// toward the player. One-shot per strike — the consumer ignores
    /// repeated Special messages while the strike is active.
    GradientCascade {
        /// Number of minions to spawn on the strike edge.
        minion_count: u8,
    },
}

// --- Concrete attack spec timings ---
//
// Each spec carries (windup, active, recover) in seconds, plus
// damage + a hitbox half-extent. Today these values mirror the
// pre-refactor enemy archetype constants so Chunk 3's migration is
// a one-for-one move. Chunk 4 / data-table work shrinks duplication.

/// Light melee swing. Striker default.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct SwipeSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
}

impl SwipeSpec {
    pub const STRIKER_DEFAULT: Self = Self {
        windup_s: 0.28,
        active_s: 0.08,
        recover_s: 0.32,
        damage: 1,
        reach_px: 28.0,
    };
}

/// Heavy lunging strike. Brute default.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct LungeSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
    /// Forward step (px) the actor takes during windup.
    pub step_px: f32,
}

impl LungeSpec {
    pub const BRUTE_DEFAULT: Self = Self {
        windup_s: 0.45,
        active_s: 0.12,
        recover_s: 0.45,
        damage: 2,
        reach_px: 40.0,
        step_px: 18.0,
    };
}

/// Pounce + slam. Reserved for future hostile aerial archetypes.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct SlamSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
    pub hop_height_px: f32,
}

/// Jaw bite — short reach, fast.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct BiteSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
}

/// Light reactive punch. Sandbag counter-attack.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct PunchSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
}

impl PunchSpec {
    pub const SANDBAG_DEFAULT: Self = Self {
        windup_s: 0.15,
        active_s: 0.08,
        recover_s: 0.40,
        damage: 1,
        reach_px: 22.0,
    };
}

/// Concrete effect a brain's abstract intent resolves to, after
/// reading the actor's `ActionSet`. The EFFECTS-stage spawn systems
/// consume this list per actor per tick and translate each into a
/// real hitbox / projectile / FX.
///
/// In Chunk 2 this is the *shape* of the resolver output — actual
/// resolver wiring (writing hitbox AABBs into a feature-output
/// channel) lands in Chunk 3 when an actor first uses a brain. For
/// now the helpers are testable in isolation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActionRequest {
    /// Spawn a melee hitbox in front of the actor.
    Melee {
        spec: MeleeActionSpec,
        origin: ae::Vec2,
        facing: f32,
        attack_axis: ae::Vec2,
    },
    /// Spawn a projectile traveling in `dir`. Used by NPC / enemy /
    /// boss ranged: a single "fire now" edge resolved from
    /// `frame.fire = Some(...)` by [`resolve`]. Player ranged uses
    /// `PlayerProjectileTick` instead so the EFFECTS consumer can
    /// drive the charge state machine + motion-recognition buffer.
    Ranged {
        spec: RangedActionSpec,
        origin: ae::Vec2,
        dir: ae::Vec2,
    },
    /// Trigger the actor's special. Resolved by the per-actor
    /// special handler (player ability system, boss encounter
    /// driver, etc.).
    Special { spec: SpecialActionSpec },
    /// Per-tick player projectile signal — drives the player
    /// projectile EFFECTS consumer's charge state machine + motion
    /// recognition buffer. Emitted by a dedicated player-projectile
    /// emit system (not by [`resolve`]) because the per-player
    /// charge tiers / projectile kinds live in the projectile
    /// system's own config rather than as a per-actor `ActionSet`
    /// capability. Keeps the player's combat verbs flowing through
    /// the same `ActorActionMessage` channel as melee and NPC
    /// ranged.
    ///
    /// The variant intentionally omits `origin` and `facing` — the
    /// projectile EFFECTS consumer reads those from its
    /// `PlayerKinematics` query (the authoritative source of player
    /// body position / facing), so dragging them through the
    /// message would just duplicate state and force the emit side
    /// to query Transform too.
    PlayerProjectileTick {
        /// Movement axis sample (mirrors `ActorControlFrame::
        /// desired_vel`). Pushed into the motion-recognition buffer
        /// every tick so QCF / half-circle detection survives the
        /// migration.
        axis: ae::Vec2,
        /// Aim direction (twin-stick / mouse). Zero = use facing.
        aim: ae::Vec2,
        /// Rising edge: projectile button pressed this tick.
        press: bool,
        /// Sustain: projectile button held this tick.
        held: bool,
        /// Falling edge: projectile button released this tick.
        released: bool,
    },
}

impl std::fmt::Display for ActionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Melee { origin, facing, .. } => {
                write!(f, "{}(at {:?} facing {:+.0})", self.label(), origin, facing,)
            }
            Self::Ranged { origin, dir, .. } => {
                write!(f, "{}(from {:?} dir {:?})", self.label(), origin, dir,)
            }
            Self::Special { .. } => write!(f, "{}", self.label()),
            Self::PlayerProjectileTick {
                press,
                held,
                released,
                ..
            } => {
                let edge = if *press {
                    "press"
                } else if *released {
                    "release"
                } else if *held {
                    "held"
                } else {
                    "sample"
                };
                write!(f, "{}({})", self.label(), edge)
            }
        }
    }
}

impl ActionRequest {
    /// Short label naming the request kind ("melee_swipe",
    /// "ranged_bolt", "special_bubble_shield", …). Useful for
    /// trace logs, debug overlays, and grep-friendly diagnostics
    /// without the verbose Debug rendering.
    #[allow(dead_code, reason = "diagnostic helper for the EFFECTS-flip migration")]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Melee { spec, .. } => match spec {
                MeleeActionSpec::Swipe(_) => "melee_swipe",
                MeleeActionSpec::Lunge(_) => "melee_lunge",
                MeleeActionSpec::Slam(_) => "melee_slam",
                MeleeActionSpec::Bite(_) => "melee_bite",
                MeleeActionSpec::PunchWeak(_) => "melee_punch_weak",
            },
            Self::Ranged { spec, .. } => match spec {
                RangedActionSpec::Rock { .. } => "ranged_rock",
                RangedActionSpec::Arrow { .. } => "ranged_arrow",
                RangedActionSpec::Pistol { .. } => "ranged_pistol",
                RangedActionSpec::Bolt { .. } => "ranged_bolt",
            },
            Self::Special { spec } => match spec {
                SpecialActionSpec::BubbleShield => "special_bubble_shield",
                SpecialActionSpec::BossSpotlight => "special_boss_spotlight",
                SpecialActionSpec::GnuAppleRain { .. } => "special_gnu_apple_rain",
                SpecialActionSpec::OverfitVolley { .. } => "special_overfit_volley",
                SpecialActionSpec::EyeBeam { .. } => "special_eye_beam",
                SpecialActionSpec::MinimaTrap { .. } => "special_minima_trap",
                SpecialActionSpec::SaddlePoint { .. } => "special_saddle_point",
                SpecialActionSpec::GradientCascade { .. } => "special_gradient_cascade",
            },
            Self::PlayerProjectileTick { .. } => "player_projectile_tick",
        }
    }
}

/// Resolve a brain's abstract control frame into 0..N concrete
/// action requests using the actor's `ActionSet`. Pure function;
/// no Bevy, no side effects. Most ticks emit zero or one request;
/// multi-request ticks are the boss-pattern case (e.g. a phase that
/// simultaneously fires and lunges).
pub fn resolve(
    actions: &ActionSet,
    frame: &crate::actor_control::ActorControlFrame,
    origin: ae::Vec2,
) -> Vec<ActionRequest> {
    let mut out = Vec::with_capacity(2);
    if frame.melee_pressed {
        if let Some(spec) = actions.melee {
            out.push(ActionRequest::Melee {
                spec,
                origin,
                facing: frame.facing,
                attack_axis: frame.attack_axis,
            });
        }
    }
    if let Some(req) = frame.fire {
        if let Some(spec) = actions.ranged {
            // Today extracts `dir` off the engine's
            // `ActorFireRequest` for compat with the existing
            // enemy/boss callers. When `frame.fire` is narrowed to
            // `Option<Vec2>` (speed in ActionSet), this becomes
            // `dir: req`.
            out.push(ActionRequest::Ranged {
                spec,
                origin,
                dir: req.dir,
            });
        }
    }
    if frame.special_pressed {
        if let Some(spec) = actions.special {
            out.push(ActionRequest::Special { spec });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaceful_action_set_has_no_attacks() {
        let s = ActionSet::peaceful();
        assert!(s.melee.is_none());
        assert!(s.ranged.is_none());
        assert!(s.special.is_none());
        assert_eq!(s.move_style, MoveStyleSpec::Walk);
        assert!(!s.can_attack());
    }

    #[test]
    fn resolve_returns_predictable_request_count_per_intent_subset() {
        // Table-driven coverage: every combo of melee/fire/special
        // bits → predictable request count when ActionSet has all
        // capabilities. Pins per-intent independence.
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ranged: Some(RangedActionSpec::Bolt {
                speed: 500.0,
                damage: 1,
            }),
            special: Some(SpecialActionSpec::BubbleShield),
            ..Default::default()
        };
        let cases = [
            (false, false, false, 0),
            (true, false, false, 1),
            (false, true, false, 1),
            (false, false, true, 1),
            (true, true, false, 2),
            (true, false, true, 2),
            (false, true, true, 2),
            (true, true, true, 3),
        ];
        for (melee, fire, special, expected) in cases {
            let mut frame = crate::actor_control::ActorControlFrame::neutral();
            frame.melee_pressed = melee;
            frame.fire = if fire {
                Some(crate::actor_control::ActorFireRequest {
                    dir: ae::Vec2::new(1.0, 0.0),
                    speed: 0.0,
                })
            } else {
                None
            };
            frame.special_pressed = special;
            let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
            assert_eq!(
                reqs.len(),
                expected,
                "melee={} fire={} special={}",
                melee,
                fire,
                special,
            );
        }
    }

    #[test]
    fn resolve_empty_when_frame_has_no_action_intent() {
        // wants_any_action()=false → resolver always returns empty.
        // Pin the contract so sandbox code that gates resolve()
        // calls behind wants_any_action() can rely on it.
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ranged: Some(RangedActionSpec::Bolt {
                speed: 500.0,
                damage: 1,
            }),
            special: Some(SpecialActionSpec::BubbleShield),
            move_style: MoveStyleSpec::Walk,
        };
        let frame = crate::actor_control::ActorControlFrame::neutral();
        assert!(!frame.wants_any_action());
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        assert!(reqs.is_empty());
    }

    #[test]
    fn resolve_with_only_ranged_capability_ignores_melee_intent() {
        // ActionSet with ranged-only capability + frame intent
        // melee_pressed+fire returns Ranged only. Pins the
        // capability gate so a brain that emits melee intent on
        // a ranged-only actor doesn't accidentally spawn a hitbox.
        let actions = ActionSet {
            ranged: Some(RangedActionSpec::Bolt {
                speed: 500.0,
                damage: 1,
            }),
            ..Default::default()
        };
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.fire = Some(crate::actor_control::ActorFireRequest {
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 0.0,
        });
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        assert_eq!(reqs.len(), 1);
        assert!(matches!(reqs[0], ActionRequest::Ranged { .. }));
    }

    #[test]
    fn resolve_passes_attack_axis_through_to_melee_request() {
        // Player tilt (up-tilt / down-air / back-air) carries
        // direction in frame.attack_axis; resolver threads it
        // through so the EFFECTS-stage spawn picks the right
        // hitbox shape.
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.facing = 1.0;
        frame.attack_axis = ae::Vec2::new(0.0, -1.0); // up-tilt
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        match reqs[0] {
            ActionRequest::Melee { attack_axis, .. } => {
                assert_eq!(attack_axis, ae::Vec2::new(0.0, -1.0));
            }
            _ => panic!("expected Melee"),
        }
    }

    #[test]
    fn resolve_peaceful_action_set_emits_nothing_for_full_intent() {
        // ActionSet::peaceful() has no melee/ranged/special. Even
        // if the brain emits every intent verb, the resolver
        // returns an empty vec — peaceful actors stay peaceful
        // even under arbitrary brain input. Pins the "ActionSet
        // is the authority on capability" invariant.
        let actions = ActionSet::peaceful();
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.fire = Some(crate::actor_control::ActorFireRequest {
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 0.0,
        });
        frame.special_pressed = true;
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        assert!(
            reqs.is_empty(),
            "ActionSet::peaceful produces no requests regardless of intent"
        );
    }

    #[test]
    fn action_set_default_is_peaceful_baseline() {
        // Default-constructed ActionSet is the peaceful baseline:
        // no attack capability, default move style. Pins the
        // contract that a fresh-spawn actor with default ActionSet
        // can't attack — sandbox code that constructs ActionSets
        // via `..Default::default()` can rely on this.
        let s = ActionSet::default();
        assert!(s.melee.is_none());
        assert!(s.ranged.is_none());
        assert!(s.special.is_none());
        assert!(!s.can_attack());
        assert_eq!(s.move_style, MoveStyleSpec::default());
        // ActionSet::default() == ActionSet::peaceful().
        let p = ActionSet::peaceful();
        assert!(p.melee.is_none() && s.melee.is_none());
    }

    #[test]
    fn action_set_can_attack_detects_melee_or_ranged() {
        let mut s = ActionSet::peaceful();
        assert!(!s.can_attack());
        s.melee = Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT));
        assert!(s.can_attack());
        s.melee = None;
        s.ranged = Some(RangedActionSpec::Bolt {
            speed: 380.0,
            damage: 1,
        });
        assert!(s.can_attack());
        // Special alone doesn't count as "attacks".
        s.ranged = None;
        s.special = Some(SpecialActionSpec::BubbleShield);
        assert!(!s.can_attack());
    }

    #[test]
    fn resolve_no_intent_yields_no_requests() {
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let frame = crate::actor_control::ActorControlFrame::neutral();
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        assert!(reqs.is_empty());
    }

    #[test]
    fn resolve_melee_pressed_emits_one_melee_request() {
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.facing = 1.0;
        let reqs = resolve(&actions, &frame, ae::Vec2::new(10.0, 5.0));
        assert_eq!(reqs.len(), 1);
        match reqs[0] {
            ActionRequest::Melee {
                spec,
                origin,
                facing,
                ..
            } => {
                assert!(matches!(spec, MeleeActionSpec::Swipe(_)));
                assert_eq!(origin, ae::Vec2::new(10.0, 5.0));
                assert_eq!(facing, 1.0);
            }
            _ => panic!("expected Melee request"),
        }
    }

    #[test]
    fn resolve_melee_pressed_without_capability_emits_nothing() {
        // Puppy slug: brain emits melee_pressed = false today, but
        // even if a possessor presses melee while inhabiting one,
        // it has no melee capability and nothing fires.
        let actions = ActionSet::peaceful();
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        assert!(reqs.is_empty());
    }

    #[test]
    fn resolve_two_actionsets_differ_by_capability() {
        // Same brain intent, different ActionSets → different
        // requests. This is the core "possession is cheap"
        // invariant: swap brains, keep the body's ActionSet.
        let goblin = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let brute = ActionSet {
            melee: Some(MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT)),
            ..Default::default()
        };
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.facing = 1.0;
        let g = resolve(&goblin, &frame, ae::Vec2::ZERO);
        let b = resolve(&brute, &frame, ae::Vec2::ZERO);
        assert_eq!(g.len(), 1);
        assert_eq!(b.len(), 1);
        match (g[0], b[0]) {
            (ActionRequest::Melee { spec: gs, .. }, ActionRequest::Melee { spec: bs, .. }) => {
                assert!(matches!(gs, MeleeActionSpec::Swipe(_)));
                assert!(matches!(bs, MeleeActionSpec::Lunge(_)));
            }
            _ => panic!("expected two Melee requests"),
        }
    }

    #[test]
    fn resolve_fire_pressed_emits_ranged_request() {
        let actions = ActionSet {
            ranged: Some(RangedActionSpec::Rock {
                speed: 400.0,
                damage: 1,
            }),
            ..Default::default()
        };
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        frame.fire = Some(crate::actor_control::ActorFireRequest {
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 0.0, // placeholder; speed comes from ActionSet
        });
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        assert_eq!(reqs.len(), 1);
        match reqs[0] {
            ActionRequest::Ranged { spec, dir, .. } => {
                assert_eq!(spec.speed(), 400.0);
                assert_eq!(dir, ae::Vec2::new(1.0, 0.0));
            }
            _ => panic!("expected Ranged"),
        }
    }

    #[test]
    fn melee_spec_defaults_have_positive_durations() {
        // Every authored default's phase timings (windup + active +
        // recover) must be strictly positive — a zero windup means
        // the attack has no telegraph for the player to read, and a
        // zero active means no hitbox window. Pins the design
        // requirement that telegraphs live inside the attack
        // animation rather than in a separate spec.
        let s = SwipeSpec::STRIKER_DEFAULT;
        assert!(s.windup_s > 0.0 && s.active_s > 0.0 && s.recover_s > 0.0);
        let l = LungeSpec::BRUTE_DEFAULT;
        assert!(l.windup_s > 0.0 && l.active_s > 0.0 && l.recover_s > 0.0);
        let p = PunchSpec::SANDBAG_DEFAULT;
        assert!(p.windup_s > 0.0 && p.active_s > 0.0 && p.recover_s > 0.0);
    }

    #[test]
    fn melee_attack_uniform_helpers_match_concrete_field_lookup() {
        // total_duration_s / damage / reach_px on MeleeActionSpec
        // should equal the same field on the inner spec struct
        // for every variant. Pins the helper consistency so a
        // future spec-struct field rename doesn't cause the
        // accessors to silently return stale values.
        for spec in [
            MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
            MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT),
            MeleeActionSpec::PunchWeak(PunchSpec::SANDBAG_DEFAULT),
        ] {
            assert!(spec.total_duration_s() > 0.0);
            assert!(spec.damage() > 0);
            assert!(spec.reach_px() > 0.0);
        }
    }

    #[test]
    fn action_request_label_covers_all_melee_variants() {
        // Every MeleeActionSpec variant maps to a distinct
        // "melee_*" label. Future Spec variants must update
        // ActionRequest::label() too — this test catches a drop.
        let specs = [
            MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
            MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT),
            MeleeActionSpec::Slam(SlamSpec {
                windup_s: 0.3,
                active_s: 0.1,
                recover_s: 0.4,
                damage: 2,
                reach_px: 40.0,
                hop_height_px: 60.0,
            }),
            MeleeActionSpec::Bite(BiteSpec {
                windup_s: 0.18,
                active_s: 0.08,
                recover_s: 0.25,
                damage: 1,
                reach_px: 22.0,
            }),
            MeleeActionSpec::PunchWeak(PunchSpec::SANDBAG_DEFAULT),
        ];
        let mut labels = Vec::new();
        for spec in specs {
            let req = ActionRequest::Melee {
                spec,
                origin: ae::Vec2::ZERO,
                facing: 1.0,
                attack_axis: ae::Vec2::ZERO,
            };
            let label = req.label();
            assert!(label.starts_with("melee_"), "{}", label);
            labels.push(label);
        }
        // Ensure all labels are distinct (no two variants share
        // a label — would break grep-friendly diagnostics).
        let mut sorted = labels.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            labels.len(),
            "every melee variant should have a distinct label"
        );
    }

    #[test]
    fn action_request_label_returns_per_variant_string() {
        let melee = ActionRequest::Melee {
            spec: MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
            origin: ae::Vec2::ZERO,
            facing: 1.0,
            attack_axis: ae::Vec2::ZERO,
        };
        assert_eq!(melee.label(), "melee_swipe");

        let lunge = ActionRequest::Melee {
            spec: MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT),
            origin: ae::Vec2::ZERO,
            facing: 1.0,
            attack_axis: ae::Vec2::ZERO,
        };
        assert_eq!(lunge.label(), "melee_lunge");

        let ranged = ActionRequest::Ranged {
            spec: RangedActionSpec::Bolt {
                speed: 380.0,
                damage: 1,
            },
            origin: ae::Vec2::ZERO,
            dir: ae::Vec2::new(1.0, 0.0),
        };
        assert_eq!(ranged.label(), "ranged_bolt");

        let special = ActionRequest::Special {
            spec: SpecialActionSpec::BubbleShield,
        };
        assert_eq!(special.label(), "special_bubble_shield");
    }

    #[test]
    fn action_request_display_includes_kind_and_origin() {
        let req = ActionRequest::Melee {
            spec: MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
            origin: ae::Vec2::new(10.0, 20.0),
            facing: 1.0,
            attack_axis: ae::Vec2::ZERO,
        };
        let s = format!("{}", req);
        assert!(s.contains("melee_swipe"));
        assert!(s.contains("facing"));

        let req2 = ActionRequest::Special {
            spec: SpecialActionSpec::BubbleShield,
        };
        assert_eq!(format!("{}", req2), "special_bubble_shield");
    }

    #[test]
    fn melee_spec_uniform_accessors_return_per_variant_fields() {
        let s = MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT);
        assert_eq!(s.damage(), SwipeSpec::STRIKER_DEFAULT.damage);
        assert_eq!(s.reach_px(), SwipeSpec::STRIKER_DEFAULT.reach_px);
        assert!(s.total_duration_s() > 0.0);

        let l = MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT);
        assert_eq!(l.damage(), LungeSpec::BRUTE_DEFAULT.damage);
        assert_eq!(l.reach_px(), LungeSpec::BRUTE_DEFAULT.reach_px);

        let p = MeleeActionSpec::PunchWeak(PunchSpec::SANDBAG_DEFAULT);
        assert_eq!(p.damage(), PunchSpec::SANDBAG_DEFAULT.damage);
        assert!(p.total_duration_s() > 0.0);
    }

    #[test]
    fn ranged_spec_speed_accessor_returns_per_variant_speed() {
        assert_eq!(
            RangedActionSpec::Rock {
                speed: 410.0,
                damage: 1
            }
            .speed(),
            410.0
        );
        assert_eq!(
            RangedActionSpec::Arrow {
                speed: 520.0,
                damage: 2
            }
            .speed(),
            520.0
        );
        assert_eq!(
            RangedActionSpec::Pistol {
                speed: 600.0,
                damage: 1
            }
            .speed(),
            600.0
        );
        assert_eq!(
            RangedActionSpec::Bolt {
                speed: 380.0,
                damage: 1
            }
            .speed(),
            380.0
        );
    }

    #[test]
    fn ranged_spec_damage_accessor_returns_per_variant_damage() {
        // Mirror of the speed accessor test: damage() must pull
        // from each variant's `damage` field independently. Pins
        // the per-variant routing so a future field rename can't
        // silently return the wrong variant's damage.
        assert_eq!(
            RangedActionSpec::Rock {
                speed: 0.0,
                damage: 1,
            }
            .damage(),
            1,
        );
        assert_eq!(
            RangedActionSpec::Arrow {
                speed: 0.0,
                damage: 3,
            }
            .damage(),
            3,
        );
        assert_eq!(
            RangedActionSpec::Pistol {
                speed: 0.0,
                damage: 2,
            }
            .damage(),
            2,
        );
        assert_eq!(
            RangedActionSpec::Bolt {
                speed: 0.0,
                damage: 4,
            }
            .damage(),
            4,
        );
    }

    #[test]
    fn resolve_multi_intent_emits_multi_request() {
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Bite(BiteSpec {
                windup_s: 0.2,
                active_s: 0.1,
                recover_s: 0.3,
                damage: 1,
                reach_px: 22.0,
            })),
            ranged: Some(RangedActionSpec::Bolt {
                speed: 380.0,
                damage: 1,
            }),
            special: Some(SpecialActionSpec::BossSpotlight),
            move_style: MoveStyleSpec::Float,
        };
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.fire = Some(crate::actor_control::ActorFireRequest {
            dir: ae::Vec2::new(0.0, -1.0),
            speed: 0.0,
        });
        frame.special_pressed = true;
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        assert_eq!(reqs.len(), 3);
    }
}
