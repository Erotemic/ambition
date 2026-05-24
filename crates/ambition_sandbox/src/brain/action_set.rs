//! `ActionSet` — per-entity capability.
//!
//! A brain emits abstract intent into [`ae::ActorControlFrame`]
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

use ambition_engine as ae;
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
}

/// Concrete melee actions an actor can perform. Each variant carries
/// its **own** animation timing (windup → active → recover) — there
/// is no separate `TelegraphSpec`.
#[derive(Clone, Copy, Debug, PartialEq)]
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
#[derive(Clone, Copy, Debug, PartialEq)]
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

/// How an actor's locomotion looks.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
}

// --- Concrete attack spec timings ---
//
// Each spec carries (windup, active, recover) in seconds, plus
// damage + a hitbox half-extent. Today these values mirror the
// pre-refactor enemy archetype constants so Chunk 3's migration is
// a one-for-one move. Chunk 4 / data-table work shrinks duplication.

/// Light melee swing. Striker default.
#[derive(Clone, Copy, Debug, PartialEq)]
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
#[derive(Clone, Copy, Debug, PartialEq)]
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlamSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
    pub hop_height_px: f32,
}

/// Jaw bite — short reach, fast.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiteSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
}

/// Light reactive punch. Sandbag counter-attack.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Spawn a projectile traveling in `dir`.
    Ranged {
        spec: RangedActionSpec,
        origin: ae::Vec2,
        dir: ae::Vec2,
    },
    /// Trigger the actor's special. Resolved by the per-actor
    /// special handler (player ability system, boss encounter
    /// driver, etc.).
    Special { spec: SpecialActionSpec },
}

/// Resolve a brain's abstract control frame into 0..N concrete
/// action requests using the actor's `ActionSet`. Pure function;
/// no Bevy, no side effects. Most ticks emit zero or one request;
/// multi-request ticks are the boss-pattern case (e.g. a phase that
/// simultaneously fires and lunges).
pub fn resolve(
    actions: &ActionSet,
    frame: &ae::ActorControlFrame,
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
    }

    #[test]
    fn resolve_no_intent_yields_no_requests() {
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let frame = ae::ActorControlFrame::neutral();
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        assert!(reqs.is_empty());
    }

    #[test]
    fn resolve_melee_pressed_emits_one_melee_request() {
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let mut frame = ae::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.facing = 1.0;
        let reqs = resolve(&actions, &frame, ae::Vec2::new(10.0, 5.0));
        assert_eq!(reqs.len(), 1);
        match reqs[0] {
            ActionRequest::Melee { spec, origin, facing, .. } => {
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
        let mut frame = ae::ActorControlFrame::neutral();
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
        let mut frame = ae::ActorControlFrame::neutral();
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
        let mut frame = ae::ActorControlFrame::neutral();
        frame.fire = Some(ae::ActorFireRequest {
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
        let mut frame = ae::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.fire = Some(ae::ActorFireRequest {
            dir: ae::Vec2::new(0.0, -1.0),
            speed: 0.0,
        });
        frame.special_pressed = true;
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        assert_eq!(reqs.len(), 3);
    }
}
