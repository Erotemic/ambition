//! Coherent first reveal for the visible direct-entry sandbox.
//!
//! Direct entry constructs the canonical session world synchronously, but most
//! presentation handles resolve asynchronously through Bevy's `AssetServer`.
//! Without a reveal boundary, the window exposes a black/partial world while
//! sprites, LDtk layers, fonts, and music arrive independently. This module
//! keeps gameplay dormant behind an opaque product-owned cover, reports honest
//! asset progress, then opens gameplay UNDER the cover and reveals only when
//! the room is drawn: its first tick has run, and nothing of it is still
//! coming in (`PresentationSettling`).

use bevy::asset::{LoadState, UntypedAssetId};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use ambition_platformer2d::publication::construction::RoomContentStagingRegistry;
use ambition_platformer2d::content::hole;
use ambition_platformer2d::ldtk_map::LdtkWorldAssets;
use ambition_platformer2d::platformer::lifecycle::{InitialGameplayReadiness, SessionRoot};
use ambition_platformer2d::render::ui_fonts::{UiFontWeight, UiFonts};
use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;
use ambition_platformer2d::world::rooms::RoomSet;

use super::world_flow::{
    build_loaded_room_asset_manifest, inspect_room_asset_manifest, RoomAssetManifest,
};
use super::PresentationSetupSet;

/// The start cover's colours, as the host's pack states them
/// (`data/cover_look.ron`). Each is red, green and blue, from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverLook {
    pub backdrop: [f32; 3],
    pub wordmark: [f32; 3],
    pub subtitle: [f32; 3],
    /// The progress bar: its track, the track's edge and the part that is done.
    pub track: [f32; 3],
    pub track_edge: [f32; 3],
    pub fill: [f32; 3],
    /// The line that says what is being prepared.
    pub progress_ink: [f32; 3],
    /// The line under it, about each thing that loads.
    pub detail_ink: [f32; 3],
}

impl CoverLook {
    fn problems(&self) -> Vec<String> {
        [
            ("backdrop", self.backdrop),
            ("wordmark", self.wordmark),
            ("subtitle", self.subtitle),
            ("track", self.track),
            ("track_edge", self.track_edge),
            ("fill", self.fill),
            ("progress_ink", self.progress_ink),
            ("detail_ink", self.detail_ink),
        ]
        .into_iter()
        .filter(|(_, colour)| colour.iter().any(|part| !(0.0..=1.0).contains(part)))
        .map(|(field, colour)| format!("`{field}` is {colour:?}; each part is from 0 to 1"))
        .collect()
    }
}

fn rgb([r, g, b]: [f32; 3]) -> Color {
    Color::srgb(r, g, b)
}

/// The authored FILE kind: the start cover's colours.
pub const COVER_LOOK_SCHEMA: &str = "cover_look";

/// The schema, for the host pack's `content_pack! { schemas: [..] }`.
pub fn cover_look_schema() -> ambition_platformer2d::content::SchemaRegistration {
    ambition_platformer2d::content::rules_schema(
        COVER_LOOK_SCHEMA,
        1,
        "ambition_host",
        "The colours of the cover the host shows while the first frame is prepared.",
        CoverLook::problems,
    )
}

