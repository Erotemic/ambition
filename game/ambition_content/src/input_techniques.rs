//! Ambition-owned motion-input technique registrations.
//!
//! The named fighting-game gestures the player kit uses — the quarter-circle
//! (`qcf`), its keyboard-friendly two-step grace form (`qcf_grace`), and the
//! half-circle (`hcf`) — are registered here into the reusable, empty-by-default
//! [`MotionTechniqueCatalog`](ambition_projectiles::MotionTechniqueCatalog). The
//! reusable input crate names none of them; the player fire system asks the
//! catalog by id whether each fired.
//!
//! Ordering note: `qcf_grace` (`Down → Right`) is a subsequence of the full
//! `qcf` (`Down → DownRight → Right`), so the fire system checks `qcf`/`hcf`
//! before `qcf_grace` (most-specific gate first) — the catalog stores patterns;
//! the consumer owns the precedence.

use ambition_projectiles::{MotionDirection, MotionTechnique, MotionTechniqueAppExt};
use bevy::prelude::App;

use MotionDirection::{Down, DownLeft, DownRight, Left, Right};

/// Register Ambition's player-kit motion techniques into the App-local catalog.
pub(super) fn register(app: &mut App) {
    // Full quarter-circle-forward: `Down → DownRight → Right` (and its mirror).
    app.register_motion_technique(
        "qcf",
        MotionTechnique::new(vec![
            vec![Down, DownRight, Right],
            vec![Down, DownLeft, Left],
        ]),
    );

    // Grace quarter-circle: just `Down → Right` (and mirror), without the awkward
    // diagonal midpoint — the keyboard-friendly path to the weaker Hadouken.
    app.register_motion_technique(
        "qcf_grace",
        MotionTechnique::new(vec![vec![Down, Right], vec![Down, Left]]),
    );

    // Half-circle: `Right → DownRight → Down → DownLeft → Left` (and mirror).
    // Fires the direction opposite the one it ends on, so it inverts facing.
    app.register_motion_technique(
        "hcf",
        MotionTechnique {
            patterns: vec![
                vec![Right, DownRight, Down, DownLeft, Left],
                vec![Left, DownLeft, Down, DownRight, Right],
            ],
            invert_facing: true,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_projectiles::{MotionInputBuffer, MotionTechniqueCatalog};

    fn buffer_of(dirs: &[MotionDirection]) -> MotionInputBuffer {
        let mut buf = MotionInputBuffer::new(1.0);
        let mut t = 0.0;
        for &dir in dirs {
            buf.push(dir, t);
            t += 0.04;
        }
        buf
    }

    #[test]
    fn ambition_registers_the_player_kit_techniques() {
        let mut app = App::new();
        register(&mut app);
        let catalog = app.world().resource::<MotionTechniqueCatalog>();
        for id in ["qcf", "qcf_grace", "hcf"] {
            assert!(catalog.get(id).is_some(), "{id} must be registered");
        }
        assert!(catalog.get("dragon_punch").is_none());
    }

    #[test]
    fn registered_qcf_and_hcf_detect_their_gestures() {
        let mut app = App::new();
        register(&mut app);
        let catalog = app.world().resource::<MotionTechniqueCatalog>();

        let qcf_right = buffer_of(&[Down, DownRight, Right]);
        assert_eq!(catalog.detect("qcf", &qcf_right), Some(1.0));

        // Half-circle right→left ends on Left (facing -1) but fires facing +1.
        let hcf = buffer_of(&[Right, DownRight, Down, DownLeft, Left]);
        assert_eq!(catalog.detect("hcf", &hcf), Some(1.0));
    }
}
