//! Symbolic music placeholders.
//!
//! The playable sandbox currently synthesizes short SFX in the Bevy layer. This
//! module is a reminder that longer-term Ambition music should be represented as
//! inspectable symbolic data: scales, motifs, rhythms, transformations, and
//! state-driven arrangements rather than imported audio files.

/// Small symbolic melodic/rhythmic seed.
///
/// The Bevy sandbox currently renders a concrete SNES-style loop from code in
/// `ambition_sandbox::audio`. Longer-term, material like this should move toward
/// these symbolic motifs plus room/state arrangement rules.
#[derive(Clone, Debug)]
pub struct Motif {
    pub name: &'static str,
    pub scale_degrees: &'static [i32],
    pub rhythm_units: &'static [u8],
}

pub const TANGENT_MOTIF: Motif = Motif {
    name: "tangent-space",
    scale_degrees: &[0, 2, 3, 7, 5, 3, 2, 0],
    rhythm_units: &[1, 1, 2, 1, 1, 2, 3, 5],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tangent_motif_has_aligned_scale_and_rhythm() {
        // Each scale degree wants a matching rhythm unit; an arrangement
        // engine would zip these and fail if the lengths drifted.
        assert_eq!(
            TANGENT_MOTIF.scale_degrees.len(),
            TANGENT_MOTIF.rhythm_units.len(),
            "scale_degrees and rhythm_units must have matching length"
        );
        assert!(!TANGENT_MOTIF.name.is_empty());
        assert!(!TANGENT_MOTIF.scale_degrees.is_empty());
    }

    #[test]
    fn tangent_motif_rhythm_units_are_positive() {
        // A zero-length rhythm unit would represent a "silent step" the
        // arrangement engine isn't designed to produce; flag the case so
        // future authoring catches it at test time.
        for &unit in TANGENT_MOTIF.rhythm_units {
            assert!(unit > 0, "rhythm units must be positive (got {unit})");
        }
    }
}
