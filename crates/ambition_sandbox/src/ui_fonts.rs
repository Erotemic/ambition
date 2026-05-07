// crates/ambition_sandbox/src/ui_fonts.rs

use std::path::Path;

use bevy::log::{info, warn};
use bevy::prelude::*;

pub const DIALOG_FONT_REGULAR: &str = "fonts/local/InterDisplay-Regular.otf";
pub const DIALOG_FONT_SEMIBOLD: &str = "fonts/local/InterDisplay-SemiBold.otf";
pub const DEBUG_FONT_MONO: &str = "fonts/local/DejaVuSansMono.ttf";

#[derive(Resource, Clone, Debug, Default)]
pub struct UiFonts {
    pub regular: Option<Handle<Font>>,
    pub semibold: Option<Handle<Font>>,
    pub mono: Option<Handle<Font>>,
}

impl UiFonts {
    pub fn has_dialog_font(&self) -> bool {
        self.regular.is_some()
    }

    pub fn selected_marker(&self) -> &'static str {
        if self.has_dialog_font() {
            "►"
        } else {
            ">"
        }
    }

    pub fn text_font(&self, size: f32, weight: UiFontWeight) -> TextFont {
        let handle = match weight {
            UiFontWeight::Regular => self.regular.clone(),
            UiFontWeight::Semibold => self.semibold.clone().or_else(|| self.regular.clone()),
            UiFontWeight::Monospace => self.mono.clone().or_else(|| self.regular.clone()),
        };

        let mut font = TextFont {
            font_size: size,
            ..default()
        };

        if let Some(handle) = handle {
            font.font = handle;
        }

        font
    }
}

#[derive(Clone, Copy, Debug)]
pub enum UiFontWeight {
    Regular,
    Semibold,
    Monospace,
}

pub fn load_ui_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    let regular = load_optional_font(
        &asset_server,
        DIALOG_FONT_REGULAR,
        "regular dialogue UI font",
    );

    let semibold = load_optional_font(
        &asset_server,
        DIALOG_FONT_SEMIBOLD,
        "semibold dialogue UI font",
    )
    .or_else(|| regular.clone());

    let mono = load_optional_font(&asset_server, DEBUG_FONT_MONO, "monospace debug UI font");

    if regular.is_none() {
        warn!(
            "No local dialogue UI font found; falling back to Bevy default font and ASCII selector. \
             Recommended local font: Inter Display. Expected asset path: assets/{DIALOG_FONT_REGULAR}"
        );
    }

    if mono.is_none() {
        warn!(
            "No monospace debug UI font found; debug HUD will fall back to the regular UI font or Bevy default. \
             Expected asset path: assets/{DEBUG_FONT_MONO}"
        );
    }

    commands.insert_resource(UiFonts {
        regular,
        semibold,
        mono,
    });
}

fn load_optional_font(
    asset_server: &AssetServer,
    relative_asset_path: &'static str,
    label: &str,
) -> Option<Handle<Font>> {
    if asset_exists(relative_asset_path) {
        info!("Using {label}: assets/{relative_asset_path}");
        Some(asset_server.load(relative_asset_path))
    } else {
        warn!("Missing {label}: assets/{relative_asset_path}");
        None
    }
}

fn asset_exists(relative_asset_path: &str) -> bool {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(relative_asset_path)
        .exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_fonts() {
        let fonts = UiFonts::default();
        assert!(!fonts.has_dialog_font());
        assert!(fonts.regular.is_none());
        assert!(fonts.semibold.is_none());
        assert!(fonts.mono.is_none());
    }

    #[test]
    fn selected_marker_falls_back_to_ascii_when_no_dialog_font() {
        // Without a dialog font, use a portable ">" marker (the
        // unicode "►" pointer needs the Inter-Display font to
        // render legibly).
        let fonts = UiFonts::default();
        assert_eq!(fonts.selected_marker(), ">");
    }

    #[test]
    fn text_font_uses_size_even_without_handle() {
        let fonts = UiFonts::default();
        let font = fonts.text_font(14.0, UiFontWeight::Regular);
        assert_eq!(font.font_size, 14.0);
    }
}
