//! **Acquisition: turning a live grab volume into a capture relationship.**
//!
//! The body half of capture. It lives here rather than in the Smash game because
//! it needs body facts — liveness, kinematics, ground state, published
//! silhouette, the move a victim is currently playing — and rather than in a new
//! crate because a grab is not a reason to carve one.
//!
//! ⛔ **a shield does NOT block a grab**, and this is the road where that is
//! true rather than a comment. A capture never consults [`BodyShieldState`]:
//! the whole rock-paper-scissors triangle rests on a grab beating the thing
//! that beats an attack, and asking the shield here would quietly delete the
//! third leg.

use bevy::math::bounding::IntersectsVolume as _;
use bevy::prelude::*;

use ambition_combat::capture::{captive_of, CaptureAttemptRequested, CapturedBy};
use ambition_combat::hitbox::StrikeVictim;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt as _;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Everything a body needs to be ALLOWED to start a capture.
#[derive(bevy::ecs::query::QueryData)]
pub struct CaptorView {
    kin: &'static ae::BodyKinematics,
    faction: &'static crate::features::ActorFaction,
    team: Option<&'static ambition_combat::targeting::MatchTeam>,
    health: Option<&'static ambition_characters::actor::BodyHealth>,
    ground: Option<&'static ambition_platformer2d_core::BodyGroundState>,
    frame: Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
}

