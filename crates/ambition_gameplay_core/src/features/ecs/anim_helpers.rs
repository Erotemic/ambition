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
    /// Movement-driven presentation overlays (wall-jump / dash-startup / landing /
    /// shoot poses), shared with the player. `Option` so an actor spawned without
    /// the component (a legacy / bespoke path) still animates its base ladder —
    /// it just shows no overlays (fable review §A9).
    pub anim: Option<&'static crate::player::BodyAnimFacts>,
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
                // Movement overlays from the shared BodyAnimFacts (None → all off).
                wall_jump: a.anim.is_some_and(|f| f.wall_jump_anim_timer > 0.0),
                dash_startup: a.anim.is_some_and(|f| f.dash_startup_timer > 0.0),
                landing: a
                    .anim
                    .filter(|f| f.land_anim_timer > 0.0)
                    .map(|f| f.land_anim_hard),
                shooting: a.anim.is_some_and(|f| f.shoot_anim_timer > 0.0),
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

/// Advance every non-player actor's movement-driven anim overlays (landing /
/// dash-startup) one frame, via the SAME [`crate::player::advance_body_anim_overlays`]
/// the player tick runs — so [`crate::character_sprites::pick_actor_anim`] can show
/// those poses (fable review §A9). The home player ([`crate::actor::PlayerEntity`])
/// is excluded (it advances its own overlays in the player tick), so no body is
/// advanced twice; a possessed non-player body IS advanced here. Uses `sim_dt`
/// (world-anchored animation), so the poses pause and slow with the sim. Scheduled
/// right before [`rebuild_actor_anim_index`] (its reader) and skipped headless
/// with it — these overlays are presentation-only.
pub fn advance_actor_anim_overlays(
    world_time: Res<ambition_time::WorldTime>,
    mut actors: Query<
        (
            &crate::actor::BodyGroundState,
            &crate::actor::BodyKinematics,
            &crate::actor::BodyDashState,
            &mut crate::player::BodyAnimFacts,
        ),
        Without<crate::actor::PlayerEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    for (ground, kin, dash, mut anim) in &mut actors {
        crate::player::advance_body_anim_overlays(
            ground.on_ground,
            kin.vel.y,
            dash.timer,
            &mut anim,
            dt,
        );
    }
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
    match profile.move_id().as_str() {
        "floor_slam" | "hand_slam" | "converging_shockwave" => Some(BossAnim::FloorSlam),
        "side_sweep" | "hand_sweep" | "broadside" => Some(BossAnim::SideSweep),
        "hazard_column" | "dive_lane" => Some(BossAnim::DashEcho),
        "wing_sweep" => None,
        // `full_body_pulse`, `head_descent`, and every content special fall back
        // to the spike-halo telegraph anim (a ring of damage around the boss) —
        // the closest generic visual cue. Covers the former DebrisRain /
        // MemorizedVolley / LockOnBeam / PitTrap / RotatingCross / MinionCascade.
        _ => Some(BossAnim::SpikeHalo),
    }
}

fn boss_animation_key_for_sample(
    profile: &ambition_characters::brain::BossAttackProfile,
    anim: crate::boss_encounter::sprites::BossAnim,
) -> Option<&'static str> {
    use crate::boss_encounter::sprites::BossAnim;
    match (profile.move_id().as_str(), anim) {
        // GNU-ton has profile-specific dangerous boxes (for example
        // `gnu_shockwave`) but the damageable head/body box should follow
        // the rendered row. Keep the sample keyed to the visual row so
        // authored row frames are the source of truth for hurtboxes.
        ("hand_slam" | "converging_shockwave", BossAnim::FloorSlam) => Some("hand_slam"),
        ("hand_sweep", BossAnim::SideSweep) => Some("hand_sweep"),
        ("head_descent", BossAnim::SpikeHalo) => Some("head_down"),
        // GNU-ton's apple rain reads the head row for its damageable hurtbox.
        ("apple_rain", BossAnim::SpikeHalo) => Some("head_down"),
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
    bosses.iter().find_map(
        |(entity, feature_id, boss, health, combat, attack_state, brain)| {
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
        },
    )
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
    bosses.iter().find_map(
        |(entity, feature_id, _boss, _health, _combat, attack_state, _brain)| {
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
        },
    )
}

