//! Coherent first reveal for the visible direct-entry sandbox.
//!
//! Direct entry constructs the canonical session world synchronously, but most
//! presentation handles resolve asynchronously through Bevy's `AssetServer`.
//! Without a reveal boundary, the window exposes a black/partial world while
//! sprites, LDtk layers, fonts, and music arrive independently. This module
//! keeps gameplay dormant behind an opaque product-owned cover, reports honest
//! asset progress, and reveals only after one complete covered ready frame.

use bevy::asset::{LoadState, UntypedAssetId};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use ambition::actors::features::RoomContentStagingRegistry;
use ambition::actors::ldtk_world::LdtkWorldAssets;
use ambition::actors::rooms::RoomSet;
use ambition::platformer::lifecycle::{InitialGameplayReadiness, SessionRoot};
use ambition::render::ui_fonts::{UiFontWeight, UiFonts};
use ambition::sprite_sheet::game_assets::GameAssets;

use super::world_flow::{
    build_loaded_room_asset_manifest, inspect_room_asset_manifest, RoomAssetManifest,
};
use super::PresentationSetupSet;

#[derive(Component)]
struct DirectStartupLoadingRoot;

#[derive(Component)]
struct DirectStartupLoadingProgressFill;

#[derive(Component)]
struct DirectStartupLoadingProgressText;

#[derive(Component)]
struct DirectStartupLoadingStatusText;

#[derive(Clone, Debug)]
struct StartupAssetDependency {
    label: String,
    asset_id: UntypedAssetId,
}

#[derive(Clone, Debug, Default)]
struct StartupAssetManifest {
    room: RoomAssetManifest,
    supporting: Vec<StartupAssetDependency>,
}

#[derive(Clone, Debug, Default)]
struct StartupReadinessSummary {
    settled: usize,
    total: usize,
    pending: Vec<String>,
    failed: Vec<String>,
}

impl StartupReadinessSummary {
    fn is_ready(&self) -> bool {
        self.pending.is_empty() && self.failed.is_empty()
    }
}

#[derive(Resource, Debug, Default)]
struct DirectStartupLoadingState {
    manifest: Option<StartupAssetManifest>,
    update_serial: u64,
    ready_observed_at: Option<u64>,
    revealed: bool,
}

impl DirectStartupLoadingState {
    /// Returns true exactly when the cover may retire.
    fn observe_readiness(&mut self, ready: bool, failed: bool) -> bool {
        if failed || !ready {
            self.ready_observed_at = None;
            return false;
        }
        match self.ready_observed_at {
            None => {
                self.ready_observed_at = Some(self.update_serial);
                false
            }
            Some(observed_at) => self.update_serial > observed_at,
        }
    }
}

#[derive(SystemParam)]
struct StartupAssetInputs<'w, 's> {
    asset_server: Res<'w, AssetServer>,
    game_assets: Res<'w, GameAssets>,
    room_sets: Query<'w, 's, &'static RoomSet, With<SessionRoot>>,
    content_staging: Res<'w, RoomContentStagingRegistry>,
    ldtk_worlds: Option<Res<'w, LdtkWorldAssets>>,
    ui_fonts: Option<Res<'w, UiFonts>>,
    #[cfg(feature = "audio")]
    audio_library: Option<Res<'w, ambition::audio::library::AudioLibrary>>,
    #[cfg(feature = "audio")]
    music_state: Option<Res<'w, ambition::audio::library::MusicPlaybackState>>,
}

#[derive(SystemParam)]
struct StartupUi<'w, 's> {
    windows: Query<'w, 's, &'static mut Window, With<PrimaryWindow>>,
    roots: Query<'w, 's, Entity, With<DirectStartupLoadingRoot>>,
    progress_fill: Query<
        'w,
        's,
        &'static mut Node,
        With<DirectStartupLoadingProgressFill>,
    >,
    texts: Query<
        'w,
        's,
        (
            &'static mut Text,
            Option<&'static DirectStartupLoadingProgressText>,
            Option<&'static DirectStartupLoadingStatusText>,
        ),
        Or<(
            With<DirectStartupLoadingProgressText>,
            With<DirectStartupLoadingStatusText>,
        )>,
    >,
}

/// Install the product-specific boot reveal used only by the visible direct
/// sandbox. Shell-hosted startup has its own route/load lifecycle, and no-window
/// harnesses deliberately do not need a presentation cover.
pub(super) fn install_direct_startup_loading(app: &mut App) {
    app.init_resource::<DirectStartupLoadingState>().add_systems(
        Startup,
        spawn_direct_startup_loading_screen.after(PresentationSetupSet),
    );
    #[cfg(feature = "audio")]
    app.add_systems(
        Update,
        drive_direct_startup_loading
            .before(ambition::platformer::schedule::GameplaySimulationRoot)
            .before(ambition::audio::library::start_default_music_when_ready),
    );
    #[cfg(not(feature = "audio"))]
    app.add_systems(
        Update,
        drive_direct_startup_loading
            .before(ambition::platformer::schedule::GameplaySimulationRoot),
    );
}

