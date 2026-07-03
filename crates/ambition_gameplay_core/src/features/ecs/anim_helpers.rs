//! ECS read-only lookup helpers for sprite/animation systems.
//!
//! Presentation code calls these by id to drive enemy/npc/boss sprite
//! swaps, hit-flash, and animation rows without taking on a query for
//! every feature family itself.

use super::*;

/// Read-only query of the unified actor cluster every actor (was-NPC, was-enemy,
/// encounter mob, mount/rider) carries — the SAME `Body*` movement/ability
/// clusters the player reads, plus the actor's identity/status/config. Systems
/// declare `Query<ActorSpriteData>`; the helpers take `&Query<ActorSpriteData>`.
///
/// All fields are required (not `Option`): every spawned actor carries the full
/// [`crate::actor::AncillaryMovementBundle`] (the same bundle the player nests)
/// plus `ActorStatus` / `ActorConfig` / `BodyMelee`, so an entity that is missing
/// any of them — a boss (its own cluster + anim path) or a prop — correctly does
/// not match and is skipped, instead of half-resolving from a sparse read. This
/// is what lets [`ecs_actor_anim_state`] build the player's FULL `BodyAnimView`
/// from an actor's real clusters, so any ability a brain drives animates.
#[derive(bevy::ecs::query::QueryData)]
pub struct ActorSpriteData {
    pub feature_id: &'static FeatureId,
    pub kin: &'static super::actor_clusters::BodyKinematics,
    pub status: &'static super::actor_clusters::ActorStatus,
    pub health: &'static ambition_characters::actor::BodyHealth,
    pub combat: &'static ambition_characters::actor::BodyCombat,
    pub config: &'static super::actor_clusters::ActorConfig,
    pub attack: &'static BodyMelee,
    pub ground: &'static crate::actor::BodyGroundState,
    pub wall: &'static crate::actor::BodyWallState,
    pub blink: &'static crate::actor::BodyBlinkState,
    pub flight: &'static crate::actor::BodyFlightState,
    pub dash: &'static crate::actor::BodyDashState,
    pub ledge: &'static crate::actor::BodyLedgeState,
    pub body_mode: &'static crate::actor::BodyModeState,
    pub env_contact: &'static crate::actor::BodyEnvironmentContact,
    pub abilities: &'static crate::actor::BodyAbilities,
    pub dodge: &'static crate::actor::BodyDodgeState,
    pub shield: &'static crate::actor::BodyShieldState,
}

/// One actor's resolved animation frame for the renderer: the chosen anim plus
/// the bits the per-frame apply needs that aren't in the anim itself — world
/// position (for localized-gravity facing), facing sign, and whether the actor
/// is mid-swing (for the warm outgoing-attack tint).
#[derive(Clone, Copy, Debug)]
pub struct ActorAnimFrame {
    pub anim: crate::character_sprites::CharacterAnim,
    pub pos: ambition_engine_core::Vec2,
    pub facing: f32,
    pub attacking: bool,
}

// The per-actor identity accessors (`ecs_actor_name`, `ecs_actor_is_sandbag`,
// `ecs_enemy_sprite_override`, `ecs_actor_render_size`) are GONE: those static
// facts are now materialized once into `ActorRenderIndex` (see
// `rebuild_actor_render_index`), which the renderer reads by id — so
// presentation no longer live-queries the actor clusters to bind a sprite. The
// per-frame ANIM frame below stays a live read until slice B materializes it.

/// Materialized per-frame animation pose for every actor, keyed by
/// [`FeatureId`] — the MOVING half of the actor read-model (`ActorAnimFrame` is
/// `Copy`, so the rebuild just overwrites; a `String` allocates only for a
/// genuinely new id). Presentation reads the pose by id and never borrows the
/// actor clusters to animate. Because this pose is presentation-ONLY, its
/// rebuild is registered in the render presentation plugin — NOT the sim
/// schedule — so a headless / RL build never pays for poses it won't draw.
#[derive(Resource, Default, Clone, Debug)]
pub struct ActorAnimIndex {
    frames: std::collections::HashMap<String, (ActorAnimFrame, u64)>,
    generation: u64,
}

