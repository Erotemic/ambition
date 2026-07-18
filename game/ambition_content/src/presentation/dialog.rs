//! Ambition's product-specific dialogue presentation.
//!
//! The engine publishes [`DialogView`] and shared input markers; this module
//! owns the actual game's visual language: a classic opaque top-of-screen
//! panel, speaker portrait/nameplate, readable body text, a bounded choice
//! viewport, and a contained input hint. Default portrait products are
//! referenced by stable character id through the assembled [`CharacterCatalog`].
//! [`AmbitionDialogPortraitCatalog`] remains a game-owned presentation override
//! layer; speakers without portrait art get a deterministic monogram placeholder.
//!
//! Long option lists are windowed around the single authoritative selected row.
//! Keyboard, physical gamepad, touch joystick/buttons, mouse wheel, touch drag,
//! and direct pointer presses therefore all move the same selection and the
//! presentation scrolls it into view; there is no second UI-only scroll cursor.

use std::collections::BTreeMap;
use std::ops::Range;

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_render::dialog_ui::{
    claim_dialog_presentation, DialogOverlayRoot, DialogPresentationSet,
};
use ambition_render::ui_fonts::{UiFontWeight, UiFonts};
use ambition_sim_view::DialogView;
use ambition_ui_nav::{visible_window_start, DialogChoiceSlot};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const CONTINUE_HINT: &str = "Confirm / Interact: continue";
const CHOICE_HINT: &str =
    "Choose: arrows, stick, swipe, or wheel    Confirm: select    Back: close";
const PRESENTER_NAME: &str = "ambition_content::AmbitionDialogUiPlugin";
const DEFAULT_VIEWPORT: Vec2 = Vec2::new(1280.0, 720.0);

/// Optional game-owned portrait override for one stable character id.
///
/// The ordinary image comes from the character catalog's generated portrait
/// product. An override replaces it; `image_path = None` deliberately forces a
/// placeholder for this game's presentation.
#[derive(Clone, Debug)]
pub struct AmbitionDialogPortraitSpec {
    pub image_path: Option<String>,
    pub placeholder_text: Option<String>,
    pub accent: Color,
}

impl AmbitionDialogPortraitSpec {
    pub fn placeholder(accent: Color) -> Self {
        Self {
            image_path: None,
            placeholder_text: None,
            accent,
        }
    }

    pub fn image(path: impl Into<String>, accent: Color) -> Self {
        Self {
            image_path: Some(path.into()),
            placeholder_text: None,
            accent,
        }
    }

    pub fn with_placeholder_text(mut self, text: impl Into<String>) -> Self {
        self.placeholder_text = Some(text.into());
        self
    }
}

/// Ambition's portrait registry, keyed by stable character id rather than a
/// localized display label. The renderer resolves the current Yarn speaker
/// label through the assembled [`CharacterCatalog`] before consulting it.
#[derive(Resource, Clone, Debug, Default)]
pub struct AmbitionDialogPortraitCatalog {
    entries: BTreeMap<String, AmbitionDialogPortraitSpec>,
}

impl AmbitionDialogPortraitCatalog {
    pub fn insert(
        &mut self,
        character_id: impl Into<String>,
        spec: AmbitionDialogPortraitSpec,
    ) -> Option<AmbitionDialogPortraitSpec> {
        self.entries.insert(character_id.into(), spec)
    }

    pub fn get(&self, character_id: &str) -> Option<&AmbitionDialogPortraitSpec> {
        self.entries.get(character_id)
    }
}

/// Ambition's concrete presenter. Other games should install the engine's
/// `DefaultDialogUiPlugin` or their own plugin claiming the same one-presenter
/// seam.
pub struct AmbitionDialogUiPlugin;

impl Plugin for AmbitionDialogUiPlugin {
    fn build(&self, app: &mut App) {
        claim_dialog_presentation(app, PRESENTER_NAME);
        app.init_resource::<AmbitionDialogPortraitCatalog>()
            .init_resource::<AmbitionDialogLayoutCache>()
            .add_systems(
                Update,
                sync_ambition_dialog_ui.in_set(DialogPresentationSet),
            );
    }
}

/// Ambition-specific root marker layered beside the engine's generic
/// [`DialogOverlayRoot`].
#[derive(Component)]
pub struct AmbitionDialogOverlayRoot;

#[derive(Component)]
pub struct AmbitionDialogPanel;

