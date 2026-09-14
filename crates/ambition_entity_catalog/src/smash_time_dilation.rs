//! Authored payload for temporarily slowing a body.
//! The technique writes the existing per-body proper-time scale. It does not define a second time authority.

use serde::{Deserialize, Serialize};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const TIME_DILATION: &str = "smash.time_dilation";

/// Authored parameters of one dilation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeDilationParams {
    /// Victim proper-time multiplier. Valid authored slows are in `0.0..1.0`.
    pub scale: f32,
    /// Duration in world seconds. Using world time prevents the slow from extending its own duration.
    pub seconds: f32,
}

/// Hydrate and semantically validate a time-dilation payload.
pub fn check_time_dilation_params(
    params: &crate::ParamValue,
) -> Result<(), String> {
    let typed: TimeDilationParams = params.hydrate().map_err(|error| error.to_string())?;
    let problems = typed.problems();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

impl TimeDilationParams {
    /// Everything wrong with these params, as sentences an author can act on.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if !(0.0..1.0).contains(&self.scale) {
            problems.push(format!(
                "scale {} is not a slow: below 1.0 is the only direction with a \
                 customer, and 0.0..1.0 is the range that has one",
                self.scale
            ));
        }
        if self.seconds <= 0.0 {
            problems.push(format!(
                "seconds {} would apply a scale nothing ever takes away",
                self.seconds
            ));
        }
        problems
    }
}
