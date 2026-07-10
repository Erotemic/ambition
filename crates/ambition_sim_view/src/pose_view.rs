//! Per-body presentation POSE read-model for player-bodied entities (E4).
//!
//! `BodyPoseView` is the plain-data snapshot the renderer draws a
//! player-bodied entity from — the same role [`ActorAnimIndex`] plays for
//! id-keyed actor visuals, expressed as a COMPONENT because a player body's
//! sprite lives on the body entity itself (no id-keyed visual to join
//! through). Rebuilt once per tick, sim-side, LAST in the sim tail
//! (`FeatureViewSync`); presentation reads ONLY this and never queries the
//! live `Body*` clusters. Extraction is a pure function of sim state — no
//! caching across ticks — so a rollback resim rebuilds it for free.
//!
//! `ShieldRingsView` is the pooled-ring analogue for the bubble-shield
//! visual: EVERY body's raised shield (player and brain-driven alike)
//! materializes one row, so the render pool is a pure consumer.

use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer_primitives::lifecycle::PlayerVisual;
use ambition_sprite_sheet::character::CharacterAnim;
use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Resource, With};

/// Sim-resolved presentation pose for one player-bodied entity: everything
/// the renderer needs to place, size, animate, and flash the sprite. Plain
/// data (Copy, no `Entity`/`Handle` borrows) — snapshot-safe by construction.
///
/// AJ14 Tier-0: `pos` + `vel` are the per-body read-model velocity fields the
/// slower-light observer views ride.
#[derive(bevy::prelude::Component, Clone, Copy, Debug)]
pub struct BodyPoseView {
    pub pos: ambition_engine_core::Vec2,
    pub vel: ambition_engine_core::Vec2,
    /// Current collision AABB size (crouch/morph compaction included).
    pub size: ambition_engine_core::Vec2,
    /// Standing (base) AABB size — the denominator of the crouch stance
    /// ratio and the body-profile sprite scale. Falls back to `size` for a
    /// body without a `BodyBaseSize`.
    pub base_size: ambition_engine_core::Vec2,
    pub facing: f32,
    /// Aerial/gravity roll angle (radians) — the sprite rotation.
    pub roll_angle: f32,
    /// `size.y / base_size.y`, clamped (0.1, 1.0] — the trimmed-sheet
    /// stance compaction the animator applies.
    pub stance_ratio_y: f32,
    /// Gravity direction used for the facing flip (the global field read the
    /// player path has always used).
    pub gravity_dir: ambition_engine_core::Vec2,
    /// The picked animation row for this tick (the player picker over the
    /// body's real clusters).
    pub anim: CharacterAnim,
    /// Seconds remaining on the damage flash (`BodyCombat::hit_flash`).
    pub hit_flash_secs: f32,
    pub hp_current: i32,
    pub hp_max: i32,
    /// The body is in morph-ball mode (draws the procedural sphere instead
    /// of the character sheet).
    pub morph_ball: bool,
    /// Fireball charge tier while the fire button is held (`None` when not
    /// charging): 0 / 1 / 2+ pick the charge-indicator size/alpha.
    pub charge_tier: Option<u8>,
}

impl Default for BodyPoseView {
    fn default() -> Self {
        Self {
            pos: ambition_engine_core::Vec2::ZERO,
            vel: ambition_engine_core::Vec2::ZERO,
            size: ambition_engine_core::Vec2::ONE,
            base_size: ambition_engine_core::Vec2::ONE,
            facing: 1.0,
            roll_angle: 0.0,
            stance_ratio_y: 1.0,
            gravity_dir: ambition_engine_core::Vec2::Y,
            anim: CharacterAnim::Idle,
            hit_flash_secs: 0.0,
            hp_current: 0,
            hp_max: 0,
            morph_ball: false,
            charge_tier: None,
        }
    }
}

