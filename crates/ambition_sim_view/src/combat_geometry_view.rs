//! Body-generic combat geometry for observers.
//!
//! This read-model answers the two questions a combat debugger needs without
//! asking who controls a body: where can this body be struck? and where
//! are live strikes right now?  The extraction mirrors the combat resolver's
//! geometry rule: an unpublished/missing `DamageableVolumes` falls back to the
//! coarse body box, a published empty list is intangible, and a published list
//! is used verbatim.

use ambition_combat::components::{CenteredAabb, DamageableVolumes};
use ambition_combat::hitbox::{Hitbox, HitboxAnchor};
use ambition_combat::strike::HitboxHits;
use ambition_platformer2d_core as ae;
use bevy::prelude::{Query, ResMut, Resource, With};

/// What a body's move is doing RIGHT NOW, projected for an observer.
///
/// The readout that turns a box renderer into a tuning instrument: a designer
/// watching a swing needs to see which phase it is in and how far through, not
/// infer it from when the red box appeared.
#[derive(Clone, Debug, PartialEq)]
pub struct CombatMoveView {
    /// Authored move id, so the readout names the thing being tuned.
    pub id: String,
    /// The window the move's clock is inside, as authored — `Startup`,
    /// `Active`, `Recovery`, `Invuln`, `Armor`, `Cancel`. `None` between
    /// authored windows, which is itself worth seeing.
    pub phase: Option<ambition_entity_catalog::WindowTag>,
    /// Seconds of the owner's proper time elapsed, and the move's total. Both,
    /// because "0.18s in" means nothing without "of 0.68".
    pub elapsed_s: f32,
    pub duration_s: f32,
    /// The orientation the move COMMITTED to at startup — not the body's live
    /// facing. Seeing the two disagree is the whole point of the snapshot.
    pub attack_facing: f32,
    /// Has this move already connected? Drives cancels, and explains why a
    /// follow-up did or did not become available.
    pub landed_hit: bool,
    /// WHICH USE of this move this is — see `MovePlayback::instance`.
    ///
    /// ⭐ AN OBSERVER CANNOT TELL ONE USE FROM THE NEXT BY ID. A self-cancel
    /// replaces `jab` with a fresh `jab` in the same update, and a reader
    /// comparing ids across ticks sees one continuous move: it reports the
    /// second press as never accepted, and credits the first use's contact to
    /// the second.
    pub instance: u32,
}

/// WHERE a body's damageable geometry came from.
///
/// The three-way rule this view already implements, named rather than left for
/// the reader to infer from an empty list. An inspector that shows a hurtbox
/// must be able to say whether the runtime AUTHORED it or fell back to the
/// coarse body box, because those answer different authoring questions: the
/// first is a silhouette somebody tuned, the second is the absence of one.
///
/// ⛔ `Published` DOES NOT MEAN "AUTHORED BY A MOVE". The default publisher
/// (`refresh_body_damageable_volumes`) publishes the coarse envelope as a
/// single volume when a body has no `ResolvedHurtboxes`, so a published single
/// box may be either. Distinguishing those needs the resolved-hurtbox component
/// itself, which lives above this crate; this view states what it can SEE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HurtboxSource {
    /// A publisher spoke for this body and named volumes.
    Published,
    /// Nothing published, so the resolver falls back to the coarse body box —
    /// and so does this view.
    BodyFallback,
    /// A publisher explicitly made this body unhittable. No volumes, on
    /// purpose: an absence that is a decision rather than a gap.
    Intangible,
}

impl HurtboxSource {
    /// The stable word an artifact writes down.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::BodyFallback => "body_fallback",
            Self::Intangible => "intangible",
        }
    }
}

