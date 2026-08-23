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
use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
use ambition_sprite_sheet::character::CharacterAnim;
use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Resource, With};

/// Sim-resolved presentation pose for one player-bodied entity: everything
/// the renderer needs to place, size, animate, and flash the sprite. Plain
/// data (Copy, no `Entity`/`Handle` borrows) — snapshot-safe by construction.
#[derive(bevy::prelude::Component, Clone, Debug)]
pub struct BodyPoseView {
    pub pos: ambition_platformer2d_core::Vec2,
    pub vel: ambition_platformer2d_core::Vec2,
    /// Current collision AABB size (crouch/morph compaction included).
    pub size: ambition_platformer2d_core::Vec2,
    /// Standing (base) AABB size — the denominator of the crouch stance
    /// ratio and the body-profile sprite scale. Falls back to `size` for a
    /// body without a `BodyBaseSize`.
    pub base_size: ambition_platformer2d_core::Vec2,
    pub facing: f32,
    /// Aerial/gravity roll angle (radians) — the sprite rotation.
    pub roll_angle: f32,
    /// `size.y / base_size.y`, clamped (0.1, 1.0] — the trimmed-sheet
    /// stance compaction the animator applies.
    pub stance_ratio_y: f32,
    /// Gravity direction used for the facing flip (the global field read the
    /// player path has always used).
    pub gravity_dir: ambition_platformer2d_core::Vec2,
    /// The picked animation row for this tick (the player picker over the
    /// body's real clusters).
    pub anim: CharacterAnim,
    /// What this body's ACTIVE MOVE asks to be drawn as, when one is
    /// playing — see `ActorAnimFrame::clip`, which is the same request on the
    /// actor road. `None` means *draw the semantic pose*.
    pub clip: Option<crate::ClipRequest>,
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
    /// The sprite quad this body's SHEET authored, when its geometry is
    /// sheet-authored (`SpritePosedBody`); `None` when the render must size the
    /// quad itself.
    ///
    /// Carried here rather than read as a component because the render layer
    /// does not depend on actor machinery — and because it is the same KIND of
    /// fact as `size` and `base_size` beside it: a sim-resolved geometry the
    /// renderer draws rather than re-derives.
    ///
    /// Its presence changes what the quad MEANS. Without it, `standing_render`
    /// is a guess about the art scaled by how far the collision box has drifted
    /// from its baseline — the ratio that drew Mary-O's tall form ~60% larger
    /// than the body it belonged to. With it, the box and the quad are two
    /// readings of ONE authored scale, so neither is derived from the other and
    /// there is no ratio to be wrong.
    pub authored_render: Option<ambition_platformer2d_core::Vec2>,
    /// Where to draw that quad, relative to the body centre — the companion
    /// to `authored_render` and gated on the same `SpritePosedBody`.
    pub authored_offset: Option<ambition_platformer2d_core::Vec2>,
}

impl Default for BodyPoseView {
    fn default() -> Self {
        Self {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::ONE,
            base_size: ambition_platformer2d_core::Vec2::ONE,
            facing: 1.0,
            // A body with no pose yet is playing no move.
            clip: None,
            roll_angle: 0.0,
            stance_ratio_y: 1.0,
            gravity_dir: ambition_platformer2d_core::Vec2::Y,
            anim: CharacterAnim::Idle,
            hit_flash_secs: 0.0,
            hp_current: 0,
            hp_max: 0,
            morph_ball: false,
            charge_tier: None,
            authored_render: None,
            authored_offset: None,
        }
    }
}