/// **Acquire a captive for every live grab attempt that reaches one.**
///
/// # Eligibility, stated rather than discovered
///
/// A captor must be alive, grounded, holding nobody, and driving itself. A
/// victim must be a different body, alive, grounded, held by nobody, driving
/// itself, and someone this captor is allowed to damage.
///
/// ⚠ **`damage_lands_between`, the SAME relational rule a strike asks.** A grab
/// that could take a teammate would be a different game; routing it through the
/// hostility question every other offensive road asks is what keeps friendly
/// fire, teams and factions answering once.
///
/// ⛔ **v1 refuses an airborne victim, and refuses an airborne captor.** Aerial
/// grabs, tether grabs and command grabs are named future techniques, and a
/// standing grab that happened to answer an airborne press would be a bad one of
/// them by accident rather than a designed one on purpose.
///
/// # Why the arbitration is spelled out
///
/// A grab volume can overlap two legal bodies. Taking whichever the query
/// yields first makes the capture depend on archetype/table order — stable
/// within a run and NOT stable across a rollback resimulation, which is the
/// definition of a desync. So candidates are ranked: nearest to the volume's
/// centre first, and an exact tie broken by the victims' stable [`SimId`].
pub fn acquire_captures(
    mut commands: Commands,
    mut attempts: MessageReader<CaptureAttemptRequested>,
    captors: Query<CaptorView, Without<ambition_characters::brain::ScriptedControl>>,
    victims: Query<StrikeVictim, Without<ambition_characters::brain::ScriptedControl>>,
    captives: Query<(Entity, &CapturedBy)>,
    grounded: Query<&ambition_platformer2d_core::BodyGroundState>,
    identities: Query<&SimId>,
    surfaces: Query<&crate::features::ActorSurfaceState>,
    // **The captive's in-flight move, so capture can END it.** See the note at
    // the insertion below for why this is not optional.
    mut playbacks: Query<&mut ambition_combat::moveset::MovePlayback>,
    tuning: Option<Res<ambition_combat::rules::ResolvedCombatTuning>>,
) {
    let friendly_fire = tuning.map(|t| t.friendly_fire()).unwrap_or_default();
    for attempt in attempts.read() {
        let Ok(captor) = captors.get(attempt.captor) else {
            continue;
        };
        if ambition_combat::util::body_is_corpse(captor.health) {
            continue;
        }
        if !captor.ground.map(|g| g.on_ground).unwrap_or(false) {
            continue;
        }
        // ⚠ **a captor already holding somebody is not a captor twice.** This is
        // where the "one captor, one captive" half of the invariant is upheld;
        // the other half is upheld by skipping a victim that is already held.
        if captive_of(attempt.captor, &captives).is_some() {
            continue;
        }

        let body_frame = captor
            .frame
            .map(|f| f.basis())
            .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
        let placed = ambition_combat::moveset::place_body_local_volume(
            &ambition_entity_catalog::VolumeShape::Rect {
                offset: (attempt.offset.x, attempt.offset.y),
                half_extents: (attempt.half_extents.x, attempt.half_extents.y),
            },
            captor.kin.facing,
            &body_frame,
        );
        let reach_centre = captor.kin.pos + placed.world_offset;
        let reach = ae::CenteredAabb::new(reach_centre, placed.half_extent).aabb();

        // ⚠ built ONCE, not asked per candidate: `captives` is the authority on
        // who is already held, and the other half of "one captive, one captor".
        let already_held: std::collections::HashSet<Entity> =
            captives.iter().map(|(entity, _)| entity).collect();

        // Gather, then rank. ⛔ never "take the first overlap": see the doc.
        let mut candidates: Vec<(f32, &SimId, Entity)> = Vec::new();
        for victim in &victims {
            if victim.entity == attempt.captor {
                continue;
            }
            if victim.is_corpse() || victim.is_intangible() {
                continue;
            }
            if !grounded
                .get(victim.entity)
                .map(|g| g.on_ground)
                .unwrap_or(false)
            {
                continue;
            }
            if already_held.contains(&victim.entity) {
                continue;
            }
            if !ambition_combat::targeting::damage_lands_between(
                *captor.faction,
                victim.effective_faction(),
                captor.team,
                victim.team,
                friendly_fire,
                None,
                victim.entity,
            ) {
                continue;
            }
            // The published silhouette when there is one, the coarse box when
            // there is not — the same fallback every body-overlap road uses.
            let body = victim.aabb.aabb();
            if !reach.intersects(&body) {
                continue;
            }
            // ⚠ a body with no `SimId` cannot participate in the deterministic
            // tie-break, so it cannot be captured. That is a fixture shape, not
            // a live one: every constructed body has an identity.
            let Ok(id) = identities.get(victim.entity) else {
                continue;
            };
            candidates.push((
                body.center().distance_squared(reach_centre),
                id,
                victim.entity,
            ));
        }
        candidates.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(b.1))
        });
        let Some((_, _, victim)) = candidates.first().copied() else {
            continue;
        };

        // ⛔⛔ **THE CAPTIVE'S MOVE ENDS HERE, AND ITS VOLUMES WITH IT.**
        //
        // Otherwise: Alice starts a forward smash, George grabs her, Alice hangs
        // in his hands — and her smash's Active window opens on schedule and hits
        // the man holding her. A captive that can still hit its captor is not a
        // capture, and the bug would read as "grabs are unsafe" rather than as a
        // missing teardown.
        //
        // ⭐ through the ONE teardown path (`cancel_move_playback`), which is why
        // that was extracted from its four hand-copies first rather than becoming
        // a fifth here.
        if let Ok(mut playback) = playbacks.get_mut(victim) {
            ambition_combat::moveset::cancel_move_playback(&mut commands, victim, &mut playback);
        }
        // **The captive's control projection.** `CapturedBy` stays the authority;
        // this is only what it means for input. Capture eligibility refuses a
        // body that already carries the marker, which is what lets release
        // remove it without stealing somebody else's hold on this body.
        //
        // ⚠ **it is a PROJECTION, and that is the shape escape needs later**: a
        // mash-to-escape read samples the captive's raw participant input into a
        // restricted capture channel, and would be impossible if capture meant
        // "this body's input ceases to exist".
        commands
            .entity(victim)
            .insert(ambition_characters::brain::ScriptedControl);
        commands.entity(victim).insert(CapturedBy {
            captor: attempt.captor,
            hold_offset_local: attempt.hold_offset,
            // ⚠ REMEMBERED, not assumed: a flying body's scale is not 1.0, and a
            // release that wrote a constant would land it on the floor.
            prior_gravity_scale: surfaces
                .get(victim)
                .map(|surface| surface.gravity_scale)
                .unwrap_or(1.0),
            pummels_landed: 0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::BodyShieldState;

    fn grounded_body(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos,
                    facing: 1.0,
                    size: ae::Vec2::new(16.0, 24.0),
                    ..Default::default()
                },
                crate::features::CenteredAabb::new(pos, ae::Vec2::new(8.0, 12.0)),
                crate::features::ActorFaction::Enemy,
                ambition_platformer2d_core::BodyGroundState {
                    on_ground: true,
                    contact_initialized: true,
                },
                SimId::placement(id),
            ))
            .id()
    }

    /// A floor-clinging surface state at `gravity_scale`. There is no `Default`
    /// on purpose — a surface normal has no sensible neutral, so a fixture states
    /// the floor explicitly.
    fn surface_state(gravity_scale: f32) -> crate::features::ActorSurfaceState {
        crate::features::ActorSurfaceState {
            surface_normal: ae::Vec2::new(0.0, -1.0),
            gravity_scale,
        }
    }

    fn capture_app() -> App {
        let mut app = App::new();
        app.add_message::<CaptureAttemptRequested>();
        app.add_systems(Update, acquire_captures);
        app
    }

    fn attempt(captor: Entity) -> CaptureAttemptRequested {
        CaptureAttemptRequested {
            captor,
            offset: ae::Vec2::new(16.0, 0.0),
            half_extents: ae::Vec2::new(12.0, 14.0),
            hold_offset: ae::Vec2::new(18.0, 0.0),
        }
    }

    /// **⭐ A SHIELD DOES NOT STOP A GRAB — the third leg of the triangle.**
    ///
    /// attack beats grab, grab beats shield, shield beats attack. If a capture
    /// ever consults `BodyShieldState`, that cycle collapses into "shield beats
    /// everything" and the whole neutral game goes with it. This is the test
    /// that would go red the day somebody adds the defensive check that looks
    /// obviously correct from inside the hit path.
    #[test]
    fn a_shielding_body_is_captured_anyway() {
        let mut app = capture_app();
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(0.0, 0.0));
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        app.world_mut()
            .entity_mut(victim)
            .insert(BodyShieldState::default());
        app.world_mut()
            .entity_mut(victim)
            .insert(crate::features::ActorFaction::Player);
        app.world_mut().write_message(attempt(captor));
        app.update();
        assert!(
            app.world().get::<CapturedBy>(victim).is_some(),
            "a raised shield stopped a grab — the rock-paper-scissors triangle \
             has no third leg if a guard beats a capture too"
        );
    }

    /// **AN AIRBORNE VICTIM IS NOT CAPTURED BY A STANDING GRAB (v1).**
    ///
    /// Aerial grabs are a named future technique. A standing grab that happened
    /// to catch a jumping body would be a bad aerial grab nobody designed.
    #[test]
    fn an_airborne_victim_is_out_of_reach_of_a_standing_grab() {
        let mut app = capture_app();
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(0.0, 0.0));
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        app.world_mut()
            .entity_mut(victim)
            .insert(crate::features::ActorFaction::Player);
        app.world_mut()
            .entity_mut(victim)
            .insert(ambition_platformer2d_core::BodyGroundState {
                on_ground: false,
                contact_initialized: true,
            });
        app.world_mut().write_message(attempt(captor));
        app.update();
        assert!(app.world().get::<CapturedBy>(victim).is_none());
    }

    /// **THE CAPTIVE IS HELD AT THE CAPTOR'S ANCHOR, MIRRORED BY ITS FACING.**
    ///
    /// The anchor is captor-body-local, so a captor that turns around swings its
    /// captive across rather than leaving it behind — which is what makes a
    /// forward throw point where the player is looking.
    #[test]
    fn a_captive_is_posed_at_its_captors_hold_anchor() {
        let mut app = App::new();
        app.add_systems(Update, constrain_captive_bodies);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(100.0, 50.0));
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(400.0, 400.0));
        app.world_mut()
            .entity_mut(victim)
            .insert(surface_state(1.0));
        app.world_mut().entity_mut(victim).insert(CapturedBy {
            captor,
            hold_offset_local: ae::Vec2::new(20.0, -4.0),
            prior_gravity_scale: 1.0,
            pummels_landed: 0,
        });
        app.update();
        let kin = app.world().get::<ae::BodyKinematics>(victim).unwrap();
        assert_eq!(
            kin.pos,
            ae::Vec2::new(120.0, 46.0),
            "facing +1: anchor adds"
        );
        let aabb = app
            .world()
            .get::<crate::features::CenteredAabb>(victim)
            .unwrap();
        assert_eq!(
            aabb.center, kin.pos,
            "the coarse-box mirror was left where the body was grabbed, so a \
             strike aimed at the floor could still reach a captive held overhead"
        );

        // Turn the captor around: the captive swings to the other side.
        app.world_mut()
            .get_mut::<ae::BodyKinematics>(captor)
            .unwrap()
            .facing = -1.0;
        app.update();
        assert_eq!(
            app.world().get::<ae::BodyKinematics>(victim).unwrap().pos,
            ae::Vec2::new(80.0, 46.0),
            "the hold anchor did not mirror with the captor's facing"
        );
    }

    /// **A CAPTOR KEEPS ITS ATTACK PRESS AND LOSES EVERYTHING ELSE.**
    ///
    /// Both halves matter and they fail in opposite directions: strip the attack
    /// and a captor can hold a body and do nothing to it forever; keep the rest
    /// and a captor walks off with somebody in its hands.
    #[test]
    fn a_captor_is_restricted_but_can_still_press_attack() {
        let mut app = App::new();
        app.add_systems(Update, restrict_captor_control);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        frame.locomotion = ae::LocalAxes::X;
        frame.jump_pressed = true;
        frame.special_pressed = true;
        frame.shield_held = true;
        frame.melee_pressed = true;
        frame.attack_axis = ae::LocalAxes::X;
        app.world_mut()
            .entity_mut(captor)
            .insert(ambition_characters::brain::ActorControl(frame));
        app.world_mut().entity_mut(victim).insert(CapturedBy {
            captor,
            hold_offset_local: ae::Vec2::new(16.0, 0.0),
            prior_gravity_scale: 1.0,
            pummels_landed: 0,
        });
        app.update();
        let held = &app
            .world()
            .get::<ambition_characters::brain::ActorControl>(captor)
            .unwrap()
            .0;
        assert_eq!(held.locomotion, ae::LocalAxes::ZERO, "a captor walked away");
        assert!(!held.jump_pressed && !held.special_pressed && !held.shield_held);
        assert!(
            held.melee_pressed && held.attack_axis == ae::LocalAxes::X,
            "the attack press was stripped, so this captor can hold a body and \
             never pummel or throw it"
        );
    }

    /// **A HIT ON EITHER BODY ENDS THE HOLD, AND GIVES BACK WHAT IT SUSPENDED.**
    ///
    /// ⚠ the restored gravity is the body's OWN prior value, not `1.0`. This
    /// captive was floating at `0.25` before it was grabbed, which is the case a
    /// constant would silently break.
    #[test]
    fn a_hit_captor_drops_its_captive_and_gravity_comes_back() {
        let mut app = App::new();
        app.add_systems(Update, release_interrupted_captures);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        for body in [captor, victim] {
            app.world_mut()
                .entity_mut(body)
                .insert(ambition_characters::actor::BodyCombat::default());
        }
        app.world_mut()
            .entity_mut(victim)
            .insert(surface_state(0.0));
        app.world_mut().entity_mut(victim).insert((
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(16.0, 0.0),
                prior_gravity_scale: 0.25,
                pummels_landed: 1,
            },
            ambition_characters::brain::ScriptedControl,
        ));

        app.update();
        assert!(
            app.world().get::<CapturedBy>(victim).is_some(),
            "the fixture is wrong: nobody has been hit, so the hold must survive"
        );

        app.world_mut()
            .get_mut::<ambition_characters::actor::BodyCombat>(captor)
            .unwrap()
            .hitstun_timer = 0.2;
        app.update();

        assert!(
            app.world().get::<CapturedBy>(victim).is_none(),
            "still held"
        );
        assert!(
            app.world()
                .get::<ambition_characters::brain::ScriptedControl>(victim)
                .is_none(),
            "the captive is free and still cannot move — the control projection \
             outlived the relationship it projects"
        );
        assert_eq!(
            app.world()
                .get::<crate::features::ActorSurfaceState>(victim)
                .unwrap()
                .gravity_scale,
            0.25,
            "gravity came back as a CONSTANT rather than as what this body had"
        );
    }

    /// **A CAPTOR ALREADY HOLDING SOMEBODY TAKES NOBODY ELSE.**
    ///
    /// Half of the "one captor, one captive" invariant. Without it a grab whose
    /// window is still live would keep acquiring, and `captive_of` — which
    /// assumes at most one — would start returning whichever the scan reached
    /// first.
    #[test]
    fn a_captor_that_already_holds_somebody_acquires_nobody() {
        let mut app = capture_app();
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(0.0, 0.0));
        let first = grounded_body(&mut app, "first", ae::Vec2::new(16.0, 0.0));
        let second = grounded_body(&mut app, "second", ae::Vec2::new(17.0, 0.0));
        for body in [first, second] {
            app.world_mut()
                .entity_mut(body)
                .insert(crate::features::ActorFaction::Player);
        }
        app.world_mut().entity_mut(first).insert(CapturedBy {
            captor,
            hold_offset_local: ae::Vec2::new(18.0, 0.0),
            prior_gravity_scale: 1.0,
            pummels_landed: 0,
        });
        app.world_mut().write_message(attempt(captor));
        app.update();
        assert!(
            app.world().get::<CapturedBy>(second).is_none(),
            "a captor holding one body grabbed a second one"
        );
    }
}

