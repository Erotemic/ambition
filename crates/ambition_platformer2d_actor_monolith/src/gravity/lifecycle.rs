//! Gravity-zone lifecycle / room-reset policy and the ambient gravity-flip
//! switch. Extracted from `ambition_portal2d::lifecycle` (Stage 6 follow-up): these
//! are a *gravity mechanic*, not portal behavior, so they own their state here
//! and must not depend on `ambition_portal2d`.

use bevy::prelude::*;

use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::{self as ae, AabbExt};

/// Reset the AMBIENT gravity to the default (down) when a new world begins, so a
/// flipped room doesn't carry over. Only the ambient is authored state; the
/// presentation `GravityField` is a per-tick mirror of the primary body's
/// resolved frame with exactly one writer (`resolve_active_gravity`), so it
/// follows on the next resolution.
///
/// ⛔⛤ **IT READ ONE LIFECYCLE FACT AND THE GAME HAS THREE — MEASURED BY A
/// 2026-09-13 REVIEW.** `BaseGravity` is canonical ROLLBACK state and a mechanic
/// (gravity-flip switches write it), and its only reset roads were
/// `RoomReplayAdmitted` and a room TRANSITION's commit. **Reset New Game has its
/// own seam** — `NewGameResetCommitted` — and nothing here listened, so *"flip
/// gravity, Reset New Game"* started the fresh run upside down.
///
/// ⇒ Both facts mean the same thing to this domain: the world you were falling
/// through is gone. ⚠ The SESSION edge is not here: `BaseGravity` is a member of
/// `SessionScopedResources`, so activation and retirement clear it with the rest
/// of the live-session mirrors and under that aggregate's compiler guard.
pub fn reset_gravity_on_room_reset(
    mut resets: MessageReader<ambition_combat::events::RoomReplayAdmitted>,
    mut new_game: MessageReader<crate::session::reset::NewGameResetCommitted>,
    mut base: ResMut<ambition_platformer2d_shared_tangle::gravity::BaseGravity>,
) {
    let replayed = resets.read().next().is_some();
    let restarted = new_game.read().next().is_some();
    if !replayed && !restarted {
        return;
    }
    *base = ambition_platformer2d_shared_tangle::gravity::BaseGravity::default();
}

/// A sandbox gravity-flip switch: a tall pressure-plate column the player steps
/// into to flip [`GravityField`] up↔down. Tall so it's reachable from both the
/// floor and the ceiling (after a flip you're on the ceiling — walk back into
/// the column to flip again). `armed` latches so one entry = one flip.
#[derive(Component, Clone, Copy, Debug)]
pub struct GravityFlipSwitch {
    pub pos: Vec2,
    pub half_extent: Vec2,
    /// True when the player is clear of the plate, so the next entry flips.
    pub armed: bool,
}

// The hub gravity flip is now an LDtk-authored `Switch` whose `action` is
// "FlipGravity" (handled in `encounter::systems::drive_wave_encounters`),
// so the old debug-spawned overlap column is gone. The `GravityFlipSwitch`
// component + `gravity_flip_switch_system` below remain only for the unit test
// + any future overlap-style gravity plate; nothing spawns one in-game.

