//! The FPS / frame-time counter, on Bevy's overlay.
//!
//! ⭐ THE OVERLAY ITSELF IS UPSTREAM'S. `bevy::dev_tools::fps_overlay` owns the
//! node, the text, the smoothing and the frame-time graph; this module owns the
//! three things that are Ambition's and not Bevy's: WHICH SETTING decides it,
//! WHERE it sits, and WHETHER it is visible on a platform where upstream's own
//! toggle cannot run.
//!
//! Visible by default on every platform — desktop, browser, Android. Toggle via
//! the Video settings page → "FPS Overlay" row (persisted by
//! `ambition_platformer2d::persistence::settings::persistence`), or press `F6`,
//! which writes the same setting.
//!
//! ## Source of truth
//!
//! [`UserSettings::video::show_fps`] is the canonical flag and is what lands on
//! disk. There is no runtime mirror any more: the old `FpsOverlayState` existed
//! only to spare the overlay systems a `UserSettings` lookup, and the systems
//! that needed it are gone.

use bevy::dev_tools::fps_overlay::{
    FpsOverlayConfig, FpsOverlayPlugin as BevyFpsOverlayPlugin, FPS_OVERLAY_ZINDEX,
};
use bevy::prelude::*;

use ambition_platformer2d::persistence::settings::UserSettings;
use ambition_platformer2d::platformer::developer_hotkeys::DeveloperAction;
use ambition_platformer2d::presentation::gameplay_presentation::ResolvedGameplayPresentation;
use ambition_platformer2d::render::ui_fonts::{UiFontWeight, UiFonts};

/// Gap between the Menu/Back row and the counter tucked under it.
const FPS_OVERLAY_GAP: f32 = 6.0;

/// Inset from the corner when there is no on-screen control row to sit under.
const FPS_OVERLAY_MARGIN: f32 = 8.0;

/// The counter's type size, in logical pixels.
///
/// Upstream's default is 32px, which is a demo size — on a 640x360 phone
/// viewport it is a quarter of the screen height.
const FPS_OVERLAY_FONT_PX: f32 = 12.0;

/// Ambition's wiring around [`BevyFpsOverlayPlugin`].
pub struct FpsOverlayPlugin;

impl Plugin for FpsOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BevyFpsOverlayPlugin {
            config: FpsOverlayConfig {
                text_config: TextFont {
                    font_size: FontSize::Px(FPS_OVERLAY_FONT_PX),
                    ..default()
                },
                text_color: Color::srgba(0.82, 0.95, 1.0, 0.88),
                // The counter is opt-OUT. `sync_fps_overlay_from_settings`
                // corrects this on the first frame `UserSettings` is loaded;
                // starting visible is what the player who never opens the
                // settings menu gets.
                enabled: true,
                ..default()
            },
        })
        .add_message::<DeveloperAction>()
        .add_systems(
            Update,
            (
                sync_fps_overlay_from_settings,
                toggle_fps_overlay_from_hotkey,
                place_fps_overlay,
            ),
        );
    }
}

/// Push the authoritative setting into upstream's config, and the product font
/// in behind it once the faces have loaded.
///
/// ⛔ THE COMPARISON BEFORE EACH WRITE IS LOAD-BEARING. Bevy runs
/// `toggle_display` and `customize_overlay` on `resource_changed::<FpsOverlayConfig>`,
/// so an unconditional assignment would re-run both every frame forever — a
/// full text restyle per frame to write the value that was already there.
fn sync_fps_overlay_from_settings(
    settings: Res<UserSettings>,
    fonts: Option<Res<UiFonts>>,
    mut config: ResMut<FpsOverlayConfig>,
) {
    if config.enabled != settings.video.show_fps {
        config.enabled = settings.video.show_fps;
    }
    // ⭐ MONOSPACE, AND THE REASON IS THE READING: a counter whose digits change
    // width jitters on every frame it updates. `UiFonts` arrives with the asset
    // load, so this is a correction rather than a construction.
    if let Some(fonts) = fonts {
        let wanted = fonts.text_font(FPS_OVERLAY_FONT_PX, UiFontWeight::Monospace);
        if config.text_config.font != wanted.font || config.text_config.weight != wanted.weight {
            config.text_config.font = wanted.font;
            config.text_config.weight = wanted.weight;
        }
    }
}