/// The cover's colours the host's pack states.
pub fn cover_look() -> CoverLook {
    *ambition_platformer2d::content::lowered_rules::<CoverLook>(super::host_pack::HOST_PACK.prepared(), COVER_LOOK_SCHEMA)
        .expect("the host's pack states the cover's colours (`data/cover_look.ron`)")
}

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
    /// What the room manifest was built from, so it can be REBUILT as the
    /// per-frame ration realizes sheets whose pages the first build never saw
    /// — the same gap the room transition closed (`inspect_demanded_characters`).
    room_spec: Option<std::sync::Arc<ambition_platformer2d::world::rooms::RoomSpec>>,
    staged_names: Vec<String>,
    demanded_characters: Vec<String>,
    realized_at_build: usize,
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
    /// The sim tick when gameplay opened under the cover: the reveal waits for
    /// a later one, so what the first tick spawns is drawn under the cover.
    opened_at_tick: Option<u64>,
    opened_at: Option<std::time::Duration>,
    presented: ambition_platformer2d::render::rendering::presentation_settling::SettledLatch,
    revealed: bool,
    /// Updates spent with every pending asset decoded and only its GPU upload
    /// owed — the upload half of the boot, measured under the cover.
    waited_on_gpu_updates: u32,
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
    images: Res<'w, Assets<Image>>,
    /// This App's render world, for the GPU-upload readiness term. Absent means
    /// none — the resource is per-App on purpose.
    render_world:
        Option<Res<'w, ambition_platformer2d::asset_manager::image_stages::RenderWorldPresent>>,
    /// And what it has uploaded. Per-App for the same reason: asset ids are
    /// local to an App, so a process-wide "prepared" set let a sibling App's
    /// upload settle this one's startup readiness.
    prepared_here:
        Option<Res<'w, ambition_platformer2d::asset_manager::image_stages::AppGpuPreparedImages>>,
    game_assets: ResMut<'w, GameAssets>,
    asset_catalog:
        Res<'w, ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog>,
    layouts: ResMut<'w, Assets<TextureAtlasLayout>>,
    quality: Res<'w, ambition_platformer2d::render::quality::ResolvedVisualQuality>,
    boss_catalog: Option<Res<'w, ambition_platformer2d::boss_encounter::BossCatalog>>,
    worn: Query<
        'w,
        's,
        &'static ambition_platformer2d::characters::actor::WornCharacter,
        Or<(
            With<ambition_platformer2d::platformer::markers::PlayerEntity>,
            With<ambition_platformer2d::characters::control::DrivingParticipant>,
        )>,
    >,
    room_sets: Query<'w, 's, &'static RoomSet, With<SessionRoot>>,
    content_staging: Res<'w, RoomContentStagingRegistry>,
    character_load_states:
        ResMut<'w, ambition_platformer2d::assets::materialization::CharacterLoadStates>,
    /// Sheets this app's providers authored — the other real source
    /// of sheet metadata, and the only one a game outside this workspace can
    /// write to.
    authored_sheets:
        Res<'w, ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets>,
    /// Registered character definitions: a real source of sheets, since a
    /// character may be declared only through `register_character`.
    prepared_characters: ambition_platformer2d::characters::prepared::SessionCast<'w>,
    ldtk_worlds: Option<Res<'w, LdtkWorldAssets>>,
    ui_fonts: Option<Res<'w, UiFonts>>,
    #[cfg(feature = "audio")]
    audio_library: Option<Res<'w, ambition_platformer2d::audio::library::AudioLibrary>>,
    #[cfg(feature = "audio")]
    music_state: Option<Res<'w, ambition_platformer2d::audio::library::MusicPlaybackState>>,
    pipelines:
        Option<Res<'w, ambition_platformer2d::render::rendering::presentation_settling::RenderPipelinesWaiting>>,
    tick: Option<Res<'w, ambition_platformer2d::time::SimTick>>,
    time: Res<'w, Time<Real>>,
}

#[derive(SystemParam)]
struct StartupUi<'w, 's> {
    windows: Query<'w, 's, &'static mut Window, With<PrimaryWindow>>,
    roots: Query<'w, 's, Entity, With<DirectStartupLoadingRoot>>,
    progress_fill: Query<'w, 's, &'static mut Node, With<DirectStartupLoadingProgressFill>>,
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
    app.init_resource::<DirectStartupLoadingState>()
        .add_systems(
            Startup,
            spawn_direct_startup_loading_screen.after(PresentationSetupSet),
        );
    #[cfg(feature = "audio")]
    app.add_systems(
        Update,
        drive_direct_startup_loading
            .before(ambition_platformer2d::platformer::schedule::GameplaySimulationRoot)
            .before(ambition_platformer2d::audio::library::DefaultMusicStart),
    );
    app.add_systems(
        Update,
        reveal_direct_startup_room
            .after(ambition_platformer2d::render::rendering::presentation_settling::PresentationSettlingSet),
    );
    #[cfg(not(feature = "audio"))]
    app.add_systems(
        Update,
        drive_direct_startup_loading
            .before(ambition_platformer2d::platformer::schedule::GameplaySimulationRoot),
    );
}