#[derive(Component)]
pub struct AmbitionDialogPortraitFrame;

#[derive(Component)]
pub struct AmbitionDialogPortraitMonogram;

#[derive(Component)]
pub struct AmbitionDialogPortraitImage;

#[derive(Component)]
pub struct AmbitionDialogContinueHint;

/// The clipped/windowed options area. Rows carry absolute [`DialogChoiceSlot`]
/// indices even when only a subset is visible.
#[derive(Component)]
pub struct AmbitionDialogChoiceViewport;

#[derive(Component)]
pub struct AmbitionDialogScrollIndicator;

#[derive(Component)]
pub struct AmbitionDialogScrollTrack;

#[derive(Component)]
pub struct AmbitionDialogScrollThumb;

#[derive(Resource, Default)]
struct AmbitionDialogLayoutCache {
    viewport: Option<Vec2>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DialogLayoutProfile {
    option_capacity: usize,
    panel_max_width: f32,
    panel_max_height_percent: f32,
    panel_top_inset: f32,
    panel_padding: f32,
    panel_gap: f32,
    portrait_width: f32,
    portrait_height: f32,
    speaker_font_size: f32,
    body_font_size: f32,
    choice_font_size: f32,
    hint_font_size: f32,
    choice_min_height: f32,
}

impl DialogLayoutProfile {
    fn for_viewport(viewport: Vec2) -> Self {
        if viewport.y < 480.0 {
            // Phone landscape / short embedded windows. Two large rows remain
            // readable while the authoritative selection scrolls the window.
            Self {
                option_capacity: 2,
                panel_max_width: 940.0,
                panel_max_height_percent: 62.0,
                panel_top_inset: 58.0,
                panel_padding: 10.0,
                panel_gap: 7.0,
                portrait_width: 68.0,
                portrait_height: 76.0,
                speaker_font_size: 24.0,
                body_font_size: 20.0,
                choice_font_size: 19.0,
                hint_font_size: 14.0,
                choice_min_height: 44.0,
            }
        } else if viewport.x < 720.0 {
            // Phone portrait / small tablet. Text never drops below the mobile
            // readability floor, and three choices fit without crowding.
            Self {
                option_capacity: 3,
                panel_max_width: 680.0,
                panel_max_height_percent: 58.0,
                panel_top_inset: 64.0,
                panel_padding: 12.0,
                panel_gap: 9.0,
                portrait_width: 82.0,
                portrait_height: 94.0,
                speaker_font_size: 27.0,
                body_font_size: 22.0,
                choice_font_size: 20.0,
                hint_font_size: 15.0,
                choice_min_height: 50.0,
            }
        } else {
            Self {
                option_capacity: 5,
                panel_max_width: 980.0,
                panel_max_height_percent: 52.0,
                panel_top_inset: 24.0,
                panel_padding: 16.0,
                panel_gap: 10.0,
                portrait_width: 104.0,
                portrait_height: 120.0,
                speaker_font_size: 29.0,
                body_font_size: 23.0,
                choice_font_size: 20.0,
                hint_font_size: 15.0,
                choice_min_height: 50.0,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DialogChoiceWindow {
    start: usize,
    end: usize,
    total: usize,
    capacity: usize,
}

impl DialogChoiceWindow {
    fn new(selected: usize, total: usize, capacity: usize) -> Self {
        let start = visible_window_start(selected, total, capacity);
        Self {
            start,
            end: (start + capacity).min(total),
            total,
            capacity,
        }
    }

    fn range(self) -> Range<usize> {
        self.start..self.end
    }

    fn is_windowed(self) -> bool {
        self.capacity > 0 && self.total > self.capacity
    }

    fn has_before(self) -> bool {
        self.start > 0
    }

    fn has_after(self) -> bool {
        self.end < self.total
    }

    fn position_label(self) -> String {
        format!("Choices {}–{} of {}", self.start + 1, self.end, self.total)
    }

    fn scrollbar(self) -> Option<(f32, f32)> {
        if !self.is_windowed() {
            return None;
        }
        let thumb_height = (self.capacity as f32 / self.total as f32 * 100.0).clamp(18.0, 100.0);
        let max_start = self.total.saturating_sub(self.capacity);
        let top = if max_start == 0 {
            0.0
        } else {
            self.start as f32 / max_start as f32 * (100.0 - thumb_height)
        };
        Some((top, thumb_height))
    }
}

fn sync_ambition_dialog_ui(
    mut commands: Commands,
    dialogue: Res<DialogView>,
    overlays: Query<Entity, With<DialogOverlayRoot>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut layout_cache: ResMut<AmbitionDialogLayoutCache>,
    ui_fonts: Option<Res<UiFonts>>,
    character_catalog: Option<Res<CharacterCatalog>>,
    portrait_catalog: Res<AmbitionDialogPortraitCatalog>,
    asset_server: Option<Res<AssetServer>>,
) {
    let viewport = windows
        .single()
        .ok()
        .map(|window| Vec2::new(window.width(), window.height()))
        .unwrap_or(DEFAULT_VIEWPORT);
    let layout_changed = layout_cache.viewport != Some(viewport);
    if layout_changed {
        layout_cache.viewport = Some(viewport);
    }

    let presentation_inputs_changed = dialogue.is_changed()
        || layout_changed
        || ui_fonts.as_ref().is_some_and(|fonts| fonts.is_changed())
        || character_catalog
            .as_ref()
            .is_some_and(|catalog| catalog.is_changed())
        || portrait_catalog.is_changed();
    if !presentation_inputs_changed {
        return;
    }

    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }
    if !dialogue.active {
        return;
    }

    let profile = DialogLayoutProfile::for_viewport(viewport);
    let choice_window = DialogChoiceWindow::new(
        dialogue.selected_option,
        dialogue.option_labels.len(),
        profile.option_capacity,
    );
    let speaker_label = if dialogue.speaker_label.trim().is_empty() {
        "Unknown speaker"
    } else {
        dialogue.speaker_label.trim()
    };
    let character_id = character_catalog
        .as_deref()
        .and_then(|catalog| catalog.id_for_display_name(speaker_label));
    let portrait_override = character_id.and_then(|id| portrait_catalog.get(id));
    let portrait_key = character_id.unwrap_or(speaker_label);
    let accent = portrait_override
        .map(|portrait| portrait.accent.clone())
        .unwrap_or_else(|| placeholder_accent(portrait_key));
    let monogram = portrait_override
        .and_then(|portrait| portrait.placeholder_text.as_deref())
        .map(str::to_owned)
        .unwrap_or_else(|| placeholder_monogram(speaker_label));
    let portrait_image_path = resolve_portrait_image_path(
        character_id,
        character_catalog.as_deref(),
        &portrait_catalog,
    );
    // Portrait products currently publish a one-frame default sheet, so the
    // image page can be shown directly. Named clip playback can consume the
    // sibling manifest later without changing this UI ownership seam.
    let portrait_image = portrait_image_path
        .zip(asset_server.as_deref())
        .map(|(path, server)| server.load::<Image>(path));

    let selected_marker = ui_fonts
        .as_deref()
        .map(UiFonts::selected_marker)
        .unwrap_or(">");
    let dialog_font = |font_size: f32, weight: UiFontWeight| {
        ui_fonts
            .as_deref()
            .map(|fonts| fonts.text_font(font_size, weight))
            .unwrap_or(TextFont {
                font_size,
                ..default()
            })
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                padding: UiRect {
                    left: Val::Px(10.0),
                    right: Val::Px(10.0),
                    top: Val::Px(profile.panel_top_inset),
                    bottom: Val::Px(10.0),
                },
                // The root is a column so the main axis is vertical:
                // `FlexStart` anchors the panel near the top, while the cross-axis
                // `Center` actually centers it horizontally. With Bevy's default
                // row direction these same values left-align the panel and center
                // it vertically.
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                ..default()
            },
            ZIndex(45),
            DialogOverlayRoot,
            AmbitionDialogOverlayRoot,
            Name::new("Ambition Dialogue Overlay Root"),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(96.0),
                    min_width: Val::Px(0.0),
                    max_width: Val::Px(profile.panel_max_width),
                    max_height: Val::Percent(profile.panel_max_height_percent),
                    padding: UiRect::all(Val::Px(profile.panel_padding)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(profile.panel_gap),
                    align_items: AlignItems::Stretch,
                    border: UiRect::all(Val::Px(4.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                // Fully opaque by product policy: the world never competes with
                // dialogue text or controls for contrast.
                BackgroundColor(Color::srgb(0.035, 0.025, 0.045)),
                BorderColor::all(Color::srgb(0.86, 0.67, 0.25)),
                AmbitionDialogPanel,
                Name::new("Ambition Dialogue Panel"),
            ))
            .with_children(|panel| {
                spawn_dialog_header(
                    panel,
                    portrait_image,
                    &monogram,
                    accent,
                    speaker_label,
                    &profile,
                    &dialog_font,
                );

                panel.spawn((
                    Text::new(dialogue.body.clone()),
                    dialog_font(profile.body_font_size, UiFontWeight::Regular),
                    TextColor(Color::srgb(0.98, 0.96, 0.90)),
                    TextLayout::new(Justify::Left, LineBreak::WordBoundary),
                    Node {
                        width: Val::Percent(100.0),
                        min_width: Val::Px(0.0),
                        min_height: Val::Px(48.0),
                        flex_shrink: 1.0,
                        ..default()
                    },
                    Name::new("Dialogue Body"),
                ));

                if !dialogue.option_labels.is_empty() {
                    spawn_choice_viewport(
                        panel,
                        &dialogue,
                        choice_window,
                        selected_marker,
                        &profile,
                        &dialog_font,
                    );
                }

                let hint = if dialogue.option_labels.is_empty() {
                    CONTINUE_HINT
                } else {
                    CHOICE_HINT
                };
                panel
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            min_height: Val::Px(24.0),
                            justify_content: JustifyContent::FlexEnd,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Name::new("Dialogue Input Hint"),
                    ))
                    .with_children(|footer| {
                        footer.spawn((
                            Text::new(hint),
                            dialog_font(profile.hint_font_size, UiFontWeight::Regular),
                            TextColor(Color::srgb(0.72, 0.69, 0.78)),
                            TextLayout::new(Justify::Right, LineBreak::WordBoundary),
                            Node {
                                width: Val::Percent(100.0),
                                min_width: Val::Px(0.0),
                                max_width: Val::Percent(100.0),
                                flex_shrink: 1.0,
                                ..default()
                            },
                            AmbitionDialogContinueHint,
                        ));
                    });
            });
        });
}

fn spawn_dialog_header(
    parent: &mut ChildSpawnerCommands,
    image: Option<Handle<Image>>,
    monogram: &str,
    accent: Color,
    speaker_label: &str,
    profile: &DialogLayoutProfile,
    dialog_font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                min_width: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(14.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Name::new("Ambition Dialogue Header"),
        ))
        .with_children(|header| {
            spawn_portrait(header, image, monogram, accent, profile, dialog_font);
            header.spawn((
                Text::new(speaker_label.to_string()),
                dialog_font(profile.speaker_font_size, UiFontWeight::Semibold),
                TextColor(Color::srgb(1.0, 0.88, 0.52)),
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                Node {
                    min_width: Val::Px(0.0),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    ..default()
                },
                Name::new("Dialogue Speaker Name"),
            ));
        });
}

