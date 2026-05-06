// crates/ambition_sandbox/src/ui_fonts.rs

use std::path::Path;

use bevy::log::{info, warn};
use bevy::prelude::*;

pub const DIALOG_FONT_REGULAR: &str = "/usr/share/fonts/opentype/inter/InterDisplay-Regular.otf";
pub const DIALOG_FONT_SEMIBOLD: &str = "/usr/share/fonts/opentype/inter/InterDisplay-SemiBold.otf"; 


#[derive(Resource, Clone, Debug, Default)]
pub struct UiFonts {
    pub regular: Option<Handle<Font>>,
    pub semibold: Option<Handle<Font>>,
}

impl UiFonts {
    pub fn has_dialog_font(&self) -> bool {
        self.regular.is_some()
    }

    pub fn selected_marker(&self) -> &'static str {
        if self.has_dialog_font() {
            ">"
            //"▶"
        } else {
            ">"
        }
    }

    pub fn text_font(&self, size: f32, weight: UiFontWeight) -> TextFont {
        let handle = match weight {
            UiFontWeight::Regular => self.regular.clone(),
            UiFontWeight::Semibold => self.semibold.clone().or_else(|| self.regular.clone()),
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
}

pub fn load_ui_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    let regular = load_optional_font(&asset_server, DIALOG_FONT_REGULAR, "regular dialogue UI font");
    let semibold = load_optional_font(
        &asset_server,
        DIALOG_FONT_SEMIBOLD,
        "semibold dialogue UI font",
    )
    .or_else(|| regular.clone());

    if regular.is_none() {
        warn!(
            "No local dialogue UI font found; falling back to Bevy default font and ASCII selector. \
             Recommended local font: Inter Display. Expected asset path: assets/{DIALOG_FONT_REGULAR}"
        );
    }

    commands.insert_resource(UiFonts { regular, semibold });
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