fn spawn_direct_startup_loading_screen(
    mut commands: Commands,
) {
    // The loading surface deliberately uses Bevy's built-in font. Depending on
    // an asynchronously loaded product font would make the loading screen
    // itself pop in late, defeating its immediate-response purpose.
    let font = |size: f32, _weight: UiFontWeight| TextFont {
        font_size: size,
        ..default()
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                padding: UiRect::all(Val::Px(28.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.012, 0.014, 0.024)),
            GlobalZIndex(10_000),
            DirectStartupLoadingRoot,
            Name::new("Ambition Direct Startup Loading Cover"),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("AMBITION"),
                font(54.0, UiFontWeight::Semibold),
                TextColor(Color::srgb(0.92, 0.72, 0.28)),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                Name::new("Ambition Startup Wordmark"),
            ));
            root.spawn((
                Text::new("TANGENT SPACE SANDBOX"),
                font(18.0, UiFontWeight::Regular),
                TextColor(Color::srgb(0.66, 0.70, 0.82)),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                Node {
                    margin: UiRect {
                        top: Val::Px(4.0),
                        ..default()
                    },
                    ..default()
                },
                Name::new("Ambition Startup Subtitle"),
            ));
            root.spawn((
                Node {
                    width: Val::Percent(72.0),
                    min_width: Val::Px(220.0),
                    max_width: Val::Px(720.0),
                    height: Val::Px(12.0),
                    margin: UiRect {
                        top: Val::Px(42.0),
                        ..default()
                    },
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.075, 0.082, 0.12)),
                BorderColor::all(Color::srgb(0.22, 0.24, 0.34)),
                Name::new("Ambition Startup Progress Track"),
            ))
            .with_children(|track| {
                track.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.92, 0.66, 0.20)),
                    DirectStartupLoadingProgressFill,
                    Name::new("Ambition Startup Progress Fill"),
                ));
            });
            root.spawn((
                Text::new("Preparing sandbox…"),
                font(17.0, UiFontWeight::Semibold),
                TextColor(Color::srgb(0.90, 0.91, 0.96)),
                TextLayout::new(Justify::Center, LineBreak::WordBoundary),
                Node {
                    width: Val::Percent(72.0),
                    min_width: Val::Px(220.0),
                    max_width: Val::Px(720.0),
                    margin: UiRect {
                        top: Val::Px(16.0),
                        ..default()
                    },
                    ..default()
                },
                DirectStartupLoadingProgressText,
                Name::new("Ambition Startup Progress Text"),
            ));
            root.spawn((
                Text::new("Building the first coherent frame"),
                font(14.0, UiFontWeight::Regular),
                TextColor(Color::srgb(0.56, 0.60, 0.72)),
                TextLayout::new(Justify::Center, LineBreak::WordBoundary),
                Node {
                    width: Val::Percent(72.0),
                    min_width: Val::Px(220.0),
                    max_width: Val::Px(720.0),
                    min_height: Val::Px(24.0),
                    margin: UiRect {
                        top: Val::Px(6.0),
                        ..default()
                    },
                    ..default()
                },
                DirectStartupLoadingStatusText,
                Name::new("Ambition Startup Status Text"),
            ));
        });
}

fn drive_direct_startup_loading(
    mut commands: Commands,
    mut state: ResMut<DirectStartupLoadingState>,
    mut gameplay: ResMut<InitialGameplayReadiness>,
    mut assets: StartupAssetInputs,
    mut ui: StartupUi,
) {
    if state.revealed {
        return;
    }
    state.update_serial = state.update_serial.saturating_add(1);

    // The OS window is created hidden. Expose it only after Startup has built
    // the opaque cover, eliminating the compositor's uninitialized/ghost frame.
    if !ui.roots.is_empty() {
        for mut window in &mut ui.windows {
            window.visible = true;
        }
    }

    if state.manifest.is_none() {
        match build_startup_manifest(&mut assets) {
            Ok(manifest) => state.manifest = Some(manifest),
            Err(message) => {
                update_startup_ui(&mut ui, 0, 1, "Startup preparation failed", &message);
                return;
            }
        }
    }

    let summary = inspect_startup_manifest(
        &assets.asset_server,
        state
            .manifest
            .as_ref()
            .expect("startup manifest was just initialized"),
    );
    let failed = !summary.failed.is_empty();
    let ready = summary.is_ready();

    let headline = if failed {
        "A required asset failed to load"
    } else if ready {
        "Ready"
    } else {
        "Loading the Ambition sandbox"
    };
    let detail = if failed {
        format!("Failed: {}", summary.failed.join(", "))
    } else if ready {
        "Finalizing the first complete frame…".to_owned()
    } else {
        summary
            .pending
            .first()
            .map(|label| format!("Loading {label}"))
            .unwrap_or_else(|| "Preparing presentation".to_owned())
    };
    update_startup_ui(
        &mut ui,
        summary.settled,
        summary.total.max(1),
        headline,
        &detail,
    );

    if !state.observe_readiness(ready, failed) {
        return;
    }

    // The all-ready state has survived one complete update/render boundary
    // while the opaque cover remained present. Open simulation first, then
    // retire the cover so the first exposed world is also the first live tick.
    gameplay.mark_ready();
    for entity in &ui.roots {
        commands.entity(entity).despawn();
    }
    state.revealed = true;
}