/// Rebuild every player-bodied entity's [`BodyPoseView`] from its real
/// clusters — the SAME reads `animate_player` used to make live, moved
/// sim-side. Runs in `FeatureViewSync` beside the other read-model rebuilds.
///
/// Only `BodyKinematics` is REQUIRED: a partial body (a test fixture that
/// spawns `PlayerVisual` + kinematics alone) still gets its transform facts;
/// the anim pick needs the full movement/ability cluster set (the same set
/// `animate_player` demanded) and holds `Idle` when any piece is absent —
/// exactly the frames the old live-query path would have skipped.
#[allow(clippy::type_complexity)]
pub fn rebuild_body_pose_views(
    mut commands: Commands,
    gravity: Option<Res<ambition_platformer_primitives::gravity::GravityField>>,
    mut bodies: Query<
        (
            (
                Entity,
                &ambition_actors::actor::BodyKinematics,
                Option<&ambition_actors::actor::BodyGroundState>,
                Option<&ambition_actors::actor::BodyWallState>,
                Option<&ambition_actors::actor::BodyBlinkState>,
                Option<&ambition_actors::actor::BodyFlightState>,
                Option<&ambition_actors::actor::BodyDashState>,
                Option<&ambition_actors::actor::BodyLedgeState>,
                Option<&BodyCombat>,
                Option<&ambition_actors::actor::BodyAnimFacts>,
                Option<&ambition_actors::player::PlayerBlinkCameraState>,
            ),
            (
                Option<&ambition_actors::actor::BodyModeState>,
                Option<&ambition_actors::actor::BodyEnvironmentContact>,
                Option<&ambition_actors::actor::BodyAbilities>,
                Option<&ambition_actors::actor::BodyDodgeState>,
                Option<&ambition_actors::actor::BodyShieldState>,
                Option<&ambition_actors::actor::BodyMelee>,
                Option<&ambition_engine_core::BodyBaseSize>,
                Option<&BodyHealth>,
                Option<&ambition_actors::platformer_runtime::orientation::ActorRoll>,
                Option<&ambition_projectiles::PlayerProjectileState>,
                Option<&mut BodyPoseView>,
            ),
        ),
        With<PlayerVisual>,
    >,
) {
    // The player path has always read the GLOBAL gravity field for its facing
    // flip (localized zone gravity is the actor path's read) — preserved.
    let gravity_dir = gravity
        .as_deref()
        .map_or(ambition_engine_core::Vec2::Y, |g| g.dir);
    for (
        (
            entity,
            kinematics,
            ground,
            wall,
            blink,
            flight,
            dash,
            ledge,
            combat,
            anim_facts,
            blink_cam,
        ),
        (
            body_mode,
            env_contact,
            abilities,
            dodge,
            shield,
            active_attack,
            base_size,
            health,
            roll,
            projectile_state,
            pose,
        ),
    ) in &mut bodies
    {
        let base = base_size.map_or(kinematics.size, |b| b.base_size);
        let stance_ratio_y = base_size
            .map(|b| (kinematics.size.y / b.base_size.y.max(1.0)).clamp(0.1, 1.0))
            .unwrap_or(1.0);
        // The anim pick runs only over the FULL cluster set `animate_player`
        // used to require — a partial body keeps `Idle` (it never animated
        // before either) while its transform facts stay live.
        let anim = match (
            (ground, wall, blink, flight, dash, ledge),
            (combat, anim_facts, blink_cam),
            (body_mode, env_contact, abilities, dodge, shield),
        ) {
            (
                (Some(ground), Some(wall), Some(blink), Some(flight), Some(dash), Some(ledge)),
                (Some(combat), Some(anim_facts), Some(blink_cam)),
                (Some(body_mode), Some(env_contact), Some(abilities), Some(dodge), Some(shield)),
            ) => ambition_actors::character_sprites::pick_player_anim(
                anim_facts,
                combat,
                blink_cam,
                active_attack.and_then(|a| a.swing.as_ref()),
                kinematics,
                ground,
                wall,
                blink,
                flight,
                dash,
                ledge,
                body_mode,
                env_contact,
                abilities,
                dodge,
                shield,
            ),
            _ => CharacterAnim::Idle,
        };
        let next = BodyPoseView {
            pos: kinematics.pos,
            vel: kinematics.vel,
            size: kinematics.size,
            base_size: base,
            facing: kinematics.facing,
            roll_angle: roll.map_or(0.0, |r| r.angle),
            stance_ratio_y,
            gravity_dir,
            anim,
            hit_flash_secs: combat.map_or(0.0, |c| c.hit_flash),
            hp_current: health.map_or(0, |h| h.current()),
            hp_max: health.map_or(0, |h| h.max()),
            morph_ball: body_mode
                .is_some_and(|m| m.body_mode == ambition_engine_core::BodyMode::MorphBall),
            charge_tier: projectile_state
                .and_then(|s| s.charging.map(|hold| s.charge_tuning.tier_for_hold(hold))),
        };
        match pose {
            Some(mut pose) => *pose = next,
            None => {
                commands.entity(entity).insert(next);
            }
        }
    }
}

/// One raised bubble shield, resolved sim-side. The renderer positions one
/// pooled ring sprite per row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShieldRingFact {
    pub pos: ambition_engine_core::Vec2,
    pub size: ambition_engine_core::Vec2,
    pub parrying: bool,
}

/// Every body (player AND brain-driven actor) whose shield is currently
/// raised, in query order — the read-model behind the pooled bubble-shield
/// rings.
#[derive(Resource, Default, Clone, Debug)]
pub struct ShieldRingsView(pub Vec<ShieldRingFact>);

pub fn rebuild_shield_rings_view(
    mut view: ResMut<ShieldRingsView>,
    bodies: Query<(
        &ambition_actors::actor::BodyKinematics,
        &ambition_actors::actor::BodyShieldState,
    )>,
) {
    view.0.clear();
    view.0.extend(
        bodies
            .iter()
            .filter(|(_, shield)| shield.active)
            .map(|(kin, shield)| ShieldRingFact {
                pos: kin.pos,
                size: kin.size,
                parrying: shield.parrying(),
            }),
    );
}

#[cfg(test)]
mod pose_view_tests {
    use super::*;

    #[test]
    fn shield_rings_view_defaults_empty() {
        let view = ShieldRingsView::default();
        assert!(view.0.is_empty());
    }

    #[test]
    fn body_pose_view_default_is_inert() {
        let pose = BodyPoseView::default();
        assert_eq!(pose.stance_ratio_y, 1.0);
        assert_eq!(pose.hit_flash_secs, 0.0);
        assert!(pose.charge_tier.is_none());
        assert!(!pose.morph_ball);
    }
}