/// **Hold the captive where its captor holds it.**
///
/// The same physical mechanism the saddle pin uses —
/// [`constrain_body_pose`](ae::movement::constrain_body_pose) — and deliberately
/// NOT the mount relationship. A rider steers its mount and a captive does not
/// steer its captor; unifying them would make one of those two false.
///
/// ⚠ **velocity follows the CAPTOR's, not `ZERO`.** A body released or
/// interrupted mid-carry then inherits the motion of the thing that was carrying
/// it, which is what a person expects to see. Zeroing it would make every
/// release look like the captive hit an invisible wall.
///
/// Runs after the bodies integrate, for the reason the mount's own note gives:
/// integration has just moved the captive under its own velocity, and this puts
/// it back. It also runs once more immediately after acquisition, so a grab that
/// lands this tick attaches this tick rather than a frame later.
pub fn constrain_captive_bodies(
    // ⭐ **`Without<CapturedBy>` is a SEMANTIC claim, not a borrow trick** — though
    // Bevy asking for it is what made the claim explicit. A captive can never be
    // a captor: acquisition refuses a captor already under `ScriptedControl`, and
    // every captive carries it, so a chain A-holds-B-holds-C cannot form. The two
    // queries are disjoint because the relationship says they are, and if that
    // ever stops being true this line is where it should be re-argued rather than
    // relaxed into a `ParamSet`.
    captors: Query<
        (
            &ae::BodyKinematics,
            Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        ),
        Without<CapturedBy>,
    >,
    mut captives: Query<(
        &CapturedBy,
        &mut ae::BodyKinematics,
        &mut crate::features::ActorSurfaceState,
        &mut ambition_platformer2d_core::BodyGroundState,
        Option<&mut crate::features::CenteredAabb>,
    )>,
) {
    for (held, mut kin, mut surface, mut ground, aabb) in &mut captives {
        let Ok((captor_kin, captor_frame)) = captors.get(held.captor) else {
            // The captor is gone. Releasing is the RELEASE path's job, not this
            // one's — a constraint system that also dissolved relationships
            // would be a second authority on when a capture ends.
            continue;
        };
        let frame = captor_frame
            .map(|f| f.basis())
            .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
        // The hold anchor is captor-body-local: mirror by the captor's live
        // facing, then rotate into its frame. So a captor that turns around
        // swings its captive across, and a captor under flipped gravity holds it
        // overhead — both for free, because the anchor was never a world offset.
        let local = ae::Vec2::new(
            held.hold_offset_local.x * captor_kin.facing,
            held.hold_offset_local.y,
        );
        let pos = captor_kin.pos + frame.to_world(local);
        ae::movement::constrain_body_pose(&mut kin, pos, captor_kin.vel);
        // Gravity is SUSPENDED, not deleted — `CapturedBy::prior_gravity_scale`
        // holds what to give back.
        surface.gravity_scale = 0.0;
        ground.on_ground = false;
        // ⚠ the coarse-box mirror moves in the SAME tick, exactly as the mount's
        // does: combat and presentation both read it, and a captive whose damage
        // box stayed where it was grabbed could be hit in mid-air by a strike
        // aimed at the floor.
        if let Some(mut aabb) = aabb {
            aabb.center = pos;
        }
    }
}

