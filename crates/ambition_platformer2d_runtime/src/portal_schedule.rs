//! Portal simulation assembly and schedule placement.
//!
//! Required ordering:
//! - Carves publish after gravity-zone collection and before core simulation.
//! - Input warp runs after interaction input and before the primary frame is committed to
//!   `SlotControls`.
//! - Frame (link resolution, straddler eviction) is a pose-dependent carry: it
//!   runs after the other carries and before hazards read the travelled path.
//! - Transit runs after body and ground-item integration so current-frame positions cross,
//!   and after the path's contacts, because a crossing may collapse the path.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::gameplay_allowed;
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_portal2d::PortalSet;

/// Turn around the body a seat drives when its transit states a facing flip.
///
/// The `portal_reverses_facing` setting reverses the CONTROLLED body's facing,
/// and the controlled body is the one carrying [`DrivingParticipant`]:
/// possession moves that component to the possessed body, while the home body
/// keeps its player-population markers. So this asks the driver, not the
/// population. It runs in `PortalSet::Transited`, after the core's transit.
///
/// [`DrivingParticipant`]: ambition_characters::control::DrivingParticipant
pub fn turn_the_driven_body_around_after_portal_transit(
    mut transited: MessageReader<ambition_portal2d::PortalBodyTransited>,
    tuning: Res<ambition_portal2d::PortalTuning>,
    mut driven: Query<
        &mut ambition_platformer2d_core::BodyKinematics,
        With<ambition_characters::control::DrivingParticipant>,
    >,
) {
    for event in transited.read() {
        if !(event.facing_flip && tuning.reorient_facing) {
            continue;
        }
        if let Ok(mut kin) = driven.get_mut(event.body) {
            kin.facing = -kin.facing;
        }
    }
}

/// Adds `PortalPlugin` and places its sets in the sandbox schedule. Part of
/// [`crate::PlatformerEnginePlugins`] when the `portal` feature is on.
pub struct PortalSchedulePlugin;

impl Plugin for PortalSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_plugins(ambition_portal2d::PortalPlugin);
        app.add_systems(
            sim,
            turn_the_driven_body_around_after_portal_transit.in_set(PortalSet::Transited),
        );

        // Carves publish after gravity-zone collection and before core
        // simulation.
        app.configure_sets(
            sim,
            PortalSet::Carves
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                .after(ambition_platformer2d_shared_tangle::gravity::GravityZonesCollected)
                .before(Platformer2dSimulationPhaseMonolith::CoreSimulation),
        );

        // InputWarp: input rewrite in the player-input phase, after
        // interaction input and before the finalized primary frame is published
        // to SlotControls (the Move-axis-fix window), gated to gameplay.
        app.configure_sets(
            sim,
            PortalSet::InputWarp
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
                .after(ambition_platformer2d_actor_monolith::control::InteractionInputBuffered)
                .before(ambition_platformer2d_actor_monolith::control::PrimarySlotInputCommit)
                .run_if(gameplay_allowed),
        );

        // Weapon maintenance stays ungated for orphan cleanup / roll
        // readiness.
        app.configure_sets(
            sim,
            PortalSet::WeaponAndProjectiles
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)
                .run_if(gameplay_allowed),
        );
        app.configure_sets(
            sim,
            PortalSet::WeaponMaintenance
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation),
        );

        // RoomReset: reset-time portal cleanup in the room-transition phase,
        // after the content layer's room-reset work (e.g. a boss-arena reset).
        app.configure_sets(
            sim,
            PortalSet::RoomReset
                .in_set(Platformer2dSimulationPhaseMonolith::RoomTransition)
                .after(ambition_platformer2d_actor_monolith::session::reset::ContentRoomResetSet),
        );

        // TransitGuards: suppress ledge-grab while transiting, BEFORE the
        // unified body integration reads it. Movement lives in `WorldPrep`
        // (`integrate_sim_bodies`), so the guard runs there too, ahead of it.
        // Gated to gameplay.
        app.configure_sets(
            sim,
            PortalSet::TransitGuards
                // The PHASE, not the movement system's name.
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
                )
                .run_if(gameplay_allowed),
        );

        // Frame: eviction shoves a straddler off a moved or closed plane, which
        // is travel — so it settles before hazard contacts read the path. It
        // TESTS the pose (is the box straddling?), so it follows every carry
        // rather than racing them. Same host edges as Transit, which follows it.
        app.configure_sets(
            sim,
            PortalSet::Frame
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)
                .after(ambition_platformer2d_shared_tangle::schedule::ItemPickupSet::CoreHeldItems)
                .in_set(ambition_platformer2d_shared_tangle::schedule::BodyPathSet::Constrain)
                .run_if(gameplay_allowed),
        );

        // Transit: teleports run after body + ground-item integration so this
        // frame's integrated body positions are what cross the portal. Body
        // integration completes in `WorldPrep`; `PlayerSimulation` runs after
        // it, so membership + the CoreHeldItems edge are enough. Gated to
        // gameplay.
        app.configure_sets(
            sim,
            PortalSet::Transit
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)
                .after(ambition_platformer2d_shared_tangle::schedule::ItemPickupSet::CoreHeldItems)
                // Portal CCD reads the settled path, after every carry.
                .in_set(ambition_platformer2d_shared_tangle::schedule::BodyPathSet::Crossing)
                .run_if(gameplay_allowed),
        );
    }
}

