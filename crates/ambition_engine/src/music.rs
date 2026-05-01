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
