//! A move in play, in frames: what a training display prints.
//!
//! Moves are authored in seconds of the owner's proper time; a fighting-game
//! player reads frames. This converts at the tick rate the caller names (the
//! sim's, normally), and derives every number from the authored windows
//! through [`MoveSpec::frame_data`], so it cannot disagree with the brain or
//! the boss validator about where a move hits.

use super::MovePlayback;
use ambition_entity_catalog::WindowTag;

/// One move in play, in frames counted from 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveFrameReadout {
    pub move_id: String,
    /// The frame the move's clock is on.
    pub frame: u32,
    pub total_frames: u32,
    /// The frame the first live hit comes out on (the genre's startup number).
    /// `None` for a move with no live hit.
    pub hits_on: Option<u32>,
    /// Every Active window, as its first and last frame.
    pub active: Vec<(u32, u32)>,
    /// Frames from the last Active window's end to the move's end.
    pub recovery_frames: u32,
    /// The authored windows the clock is inside now (Startup, Active, Armor…).
    pub windows_now: Vec<String>,
    /// The move has overlapped a victim (see [`MovePlayback::landed_hit`]).
    pub landed: bool,
}

/// Seconds to whole frames at `tick_hz`, to the nearest frame: an authored
/// window is meant to be a whole number of frames.
fn frames(seconds: f32, tick_hz: f32) -> u32 {
    (seconds * tick_hz).round().max(0.0) as u32
}

impl MoveFrameReadout {
    pub fn of(playback: &MovePlayback, tick_hz: f32) -> Self {
        let spec = &playback.spec;
        let data = spec.frame_data();
        let total_frames = frames(data.total_s, tick_hz).max(1);
        let frame = ((playback.t * tick_hz).floor().max(0.0) as u32 + 1).min(total_frames);
        let hits_on = (!data.active_spans.is_empty() && data.startup_s < data.total_s)
            .then(|| frames(data.startup_s, tick_hz) + 1);
        let active = data
            .active_spans
            .iter()
            .map(|(start, end)| (frames(*start, tick_hz) + 1, frames(*end, tick_hz).max(1)))
            .collect();
        let windows_now = spec
            .windows
            .iter()
            .filter(|window| playback.t >= window.start_s && playback.t < window.end_s)
            .map(|window| window_name(&window.tag))
            .collect();
        Self {
            move_id: spec.id.clone(),
            frame,
            total_frames,
            hits_on,
            active,
            recovery_frames: frames(data.recovery_s, tick_hz),
            windows_now,
            landed: playback.landed_hit,
        }
    }

    /// One line: `jab  f7/24  startup 5  active 5-7  recovery 17  [Active] hit`.
    pub fn line(&self) -> String {
        let mut line = format!("{}  f{}/{}", self.move_id, self.frame, self.total_frames);
        match self.hits_on {
            Some(frame) => line.push_str(&format!("  startup {frame}")),
            None => line.push_str("  no hit"),
        }
        if !self.active.is_empty() {
            let spans: Vec<String> = self
                .active
                .iter()
                .map(|(first, last)| if first == last { first.to_string() } else { format!("{first}-{last}") })
                .collect();
            line.push_str(&format!("  active {}", spans.join(",")));
        }
        line.push_str(&format!("  recovery {}", self.recovery_frames));
        if !self.windows_now.is_empty() {
            line.push_str(&format!("  [{}]", self.windows_now.join(" ")));
        }
        if self.landed {
            line.push_str(" hit");
        }
        line
    }
}

/// A window tag's name without its payload: `Cancelable { .. }` prints as
/// `Cancelable`.
fn window_name(tag: &WindowTag) -> String {
    let debug = format!("{tag:?}");
    debug
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::MoveSpec;

    /// A 24-frame move at 60 Hz: windup 4 frames, live on 5-7, recovery 17.
    /// The melee prefab, retimed by window tag.
    fn jab() -> MoveSpec {
        let f = |n: f32| n / 60.0;
        let mut spec = ambition_characters::moveset_prefabs::simple_melee(&Default::default());
        spec.id = "jab".into();
        spec.duration_s = f(24.0);
        for window in &mut spec.windows {
            let (start, end) = match window.tag {
                WindowTag::Startup => (0.0, 4.0),
                WindowTag::Active => (4.0, 7.0),
                WindowTag::Recovery => (7.0, 24.0),
                _ => continue,
            };
            window.start_s = f(start);
            window.end_s = f(end);
        }
        let tags: Vec<String> = spec.windows.iter().map(|window| window_name(&window.tag)).collect();
        assert_eq!(tags, ["Startup", "Active", "Recovery"], "the prefab this fixture retimes changed shape");
        assert!(spec.windows[1].volumes.iter().count() > 0, "the prefab's active window strikes");
        spec
    }

    #[test]
    fn a_move_reads_in_frames_from_its_authored_windows() {
        let mut playback = MovePlayback::new(jab(), 1.0);
        playback.t = 5.5 / 60.0;
        let readout = MoveFrameReadout::of(&playback, 60.0);
        assert_eq!(
            readout.line(),
            "jab  f6/24  startup 5  active 5-7  recovery 17  [Active]",
            "{readout:?}"
        );
        playback.t = 0.0;
        assert_eq!(MoveFrameReadout::of(&playback, 60.0).windows_now, vec!["Startup".to_string()]);
    }
}