impl ActorAnimIndex {
    pub fn get(&self, id: &str) -> Option<ActorAnimFrame> {
        self.frames.get(id).map(|(frame, _)| *frame)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    fn begin_rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn end_rebuild(&mut self) {
        let gen = self.generation;
        self.frames.retain(|_, (_, g)| *g == gen);
    }

    fn insert(&mut self, id: &str, frame: ActorAnimFrame) {
        let gen = self.generation;
        if let Some(slot) = self.frames.get_mut(id) {
            slot.0 = frame;
            slot.1 = gen;
        } else {
            self.frames.insert(id.to_string(), (frame, gen));
        }
    }
}

/// Resolve EVERY brain-driven actor's animation frame from its REAL ECS clusters
/// — the SAME `Body*` movement/ability clusters, and the SAME picker, the player
/// uses ([`crate::character_sprites::pick_actor_anim`] → `body_view_from_clusters`).
/// One path, disposition-agnostic: an enemy and an NPC animate from identical
/// reads. Whatever a brain (or an LLM) drives the actor's clusters into — a dash,
/// a blink, flight, a shield, a ladder climb, a wall-grab, a dodge-roll, a
/// crouch/slide, an in-flight swing — animates with no per-archetype branch; the
/// sheet's anim set decides how richly each pose reads. The picked poses land in
/// [`ActorAnimIndex`] for the renderer to consume by id.
pub fn rebuild_actor_anim_index(mut index: ResMut<ActorAnimIndex>, actors: Query<ActorSpriteData>) {
    index.begin_rebuild();
    for a in &actors {
        let attacking = a.attack.is_active() || a.attack.is_winding_up();
        let anim = crate::character_sprites::pick_actor_anim(
            a.kin,
            a.ground,
            a.wall,
            a.blink,
            a.flight,
            a.dash,
            a.ledge,
            a.body_mode,
            a.env_contact,
            a.abilities,
            a.dodge,
            a.shield,
            a.attack.swing.as_ref(),
            crate::character_sprites::ActorAnimState {
                alive: a.health.alive(),
                hit_flash: a.combat.hit_flash > 0.0,
                // Gravity-free FLIGHT archetype (parrot / shark): the locomotion
                // tail reads Fly/Idle and the airborne gate is suppressed.
                aerial: a.config.tuning.is_aerial,
            },
        );
        index.insert(
            a.feature_id.as_str(),
            ActorAnimFrame {
                anim,
                pos: a.kin.pos,
                facing: a.kin.facing,
                attacking,
            },
        );
    }
    index.end_rebuild();
}

/// ECS chest-opened lookup for sprite swapping.
pub fn ecs_chest_opened(
    id: &str,
    chests: &Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
) -> Option<bool> {
    chests
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, opened)| opened.is_some())
}

/// ECS breakable-state lookup for sprite swapping.
pub fn ecs_breakable_state(
    id: &str,
    breakables: &Query<(&FeatureId, &BreakableFeature)>,
) -> Option<ambition_interaction::BreakableState> {
    breakables
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, breakable)| breakable.breakable.state)
}

// `ecs_boss_name` is GONE: the boss's static identity (name + behavior id) is
// materialized into `BossRenderIndex` (see `rebuild_boss_render_index`), which
// `upgrade_boss_sprites` reads by id — so binding a boss sheet no longer
// live-queries the boss clusters.

fn boss_anim_for_attack_profile(
    profile: &ambition_characters::brain::BossAttackProfile,
) -> Option<crate::boss_encounter::sprites::BossAnim> {
    use crate::boss_encounter::sprites::BossAnim;
    use ambition_characters::brain::BossAttackProfile;
    match profile {
        BossAttackProfile::FloorSlam
        | BossAttackProfile::HandSlam
        | BossAttackProfile::ConvergingShockwave => Some(BossAnim::FloorSlam),
        BossAttackProfile::SideSweep
        | BossAttackProfile::HandSweep
        | BossAttackProfile::Broadside => Some(BossAnim::SideSweep),
        // Every content special falls back to the spike-halo telegraph anim
        // (a ring of damage around the boss) — the closest generic visual cue.
        // Covers the former DebrisRain / MemorizedVolley / LockOnBeam / PitTrap
        // / RotatingCross / MinionCascade.
        BossAttackProfile::FullBodyPulse
        | BossAttackProfile::HeadDescent
        | BossAttackProfile::Special(_) => Some(BossAnim::SpikeHalo),
        BossAttackProfile::HazardColumn | BossAttackProfile::DiveLane => Some(BossAnim::DashEcho),
        BossAttackProfile::WingSweep => None,
    }
}

fn boss_animation_key_for_sample(
    profile: &ambition_characters::brain::BossAttackProfile,
    anim: crate::boss_encounter::sprites::BossAnim,
) -> Option<&'static str> {
    use crate::boss_encounter::sprites::BossAnim;
    use ambition_characters::brain::BossAttackProfile;
    match (profile, anim) {
        // GNU-ton has profile-specific dangerous boxes (for example
        // `gnu_shockwave`) but the damageable head/body box should follow
        // the rendered row. Keep the sample keyed to the visual row so
        // authored row frames are the source of truth for hurtboxes.
        (
            BossAttackProfile::HandSlam | BossAttackProfile::ConvergingShockwave,
            BossAnim::FloorSlam,
        ) => Some("hand_slam"),
        (BossAttackProfile::HandSweep, BossAnim::SideSweep) => Some("hand_sweep"),
        (BossAttackProfile::HeadDescent, BossAnim::SpikeHalo) => Some("head_down"),
        // GNU-ton's apple rain reads the head row for its damageable hurtbox.
        (BossAttackProfile::Special(key), BossAnim::SpikeHalo) if key == "apple_rain" => {
            Some("head_down")
        }
        _ => super::super::bosses::boss_animation_keys_for_profile(profile)
            .first()
            .copied(),
    }
}

