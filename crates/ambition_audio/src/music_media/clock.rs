/// A section-local clock. Encoded grids use the producer's global rounding.
#[derive(Clone, Debug, PartialEq)]
pub enum PreparedMusicClock {
    Constant {
        seconds_per_bar: f64,
        first_bar_seconds: f64,
        frames: Option<MusicFrameOrigin>,
        /// Keep the exported count at a rounded section end.
        bar_count: Option<usize>,
    },
    Boundaries {
        bar_seconds: Vec<f64>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicFrameOrigin {
    pub sample_rate: u32,
    pub score_seconds: f64,
    pub score_frame: u64,
}

impl MusicFrameOrigin {
    pub fn local_frame(self, seconds: f64) -> f64 {
        ((self.score_seconds + seconds) * f64::from(self.sample_rate)).round_ties_even()
            - self.score_frame as f64
    }
    fn quantize(self, seconds: f64) -> f64 {
        self.local_frame(seconds) / f64::from(self.sample_rate)
    }
}

impl PreparedMusicClock {
    /// Query every `bars` local bar boundaries. A loop end is also a boundary.
    pub fn delay_to_boundary(
        &self,
        seconds: f64,
        bars: usize,
        loop_seconds: Option<f64>,
    ) -> Option<f64> {
        if !seconds.is_finite() || seconds < 0.0 || bars == 0 {
            return None;
        }
        let position = match loop_seconds {
            Some(pass) if pass.is_finite() && pass > 0.0 => seconds.rem_euclid(pass),
            Some(_) => return None,
            None => seconds,
        };
        let boundary = match self {
            Self::Constant {
                seconds_per_bar,
                first_bar_seconds,
                frames,
                bar_count,
            } => {
                let first = first_bar_seconds
                    + (-first_bar_seconds / seconds_per_bar).ceil().max(0.0) * seconds_per_bar;
                let period = seconds_per_bar * bars as f64;
                if !period.is_finite() || period <= 0.0 {
                    return None;
                }
                let mut index = ((position - first) / period).floor().max(0.0);
                let quantize = |value| frames.map_or(value, |origin| origin.quantize(value));
                let mut boundary = quantize(first + index * period);
                if boundary + 1e-9 < position {
                    index += 1.0;
                    boundary = quantize(first + index * period);
                }
                if bar_count.is_some_and(|count| index * bars as f64 >= count as f64) {
                    None
                } else {
                    Some(boundary)
                }
            }
            Self::Boundaries { bar_seconds } => {
                let at = bar_seconds.partition_point(|boundary| *boundary + 1e-9 < position);
                bar_seconds.get(at.div_ceil(bars) * bars).copied()
            }
        };
        let boundary = match (boundary, loop_seconds) {
            (Some(boundary), Some(pass)) => boundary.min(pass),
            (None, Some(pass)) => pass,
            (Some(boundary), None) => boundary,
            (None, None) => return None,
        };
        Some((boundary - position).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A three-bar loop with two-bar phrases (GNU-ton's fight loop is 73
    /// bars): the marker of each pass is at its bar 2, and the end of a pass
    /// is a marker. A grid that did not start again put the markers of the
    /// second pass on its odd bars.
    #[test]
    fn the_phrase_grid_starts_again_with_each_pass_of_a_loop() {
        let clock = PreparedMusicClock::Constant {
            seconds_per_bar: 4.0,
            first_bar_seconds: 0.0,
            frames: None,
            bar_count: None,
        };
        let pass = Some(12.0);
        // Pass 1, half a bar before its end: the end of the pass.
        assert_eq!(clock.delay_to_boundary(10.0, 2, pass).unwrap(), 2.0);
        // Pass 2, half a bar in: its bar 2, not bar 1.
        assert_eq!(clock.delay_to_boundary(14.0, 2, pass).unwrap(), 6.0);
        // A section that does not loop keeps one grid.
        assert_eq!(clock.delay_to_boundary(14.0, 2, None).unwrap(), 2.0);
    }
}