/// One combat body's geometry AND the state a designer tunes against.
#[derive(Clone, Debug, PartialEq)]
pub struct CombatBodyGeometryView {
    /// Which body this row is, so an observer can label it and follow it.
    pub body: bevy::prelude::Entity,
    pub collision: ae::Aabb,
    pub hurtboxes: Vec<ae::CombatVolume>,
    /// Which of the runtime's three damageable-volume states produced
    /// [`Self::hurtboxes`]. An empty list means two different things —
    /// deliberately intangible, or a body this view could not resolve — and an
    /// inspector that cannot tell them apart reports a bug for a rule.
    pub hurtbox_source: HurtboxSource,
    /// Accumulated damage — "percent". The number every knockback
    /// calculation in the game reads, and the one a tuner is watching.
    pub damage_taken: i32,
    /// The body's LIVE locomotion facing. Compare with
    /// [`CombatMoveView::attack_facing`].
    pub facing: f32,
    /// Seconds of hitstun left: the window in which this body has reduced
    /// authority because something hit it.
    pub hitstun_s: f32,
    /// Seconds of hitlag left: the freeze a connect bought, on either side of
    /// it.
    pub hitlag_s: f32,
    /// Seconds of authored landing lag left — a hard lock that is NOT hitstun,
    /// and reads identically on screen unless the instrument distinguishes them.
    pub landing_lag_s: f32,
    /// Seconds of jump-squat left — the body is CROUCHING, on purpose, before a
    /// leap it already committed to. on screen this is indistinguishable from
    /// "the jump input did nothing", which is exactly why the instrument names
    /// it separately.
    pub jump_squat_s: f32,
    /// outside hitstun it is ordinary locomotion; the observer pairs it with
    /// [`Self:hitstun_s`].
    pub velocity: ae::Vec2,
    /// Semantic contact facts, so "why did it not turn / not jump" is visible.
    pub grounded: bool,
    /// Is this body touching a wall, and which way does the wall face? The
    /// SEMANTIC contact the movement kernel publishes — the fact autonomous
    /// policy turns on, and the one that explains a body pressing into geometry.
    pub on_wall: bool,
    pub wall_normal_x: f32,
    /// The move this body is playing, if any.
    pub move_state: Option<CombatMoveView>,
}

/// Exact live strike geometry, already resolved into world space.
#[derive(Clone, Debug, PartialEq)]
pub struct CombatStrikeGeometryView {
    pub volume: ae::CombatVolume,
    /// The live strike entity, so an observer can tie a per-strike visual to
    /// the volume that owns it and retire the visual when the strike ends.
    ///
    /// an identity, not a handle to reach back through. An observer may
    /// compare it and key on it; it must not use it to `get::<Hitbox>()` and
    /// read the authoritative component, which is the coupling this row exists
    /// to remove.
    pub strike: bevy::prelude::Entity,
    /// The body whose strike this is.
    pub owner: bevy::prelude::Entity,
    /// What connecting with this volume is authored to cost. The one attack
    /// PROPERTY an observer needs to interpret a contact at all — a geometry
    /// row that omits it forces every consumer back to the `Hitbox` component
    /// this view exists to keep them away from.
    pub damage: i32,
    /// The distinction presentation actually needs: only a body-tracking strike
    /// stands in for somebody's attack, and only a body-tracking strike takes
    /// its owner's presentation translation.
    pub anchored_to_body: bool,
    /// The bodies this strike HAS ALREADY CONNECTED WITH.
    ///
    /// ⭐⭐ OVERLAP IS NOT A HIT, and an observer must never conclude one from
    /// the other. A volume can pass through a body that is intangible, on
    /// another team, already hit by this same strike, or shielded — so a picture
    /// showing two rectangles touching cannot say whether the game agreed. This
    /// is the runtime's own answer: `HitboxHits` is the hit-once set the
    /// resolver writes, and it is sim truth under rollback.
    ///
    /// ⛔ SORTED. The set behind it is a `HashSet`, whose iteration order is
    /// randomized per process — an observation built from it unsorted differs
    /// between two runs of one binary that simulated identically.
    pub hit: Vec<bevy::prelude::Entity>,
}

/// Presentation-facing snapshot of authoritative combat geometry.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct CombatGeometryView {
    pub bodies: Vec<CombatBodyGeometryView>,
    pub strikes: Vec<CombatStrikeGeometryView>,
}

