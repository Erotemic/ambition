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

pub fn ecs_npc_name(id: &str, actors: &Query<ActorSpriteData>) -> Option<String> {
    actors
        .iter()
        .find_map(|a| (a.feature_id.as_str() == id).then(|| a.config.name.clone()))
}

/// Explicit sprite render-quad size for an actor whose collision was derived
/// from published sprite `body_metrics` (see
/// [`crate::features::ActorRenderSize`]). The renderer draws the sprite at this
/// size instead of `collision * collision_scale` so the visible art is
/// preserved even though the hitbox equals the body. SHARED across dispositions
/// (NPC + the enemy it becomes when hostile), so the sprite never balloons on a
/// hostile flip. `None` → the actor uses the legacy `collision_scale` path.
pub fn ecs_actor_render_size(
    id: &str,
    render_sizes: &Query<(&FeatureId, &crate::features::ActorRenderSize)>,
) -> Option<ambition_engine_core::Vec2> {
    render_sizes
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, size)| size.0)
}

pub fn ecs_enemy_sprite_override(id: &str, actors: &Query<ActorSpriteData>) -> Option<String> {
    actors.iter().find_map(|a| {
        if a.feature_id.as_str() != id {
            return None;
        }
        a.config.sprite_override_npc_name.clone()
    })
}

/// Per-enemy display name, used by `upgrade_enemy_sprites` as a
/// fallback sprite-lookup key when no explicit `sprite_override` is
/// set. Lets direct `EnemySpawn` entities (no NPC migration history)
/// pick up a content sheet by name — e.g. the intro raiders resolve
/// to `raid_enforcer_spritesheet`
/// via the intro NPC sprite registry without authors having to
/// double-register them as an `enemy_sprite_registry`.
pub fn ecs_enemy_name(id: &str, actors: &Query<ActorSpriteData>) -> Option<String> {
    actors
        .iter()
        .find_map(|a| (a.feature_id.as_str() == id).then(|| a.config.name.clone()))
}

/// Resolve ANY brain-driven actor's animation frame from its REAL ECS clusters —
/// the SAME `Body*` movement/ability clusters, and the SAME picker, the player
/// uses ([`crate::character_sprites::pick_actor_anim`] → `body_view_from_clusters`).
/// One path, disposition-agnostic: an enemy and an NPC animate from identical
/// reads. Whatever a brain (or an LLM) drives the actor's clusters into — a dash,
/// a blink, flight, a shield, a ladder climb, a wall-grab, a dodge-roll, a
/// crouch/slide, an in-flight swing — animates with no per-archetype branch; the
/// sheet's anim set decides how richly each pose reads.
pub fn ecs_actor_anim_state(id: &str, actors: &Query<ActorSpriteData>) -> Option<ActorAnimFrame> {
    actors.iter().find_map(|a| {
        if a.feature_id.as_str() != id {
            return None;
        }
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
        Some(ActorAnimFrame {
            anim,
            pos: a.kin.pos,
            facing: a.kin.facing,
            attacking,
        })
    })
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

pub fn ecs_boss_name<'a>(
    id: &str,
    bosses: &'a Query<(
        &FeatureId,
        super::boss_clusters::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
    )>,
) -> Option<&'a str> {
    bosses.iter().find_map(|(feature_id, boss, _, _, _)| {
        (feature_id.as_str() == id).then_some(boss.config.name.as_str())
    })
}

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