fn build_startup_manifest(
    inputs: &mut StartupAssetInputs<'_, '_>,
) -> Result<StartupAssetManifest, String> {
    let room_set = inputs
        .room_sets
        .single()
        .map_err(|_| "expected exactly one canonical session room set".to_owned())?;
    let room = room_set.active_spec();
    let staged = inputs
        .content_staging
        .try_requests_for(room)
        .map_err(|error| format!("content staging failed: {error}"))?;
    let staged_names = staged
        .iter()
        .map(|request| request.name.clone())
        .collect::<Vec<_>>();
    let room_manifest = build_loaded_room_asset_manifest(room, &staged_names, &inputs.game_assets);

    let mut supporting = Vec::new();
    if let Some(worlds) = inputs.ldtk_worlds.as_deref() {
        for (index, handle) in worlds.0.iter().enumerate() {
            supporting.push(StartupAssetDependency {
                label: format!("world data {}", index + 1),
                asset_id: UntypedAssetId::from(handle),
            });
        }
    }
    if let Some(fonts) = inputs.ui_fonts.as_deref() {
        for (label, handle) in [
            ("dialogue font", fonts.regular.as_ref()),
            ("display font", fonts.semibold.as_ref()),
            ("debug font", fonts.mono.as_ref()),
        ] {
            if let Some(handle) = handle {
                supporting.push(StartupAssetDependency {
                    label: label.to_owned(),
                    asset_id: UntypedAssetId::from(handle),
                });
            }
        }
    }
    #[cfg(feature = "audio")]
    if let (Some(library), Some(music_state)) =
        (inputs.audio_library.as_deref(), inputs.music_state.as_deref())
    {
        if let Some(handle) = library.resolved_track_handle(&music_state.active_track) {
            supporting.push(StartupAssetDependency {
                label: format!("music '{}'", music_state.active_track),
                asset_id: UntypedAssetId::from(&handle),
            });
        }
    }

    // De-duplicate aliases by runtime asset identity while keeping deterministic
    // first-label ordering for the progress display.
    let mut seen = Vec::<UntypedAssetId>::new();
    supporting.retain(|dependency| {
        if seen.iter().any(|asset_id| asset_id == &dependency.asset_id) {
            false
        } else {
            seen.push(dependency.asset_id.clone());
            true
        }
    });

    Ok(StartupAssetManifest {
        room: room_manifest,
        supporting,
    })
}

fn inspect_startup_manifest(
    asset_server: &AssetServer,
    manifest: &StartupAssetManifest,
) -> StartupReadinessSummary {
    let room = inspect_room_asset_manifest(asset_server, &manifest.room);
    let mut summary = StartupReadinessSummary {
        settled: room.settled,
        total: room.total + manifest.supporting.len(),
        pending: room.pending,
        failed: room.failed,
    };
    for dependency in &manifest.supporting {
        if asset_server.is_loaded_with_dependencies(dependency.asset_id.clone()) {
            summary.settled += 1;
            continue;
        }
        match asset_server.load_state(dependency.asset_id.clone()) {
            LoadState::Failed(_) => {
                summary.settled += 1;
                summary.failed.push(dependency.label.clone());
            }
            LoadState::NotLoaded | LoadState::Loading | LoadState::Loaded => {
                summary.pending.push(dependency.label.clone());
            }
        }
    }
    summary
}

fn update_startup_ui(
    ui: &mut StartupUi<'_, '_>,
    settled: usize,
    total: usize,
    headline: &str,
    detail: &str,
) {
    let percent = ((settled as f32 / total.max(1) as f32) * 100.0).clamp(0.0, 100.0);
    for mut node in &mut ui.progress_fill {
        node.width = Val::Percent(percent);
    }
    for (mut text, progress, status) in &mut ui.texts {
        if progress.is_some() {
            text.0 = format!("{headline}  ·  {settled}/{total}");
        } else if status.is_some() {
            text.0 = detail.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_assets_must_survive_a_later_update_before_reveal() {
        let mut state = DirectStartupLoadingState::default();
        state.update_serial = 4;
        assert!(!state.observe_readiness(true, false));
        state.update_serial = 5;
        assert!(state.observe_readiness(true, false));
    }

    #[test]
    fn a_pending_or_failed_asset_resets_the_ready_latch() {
        let mut state = DirectStartupLoadingState::default();
        state.update_serial = 4;
        assert!(!state.observe_readiness(true, false));
        state.update_serial = 5;
        assert!(!state.observe_readiness(false, false));
        state.update_serial = 6;
        assert!(!state.observe_readiness(true, false));
        state.update_serial = 7;
        assert!(!state.observe_readiness(true, true));
    }
}