/// Only `BodyKinematics` is REQUIRED: a partial body (a test fixture that
/// spawns `PlayerVisual` + kinematics alone) still gets its transform facts;
/// the anim pick needs the full movement/ability cluster set (the same set
/// `animate_player` demanded) and holds `Idle` when any piece is absent —
/// exactly the frames the old live-query path would have skipped.
#[allow(clippy::type_complexity)]
pub fn rebuild_body_pose_views(
    mut commands: Commands,
    gravity: Option<Res<ambition_platformer2d_shared_tangle::gravity::GravityField>>,
    mut bodies: Query<
        (
            (
                Entity,
                &ambition_platformer2d_actor_monolith::actor::BodyKinematics,
                Option<&ambition_platformer2d_actor_monolith::actor::BodyGroundState>,
                Option<&ambition_platformer2d_core::BodyMotionFacts>,
                Option<&ambition_platformer2d_actor_monolith::actor::BodyFlightState>,
                Option<&BodyCombat>,
                Option<&ambition_platformer2d_actor_monolith::actor::BodyAnimFacts>,
                Option<&ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState>,
                // This body's own resolved basis, so the locomotion metric is
                // measured along ITS run axis.  deliberately not the global
                // `GravityField` read below: that one drives the facing flip and
                // is a mirror of the PRIMARY body's frame.
                Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
            ),
            (
                Option<&ambition_platformer2d_actor_monolith::actor::BodyModeState>,
                Option<&ambition_platformer2d_actor_monolith::actor::BodyEnvironmentContact>,
                Option<&ambition_platformer2d_actor_monolith::actor::BodyAbilities>,
                Option<&ambition_platformer2d_actor_monolith::actor::BodyShieldState>,
                Option<&ambition_platformer2d_actor_monolith::actor::BodyMelee>,
                Option<&ambition_platformer2d_core::BodyBaseSize>,
                Option<&BodyHealth>,
                Option<&ambition_platformer2d_actor_monolith::platformer_runtime::orientation::ActorRoll>,
                Option<&ambition_projectiles::PlayerProjectileState>,
                // Content-driven pose PIN — the SAME component the actor path
                // honours in `rebuild_actor_anim_index`. See the read below.
                Option<
                    &ambition_sprite_sheet::character::ActorAnimOverride,
                >,
                // The sheet-authored sprite quad, when this body's geometry is its art.
                //
                //  gated on `SpritePosedBody`, NOT on the render size alone.
                // `ActorRenderSize` is the SHARED sprite-quad component — several
                // spawn paths insert it from sheet metadata for bodies whose
                // collision box is still hand-authored. Reading it bare here
                // would tell the renderer "the sheet authored this geometry"
                // about every one of them, and hand it a quad that was never
                // derived from their boxes.
                Option<&ambition_platformer2d_actor_monolith::features::ActorRenderSize>,
                // The quad's PLACEMENT, published beside its size by the same
                // pass. Gated on `SpritePosedBody` below for the same reason the
                // size is: several spawn paths insert these from sheet metadata
                // for bodies whose collision box is still hand-authored.
                Option<&ambition_platformer2d_actor_monolith::features::ActorSpriteOffset>,
                bevy::prelude::Has<
                    ambition_sprite_sheet::character::SpritePosedBody,
                >,
                // The move this body is playing, so the drawn row can be the
                // one the move NAMES — the same request the actor path carries
                // on `ActorAnimFrame::clip` (sprite redirect P0). Fifteen members
                // in this sub-tuple; Bevy's limit is sixteen.
                Option<&ambition_combat::moveset::MovePlayback>,
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
        .map_or(ambition_platformer2d_core::Vec2::Y, |g| g.dir);
    for (
        (
            entity,
            kinematics,
            ground,
            motion_facts,
            flight,
            combat,
            anim_facts,
            blink_cam,
            body_frame,
        ),
        (
            body_mode,
            env_contact,
            abilities,
            shield,
            active_attack,
            base_size,
            health,
            roll,
            projectile_state,
            anim_override,
            authored_render,
            authored_offset,
            sheet_authored_body,
            playback,
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
            (ground, motion_facts, flight),
            (combat, anim_facts, blink_cam),
            (body_mode, env_contact, abilities, shield),
        ) {
            (
                (Some(ground), Some(motion_facts), Some(flight)),
                (Some(combat), Some(anim_facts), Some(blink_cam)),
                (Some(body_mode), Some(env_contact), Some(abilities), Some(shield)),
            ) => ambition_character_sprites::pick_player_anim(
                anim_facts,
                combat,
                blink_cam,
                active_attack.and_then(|a| a.swing.as_ref()),
                kinematics,
                ground,
                motion_facts,
                flight,
                body_mode,
                env_contact,
                abilities,
                shield,
                body_frame.map_or_else(
                    || {
                        ambition_platformer2d_core::AccelerationFrame::new(
                            ambition_platformer2d_core::DEFAULT_GRAVITY_DIR,
                        )
                    },
                    |f| f.basis(),
                ),
            ),
            _ => CharacterAnim::Idle,
        };
        // A content pose PIN wins over the picked pose — the SAME rule `rebuild_actor_anim_index`
        // applies to every brain-driven actor, and it belongs here for the identical reason: a pose
        // the disposition-agnostic picker cannot infer (a shelled enemy's withdraw, a body
        // mid-power-up) is stated by the content that owns the state machine.
        let anim = anim_override.map(|o| o.0).unwrap_or(anim);
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
            //  a MOVE names its row; failing that, a fighter STATE does — the
            // same two-step the actor road takes, so the two never disagree.
            clip: playback
                .map(|playback| crate::ClipRequest {
                    clip: playback.spec.clip.clip.clone(),
                    fallbacks: playback.spec.clip.fallbacks.clone(),
                })
                .or_else(|| {
                    crate::ClipRequest::from_chain(ambition_character_sprites::body_state_clip(
                        motion_facts?,
                        ambition_character_sprites::FighterClipFacts {
                            held: anim_facts.is_some_and(|f| f.held),
                            holding: anim_facts.is_some_and(|f| f.holding),
                            guard_break: shield
                                .and_then(|s| s.break_phase())
                                .map(ambition_character_sprites::GuardBreakBeat::from_phase),
                            parrying: shield.is_some_and(|s| s.parrying()),
                            guard_stunned: shield.is_some_and(|s| s.stun_timer > 0.0),
                        },
                    )?)
                }),
            hit_flash_secs: combat.map_or(0.0, |c| c.hit_flash),
            hp_current: health.map_or(0, |h| h.current()),
            hp_max: health.map_or(0, |h| h.max()),
            morph_ball: body_mode
                .is_some_and(|m| m.body_mode == ambition_platformer2d_core::BodyMode::MorphBall),
            charge_tier: projectile_state
                .and_then(|s| s.charging.map(|hold| s.charge_tuning.tier_for_hold(hold))),
            authored_render: sheet_authored_body
                .then(|| authored_render.map(|r| r.0))
                .flatten(),
            authored_offset: sheet_authored_body
                .then(|| authored_offset.map(|o| o.0))
                .flatten(),
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
    pub pos: ambition_platformer2d_core::Vec2,
    pub size: ambition_platformer2d_core::Vec2,
    pub parrying: bool,
    /// `1.0` whole, `0.0` about to break — and `1.0` for a body whose guard is
    /// not a resource, so the ring reads the same for every body that has one.
    pub integrity: f32,
}

/// Every body (player AND brain-driven actor) whose shield is currently
/// raised, in query order — the read-model behind the pooled bubble-shield
/// rings.
#[derive(Resource, Default, Clone, Debug)]
pub struct ShieldRingsView(pub Vec<ShieldRingFact>);

pub fn rebuild_shield_rings_view(
    mut view: ResMut<ShieldRingsView>,
    bodies: Query<(
        &ambition_platformer2d_actor_monolith::actor::BodyKinematics,
        &ambition_platformer2d_actor_monolith::actor::BodyShieldState,
        // Publish the presented pose because pooled shield rendering has no
        // owner entity left with which to resolve presentation offsets.
        Option<&crate::presented_pose::PresentedPose>,
        // The body's own shield tuning, so the ring can SHOW the resource
        // draining. A body with no motion model draws a whole ring.
        Option<&ambition_platformer2d_core::MotionModel>,
    )>,
) {
    view.0.clear();
    view.0
        .extend(bodies.iter().filter(|(_, shield, _, _)| shield.active).map(
            |(kin, shield, presented, model)| ShieldRingFact {
                pos: presented.map_or(kin.pos, |p| p.presented()),
                size: kin.size,
                parrying: shield.parrying(),
                integrity: model.map_or(1.0, |m| shield.integrity_fraction(m.shield_tuning())),
            },
        ));
}

/// One body in INVOLUNTARY flight, resolved sim-side.
///
/// A row exists only while the body is launched — tumbling from a hit, or
/// still inside the hitstun that hit gave it. Presentation reads the row's
/// speed to decide how hard the launch reads; it never has to ask why a body
/// is moving fast, which is the question velocity alone answers wrongly for a
/// run, a fast fall or a recovery.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchedBodyFact {
    pub pos: ambition_platformer2d_core::Vec2,
    pub vel: ambition_platformer2d_core::Vec2,
    /// The body's current collision AABB — presentation offsets the trail
    /// behind the body by a fraction of it.
    pub size: ambition_platformer2d_core::Vec2,
}

/// Every body (player AND brain-driven fighter) currently in an involuntary
/// flight, in query order — the read-model behind the hard-launch trail.
///
/// Pooled rather than a per-entity component for the reason
/// [`ShieldRingsView`] is: the effect is world particles with no owner entity
/// to hang a view on, and a seated fighter's sprite is a SEPARATE entity from
/// its body, joined by feature id. A pooled row is the one shape both roads
/// share.
#[derive(Resource, Default, Clone, Debug)]
pub struct LaunchedBodiesView(pub Vec<LaunchedBodyFact>);

pub fn rebuild_launched_bodies_view(
    mut view: ResMut<LaunchedBodiesView>,
    bodies: Query<(
        &ambition_platformer2d_actor_monolith::actor::BodyKinematics,
        Option<&ambition_platformer2d_core::BodyMotionFacts>,
        Option<&BodyCombat>,
        // The presented position, so the plume leaves the body where the
        // sprite is drawn rather than at the tick position it is interpolating
        // away from.
        Option<&crate::presented_pose::PresentedPose>,
    )>,
) {
    view.0.clear();
    view.0.extend(bodies.iter().filter_map(|(kin, motion, combat, presented)| {
        // Two published sim facts, one resolved answer: the tumble is the
        // helpless half of a launch and the hitstun is the rest of it. A
        // consumer reading only the tumble would drop the row the instant a
        // launched body stopped tumbling, mid-flight.
        let launched = motion.is_some_and(|f| f.tumbling)
            || combat.is_some_and(|c| c.hitstun_timer > 0.0);
        launched.then(|| LaunchedBodyFact {
            pos: presented.map_or(kin.pos, |p| p.presented()),
            vel: kin.vel,
            size: kin.size,
        })
    }));
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

    /// A sheet-authored quad is reported only by a body that HAS one.
    ///
    /// `ActorRenderSize` alone cannot support that claim: it is the shared sprite-quad
    /// component, and several spawn paths set it from sheet metadata for bodies whose collision
    /// box is still hand-authored. Reading it bare would silently retarget the render for every
    /// such body, which is a change nothing would report.
    ///
    /// The negative case is the whole test — the positive one only proves the
    /// field is wired at all.
    #[test]
    fn only_a_sheet_authored_body_reports_an_authored_quad() {
        use ambition_platformer2d_actor_monolith::features::ActorRenderSize;
        use ambition_sprite_sheet::character::SpritePosedBody;

        let mut app = bevy::prelude::App::new();
        app.add_systems(bevy::prelude::Update, rebuild_body_pose_views);

        let kin = ambition_platformer2d_actor_monolith::actor::BodyKinematics {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
            facing: 1.0,
        };
        let quad = ambition_platformer2d_core::Vec2::new(61.0, 73.0);

        // A quad, but the body's box is its own business.
        let hand_authored = app
            .world_mut()
            .spawn((PlayerVisual, kin, ActorRenderSize(quad)))
            .id();
        // A quad that came from the same scale as the box.
        let sheet_authored = app
            .world_mut()
            .spawn((
                PlayerVisual,
                kin,
                ActorRenderSize(quad),
                SpritePosedBody::new("robot", 2.0),
            ))
            .id();

        app.update();

        assert_eq!(
            app.world()
                .get::<BodyPoseView>(hand_authored)
                .and_then(|p| p.authored_render),
            None,
            "a render size without a sheet-authored body is not a claim about \
             that body's geometry, and must not be reported as one"
        );
        assert_eq!(
            app.world()
                .get::<BodyPoseView>(sheet_authored)
                .and_then(|p| p.authored_render),
            Some(quad),
            "and a body whose geometry IS its art reports the quad that came \
             from the same scale as its box"
        );
    }

    /// A content pose pin reaches the PLAYER's view, not only an actor's.
    ///
    /// `ActorAnimOverride` is how content states a pose the locomotion picker cannot infer — a
    /// shell withdrawing, a body mid-transformation.
    ///
    /// The pin is the whole assertion: an unpinned body here picks `Idle` (the
    /// partial-cluster fallback), so a view that answers `Idle` under a pin is
    /// exactly the silent-drop failure, and one that answers `Idle` without a
    /// pin proves the test is reading the field the pin would have changed.
    #[test]
    fn a_content_pose_pin_reaches_the_players_pose_view() {
        use ambition_sprite_sheet::character::ActorAnimOverride;

        let mut app = bevy::prelude::App::new();
        app.add_systems(bevy::prelude::Update, rebuild_body_pose_views);

        let kin = ambition_platformer2d_actor_monolith::actor::BodyKinematics {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
            facing: 1.0,
        };
        let unpinned = app.world_mut().spawn((PlayerVisual, kin)).id();
        let pinned = app
            .world_mut()
            .spawn((PlayerVisual, kin, ActorAnimOverride(CharacterAnim::Grow)))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<BodyPoseView>(unpinned).map(|p| p.anim),
            Some(CharacterAnim::Idle),
            "an unpinned body picks its pose; `Idle` is the partial-cluster read"
        );
        assert_eq!(
            app.world().get::<BodyPoseView>(pinned).map(|p| p.anim),
            Some(CharacterAnim::Grow),
            "and a pinned one shows what the content pinned"
        );
    }
}