/// **A captor is restricted, not blanked.**
///
/// While holding somebody a body may not walk, jump, dodge, shield, blink,
/// special, or shoot — but it MUST still be able to feed an attack press and its
/// direction, because that is how a pummel and a throw are chosen.
///
/// ⛔ **deliberately not `ScriptedControl`.** That marker means *"normal input
/// does not drive this body"*, and it is what the CAPTIVE gets. A captor still
/// has meaningful agency; it is in a restricted action context, which is a
/// different thing and stays a different thing.
///
/// ⚠ a future carry relationship may permit locomotion — cargo throws exist in
/// the genre. That is a property of the RELATIONSHIP, so it belongs on whatever
/// component expresses it, not baked in here.
pub fn restrict_captor_control(
    captives: Query<&CapturedBy>,
    mut captors: Query<(Entity, &mut ambition_characters::brain::ActorControl)>,
) {
    let holding: std::collections::HashSet<Entity> =
        captives.iter().map(|held| held.captor).collect();
    if holding.is_empty() {
        return;
    }
    for (entity, mut control) in &mut captors {
        if !holding.contains(&entity) {
            continue;
        }
        let frame = &mut control.0;
        frame.locomotion = ae::LocalAxes::ZERO;
        frame.velocity_target = ae::WorldVec2(ae::Vec2::ZERO);
        frame.jump_pressed = false;
        frame.jump_held = false;
        frame.jump_released = false;
        frame.dash_pressed = false;
        frame.blink_pressed = false;
        frame.blink_held = false;
        frame.blink_released = false;
        frame.special_pressed = false;
        frame.shield_held = false;
        frame.fire = None;
        frame.projectile_pressed = false;
        frame.projectile_held = false;
        frame.projectile_released = false;
        frame.fly_toggle_pressed = false;
        frame.fast_fall_pressed = false;
        frame.interact_pressed = false;
        // ⛔ a second grab while already holding somebody is not a thing.
        frame.grab_pressed = false;
        // ⭐ PRESERVED: `melee_pressed`, `melee_held`, `melee_released` and
        // `attack_axis`. Those four ARE the capture-context vocabulary — strip
        // them and a captor can hold a body and do nothing to it forever.
    }
}