/// One boss's hazard-column lane for this frame — the visible rectangle is
/// computed sim-side from the SAME volume math as damage, so the visual and
/// the hitbox are exactly coincident (E4 slice 7).
#[derive(Clone, Copy, Debug)]
pub struct HazardLaneFact {
    /// `true` during the strike window (red solid); `false` during
    /// telegraph (yellow pulsing).
    pub striking: bool,
    pub center: ambition_engine_core::Vec2,
    pub size: ambition_engine_core::Vec2,
}

/// Materialized per-frame boss presentation facts, keyed by [`FeatureId`]:
/// the resolved [`BossAnimState`] (facing / tint / row-selection facts), the
/// boss's collision AABB, and the hazard-column lane when one is live. The
/// MOVING half of the boss read-model — `BossRenderIndex` carries the static
/// identity. Presentation reads this by id and never borrows the live boss
/// clusters (E4 slice 7); rows are `Copy`, so the rebuild just overwrites.
#[derive(Resource, Default, Clone, Debug)]
pub struct BossFrameIndex {
    frames: std::collections::HashMap<String, (BossFrameView, u64)>,
    generation: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct BossFrameView {
    pub anim: crate::boss_encounter::sprites::BossAnimState,
    /// The boss's combat AABB (debug health bars anchor here).
    pub aabb: ambition_engine_core::Aabb,
    pub hazard_lane: Option<HazardLaneFact>,
}

impl BossFrameIndex {
    pub fn get(&self, id: &str) -> Option<BossFrameView> {
        self.frames.get(id).map(|(frame, _)| *frame)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &BossFrameView)> {
        self.frames
            .iter()
            .map(|(id, (frame, _))| (id.as_str(), frame))
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

    fn insert(&mut self, id: &str, frame: BossFrameView) {
        let gen = self.generation;
        if let Some(slot) = self.frames.get_mut(id) {
            slot.0 = frame;
            slot.1 = gen;
        } else {
            self.frames.insert(id.to_string(), (frame, gen));
        }
    }
}

/// Rebuild [`BossFrameIndex`] from the live boss clusters — the same reads
/// `animate_bosses` / `manage_gradient_lane_visual` used to make live from
/// render, moved sim-side. Runs in `FeatureViewSync`.
pub fn rebuild_boss_frame_index(
    mut index: ResMut<BossFrameIndex>,
    bosses: Query<(
        &FeatureId,
        super::boss_clusters::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::actor::BodyCombat,
        &ambition_characters::brain::BossAttackState,
        &ambition_characters::brain::Brain,
    )>,
) {
    use ambition_characters::brain::BossAttackProfile;
    index.begin_rebuild();
    for (id, feature, health, combat, attack_state, brain) in &bosses {
        let boss = feature.as_boss_ref();
        let anim = boss_anim_state_for(boss, health.alive(), combat.hit_flash, attack_state, brain);
        // Hazard-column lane: live only while an ALIVE boss telegraphs or
        // strikes `hazard_column`; the rect reuses the damage volume math.
        let in_telegraph = matches!(
            &attack_state.telegraph_profile,
            Some(p) if p.move_id() == "hazard_column"
        );
        let in_strike = matches!(
            &attack_state.active_profile,
            Some(p) if p.move_id() == "hazard_column"
        );
        let hazard_lane = if health.alive() && (in_telegraph || in_strike) {
            let boss = feature.as_boss_ref();
            crate::features::volumes_for_profile(
                &BossAttackProfile::Strike("hazard_column".to_string()),
                boss.kin.pos,
                boss.combat_size(),
                &boss.config.behavior,
            )
            .pop()
            .map(|volume| HazardLaneFact {
                striking: in_strike,
                center: volume.center(),
                size: volume.half_size() * 2.0,
            })
        } else {
            None
        };
        let boss = feature.as_boss_ref();
        index.insert(
            id.as_str(),
            BossFrameView {
                anim,
                aabb: boss.aabb(),
                hazard_lane,
            },
        );
    }
    index.end_rebuild();
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