/// The runtime's three-way damageable rule, and WHICH of the three it was.
///
/// The rule and its provenance are one decision, so they are returned together:
/// a caller that recomputed the source from the volumes would be inferring a
/// rule from its consequence, and an intangible body and an unresolvable one
/// both hand back an empty list.
fn effective_hurtboxes(
    collision: ae::Aabb,
    damageable: Option<&DamageableVolumes>,
) -> (Vec<ae::CombatVolume>, HurtboxSource) {
    match damageable {
        Some(published) if published.intangible() => (Vec::new(), HurtboxSource::Intangible),
        Some(published) if published.published() => {
            (published.volumes.clone(), HurtboxSource::Published)
        }
        _ => (
            vec![ae::CombatVolume::aabb(collision)],
            HurtboxSource::BodyFallback,
        ),
    }
}

/// Rebuild the combat-geometry observation from current simulation truth.
///
/// `BodyCombat` is the participation predicate: human-controlled fighters,
/// brain-controlled fighters, possessed bodies, and bosses all qualify through
/// the same component. No `PrimaryPlayerOnly`, controller, or faction marker is
/// consulted.
pub fn rebuild_combat_geometry_view(
    bodies: Query<
        (
            bevy::prelude::Entity,
            &CenteredAabb,
            Option<&DamageableVolumes>,
            &ambition_characters::actor::BodyCombat,
            Option<&ambition_characters::actor::BodyHealth>,
            Option<&ae::BodyKinematics>,
            Option<&ae::BodyGroundState>,
            Option<&ae::BodyWallState>,
            Option<&ambition_combat::moveset::MovePlayback>,
            Option<&ae::MotionModel>,
        ),
        With<ambition_characters::actor::BodyCombat>,
    >,
    hitboxes: Query<(bevy::prelude::Entity, &Hitbox, Option<&HitboxHits>)>,
    owner_boxes: Query<&CenteredAabb>,
    owner_kinematics: Query<&ae::BodyKinematics>,
    mut view: ResMut<CombatGeometryView>,
) {
    view.bodies.clear();
    view.strikes.clear();

    for (body, aabb, damageable, combat, health, kin, ground, wall, playback, motion) in &bodies {
        let collision = aabb.aabb();
        let (hurtboxes, hurtbox_source) = effective_hurtboxes(collision, damageable);
        view.bodies.push(CombatBodyGeometryView {
            body,
            collision,
            hurtboxes,
            hurtbox_source,
            damage_taken: health.map(|h| h.damage_taken()).unwrap_or(0),
            facing: kin.map(|k| k.facing).unwrap_or(1.0),
            hitstun_s: combat.hitstun_timer,
            hitlag_s: combat.hitstop_timer,
            landing_lag_s: combat.landing_lag_timer,
            jump_squat_s: motion.map(|m| m.jump_squat_remaining()).unwrap_or(0.0),
            velocity: kin.map(|k| k.vel).unwrap_or_default(),
            grounded: ground.map(|g| g.on_ground).unwrap_or(false),
            on_wall: wall.map(|w| w.on_wall).unwrap_or(false),
            wall_normal_x: wall.map(|w| w.wall_normal_x).unwrap_or(0.0),
            move_state: playback.map(|pb| CombatMoveView {
                id: pb.spec.id.clone(),
                phase: pb
                    .spec
                    .windows
                    .iter()
                    .find(|w| pb.t >= w.start_s && pb.t < w.end_s)
                    .map(|w| w.tag.clone()),
                elapsed_s: pb.t,
                duration_s: pb.spec.duration_s,
                attack_facing: pb.facing,
                landed_hit: pb.landed_hit,
                instance: pb.instance,
            }),
        });
    }

    for (strike, hitbox, hits) in &hitboxes {
        let owner_pos = match hitbox.anchor {
            HitboxAnchor::World { .. } => Some(ae::Vec2::ZERO),
            HitboxAnchor::FollowOwner { .. } => owner_boxes
                .get(hitbox.owner)
                .map(|aabb| aabb.center)
                .or_else(|_| owner_kinematics.get(hitbox.owner).map(|kin| kin.pos))
                .ok(),
        };
        let Some(owner_pos) = owner_pos else {
            continue;
        };
        view.strikes.push(CombatStrikeGeometryView {
            volume: hitbox.world_volume(owner_pos),
            strike,
            owner: hitbox.owner,
            damage: hitbox.damage,
            anchored_to_body: matches!(hitbox.anchor, HitboxAnchor::FollowOwner { .. }),
            hit: {
                let mut victims: Vec<_> =
                    hits.map(|hits| hits.hit.iter().copied().collect()).unwrap_or_default();
                victims.sort();
                victims
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat::hitbox::{HitSide, HitboxKnockback};
    use ambition_platformer2d_core::AabbExt;
    use bevy::prelude::*;

    #[test]
    fn combat_geometry_needs_no_privileged_primary_body() {
        let mut app = App::new();
        app.init_resource::<CombatGeometryView>();
        app.add_systems(Update, rebuild_combat_geometry_view);

        let body_center = ae::Vec2::new(120.0, 80.0);
        let collision = ae::Aabb::new(body_center, ae::Vec2::new(12.0, 18.0));
        let authored_hurt = ae::Aabb::new(
            body_center + ae::Vec2::new(2.0, -3.0),
            ae::Vec2::new(7.0, 11.0),
        );
        let owner = app
            .world_mut()
            .spawn((
                CenteredAabb::from_aabb(collision),
                DamageableVolumes::single(authored_hurt),
                ambition_characters::actor::BodyCombat::default(),
            ))
            .id();
        app.world_mut().spawn(Hitbox {
            // Not a windbox: these fixtures are about the geometry VIEW.
            owner,
            source: HitSide::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::new(20.0, 0.0),
            },
            half_extent: ae::Vec2::new(5.0, 6.0),
            shape: None,
            facing: 1.0,
            damage: 2,
            knockback: HitboxKnockback::LaunchSpeed {
                base: 100.0,
                growth: Some(2.0),
            },
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
            strike_sfx: None,
            reaction: None,
        });

        app.update();

        let view = app.world().resource::<CombatGeometryView>();
        assert_eq!(view.bodies.len(), 1);
        assert_eq!(view.bodies[0].collision, collision);
        assert_eq!(
            view.bodies[0].hurtboxes,
            vec![ae::CombatVolume::aabb(authored_hurt)]
        );
        assert_eq!(view.strikes.len(), 1);
        assert_eq!(
            view.strikes[0].volume.bounds().center(),
            body_center + ae::Vec2::new(20.0, 0.0)
        );
    }

    #[test]
    fn world_anchored_strike_does_not_need_a_live_owner() {
        let mut app = App::new();
        app.init_resource::<CombatGeometryView>();
        app.add_systems(Update, rebuild_combat_geometry_view);

        let owner = app.world_mut().spawn_empty().id();
        app.world_mut().despawn(owner);
        let center = ae::Vec2::new(310.0, 170.0);
        app.world_mut().spawn(Hitbox {
            // Not a windbox: these fixtures are about the geometry VIEW.
            owner,
            source: HitSide::Boss,
            anchor: HitboxAnchor::World { center },
            half_extent: ae::Vec2::new(16.0, 9.0),
            shape: None,
            facing: 1.0,
            damage: 1,
            knockback: HitboxKnockback::FeelScale(1.0),
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
            strike_sfx: None,
            reaction: None,
        });

        app.update();

        let view = app.world().resource::<CombatGeometryView>();
        assert_eq!(view.strikes.len(), 1);
        assert_eq!(view.strikes[0].volume.bounds().center(), center);
    }

    #[test]
    fn combat_geometry_preserves_intangible_and_fallback_states() {
        let collision = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(10.0, 12.0));
        assert_eq!(
            effective_hurtboxes(collision, None),
            (
                vec![ae::CombatVolume::aabb(collision)],
                HurtboxSource::BodyFallback
            )
        );
        let unpublished = DamageableVolumes::default();
        assert_eq!(
            effective_hurtboxes(collision, Some(&unpublished)),
            (
                vec![ae::CombatVolume::aabb(collision)],
                HurtboxSource::BodyFallback
            )
        );
        let mut intangible = DamageableVolumes::default();
        intangible.clear();
        assert_eq!(
            effective_hurtboxes(collision, Some(&intangible)),
            (Vec::new(), HurtboxSource::Intangible)
        );
    }

    /// An empty hurtbox list has TWO meanings, and the view now says which.
    ///
    /// ⛔⛔ THE ONE THAT LOOKS LIKE A BUG IS THE CORRECT ONE. A body mid-dodge
    /// publishes no volumes ON PURPOSE; a body whose publisher has not run yet
    /// falls back to its coarse box. Both used to reach an observer as
    /// geometry with no label, so an inspector could report "this attack should
    /// have hit" for a frame where the rule says it could not.
    #[test]
    fn an_intangible_body_is_not_a_body_with_no_geometry() {
        let mut app = App::new();
        app.init_resource::<CombatGeometryView>();
        app.add_systems(Update, rebuild_combat_geometry_view);

        let collision = ae::Aabb::new(ae::Vec2::new(10.0, 10.0), ae::Vec2::new(6.0, 12.0));
        let mut intangible = DamageableVolumes::default();
        intangible.clear();
        app.world_mut().spawn((
            CenteredAabb::from_aabb(collision),
            intangible,
            ambition_characters::actor::BodyCombat::default(),
        ));
        // No `DamageableVolumes` at all: the coarse fallback, which is a
        // different fact from being deliberately unhittable.
        app.world_mut().spawn((
            CenteredAabb::from_aabb(collision),
            ambition_characters::actor::BodyCombat::default(),
        ));

        app.update();
        let view = app.world().resource::<CombatGeometryView>();
        let sources: Vec<_> = view.bodies.iter().map(|b| b.hurtbox_source).collect();
        assert!(sources.contains(&HurtboxSource::Intangible));
        assert!(sources.contains(&HurtboxSource::BodyFallback));
        for body in &view.bodies {
            assert_eq!(
                body.hurtboxes.is_empty(),
                body.hurtbox_source == HurtboxSource::Intangible,
                "only the intangible state publishes no volumes"
            );
        }
    }

    /// ⛔⛔ OVERLAP IS NOT A HIT. Two rectangles touching cannot say whether the
    /// game agreed — the victim may be intangible, on the same team, or already
    /// struck by this same strike. The runtime's own hit-once set is carried so
    /// an observer never has to conclude a connect from geometry.
    #[test]
    fn a_strike_reports_the_bodies_it_has_actually_connected_with() {
        use ambition_combat::strike::HitboxHits;
        let mut app = App::new();
        app.init_resource::<CombatGeometryView>();
        app.add_systems(Update, rebuild_combat_geometry_view);

        let owner = app
            .world_mut()
            .spawn((
                CenteredAabb::from_center_size(ae::Vec2::ZERO, ae::Vec2::new(8.0, 16.0)),
                ambition_characters::actor::BodyCombat::default(),
            ))
            .id();
        let victim = app.world_mut().spawn_empty().id();
        app.world_mut().spawn((
            Hitbox {
                // Not a windbox: this fixture is about the geometry VIEW.
                owner,
                source: HitSide::Player,
                anchor: HitboxAnchor::FollowOwner {
                    local_offset: ae::Vec2::new(12.0, 0.0),
                },
                half_extent: ae::Vec2::new(5.0, 6.0),
                shape: None,
                facing: 1.0,
                damage: 3,
                knockback: HitboxKnockback::FeelScale(1.0),
                launch_dir: None,
                frame_down: ae::Vec2::new(0.0, 1.0),
                strike_sfx: None,
                reaction: None,
            },
            HitboxHits {
                hit: std::iter::once(victim).collect(),
            },
        ));

        app.update();
        let view = app.world().resource::<CombatGeometryView>();
        assert_eq!(view.strikes[0].hit, vec![victim]);
    }

    /// The strike row carries what a contact COSTS, so no observer needs the
    /// `Hitbox` component to interpret it.
    #[test]
    fn a_strike_row_carries_its_damage() {
        let mut app = App::new();
        app.init_resource::<CombatGeometryView>();
        app.add_systems(Update, rebuild_combat_geometry_view);

        let owner = app
            .world_mut()
            .spawn((
                CenteredAabb::from_center_size(ae::Vec2::ZERO, ae::Vec2::new(8.0, 16.0)),
                ambition_characters::actor::BodyCombat::default(),
            ))
            .id();
        app.world_mut().spawn(Hitbox {
            // Not a windbox: this fixture is about the geometry VIEW.
            owner,
            source: HitSide::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::new(12.0, 0.0),
            },
            half_extent: ae::Vec2::new(5.0, 6.0),
            shape: None,
            facing: 1.0,
            damage: 13,
            knockback: HitboxKnockback::FeelScale(1.0),
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
            strike_sfx: None,
            reaction: None,
        });

        app.update();
        let view = app.world().resource::<CombatGeometryView>();
        assert_eq!(view.strikes.len(), 1);
        assert_eq!(view.strikes[0].damage, 13);
    }

    /// The tuning readout is a projection, and it needs no protagonist.
    ///
    /// percent, hitstop and hitstun, semantic contact — is what a designer reads
    /// INSTEAD of a log while dialling combat. This asserts the read model
    /// carries all of it for a body with no `PrimaryPlayerOnly`, no controller
    /// marker and no faction, because "do not make any of those require a
    /// designated primary protagonist" is half the ask.
    #[test]
    fn the_tuning_readout_reaches_a_body_no_human_controls() {
        let mut app = App::new();
        app.init_resource::<CombatGeometryView>();
        app.add_systems(Update, rebuild_combat_geometry_view);

        let mut spec = ambition_combat::moveset::simple_melee(
            &ambition_combat::moveset::SimpleMeleeParams::default(),
        );
        spec.id = "test_swat".to_string();
        let mut playback = ambition_combat::moveset::MovePlayback::new(spec, -1.0);
        // Park the clock inside the authored Startup window.
        playback.t = 0.01;

        let center = ae::Vec2::new(40.0, 60.0);
        app.world_mut().spawn((
            CenteredAabb::from_center_size(center, ae::Vec2::new(12.0, 24.0)),
            ambition_characters::actor::BodyCombat {
                hitstun_timer: 0.21,
                hitstop_timer: 0.07,
                landing_lag_timer: 0.13,
                ..Default::default()
            },
            ambition_characters::actor::BodyHealth::restored(
                ambition_characters::actor::Health::new(100),
                47,
                Default::default(),
            ),
            ae::BodyKinematics {
                pos: center,
                vel: ae::Vec2::new(220.0, -180.0),
                size: ae::Vec2::new(12.0, 24.0),
                // the body has TURNED since its move committed.
                facing: 1.0,
            },
            ae::BodyGroundState::default(),
            ae::BodyWallState {
                on_wall: true,
                wall_normal_x: -1.0,
            },
            playback,
        ));

        app.update();
        let view = app.world().resource::<CombatGeometryView>();
        let row = view.bodies.first().expect("the body is observed at all");

        assert_eq!(
            row.damage_taken, 47,
            "percent is what knockback growth reads"
        );
        assert!((row.hitstun_s - 0.21).abs() < 1e-6);
        assert!((row.hitlag_s - 0.07).abs() < 1e-6);
        assert!(
            (row.landing_lag_s - 0.13).abs() < 1e-6,
            "landing lag is its OWN readout — it looks like hitstun on screen \
             and is a different fact"
        );
        assert_eq!(row.velocity, ae::Vec2::new(220.0, -180.0));
        assert!(row.on_wall && row.wall_normal_x < 0.0);

        let move_state = row
            .move_state
            .as_ref()
            .expect("a body mid-move publishes its move");
        assert_eq!(move_state.id, "test_swat");
        assert_eq!(
            move_state.phase,
            Some(ambition_entity_catalog::WindowTag::Startup),
            "the phase is the authored window the clock is inside"
        );
        assert!(move_state.duration_s > 0.0);
        // the disagreement is the point. The body faces +1 and the move
        // committed to -1; an instrument that showed only one of them could not
        // explain why the strike is on the far side.
        assert_eq!(move_state.attack_facing, -1.0);
        assert_eq!(row.facing, 1.0);
    }
}