fn spawn_portrait(
    parent: &mut ChildSpawnerCommands,
    image: Option<Handle<Image>>,
    monogram: &str,
    accent: Color,
    profile: &DialogLayoutProfile,
    dialog_font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(profile.portrait_width),
                min_width: Val::Px(profile.portrait_width),
                height: Val::Px(profile.portrait_height),
                padding: UiRect::all(Val::Px(5.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(3.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(accent),
            BorderColor::all(Color::srgb(0.96, 0.84, 0.48)),
            AmbitionDialogPortraitFrame,
            Name::new("Dialogue Speaker Portrait"),
        ))
        .with_children(|portrait| {
            if let Some(handle) = image {
                portrait.spawn((
                    ImageNode::new(handle),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    AmbitionDialogPortraitImage,
                    Name::new("Dialogue Portrait Image"),
                ));
            } else {
                portrait.spawn((
                    Text::new(monogram.to_string()),
                    dialog_font(profile.speaker_font_size + 12.0, UiFontWeight::Semibold),
                    TextColor(Color::srgb(1.0, 0.96, 0.82)),
                    TextLayout::new_with_justify(Justify::Center),
                    AmbitionDialogPortraitMonogram,
                    Name::new("Dialogue Portrait Placeholder"),
                ));
            }
        });
}

fn spawn_choice_viewport(
    parent: &mut ChildSpawnerCommands,
    dialogue: &DialogView,
    window: DialogChoiceWindow,
    selected_marker: &str,
    profile: &DialogLayoutProfile,
    dialog_font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                min_width: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                flex_shrink: 1.0,
                overflow: Overflow::clip(),
                ..default()
            },
            AmbitionDialogChoiceViewport,
            Name::new("Dialogue Choice Viewport"),
        ))
        .with_children(|viewport| {
            if window.is_windowed() {
                let before = if window.has_before() { "▲ more" } else { "" };
                let after = if window.has_after() { "▼ more" } else { "" };
                viewport.spawn((
                    Text::new(format!("{before:<8}{}   {after}", window.position_label())),
                    dialog_font(profile.hint_font_size, UiFontWeight::Regular),
                    TextColor(Color::srgb(0.78, 0.73, 0.82)),
                    TextLayout::new(Justify::Center, LineBreak::WordBoundary),
                    Node {
                        width: Val::Percent(100.0),
                        min_width: Val::Px(0.0),
                        ..default()
                    },
                    AmbitionDialogScrollIndicator,
                    Name::new("Dialogue Choice Window Indicator"),
                ));
            }

            viewport
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        min_width: Val::Px(0.0),
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                    Name::new("Dialogue Choice Rows And Scrollbar"),
                ))
                .with_children(|row_area| {
                    row_area
                        .spawn((
                            Node {
                                min_width: Val::Px(0.0),
                                flex_grow: 1.0,
                                flex_shrink: 1.0,
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                ..default()
                            },
                            Name::new("Dialogue Choice Rows"),
                        ))
                        .with_children(|rows| {
                            for index in window.range() {
                                let label = &dialogue.option_labels[index];
                                spawn_choice_row(
                                    rows,
                                    index,
                                    label,
                                    index == dialogue.selected_option,
                                    selected_marker,
                                    profile,
                                    dialog_font,
                                );
                            }
                        });

                    if let Some((thumb_top, thumb_height)) = window.scrollbar() {
                        row_area
                            .spawn((
                                Node {
                                    width: Val::Px(10.0),
                                    min_width: Val::Px(10.0),
                                    height: Val::Percent(100.0),
                                    border_radius: BorderRadius::all(Val::Px(5.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.12, 0.09, 0.15)),
                                AmbitionDialogScrollTrack,
                                Name::new("Dialogue Choice Scroll Track"),
                            ))
                            .with_children(|track| {
                                track.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(1.0),
                                        right: Val::Px(1.0),
                                        top: Val::Percent(thumb_top),
                                        height: Val::Percent(thumb_height),
                                        border_radius: BorderRadius::all(Val::Px(4.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.86, 0.67, 0.25)),
                                    AmbitionDialogScrollThumb,
                                    Name::new("Dialogue Choice Scroll Thumb"),
                                ));
                            });
                    }
                });
        });
}

