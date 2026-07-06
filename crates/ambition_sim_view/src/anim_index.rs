//! The per-actor POSE index + the per-boss FRAME index (E4 slices 3, 7,
//! 19): id-keyed read-models rebuilt once per sim tick; presentation
//! animates from these snapshots and never borrows the live clusters.

use bevy::prelude::{Query, ResMut, Resource};

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use ambition_gameplay_core::features::{
    boss_anim_state_for, ActorConfig, ActorStatus, BodyKinematics, BodyMelee, FeatureId,
};

/// Read-only query of the unified actor cluster every actor (was-NPC, was-enemy,
/// encounter mob, mount/rider) carries — the SAME `Body*` movement/ability
/// clusters the player reads, plus the actor's identity/status/config. Systems
/// declare `Query<ActorSpriteData>`; the helpers take `&Query<ActorSpriteData>`.
///
/// All fields are required (not `Option`): every spawned actor carries the full
/// [`ambition_gameplay_core::actor::AncillaryMovementBundle`] (the same bundle the player nests)
/// plus `ActorStatus` / `ActorConfig` / `BodyMelee`, so an entity that is missing
/// any of them — a boss (its own cluster + anim path) or a prop — correctly does
/// not match and is skipped, instead of half-resolving from a sparse read. This
/// is what lets [`ecs_actor_anim_state`] build the player's FULL `BodyAnimView`
/// from an actor's real clusters, so any ability a brain drives animates.
#[derive(bevy::ecs::query::QueryData)]
pub struct ActorSpriteData {
    pub feature_id: &'static FeatureId,
    pub kin: &'static BodyKinematics,
    pub status: &'static ActorStatus,
    pub health: &'static ambition_characters::actor::BodyHealth,
    pub combat: &'static ambition_characters::actor::BodyCombat,
    pub config: &'static ActorConfig,
    pub attack: &'static BodyMelee,
    pub ground: &'static ambition_gameplay_core::actor::BodyGroundState,
    pub wall: &'static ambition_gameplay_core::actor::BodyWallState,
    pub blink: &'static ambition_gameplay_core::actor::BodyBlinkState,
    pub flight: &'static ambition_gameplay_core::actor::BodyFlightState,
    pub dash: &'static ambition_gameplay_core::actor::BodyDashState,
    pub ledge: &'static ambition_gameplay_core::actor::BodyLedgeState,
    pub body_mode: &'static ambition_gameplay_core::actor::BodyModeState,
    pub env_contact: &'static ambition_gameplay_core::actor::BodyEnvironmentContact,
    pub abilities: &'static ambition_gameplay_core::actor::BodyAbilities,
    pub dodge: &'static ambition_gameplay_core::actor::BodyDodgeState,
    pub shield: &'static ambition_gameplay_core::actor::BodyShieldState,
    /// Movement-driven presentation overlays (wall-jump / dash-startup / landing /
    /// shoot poses), shared with the player. `Option` so an actor spawned without
    /// the component (a legacy / bespoke path) still animates its base ladder —
    /// it just shows no overlays (fable review §A9).
    pub anim: Option<&'static ambition_gameplay_core::player::BodyAnimFacts>,
}

/// One actor's resolved animation frame for the renderer: the chosen anim plus
/// the bits the per-frame apply needs that aren't in the anim itself — world
/// position (for localized-gravity facing), facing sign, and whether the actor
/// is mid-swing (for the warm outgoing-attack tint).
#[derive(Clone, Copy, Debug)]
pub struct ActorAnimFrame {
    pub anim: ambition_gameplay_core::character_sprites::CharacterAnim,
    pub pos: ae::Vec2,
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
/// uses ([`ambition_gameplay_core::character_sprites::pick_actor_anim`] → `body_view_from_clusters`).
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
        let anim = ambition_gameplay_core::character_sprites::pick_actor_anim(
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
            ambition_gameplay_core::character_sprites::ActorAnimState {
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

/// One boss's hazard-column lane for this frame — the visible rectangle is
/// computed sim-side from the SAME volume math as damage, so the visual and
/// the hitbox are exactly coincident (E4 slice 7).
#[derive(Clone, Copy, Debug)]
pub struct HazardLaneFact {
    /// `true` during the strike window (red solid); `false` during
    /// telegraph (yellow pulsing).
    pub striking: bool,
    pub center: ae::Vec2,
    pub size: ae::Vec2,
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
    pub anim: ambition_gameplay_core::boss_encounter::sprites::BossAnimState,
    /// The boss's combat AABB (debug health bars anchor here).
    pub aabb: ae::Aabb,
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
        ambition_gameplay_core::features::BossClusterRef,
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
            ambition_gameplay_core::features::volumes_for_profile(
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