fn spawn_direct_startup_loading_screen(mut commands: Commands) {
    let text = super::host_pack::host_text();
    let look = cover_look();
    // The loading surface deliberately uses Bevy's built-in font. Depending on
    // an asynchronously loaded product font would make the loading screen
    // itself pop in late, defeating its immediate-response purpose.
    let font = |size: f32, _weight: UiFontWeight| TextFont {
        font_size: FontSize::Px(size),
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
            BackgroundColor(rgb(look.backdrop)),
            GlobalZIndex(10_000),
            DirectStartupLoadingRoot,
            ambition_platformer2d::render::rendering::presentation_settling::PresentationCover,
            Name::new("Ambition Direct Startup Loading Cover"),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(text.line("loading.title")),
                font(54.0, UiFontWeight::Semibold),
                TextColor(rgb(look.wordmark)),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                Name::new("Ambition Startup Wordmark"),
            ));
            root.spawn((
                Text::new(text.line("loading.subtitle")),
                font(18.0, UiFontWeight::Regular),
                TextColor(rgb(look.subtitle)),
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
                BackgroundColor(rgb(look.track)),
                BorderColor::all(rgb(look.track_edge)),
                Name::new("Ambition Startup Progress Track"),
            ))
            .with_children(|track| {
                track.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(rgb(look.fill)),
                    DirectStartupLoadingProgressFill,
                    Name::new("Ambition Startup Progress Fill"),
                ));
            });
            root.spawn((
                Text::new(text.line("loading.preparing")),
                font(17.0, UiFontWeight::Semibold),
                TextColor(rgb(look.progress_ink)),
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
                Text::new(text.line("loading.building")),
                font(14.0, UiFontWeight::Regular),
                TextColor(rgb(look.detail_ink)),
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

fn drive_direct_startup_loading(sprite_content: bevy::prelude::Res<ambition_platformer2d::sprite_sheet::SpriteContent>,
    mut state: ResMut<DirectStartupLoadingState>,
    mut gameplay: ResMut<InitialGameplayReadiness>,
    mut assets: StartupAssetInputs,
    mut ui: StartupUi,
) {
    let text = super::host_pack::host_text();
    if state.revealed || state.opened_at_tick.is_some() {
        return;
    }
    state.update_serial = state.update_serial.saturating_add(1);

    // The OS window is created hidden. Expose it only once the opaque cover has
    // been drawn: after a rendered frame, with no pipeline still compiling (the
    // cover's own is among them), or after a short wait. Before, the window
    // showed the room for the frames the cover's pipeline compiled in.
    let pipelines_ready = assets.pipelines.as_ref().is_none_or(|pipelines| pipelines.count() == 0);
    if !ui.roots.is_empty() && state.update_serial >= 2 && (pipelines_ready || state.update_serial > 30) {
        for mut window in &mut ui.windows {
            if !window.visible {
                window.visible = true;
            }
        }
    }

    if state.manifest.is_none() {
        match build_startup_manifest(&sprite_content, &mut assets) {
            Ok(manifest) => state.manifest = Some(manifest),
            Err(message) => {
                update_startup_ui(&mut ui, text, 0, 1, text.line("loading.preparation_failed"), &message);
                return;
            }
        }
    }

    let summary = inspect_startup_manifest(
        &assets.asset_server,
        &assets.images,
        ambition_platformer2d::asset_manager::image_stages::RenderWorldPresent::from_option(
            assets.render_world.as_deref(),
        ),
        assets.prepared_here.as_deref(),
        &assets.game_assets,
        &assets.character_load_states,
        assets.boss_catalog.as_deref(),
        state
            .manifest
            .as_mut()
            .expect("startup manifest was just initialized"),
    );
    let failed = !summary.failed.is_empty();
    let ready = summary.is_ready();

    let headline = if failed {
        text.line("loading.asset_failed")
    } else if ready {
        text.line("loading.ready")
    } else {
        text.line("loading.loading")
    };
    let detail = if failed {
        text.fill("loading.failed_list", &[hole("assets", &summary.failed.join(", "))])
    } else if ready {
        text.line("loading.finalizing").to_owned()
    } else {
        summary
            .pending
            .first()
            .map(|label| text.fill("loading.item", &[("label", label)]))
            .unwrap_or_else(|| text.line("loading.presentation").to_owned())
    };
    update_startup_ui(
        &mut ui,
        text,
        summary.settled,
        summary.total.max(1),
        headline,
        &detail,
    );

    if !state.observe_readiness(ready, failed) {
        // The last stage the cover is still waiting on, so a log can tell a
        // decode wait from an upload wait. `(gpu upload)` labels come from
        // `inspect_room_asset_manifest`'s render-world term.
        if let Some(first) = summary.pending.first() {
            if summary
                .pending
                .iter()
                .all(|label| label.ends_with("(gpu upload)"))
            {
                state.waited_on_gpu_updates = state.waited_on_gpu_updates.saturating_add(1);
            }
            bevy::log::debug!(
                target: "ambition_app::startup_cover",
                "startup cover waiting: {} pending, first `{first}`",
                summary.pending.len()
            );
        }
        return;
    }

    // Open the simulation under the cover: what its first tick spawns (the
    // player, the views of the room's features) is drawn before the reveal
    // (`reveal_direct_startup_room`).
    eprintln!(
        "[startup-cover] assets ready after {} updates ({} of them waiting only on GPU uploads), {} assets",
        state.update_serial, state.waited_on_gpu_updates, summary.total
    );
    gameplay.mark_ready();
    state.opened_at_tick = Some(assets.tick.as_deref().map_or(0, |tick| tick.get()));
    state.opened_at = Some(assets.time.elapsed());
}

/// How long the cover waits, after gameplay opened, for the room to be drawn.
/// Not a budget: past it, showing the room as it is beats a black screen, and
/// the warning names what was still coming.
const PRESENTATION_DEADLINE: std::time::Duration = std::time::Duration::from_secs(8);

/// Retire the cover once the room is drawn under it: a sim tick has run since
/// gameplay opened, and nothing of the room is still coming in.
fn reveal_direct_startup_room(
    mut commands: Commands,
    mut state: ResMut<DirectStartupLoadingState>,
    settling: Option<Res<ambition_platformer2d::render::rendering::presentation_settling::PresentationSettling>>,
    tick: Option<Res<ambition_platformer2d::time::SimTick>>,
    time: Res<Time<Real>>,
    roots: Query<Entity, With<DirectStartupLoadingRoot>>,
) {
    let (Some(opened_tick), Some(opened_at)) = (state.opened_at_tick, state.opened_at) else {
        return;
    };
    if state.revealed {
        return;
    }
    state.update_serial = state.update_serial.saturating_add(1);
    let ticked = tick.is_none_or(|tick| tick.get() > opened_tick);
    let settled = settling.as_deref().is_none_or(|settling| settling.is_empty());
    let update = state.update_serial;
    let presented = state.presented.observe(update, ticked && settled);
    let waited = time.elapsed().saturating_sub(opened_at);
    if !presented {
        if waited < PRESENTATION_DEADLINE {
            return;
        }
        bevy::log::warn!(
            target: "ambition_app::startup_cover",
            "startup cover revealed after {:.0} ms with the room still coming in (ticked={ticked}): {:?}",
            waited.as_secs_f64() * 1000.0,
            settling.as_deref().map(|settling| settling.pending()).unwrap_or_default(),
        );
    }
    eprintln!("[startup-cover] revealed {:.0} ms after gameplay opened", waited.as_secs_f64() * 1000.0);
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    state.revealed = true;
}

fn build_startup_manifest(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent,
    inputs: &mut StartupAssetInputs<'_, '_>,
) -> Result<StartupAssetManifest, String> {
    let room_set = inputs
        .room_sets
        .single()
        .map_err(|_| "expected exactly one canonical session room set".to_owned())?;
    // Startup builds the room a session activates into.
    let room = room_set.activation_spec();
    let staged = inputs
        .content_staging
        .try_requests_for(room)
        .map_err(|error| format!("content staging failed: {error}"))?;
    let mut staged_names = staged
        .iter()
        .map(|request| request.name.clone())
        .collect::<Vec<_>>();
    // Deferred character sheets for the FIRST room materialize here, behind
    // the startup cover — the same barrier semantics room transitions get.
    // Sheets beyond the per-frame ration go to the global demand, exactly as
    // a room transition hands them over (see `demand_room_character_sheets`).
    let worn: Vec<String> = inputs
        .worn
        .iter()
        .map(|worn| worn.0.as_str().to_string())
        .collect();
    // The worn characters join the staged list so ONE list demands them,
    // waits for their realization and puts their pages in the manifest — the
    // room's placements never name the player, and a body's own demand
    // (`demand_worn_character_sheets`) realizes a sheet the manifest would
    // otherwise never list, so the cover could lift on its page still loading.
    staged_names.extend(worn.iter().cloned());
    staged_names.sort();
    staged_names.dedup();
    super::world_flow::demand_room_character_sheets(&sprite_content,
        room,
        &staged_names,
        &mut inputs.game_assets,
        &inputs.asset_catalog,
        &inputs.asset_server,
        &mut inputs.layouts,
        &inputs.quality,
        &mut inputs.character_load_states,
        inputs
            .prepared_characters
            .get()
            .unwrap_or(&Default::default()),
        &inputs.authored_sheets,
        &worn,
        // A fresh process has nothing an earlier room realized; the first
        // room commit (`RoomResidencyOwners`) is where ownership is applied.
        None,
    );
    // A first room that authors a boss demands the dedicated boss sheets now,
    // behind the cover; no room without one pays for them (asset open work 2).
    if !room.boss_spawns.is_empty() {
        if let Some(boss_catalog) = inputs.boss_catalog.as_deref() {
            let keys = ambition_platformer2d::game_assets::boss_sheet_keys_for_room(
                room,
                boss_catalog,
            );
            ambition_platformer2d::game_assets::ensure_boss_sheets_loaded(&sprite_content,
                &mut inputs.game_assets,
                boss_catalog,
                Some(&keys),
                &inputs.asset_catalog,
                &inputs.asset_server,
                &mut inputs.layouts,
                Some(&inputs.quality.budget),
            );
        }
    }
    let room_manifest = build_loaded_room_asset_manifest(room, &staged_names, &inputs.game_assets, inputs.boss_catalog.as_deref());
    let demanded_characters = super::world_flow::room_character_tokens(room, &staged_names);
    let realized_at_build =
        super::world_flow::realized_character_count(&demanded_characters, &inputs.game_assets);
    let room_spec = Some(std::sync::Arc::new(room.clone()));

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
    if let (Some(library), Some(music_state)) = (
        inputs.audio_library.as_deref(),
        inputs.music_state.as_deref(),
    ) {
        if let Some(handle) = library.resolved_track_handle(music_state.active_track()) {
            supporting.push(StartupAssetDependency {
                label: format!("music '{}'", music_state.active_track()),
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
        room_spec,
        staged_names,
        demanded_characters,
        realized_at_build,
    })
}

fn inspect_startup_manifest(
    asset_server: &AssetServer,
    images: &Assets<Image>,
    render_world: ambition_platformer2d::asset_manager::image_stages::RenderWorldPresent,
    prepared_here: Option<
        &ambition_platformer2d::asset_manager::image_stages::AppGpuPreparedImages,
    >,
    game_assets: &GameAssets,
    character_load_states: &ambition_platformer2d::assets::materialization::CharacterLoadStates,
    boss_catalog: Option<&ambition_platformer2d::boss_encounter::BossCatalog>,
    manifest: &mut StartupAssetManifest,
) -> StartupReadinessSummary {
    // Sheets realize one per frame after the first build; each brings pages
    // the first manifest never saw. Rebuild the description when the count moved.
    if let Some(room_spec) = manifest.room_spec.clone() {
        let realized =
            super::world_flow::realized_character_count(&manifest.demanded_characters, game_assets);
        if realized != manifest.realized_at_build {
            manifest.room =
                build_loaded_room_asset_manifest(&room_spec, &manifest.staged_names, game_assets, boss_catalog);
            manifest.realized_at_build = realized;
        }
    }
    let mut room = inspect_room_asset_manifest(
        asset_server,
        Some(images),
        render_world,
        prepared_here,
        &manifest.room,
    );
    super::world_flow::inspect_demanded_characters(
        &manifest.demanded_characters,
        game_assets,
        Some(character_load_states),
        &mut room,
    );
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
    text: &ambition_platformer2d::content::ShownText,
    settled: usize,
    total: usize,
    headline: &str,
    detail: &str,
) {
    let percent = ((settled as f32 / total.max(1) as f32) * 100.0).clamp(0.0, 100.0);
    for mut node in &mut ui.progress_fill {
        node.width = Val::Percent(percent);
    }
    let progress_line = text.fill("loading.progress", &[hole("headline", &headline), hole("settled", &settled), hole("total", &total)]);
    for (mut shown, progress, status) in &mut ui.texts {
        if progress.is_some() {
            shown.0 = progress_line.clone();
        } else if status.is_some() {
            shown.0 = detail.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cover's words are the host's text file's (`host_text.ron`).
    #[test]
    fn the_cover_says_what_the_host_file_states() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.run_system_once(spawn_direct_startup_loading_screen).expect("the cover spawns");
        let text = super::super::host_pack::host_text();
        let shown = |world: &mut World, name: &str| {
            let mut texts = world.query::<(&Name, &Text)>();
            texts
                .iter(world)
                .find(|(each, _)| each.as_str() == name)
                .map(|(_, text)| text.0.clone())
                .unwrap_or_else(|| panic!("the cover has no `{name}`"))
        };
        assert_eq!(shown(&mut world, "Ambition Startup Wordmark"), text.line("loading.title"));
        assert_eq!(shown(&mut world, "Ambition Startup Subtitle"), text.line("loading.subtitle"));
    }

    /// The cover's colours are the host's `cover_look.ron`.
    #[test]
    fn the_cover_wears_what_the_host_file_states() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.run_system_once(spawn_direct_startup_loading_screen).expect("the cover spawns");
        let look = cover_look();
        let mut roots = world.query_filtered::<&BackgroundColor, With<DirectStartupLoadingRoot>>();
        assert_eq!(roots.single(&world).unwrap().0, rgb(look.backdrop), "the backdrop");
        let mut fills = world.query_filtered::<&BackgroundColor, With<DirectStartupLoadingProgressFill>>();
        assert_eq!(fills.single(&world).unwrap().0, rgb(look.fill), "the progress fill");
        let mut inks = world.query_filtered::<&TextColor, With<DirectStartupLoadingProgressText>>();
        assert_eq!(inks.single(&world).unwrap().0, rgb(look.progress_ink), "the progress line");
    }

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