fn spawn_choice_row(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    label: &str,
    selected: bool,
    selected_marker: &str,
    profile: &DialogLayoutProfile,
    dialog_font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    let (background, foreground, border) = if selected {
        (
            Color::srgb(0.88, 0.69, 0.27),
            Color::srgb(0.12, 0.06, 0.10),
            Color::srgb(1.0, 0.90, 0.58),
        )
    } else {
        (
            Color::srgb(0.09, 0.055, 0.105),
            Color::srgb(0.94, 0.90, 0.96),
            Color::srgb(0.28, 0.20, 0.32),
        )
    };
    let marker = if selected { selected_marker } else { " " };
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(profile.choice_min_height),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(7.0)),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(background),
            BorderColor::all(border),
            DialogChoiceSlot { index },
            Name::new(format!("Ambition dialogue choice {index}: {label}")),
        ))
        .with_children(|choice| {
            choice.spawn((
                Text::new(format!("{marker} {label}")),
                dialog_font(profile.choice_font_size, UiFontWeight::Regular),
                TextColor(foreground),
                TextLayout::new(Justify::Left, LineBreak::WordBoundary),
                Node {
                    width: Val::Percent(100.0),
                    min_width: Val::Px(0.0),
                    flex_shrink: 1.0,
                    ..default()
                },
            ));
        });
}