fn boss_anim_state_for(
    boss: super::boss_clusters::BossRef<'_>,
    // Liveness + damage-blink from the boss's shared body components (§A1).
    alive: bool,
    hit_flash: f32,
    attack_state: &ambition_characters::brain::BossAttackState,
    brain: &ambition_characters::brain::Brain,
) -> crate::boss_encounter::sprites::BossAnimState {
    // attack_active / attack_windup read the brain's
    // BossAttackState (single source of truth) instead of
    // mirror fields on BossRuntime. pattern_timer comes from
    // the brain's BossPatternState; non-BossPattern brains
    // (test fixtures) fall back to 0.0.
    let pattern_timer = brain
        .boss_pattern_state()
        .map(|s| s.pattern_timer)
        .unwrap_or(0.0);
    crate::boss_encounter::sprites::BossAnimState {
        alive,
        attack_active: attack_state.active_profile.is_some(),
        attack_windup: attack_state.telegraph_profile.is_some(),
        hit_flash: hit_flash > 0.0,
        windup_anim: attack_state
            .telegraph_profile
            .as_ref()
            .and_then(boss_anim_for_attack_profile),
        active_anim: attack_state
            .active_profile
            .as_ref()
            .and_then(boss_anim_for_attack_profile),
        pattern_timer,
        facing: boss.kin.facing,
        pos: boss.kin.pos,
    }
}

pub fn ecs_boss_anim_state_and_entity(
    id: &str,
    bosses: &Query<(
        bevy::prelude::Entity,
        &FeatureId,
        super::boss_clusters::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
) -> Option<(
    bevy::prelude::Entity,
    crate::boss_encounter::sprites::BossAnimState,
)> {
    bosses
        .iter()
        .find_map(|(entity, feature_id, boss, health, combat, attack_state, brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            Some((
                entity,
                boss_anim_state_for(
                    boss.as_boss_ref(),
                    health.alive(),
                    combat.hit_flash,
                    attack_state,
                    brain,
                ),
            ))
        })
}

/// Return the currently rendered attack-frame sample for a boss,
/// but only when the chosen visual row is directly driven by the
/// boss attack profile.
///
/// Hit/death/rest overrides deliberately return `None`; geometry
/// callers then fall back to elapsed-time sampling instead of using a
/// frame from the wrong visual row.
pub fn ecs_boss_animation_frame_sample(
    id: &str,
    bosses: &Query<(
        bevy::prelude::Entity,
        &FeatureId,
        super::boss_clusters::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
    anim: crate::boss_encounter::sprites::BossAnim,
    frame_index: usize,
) -> Option<(
    bevy::prelude::Entity,
    crate::features::BossAnimationFrameSample,
)> {
    bosses
        .iter()
        .find_map(|(entity, feature_id, _boss, _health, _combat, attack_state, _brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            let active_expected = attack_state
                .active_profile
                .as_ref()
                .and_then(boss_anim_for_attack_profile);
            let telegraph_expected = attack_state
                .telegraph_profile
                .as_ref()
                .and_then(boss_anim_for_attack_profile);
            let mut result = None;
            if let Some(profile) = attack_state.active_profile.as_ref() {
                if active_expected == Some(anim) {
                    result = Some((
                        entity,
                        crate::features::BossAnimationFrameSample {
                            profile: Some(profile.clone()),
                            frame_index,
                            animation_key: boss_animation_key_for_sample(profile, anim),
                        },
                    ));
                }
            }
            if result.is_none() {
                if let Some(profile) = attack_state.telegraph_profile.as_ref() {
                    if telegraph_expected == Some(anim) {
                        result = Some((
                            entity,
                            crate::features::BossAnimationFrameSample {
                                profile: Some(profile.clone()),
                                frame_index,
                                animation_key: boss_animation_key_for_sample(profile, anim),
                            },
                        ));
                    }
                }
            }
            // Idle/rest: not driven by any attack profile, but still emit
            // a sample so the rest-pose hurtbox bobs with the breathing
            // animation instead of locking to frame 0. Hit/Death rows are
            // deliberately left as `None` — geometry should stay on the
            // rest-pose shape rather than chase a recoil/death frame.
            if result.is_none() && anim == crate::boss_encounter::sprites::BossAnim::Rest {
                result = Some((
                    entity,
                    crate::features::BossAnimationFrameSample {
                        profile: None,
                        frame_index,
                        animation_key: Some("rest"),
                    },
                ));
            }
            result
        })
}

pub fn ecs_boss_anim_state(
    id: &str,
    bosses: &Query<(
        &FeatureId,
        super::boss_clusters::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
) -> Option<crate::boss_encounter::sprites::BossAnimState> {
    bosses
        .iter()
        .find_map(|(feature_id, boss, health, combat, attack_state, brain)| {
            if feature_id.as_str() != id {
                return None;
            }
            Some(boss_anim_state_for(
                boss.as_boss_ref(),
                health.alive(),
                combat.hit_flash,
                attack_state,
                brain,
            ))
        })
}
