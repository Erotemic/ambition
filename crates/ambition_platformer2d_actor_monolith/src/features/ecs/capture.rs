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
