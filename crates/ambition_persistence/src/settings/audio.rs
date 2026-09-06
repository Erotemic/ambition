//! Audio settings: master / music / SFX volume + mute.
//!
//! Volumes are stored as `f32` in `[0, 1]`. The presentation-side audio
//! plugin reads `AudioSettings` and applies it to the Kira channels.
//! Mute snapshots prior values so it can restore on toggle off.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub muted: bool,
    /// Snapshot of pre-mute master volume so toggling un-mute restores
    /// the prior level rather than collapsing to a default.
    pub muted_snapshot_master: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 0.85,
            music_volume: 0.65,
            sfx_volume: 0.85,
            muted: false,
            muted_snapshot_master: 0.85,
        }
    }
}

impl AudioSettings {
    pub const VOLUME_STEP: f32 = 0.05;

    pub fn clamp_all(&mut self) {
        self.master_volume = self.master_volume.clamp(0.0, 1.0);
        self.music_volume = self.music_volume.clamp(0.0, 1.0);
        self.sfx_volume = self.sfx_volume.clamp(0.0, 1.0);
        self.muted_snapshot_master = self.muted_snapshot_master.clamp(0.0, 1.0);
    }

    /// Effective master volume after mute. `0.0` if muted.
    pub fn effective_master(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.master_volume
        }
    }

    /// Effective music output volume = master × music after mute.
    pub fn effective_music(&self) -> f32 {
        self.effective_master() * self.music_volume
    }

    /// Effective SFX output volume = master × sfx after mute.
    pub fn effective_sfx(&self) -> f32 {
        self.effective_master() * self.sfx_volume
    }

    /// Toggle the mute flag. When entering mute we snapshot the master
    /// volume so the next un-mute restores it; when leaving mute we
    /// restore the snapshot if master was set to zero while muted.
    pub fn toggle_mute(&mut self) {
        if !self.muted {
            self.muted_snapshot_master = self.master_volume;
            self.muted = true;
        } else {
            self.muted = false;
            if self.master_volume <= f32::EPSILON {
                self.master_volume = if self.muted_snapshot_master > 0.0 {
                    self.muted_snapshot_master
                } else {
                    0.5
                };
            }
        }
        self.clamp_all();
    }

    pub fn nudge_master(&mut self, delta: f32) {
        self.master_volume = (self.master_volume + delta).clamp(0.0, 1.0);
        if self.master_volume > 0.0 && self.muted {
            self.muted = false;
        }
    }

    pub fn nudge_music(&mut self, delta: f32) {
        self.music_volume = (self.music_volume + delta).clamp(0.0, 1.0);
    }

    pub fn nudge_sfx(&mut self, delta: f32) {
        self.sfx_volume = (self.sfx_volume + delta).clamp(0.0, 1.0);
    }

    /// Display percent (`0..=100`) for HUD/menu rows.
    pub fn percent(value: f32) -> u32 {
        (value.clamp(0.0, 1.0) * 100.0).round() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_in_unit_range() {
        let s = AudioSettings::default();
        assert!((0.0..=1.0).contains(&s.master_volume));
        assert!((0.0..=1.0).contains(&s.music_volume));
        assert!((0.0..=1.0).contains(&s.sfx_volume));
    }

    #[test]
    fn nudge_clamps_to_unit_range() {
        let mut s = AudioSettings::default();
        for _ in 0..100 {
            s.nudge_master(1.0);
        }
        assert_eq!(s.master_volume, 1.0);
        for _ in 0..100 {
            s.nudge_master(-1.0);
        }
        assert_eq!(s.master_volume, 0.0);
    }

    #[test]
    fn mute_round_trip_restores_master() {
        let mut s = AudioSettings::default();
        s.master_volume = 0.7;
        s.toggle_mute();
        assert!(s.muted);
        assert_eq!(s.effective_master(), 0.0);
        s.toggle_mute();
        assert!(!s.muted);
        assert!((s.master_volume - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn unmute_when_master_was_zeroed_restores_snapshot() {
        let mut s = AudioSettings::default();
        s.master_volume = 0.6;
        s.toggle_mute();
        s.master_volume = 0.0;
        s.toggle_mute();
        assert!(!s.muted);
        assert!(s.master_volume > 0.0);
    }

    #[test]
    fn raising_master_clears_mute() {
        let mut s = AudioSettings::default();
        s.toggle_mute();
        assert!(s.muted);
        s.nudge_master(0.1);
        assert!(!s.muted);
    }

    #[test]
    fn percent_format_rounds_correctly() {
        assert_eq!(AudioSettings::percent(0.0), 0);
        assert_eq!(AudioSettings::percent(0.5), 50);
        assert_eq!(AudioSettings::percent(1.0), 100);
        assert_eq!(AudioSettings::percent(0.875), 88);
    }

    #[test]
    fn effective_volumes_compose_master_and_channel() {
        let mut s = AudioSettings::default();
        s.master_volume = 0.5;
        s.music_volume = 0.5;
        s.sfx_volume = 1.0;
        assert!((s.effective_music() - 0.25).abs() < 1e-6);
        assert!((s.effective_sfx() - 0.5).abs() < 1e-6);
    }
}