/// Keep portal construction capability and its schema installed by the same engine
/// composition. The room planner's portal-gun lane is compile-time feature gated, while
/// prepared-content fingerprinting reads runtime schema registration; `PortalSchedulePlugin`
/// must therefore install the full `PortalPlugin`, not simulation alone.
#[cfg(test)]
mod tests {
    use super::PortalSchedulePlugin;
    use ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog;

    /// One decision, two consequences. If this app can plan the portal-gun
    /// construction lane — and it can, because this module only compiles under
    /// the same feature — then the schema catalog must already know the domain.
    #[test]
    fn installing_portal_simulation_also_installs_the_gun_capability_it_can_construct() {
        let mut app = bevy::prelude::App::new();
        app.add_plugins(PortalSchedulePlugin);

        let catalog = app
            .world()
            .get_resource::<ConstructionSchemaCatalog>()
            .expect(
                "the portal composition contributed no construction schema at all, so the \
                 assertion below could not have failed",
            );
        assert!(
            catalog.contains_domain(ambition_portal2d::PORTAL_GUN_CONSTRUCTION_DOMAIN),
            "this composition compiles the portal-gun construction lane into room planning \
             (the lane is `#[cfg(feature = \"portal\")]`, and so is this test) but installs \
             no portal-gun capability, so prepared-content fingerprinting reports a world \
             without guns while rooms still build authored gun pickups. If installing \
             `PortalSimulationPlugin` alone is now a composition the engine wants, the lane \
             needs a runtime capability token threaded into \
             `ActorConstructionContext::for_live_room_construction` — see this module's header."
        );
    }

    /// Every body transits the same way, and only the body a seat DRIVES turns
    /// around through a same-wall pair: its facing follows its seat's input, and
    /// a brain decides the facing of any other body.
    ///
    /// Possession is the case that tells the two apart. The home body keeps its
    /// player-population markers but not the seat; the possessed actor has the
    /// seat and none of the markers. Both carry their momentum out of the exit.
    /// The bodies step through `PortalSchedulePlugin`, so the turn is measured
    /// where each game gets it, on the tick of the transit.
    #[test]
    fn only_the_driven_body_turns_around_through_a_same_wall_pair() {
        use ambition_characters::control::{DrivingParticipant, PlayerSlot};
        use ambition_platformer2d_core::BodyKinematics;
        use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
        use ambition_platformer2d_shared_tangle::schedule::{GameMode, SimScheduleExt};
        use ambition_platformer2d_shared_tangle::time::SimDt;
        use ambition_portal2d::{
            portal_half_extent, PlacedPortal, PortalChannel, PortalConvention, PortalGunColor,
            PortalTuning,
        };
        use bevy::prelude::*;

        #[derive(Clone, Copy, Debug)]
        enum Body {
            HomeWithoutTheSeat,
            PossessedWithTheSeat,
        }
        let run = |who: Body| {
            let mut app = App::new();
            app.set_sim_schedule(Update);
            app.init_resource::<SimDt>();
            app.insert_resource(State::new(GameMode::Playing));
            // The reflection convention supplies the mirror of a same-wall
            // turn-around as a facing flip rather than a roll.
            app.insert_resource(PortalTuning {
                convention: PortalConvention::Reflection,
                reorient_facing: true,
                ..Default::default()
            });
            app.add_plugins(super::PortalSchedulePlugin);
            for (color, y) in [(PortalGunColor::BLUE, 200.0), (PortalGunColor::ORANGE, 600.0)] {
                let channel = PortalChannel::Gun(color);
                app.world_mut().spawn(PlacedPortal::fixed(
                    channel,
                    Vec2::new(20.0, y),
                    Vec2::new(1.0, 0.0),
                    portal_half_extent(Vec2::new(1.0, 0.0)),
                ));
            }
            let mut body = app.world_mut().spawn(BodyKinematics {
                pos: Vec2::new(20.0, 200.0),
                vel: Vec2::new(-100.0, 0.0),
                size: Vec2::new(24.0, 40.0),
                facing: -1.0,
            });
            match who {
                Body::HomeWithoutTheSeat => body.insert((PlayerEntity, PrimaryPlayer)),
                Body::PossessedWithTheSeat => body.insert(DrivingParticipant(PlayerSlot::PRIMARY)),
            };
            let body = body.id();
            // Read the body on the tick it arrives: a turn that ran before the
            // transit would show one tick late.
            for _ in 0..4 {
                app.update();
                let kin = *app.world().get::<BodyKinematics>(body).unwrap();
                if kin.pos.y > 400.0 {
                    return kin;
                }
            }
            *app.world().get::<BodyKinematics>(body).unwrap()
        };

        for (who, facing) in [(Body::PossessedWithTheSeat, 1.0), (Body::HomeWithoutTheSeat, -1.0)] {
            let kin = run(who);
            assert!(kin.pos.y > 400.0, "{who:?} must transit to the orange portal, pos={:?}", kin.pos);
            assert!(kin.vel.x > 0.0, "{who:?} must carry its momentum out of the exit, vel={:?}", kin.vel);
            assert_eq!(kin.facing, facing, "{who:?}: only the body the seat drives turns around");
        }
    }
}