/// **THE one release.** Every path out of a capture goes through it.
///
/// ⛔ **no caller clears half the relation.** A capture suspends three things —
/// the relationship, the control projection, and gravity — and a release that
/// forgot any one leaves a body that is free but frozen, or falling but still
/// listed as held. Making that impossible is the entire reason this is a
/// function rather than three lines repeated at each exit.
///
/// ⭐ **this is also what makes escape cheap.** Mash-to-escape, when it lands,
/// is another caller: it decides WHEN, and this decides what release means.
pub fn release_capture(
    commands: &mut Commands,
    victim: Entity,
    held: &CapturedBy,
    surface: Option<&mut crate::features::ActorSurfaceState>,
) {
    commands
        .entity(victim)
        .remove::<CapturedBy>()
        .remove::<ambition_characters::brain::ScriptedControl>();
    if let Some(surface) = surface {
        // ⚠ what it WAS, not `1.0`. A flying body's scale is not the reference
        // one, and a release that wrote a constant would land it on the floor.
        surface.gravity_scale = held.prior_gravity_scale;
    }
}

/// **A capture ends when either body takes a real hit, or the captor stops
/// existing.**
///
/// ⭐ **derived from the ACCEPTED reaction, not from raw overlap.** `BodyCombat`
/// already records that a body entered hitstun or a recoil lock — facts written
/// only when a hit actually landed and was not shrugged off — so a third party
/// interrupting a grab needs no parallel "grab interrupted" pipeline. The
/// interruption is the hit reaction the engine already agreed happened.
///
/// ⛔ **hitstop alone does NOT break it.** A pummel may deliberately produce a
/// little hitstop while preserving the hold; breaking on that would make the
/// mechanic destroy itself on its own second beat.
pub fn release_interrupted_captures(
    mut commands: Commands,
    captives: Query<(Entity, &CapturedBy)>,
    combat: Query<&ambition_characters::actor::BodyCombat>,
    mut surfaces: Query<&mut crate::features::ActorSurfaceState>,
) {
    let reacted = |body: Entity| {
        combat
            .get(body)
            .map(|c| c.hitstun_timer > 0.0 || c.recoil_lock_timer > 0.0)
            .unwrap_or(false)
    };
    for (victim, held) in &captives {
        // A captor that no longer exists cannot hold anything. Checked through
        // the combat query rather than a marker: an entity that is gone answers
        // no query, which is the fact wanted here.
        let captor_gone = combat.get(held.captor).is_err();
        if !(captor_gone || reacted(victim) || reacted(held.captor)) {
            continue;
        }
        let mut surface = surfaces.get_mut(victim).ok();
        release_capture(&mut commands, victim, held, surface.as_deref_mut());
    }
}
