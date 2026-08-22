//! Declarative movement paths for moving platforms, spike balls, patrol
//! dummies, and scripted hazards.
//!
//! This is world/geometry vocabulary: a path is a list of `Vec2` waypoints
//! plus a speed and playback mode. It lives beside `World`/`Block`/
//! `SurfaceChain` in the engine because it carries spatial primitives and is
//! authored INPUT to the sim, not actor mind/behavior data.

use crate::Vec2;

/// Declarative movement path for moving platforms, spike balls, patrol dummies,
/// and later scripted boss hazards.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KinematicPath {
    pub points: Vec<Vec2>,
    pub speed: f32,
    pub mode: KinematicPathMode,
    pub start_offset_seconds: f32,
}

impl KinematicPath {
    pub fn line(a: Vec2, b: Vec2, speed: f32) -> Self {
        Self {
            points: vec![a, b],
            speed,
            mode: KinematicPathMode::PingPong,
            start_offset_seconds: 0.0,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.points.len() >= 2 && self.speed > 0.0
    }
}

/// Resolve an authored path reference through the shared lookup rule.
/// References are trimmed and an empty reference names nothing. Authored path
/// keys are not normalized here, so malformed path definitions remain visible to
/// validation rather than being repaired during lookup.
pub fn resolve_kinematic_path<'a>(
    paths: &'a [(String, KinematicPath)],
    reference: &str,
) -> Option<&'a KinematicPath> {
    let reference = reference.trim();
    if reference.is_empty() {
        return None;
    }
    paths
        .iter()
        .find(|(key, _)| key == reference)
        .map(|(_, path)| path)
}

/// Playback style for a kinematic path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KinematicPathMode {
    Once,
    Loop,
    PingPong,
}

#[cfg(test)]
mod reference_resolution_tests {
    use super::*;

    fn table() -> Vec<(String, KinematicPath)> {
        vec![
            // A room built in Rust rather than LDtk can carry a path whose
            // authored id is blank — LDtk's converter falls back to the entity
            // name and then the iid, and nothing outside it does.
            (
                String::new(),
                KinematicPath::line(Vec2::ZERO, Vec2::ZERO, 1.0),
            ),
            (
                "lab_patrol".to_string(),
                KinematicPath::line(Vec2::new(0.0, 0.0), Vec2::new(64.0, 0.0), 40.0),
            ),
        ]
    }

    /// A padded authored reference names the same path a clean one does, and a
    /// blank one names nothing. The whole point of the rule living in one
    /// place: a leading space in an NPC's `patrol_path_id` used to disable that
    /// patrol silently, because a miss here degrades to "no motion" rather than
    /// to an error anyone sees.
    #[test]
    fn an_authored_reference_is_trimmed_and_a_blank_one_names_nothing() {
        let paths = table();
        let clean = resolve_kinematic_path(&paths, "lab_patrol");
        assert!(clean.is_some(), "precondition: the clean spelling resolves");
        assert_eq!(
            resolve_kinematic_path(&paths, "  lab_patrol "),
            clean,
            "padding around an authored reference must not change what it names"
        );
        assert!(
            resolve_kinematic_path(&paths, "   ").is_none(),
            "a whitespace-only reference names nothing — it must NOT fall through \
             onto a path whose own id happens to be blank"
        );
    }
}