/// Flip the room's ambient gravity ([`ambition_platformer2d_shared_tangle::gravity::BaseGravity`]) up↔down
/// when the player steps into a [`GravityFlipSwitch`] (rising-edge latched by
/// `armed`). Flipping the ambient (not the live `GravityField` directly) lets
/// gravity zones override locally while the switch sets the room default.
pub fn gravity_flip_switch_system(
    mut base: ResMut<ambition_platformer2d_shared_tangle::gravity::BaseGravity>,
    // SLOT-0 BY DESIGN: a gravity-flip switch rewrites the ROOM's ambient gravity
    // for everyone, so exactly one body may arm it — the local human's own. (If a
    // possessed actor should be able to arm it too, this becomes `ControlledSubject`;
    // that is a design call about the Noether Chamber, not a refactor.)
    players: Query<
        &BodyKinematics,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    mut switches: Query<&mut GravityFlipSwitch>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    let Ok(kin) = players.single() else {
        return;
    };
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    for mut sw in &mut switches {
        let overlapping = player_aabb.strict_intersects(ae::Aabb::new(sw.pos, sw.half_extent));
        if overlapping && sw.armed {
            // INVERT the ambient gravity — a flip means "down becomes the
            // opposite of wherever it points", so the switch keeps working
            // after a sideways SetGravity (§B13: `-dir.y` alone is a no-op on
            // horizontal gravity).
            base.dir = -base.dir;
            sw.armed = false;
            sfx.write(ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::PORTAL_POWERUP,
                pos: kin.pos,
            });
            bevy::log::info!(target: "ambition_platformer2d::gravity", "ambient gravity flipped: dir = {:?}", base.dir);
        } else if !overlapping {
            sw.armed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::BodyBaseSize;
    use ambition_platformer2d_shared_tangle::gravity::{BaseGravity, GravityField};
    use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

    fn spawn_player(app: &mut App, pos: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                BodyKinematics {
                    pos,
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                BodyBaseSize {
                    base_size: Vec2::new(24.0, 40.0),
                },
            ))
            .id()
    }

    #[test]
    fn gravity_switch_flips_on_entry_and_rearms_on_exit() {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.init_resource::<GravityField>();
        app.init_resource::<BaseGravity>();
        app.add_systems(Update, gravity_flip_switch_system);
        let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
        app.world_mut().spawn(GravityFlipSwitch {
            pos: Vec2::new(400.0, 100.0),
            half_extent: Vec2::new(16.0, 220.0),
            armed: true,
        });

        // Not overlapping → gravity stays down.
        app.update();
        assert!(
            app.world().resource::<BaseGravity>().dir.y > 0.0,
            "starts down"
        );

        // Step onto the switch → flips up.
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = Vec2::new(400.0, 100.0);
        app.update();
        assert!(
            app.world().resource::<BaseGravity>().dir.y < 0.0,
            "stepping on the switch flips ambient gravity up"
        );
        // Staying on it does not re-flip (latched).
        app.update();
        assert!(
            app.world().resource::<BaseGravity>().dir.y < 0.0,
            "stays flipped while on it"
        );

        // Leave, then re-enter → flips back down.
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = Vec2::new(100.0, 100.0);
        app.update();
        app.world_mut()
            .get_mut::<BodyKinematics>(player)
            .unwrap()
            .pos = Vec2::new(400.0, 100.0);
        app.update();
        assert!(
            app.world().resource::<BaseGravity>().dir.y > 0.0,
            "re-entering flips ambient gravity back down"
        );
    }

    /// ⛔⛤ **RESET NEW GAME HAS ITS OWN SEAM, AND AMBIENT GRAVITY WAS NOT
    /// LISTENING TO IT — MEASURED BY A 2026-09-13 REVIEW.**
    ///
    /// This reducer answered `RoomReplayAdmitted` alone. `Reset New Game` commits
    /// through `NewGameResetCommitted` and rebuilds the room WITHOUT a replay, so
    /// *"flip gravity, Reset New Game"* left the fresh run falling upward until
    /// some later room transition happened to correct it.
    ///
    /// ⭐ **BOTH ARMS, AND THE FIRST IS THE CONTROL.** A reducer wired only to the
    /// new seam would pass the second assertion and silently drop the replay road
    /// it already had.
    #[test]
    fn a_new_game_and_a_replay_both_put_gravity_back_down() {
        for (what, write) in [
            ("a room replay", 0usize),
            ("Reset New Game", 1usize),
        ] {
            let mut app = App::new();
            app.add_message::<ambition_combat::events::RoomReplayAdmitted>();
            app.add_message::<crate::session::reset::NewGameResetCommitted>();
            app.init_resource::<BaseGravity>();
            app.add_systems(Update, reset_gravity_on_room_reset);

            let flipped = ambition_platformer2d_core::Vec2::new(0.0, -1.0);
            app.world_mut().insert_resource(BaseGravity { dir: flipped });
            app.update();
            // ⚠ THE PREMISE: with no message, the flip must SURVIVE — otherwise
            // a reducer that resets unconditionally would pass both arms.
            assert_eq!(
                app.world().resource::<BaseGravity>().dir,
                flipped,
                "gravity reset with no lifecycle message at all, so neither arm \
                 below measures a reducer reacting to anything"
            );

            if write == 0 {
                app.world_mut()
                    .write_message(ambition_combat::events::RoomReplayAdmitted::manual());
            } else {
                app.world_mut()
                    .write_message(crate::session::reset::NewGameResetCommitted);
            }
            app.update();
            assert_eq!(
                app.world().resource::<BaseGravity>().dir,
                BaseGravity::default().dir,
                "{what} left the ambient gravity flipped, so the new world starts \
                 upside down"
            );
        }
    }
}
