//! [`PreloadGroup`] — coarse "load-this-set-up-front" tag.
//!
//! Asset entries declare which group they belong to (or `None` for
//! lazy/on-demand). The resolver groups entries by tag so the consumer
//! can drive preloads with [`bevy_asset_loader`] or a hand-rolled state
//! machine. The catalog itself does not block on loads.
//!
//! The set of groups is deliberately small (~10 max). New ones land
//! when there's a real concurrent-load decision to make, not for every
//! room.

use serde::{Deserialize, Serialize};

/// Coarse preload bucket. Entries with the same group are loaded as one
/// batch; nothing else is implied (no ordering between groups in the
/// first slice).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PreloadGroup {
    /// Bootstrap assets the app needs to render the first frame:
    /// default font, LDtk project, color shader. Resolution failure
    /// here is a hard error.
    Bootstrap,
    /// Title-screen / splash / loading-screen visuals.
    TitleScreen,
    /// Persistent HUD chrome (font subset, life pips, button glyphs).
    Hud,
    /// Sprite/audio sets the player runs into in the first sandbox
    /// session (entity sprites, default music, default SFX bank).
    SandboxCore,
    /// Per-zone art: parallax layers, biome-specific tiles, boss-room
    /// sprites. Loaded when entering the zone.
    Zone,
    /// Cutscene-only assets (boss intro stills, special audio).
    Cutscene,
    /// Dev-tool / inspector / debug overlay assets. Skipped in
    /// release profiles.
    DevTools,
}

impl PreloadGroup {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::TitleScreen => "title_screen",
            Self::Hud => "hud",
            Self::SandboxCore => "sandbox_core",
            Self::Zone => "zone",
            Self::Cutscene => "cutscene",
            Self::DevTools => "dev_tools",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_distinct() {
        let groups = [
            PreloadGroup::Bootstrap,
            PreloadGroup::TitleScreen,
            PreloadGroup::Hud,
            PreloadGroup::SandboxCore,
            PreloadGroup::Zone,
            PreloadGroup::Cutscene,
            PreloadGroup::DevTools,
        ];
        let labels: std::collections::HashSet<_> = groups.iter().map(|g| g.label()).collect();
        assert_eq!(labels.len(), groups.len());
    }
}
