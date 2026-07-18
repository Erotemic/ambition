//! Ambition's product-specific dialogue presentation.
//!
//! The engine publishes [`DialogView`] and shared input markers; this module
//! owns the actual game's visual language: a classic opaque bottom-of-screen
//! box, speaker nameplate, portrait frame, choices, and a bounded footer hint.
//! Default portrait products are referenced by stable character id through the
//! assembled [`CharacterCatalog`]. [`AmbitionDialogPortraitCatalog`] remains a
//! game-owned presentation override layer; speakers without portrait art get a
//! deterministic monogram placeholder.

use std::collections::BTreeMap;

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_render::dialog_ui::{
    claim_dialog_presentation, DialogOverlayRoot, DialogPresentationSet,
};
use ambition_render::ui_fonts::{UiFontWeight, UiFonts};
use ambition_sim_view::DialogView;
use ambition_ui_nav::DialogChoiceSlot;
use bevy::prelude::*;

const CONTINUE_HINT: &str = "Confirm / Interact: continue";
const CHOICE_HINT: &str = "Up / Down: choose    Confirm: select    Back: close";
const PRESENTER_NAME: &str = "ambition_content::AmbitionDialogUiPlugin";

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

fn sync_ambition_dialog_ui(
    mut commands: Commands,
    dialogue: Res<DialogView>,
    overlays: Query<Entity, With<DialogOverlayRoot>>,
    ui_fonts: Option<Res<UiFonts>>,
    character_catalog: Option<Res<CharacterCatalog>>,
    portrait_catalog: Res<AmbitionDialogPortraitCatalog>,
    asset_server: Option<Res<AssetServer>>,
) {
    let presentation_inputs_changed = dialogue.is_changed()
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
    // Overlay 1 publishes one-frame default portrait sheets, so the image
    // page can be shown directly. Named clip/frame playback will consume the
    // sibling manifest when animated dialogue portraits land.
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
                padding: UiRect::axes(Val::Px(18.0), Val::Px(20.0)),
                justify_content: JustifyContent::FlexEnd,
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
                    width: Val::Percent(92.0),
                    min_width: Val::Px(0.0),
                    max_width: Val::Px(1180.0),
                    min_height: Val::Px(188.0),
                    max_height: Val::Percent(72.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(16.0),
                    row_gap: Val::Px(12.0),
                    align_items: AlignItems::Stretch,
                    border: UiRect::all(Val::Px(4.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
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
                spawn_portrait(panel, portrait_image, &monogram, accent, &dialog_font);

                panel
                    .spawn((
                        Node {
                            min_width: Val::Px(200.0),
                            flex_basis: Val::Px(360.0),
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            ..default()
                        },
                        Name::new("Ambition Dialogue Text Column"),
                    ))
                    .with_children(|content| {
                        content.spawn((
                            Text::new(speaker_label.to_string()),
                            dialog_font(24.0, UiFontWeight::Semibold),
                            TextColor(Color::srgb(1.0, 0.88, 0.52)),
                            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                            Node {
                                width: Val::Percent(100.0),
                                min_height: Val::Px(32.0),
                                ..default()
                            },
                            Name::new("Dialogue Speaker Name"),
                        ));
                        content.spawn((
                            Text::new(dialogue.body.clone()),
                            dialog_font(21.0, UiFontWeight::Regular),
                            TextColor(Color::srgb(0.98, 0.96, 0.90)),
                            TextLayout::new(Justify::Left, LineBreak::WordBoundary),
                            Node {
                                width: Val::Percent(100.0),
                                min_width: Val::Px(0.0),
                                min_height: Val::Px(74.0),
                                flex_grow: 1.0,
                                flex_shrink: 1.0,
                                ..default()
                            },
                            Name::new("Dialogue Body"),
                        ));

                        for (index, label) in dialogue.option_labels.iter().enumerate() {
                            spawn_choice_row(
                                content,
                                index,
                                label,
                                index == dialogue.selected_option,
                                selected_marker,
                                &dialog_font,
                            );
                        }

                        let hint = if dialogue.option_labels.is_empty() {
                            CONTINUE_HINT
                        } else {
                            CHOICE_HINT
                        };
                        content
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
                                    dialog_font(14.0, UiFontWeight::Regular),
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
        });
}

fn spawn_portrait(
    parent: &mut ChildSpawnerCommands,
    image: Option<Handle<Image>>,
    monogram: &str,
    accent: Color,
    dialog_font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(112.0),
                min_width: Val::Px(112.0),
                height: Val::Px(136.0),
                padding: UiRect::all(Val::Px(6.0)),
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
                    dialog_font(46.0, UiFontWeight::Semibold),
                    TextColor(Color::srgb(1.0, 0.96, 0.82)),
                    TextLayout::new_with_justify(Justify::Center),
                    AmbitionDialogPortraitMonogram,
                    Name::new("Dialogue Portrait Placeholder"),
                ));
            }
        });
}

fn spawn_choice_row(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    label: &str,
    selected: bool,
    selected_marker: &str,
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
                min_height: Val::Px(40.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(2.0)),
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
                dialog_font(18.0, UiFontWeight::Regular),
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
        .and_then(|catalog| catalog.get(character_id))
        .and_then(|entry| entry.portrait.as_ref())
        .map(|portrait| portrait.image.clone())
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
    fn ambition_dialog_is_opaque_and_keeps_the_hint_inside_the_panel() {
        let mut app = App::new();
        app.insert_resource(active_view());
        app.add_plugins(AmbitionDialogUiPlugin);
        app.update();

        let mut panels = app
            .world_mut()
            .query_filtered::<(&BackgroundColor, &Node), With<AmbitionDialogPanel>>();
        let (background, panel) = panels.single(app.world()).expect("one dialog panel");
        assert_eq!(background.0, Color::srgb(0.035, 0.025, 0.045));
        assert_eq!(panel.flex_wrap, FlexWrap::Wrap);

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
    fn choices_keep_the_shared_pointer_navigation_contract() {
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