/// The developer action (F6) toggles the counter by writing the SETTING, so the
/// keyboard toggle persists exactly like the menu row does.
fn toggle_fps_overlay_from_hotkey(
    mut actions: MessageReader<DeveloperAction>,
    mut settings: ResMut<UserSettings>,
) {
    if actions
        .read()
        .any(|action| *action == DeveloperAction::ToggleFpsOverlay)
    {
        settings.video.show_fps = !settings.video.show_fps;
    }
}

/// Tuck the counter under the Menu/Back row instead of leaving it in a corner,
/// and own its visibility.
///
/// ⛔⛔ THE VISIBILITY HALF IS NOT REDUNDANT WITH UPSTREAM, AND THE PLATFORM
/// THAT PROVES IT IS THE WEB. Bevy's `toggle_display` takes
/// `Single<&mut Node, With<FrameTimeGraph>>` — and the graph node is spawned
/// under `#[cfg(not(all(target_arch = "wasm32", not(feature = "webgpu"))))]`.
/// Ambition's web persona is `bevy/webgl2`, so on the shipping browser build
/// that node DOES NOT EXIST, `Single` returns `skipped`, and `toggle_display`
/// never runs. `FpsOverlayConfig.enabled` would be a setting that does nothing
/// in a browser — which is precisely the requirement ("apply on desktop, web
/// and Android") the upstream implementation cannot meet on its own.
///
/// Setting `display` on the ROOT node covers every platform, and it wins over
/// upstream's per-text toggle rather than fighting it: a hidden root hides the
/// text and the graph whatever their own `display` says.
///
/// ⭐ THE POSITION IS READ, NOT RE-DERIVED. The bottom-right corner is where the
/// action cluster goes on a touch device, so the counter spent every phone
/// session sitting underneath the buttons — present, updating, unreadable.
/// `ResolvedGameplayPresentation` says of itself that no camera, HUD, touch or
/// pointer system should independently recalculate margins: the row moves with
/// the safe-area insets and with its own resolved scale, so a constant would be
/// right on one device and wrong on the next. `ScreenRect` is logical pixels —
/// the space `Val::Px` is in — which is why no scale factor appears here.
///
/// With no row published — a keyboard session publishes no touch footprint —
/// there is nothing to sit under and the counter keeps its corner.
fn place_fps_overlay(
    settings: Res<UserSettings>,
    presentation: Option<Res<ResolvedGameplayPresentation>>,
    mut overlays: Query<(&mut Node, &GlobalZIndex)>,
) {
    let menu_row = presentation
        .as_deref()
        .and_then(|presentation| presentation.controls.system_controls)
        .map(|placed| placed.rect);

    let (left, top, right, bottom) = match menu_row {
        Some(rect) => (
            Val::Px(rect.min.x),
            Val::Px(rect.max.y + FPS_OVERLAY_GAP),
            Val::Auto,
            Val::Auto,
        ),
        None => (
            Val::Auto,
            Val::Auto,
            Val::Px(FPS_OVERLAY_MARGIN),
            Val::Px(FPS_OVERLAY_MARGIN),
        ),
    };
    let display = if settings.video.show_fps {
        Display::DEFAULT
    } else {
        Display::None
    };

    for (mut node, z) in &mut overlays {
        // ⭐ THE Z-INDEX IS THE IDENTITY. Upstream's `FpsText` marker is private,
        // and the root node carries no marker at all — but `FPS_OVERLAY_ZINDEX`
        // is public and is the one thing the overlay root is guaranteed to have.
        if z.0 != FPS_OVERLAY_ZINDEX {
            continue;
        }
        // Compared before writing: `Node` is change-detected and this runs every
        // frame, so assigning unconditionally would dirty UI layout forever over
        // values that only move on a resize, a rotation or a settings change.
        if node.left != left || node.top != top || node.right != right || node.bottom != bottom {
            node.left = left;
            node.top = top;
            node.right = right;
            node.bottom = bottom;
        }
        if node.display != display {
            node.display = display;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_video_settings_show_fps_is_true() {
        let settings = UserSettings::default();
        assert!(
            settings.video.show_fps,
            "VideoSettings::show_fps default must be true so the counter shows out of the box",
        );
    }
}
