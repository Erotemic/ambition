//! Gravity-zone lifecycle and room-reset policy. Extracted from
//! `ambition_portal2d::lifecycle` (Stage 6 follow-up): this is a *gravity
//! mechanic*, not portal behavior, so it owns its state here and must not
//! depend on `ambition_portal2d`.

use bevy::prelude::*;


/// Reset the AMBIENT gravity to the default (down) when a new world begins, so a
/// flipped room doesn't carry over. Only the ambient is authored state; the
/// presentation `GravityField` is a per-tick mirror of the primary body's
/// resolved frame with exactly one writer (`resolve_active_gravity`), so it
/// follows on the next resolution.
///
/// ⭐ ONE FACT COVERS A DEATH, A RETRY AND A NEW GAME. A 2026-09-13 review found
/// that *"flip gravity, Reset New Game"* started the fresh run upside down,
/// because a New Game had its own commit message and nothing here read it.
/// Since 2026-09-29 a New Game is a checkpoint restore, and its admission
/// writes `RoomReplayAdmitted` like a death does, so this one reader serves all
/// three. ⚠ The SESSION edge is not here: `BaseGravity` is a member of
/// `SessionScopedResources`, so activation and retirement clear it with the rest
/// of the live-session mirrors and under that aggregate's compiler guard.
pub fn reset_gravity_on_room_reset(
    mut resets: MessageReader<ambition_combat::events::RoomReplayAdmitted>,
    mut base: ResMut<ambition_platformer2d_shared_tangle::gravity::BaseGravity>,
) {
    if resets.read().next().is_none() {
        return;
    }
    *base = ambition_platformer2d_shared_tangle::gravity::BaseGravity::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::gravity::BaseGravity;

    /// A replay admission puts gravity back down. The first update, with no
    /// message, is the control: the flip must survive it.
    #[test]
    fn a_replay_admission_puts_gravity_back_down() {
        let mut app = App::new();
        app.add_message::<ambition_combat::events::RoomReplayAdmitted>();
        app.init_resource::<BaseGravity>();
        app.add_systems(Update, reset_gravity_on_room_reset);

        let flipped = ambition_platformer2d_core::Vec2::new(0.0, -1.0);
        app.world_mut().insert_resource(BaseGravity { dir: flipped });
        app.update();
        assert_eq!(
            app.world().resource::<BaseGravity>().dir,
            flipped,
            "gravity reset with no lifecycle message at all"
        );

        app.world_mut()
            .write_message(ambition_combat::events::RoomReplayAdmitted::manual());
        app.update();
        assert_eq!(
            app.world().resource::<BaseGravity>().dir,
            BaseGravity::default().dir,
            "a replay left the ambient gravity flipped, so the new world starts \
             upside down"
        );
    }
}