fn resolve_portrait_image_path(
    character_id: Option<&str>,
    character_catalog: Option<&CharacterCatalog>,
    portrait_catalog: &AmbitionDialogPortraitCatalog,
) -> Option<String> {
    let character_id = character_id?;
    if let Some(override_spec) = portrait_catalog.get(character_id) {
        return override_spec.image_path.clone();
    }
    character_catalog
        .and_then(|catalog| catalog.portrait_image_path(character_id))
        .map(str::to_owned)
}

fn placeholder_monogram(label: &str) -> String {
    let mut words = label.split_whitespace().filter_map(|word| {
        word.chars()
            .find(|ch| ch.is_alphanumeric())
            .map(|ch| ch.to_uppercase().collect::<String>())
    });
    let first = words.next().unwrap_or_else(|| "?".to_string());
    match words.next() {
        Some(second) => format!("{first}{second}"),
        None => first,
    }
}

fn placeholder_accent(key: &str) -> Color {
    const PALETTE: [(f32, f32, f32); 8] = [
        (0.31, 0.12, 0.29),
        (0.17, 0.26, 0.42),
        (0.24, 0.34, 0.20),
        (0.43, 0.20, 0.13),
        (0.23, 0.18, 0.39),
        (0.12, 0.34, 0.34),
        (0.42, 0.16, 0.25),
        (0.31, 0.28, 0.12),
    ];
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in key.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let (r, g, b) = PALETTE[(hash as usize) % PALETTE.len()];
    Color::srgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_view() -> DialogView {
        DialogView {
            active: true,
            dialogue_id: "hall_ada".to_string(),
            speaker_label: "Ada Lovelace".to_string(),
            conversation_label: "Ada Lovelace".to_string(),
            body: "The engine should expose facts, not dictate presentation.".to_string(),
            ..default()
        }
    }

    #[test]
    fn placeholder_monograms_follow_the_current_speaker() {
        assert_eq!(placeholder_monogram("Ada Lovelace"), "AL");
        assert_eq!(placeholder_monogram("Oiler"), "O");
        assert_eq!(placeholder_monogram(""), "?");
    }

    fn portrait_catalog_fixture() -> CharacterCatalog {
        let data = ambition_characters::actor::character_catalog::parse_catalog(
            r#"(
                brain_presets: { "idle": StandStill },
                action_set_presets: { "peaceful": (move_style: Walk) },
                characters: {
                    "npc_alice": (
                        display_name: "Alice",
                        spritesheet: "sprites/alice_spritesheet.png",
                        manifest: "sprites/alice_spritesheet.ron",
                        portrait: Some((
                            image: "sprites/alice_portraits.png",
                            manifest: "sprites/alice_portraits.ron",
                            default_clip: "default",
                        )),
                        tier: MainHall,
                        body_kind: Standard,
                        composition: None,
                        default_brain: "idle",
                        default_action_set: "peaceful",
                        tags: [],
                    ),
                },
            )"#,
        );
        CharacterCatalog::from_data(data)
    }

    #[test]
    fn catalog_portrait_is_the_default_dialog_image() {
        let catalog = portrait_catalog_fixture();
        let overrides = AmbitionDialogPortraitCatalog::default();
        assert_eq!(
            resolve_portrait_image_path(Some("npc_alice"), Some(&catalog), &overrides),
            Some("sprites/alice_portraits.png".to_string())
        );
    }

    #[test]
    fn game_override_can_deliberately_force_a_placeholder() {
        let catalog = portrait_catalog_fixture();
        let mut overrides = AmbitionDialogPortraitCatalog::default();
        overrides.insert(
            "npc_alice",
            AmbitionDialogPortraitSpec::placeholder(Color::srgb(0.0, 0.0, 0.0)),
        );
        assert_eq!(
            resolve_portrait_image_path(Some("npc_alice"), Some(&catalog), &overrides),
            None
        );
    }

    #[test]
    fn mobile_layout_preserves_readable_text_and_limits_visible_choices() {
        let portrait = DialogLayoutProfile::for_viewport(Vec2::new(390.0, 844.0));
        assert!(portrait.body_font_size >= 22.0);
        assert!(portrait.choice_font_size >= 20.0);
        assert_eq!(portrait.option_capacity, 3);
        assert_eq!(portrait.panel_top_inset, 64.0);
        assert_eq!(portrait.panel_max_height_percent, 58.0);

        let landscape = DialogLayoutProfile::for_viewport(Vec2::new(844.0, 390.0));
        assert!(landscape.body_font_size >= 20.0);
        assert_eq!(landscape.option_capacity, 2);
        assert_eq!(landscape.panel_top_inset, 58.0);
        assert_eq!(landscape.panel_max_height_percent, 62.0);
    }

    #[test]
    fn choice_window_keeps_selection_visible_and_reports_scroll_position() {
        let window = DialogChoiceWindow::new(7, 9, 4);
        assert_eq!(window.range(), 5..9);
        assert!(window.has_before());
        assert!(!window.has_after());
        assert_eq!(window.position_label(), "Choices 6–9 of 9");
        let (top, height) = window.scrollbar().expect("long list has a thumb");
        assert!((top - (100.0 - height)).abs() < 0.001);
    }

    #[test]
    fn ambition_dialog_is_horizontally_centered_in_the_upper_safe_band() {
        let mut app = App::new();
        app.insert_resource(active_view());
        app.add_plugins(AmbitionDialogUiPlugin);
        app.update();

        let mut roots = app
            .world_mut()
            .query_filtered::<&Node, With<AmbitionDialogOverlayRoot>>();
        let root = roots.single(app.world()).expect("one dialog root");
        assert_eq!(root.flex_direction, FlexDirection::Column);
        assert_eq!(root.justify_content, JustifyContent::FlexStart);
        assert_eq!(root.align_items, AlignItems::Center);
        assert_eq!(root.padding.top, Val::Px(24.0));
        assert_eq!(root.top, Val::Px(0.0));
        assert_eq!(root.bottom, Val::Px(0.0));

        let mut panels = app
            .world_mut()
            .query_filtered::<(&BackgroundColor, &Node), With<AmbitionDialogPanel>>();
        let (background, panel) = panels.single(app.world()).expect("one dialog panel");
        assert_eq!(background.0, Color::srgb(0.035, 0.025, 0.045));
        assert_eq!(panel.width, Val::Percent(96.0));
        assert_eq!(panel.max_height, Val::Percent(52.0));
        assert_eq!(panel.flex_direction, FlexDirection::Column);

        let mut hints = app
            .world_mut()
            .query_filtered::<(&Node, &TextLayout), With<AmbitionDialogContinueHint>>();
        let (hint, layout) = hints.single(app.world()).expect("one bounded hint");
        assert_eq!(hint.width, Val::Percent(100.0));
        assert_eq!(hint.max_width, Val::Percent(100.0));
        assert_eq!(layout.linebreak, LineBreak::WordBoundary);
    }

    #[test]
    fn ambition_dialog_uses_placeholder_portraits_until_art_is_registered() {
        let mut app = App::new();
        app.insert_resource(active_view());
        app.add_plugins(AmbitionDialogUiPlugin);
        app.update();

        let mut monograms = app
            .world_mut()
            .query_filtered::<&Text, With<AmbitionDialogPortraitMonogram>>();
        let monogram = monograms.single(app.world()).expect("one monogram");
        assert_eq!(monogram.0, "AL");
    }

    #[test]
    fn long_choice_lists_render_only_the_selected_window_with_absolute_slots() {
        let mut app = App::new();
        let mut view = active_view();
        view.option_labels = (0..9).map(|index| format!("Option {index}")).collect();
        view.selected_option = 7;
        app.insert_resource(view);
        app.add_plugins(AmbitionDialogUiPlugin);
        app.update();

        let mut choices = app.world_mut().query::<&DialogChoiceSlot>();
        let mut slots = choices
            .iter(app.world())
            .map(|slot| slot.index)
            .collect::<Vec<_>>();
        slots.sort_unstable();
        assert_eq!(slots, vec![4, 5, 6, 7, 8]);

        let mut indicators = app
            .world_mut()
            .query_filtered::<&Text, With<AmbitionDialogScrollIndicator>>();
        let indicator = indicators
            .single(app.world())
            .expect("one scroll indicator");
        assert!(indicator.0.contains("Choices 5–9 of 9"));

        let mut thumbs = app
            .world_mut()
            .query_filtered::<Entity, With<AmbitionDialogScrollThumb>>();
        assert_eq!(thumbs.iter(app.world()).count(), 1);
    }

    #[test]
    fn short_choice_lists_keep_every_shared_pointer_slot() {
        let mut app = App::new();
        let mut view = active_view();
        view.option_labels = vec!["Continue".to_string(), "Leave".to_string()];
        view.selected_option = 1;
        app.insert_resource(view);
        app.add_plugins(AmbitionDialogUiPlugin);
        app.update();

        let mut choices = app.world_mut().query::<&DialogChoiceSlot>();
        let mut slots = choices
            .iter(app.world())
            .map(|slot| slot.index)
            .collect::<Vec<_>>();
        slots.sort_unstable();
        assert_eq!(slots, vec![0, 1]);
    }
}
