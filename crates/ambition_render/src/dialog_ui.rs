//! Provider-selectable dialogue presentation.
//!
//! The reusable engine owns the presentation-neutral [`DialogView`], the shared
//! [`DialogChoiceSlot`] pointer-navigation marker, and this module's lifecycle /
//! ordering seam. It does not require games to use one visual composition.
//! A visible host installs exactly one presenter by calling
//! [`claim_dialog_presentation`] from its plugin and adding its renderer to
//! [`DialogPresentationSet`].
//!
//! [`DefaultDialogUiPlugin`] is the deliberately plain engine fallback. Ambition
//! installs its own opaque portrait layout from `ambition_content::presentation`
//! instead of configuring this renderer through an ever-growing style resource.

use bevy::log::info;
use bevy::prelude::*;

use crate::ui_fonts::{UiFontWeight, UiFonts};
use ambition_sim_view::DialogView;
#[cfg(test)]
use ambition_sim_view::{LocalView, LocalViewId, ViewParticipant, ViewPlacement};
use crate::ui_surface::ParticipantSurface;
use ambition_ui_nav::DialogChoiceSlot;

/// The default box's input hint, naming the answering seat's own controls
/// (the action's name when there is no control to show).
fn default_hint(dialogue: &DialogView) -> String {
    let named = |control: &str, action: &'static str| if control.is_empty() { action.to_owned() } else { control.to_owned() };
    let confirm = named(&dialogue.controls.confirm, "Confirm");
    let back = named(&dialogue.controls.back, "Back");
    if dialogue.controls.hold_to_skip {
        format!("{confirm}: continue    hold {back}: skip")
    } else if dialogue.option_labels.is_empty() {
        format!("{confirm}: continue    {back}: close")
    } else {
        format!("Up / Down: choose    {confirm}: select    {back}: close")
    }
}

/// Ordering seam shared by every concrete dialogue presenter.
///
/// Systems that need the current UI tree to exist should order after this set,
/// never after one game's renderer function.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DialogPresentationSet;

/// Shared root marker for whichever dialogue presenter the game selected.
/// Session/shell tests can use this without knowing the concrete skin.
#[derive(Component)]
pub struct DialogOverlayRoot;

pub fn dialog_owner_label(owner: Option<ambition_input::InputOwner>) -> String {
    match owner {
        Some(ambition_input::InputOwner::Participant(id)) => format!("P{}", u16::from(id.slot()) + 1),
        Some(ambition_input::InputOwner::Primary) => "P1".to_owned(),
        Some(ambition_input::InputOwner::AllParticipants) => "P*".to_owned(),
        None => String::new(),
    }
}


fn fit_dialog_surface(
    surface: ParticipantSurface,
    dialogue: Res<DialogView>,
    mut pointer_region: ResMut<ambition_ui_nav::DialogPointerRegion>,
    mut roots: Query<(&mut Node, &mut Visibility), With<DialogOverlayRoot>>,
) {
    let rect = dialogue
        .active
        .then(|| surface.rect(dialogue.input_owner))
        .flatten();
    if pointer_region.0 != rect {
        pointer_region.0 = rect;
    }
    for (mut node, mut visibility) in &mut roots {
        let next_visibility = if rect.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != next_visibility {
            *visibility = next_visibility;
        }
        if let Some(rect) = rect {
            let mut next = node.clone();
            surface.place(&mut next, rect);
            if *node != next {
                *node = next;
            }
        }
    }
}

/// Records which plugin claimed the one dialogue-presentation slot.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct DialogPresentationOwner(&'static str);

impl DialogPresentationOwner {
    pub fn name(&self) -> &'static str {
        self.0
    }
}

/// Claim the app's one concrete dialogue presenter.
///
/// Dialogue presentation is intentionally plugin-selected rather than globally
/// configured. Installing two renderers is always a composition error because
/// both would own the same overlay lifecycle and choice buttons.
pub fn claim_dialog_presentation(app: &mut App, owner: &'static str) {
    if let Some(existing) = app.world().get_resource::<DialogPresentationOwner>() {
        panic!(
            "dialogue presentation already claimed by '{}'; cannot also install '{owner}'",
            existing.name()
        );
    }
    app.insert_resource(DialogPresentationOwner(owner));
    app.init_resource::<ambition_ui_nav::DialogPointerRegion>();
    app.add_systems(Update, fit_dialog_surface.after(DialogPresentationSet).run_if(a_dialogue_is_shown_or_changed));
    // The presenter runs while a dialogue is shown and on the frame its view
    // changes, so the frame it closes takes the box down.
    app.configure_sets(Update, DialogPresentationSet
        .after(ambition_sim_view::camera_snapshot::CameraObservationSet)
        .after(ambition_sim_view::DialogViewSync)
        .run_if(a_dialogue_is_shown_or_changed));
}

/// Run condition of [`DialogPresentationSet`]: a dialogue is shown, or its
/// view changed since the last frame.
pub fn a_dialogue_is_shown_or_changed(view: Option<Res<DialogView>>) -> bool {
    view.is_some_and(|view| view.active || view.is_changed())
}

/// Plain reusable-engine fallback. Games with product-specific framing should
/// install their own plugin into [`DialogPresentationSet`] instead.
pub struct DefaultDialogUiPlugin;

impl Plugin for DefaultDialogUiPlugin {
    fn build(&self, app: &mut App) {
        claim_dialog_presentation(app, "ambition_render::DefaultDialogUiPlugin");
        app.add_systems(
            Update,
            sync_default_dialog_ui.in_set(DialogPresentationSet),
        );
    }
}

#[derive(Component)]
pub struct DefaultDialogPanel;

#[derive(Component)]
pub struct DefaultDialogFooter;

pub fn sync_default_dialog_ui(
    mut commands: Commands,
    dialogue: Res<DialogView>,
    overlays: Query<Entity, With<DialogOverlayRoot>>,
    ui_fonts: Option<Res<UiFonts>>,
    surface: ParticipantSurface,
    mut previous_rect: Local<Option<Rect>>,
    mut logged_font_state: Local<bool>,
) {
    let fonts_changed = ui_fonts.as_ref().is_some_and(|fonts| fonts.is_changed());
    let rect = surface.rect(dialogue.input_owner);
    let layout_changed = *previous_rect != rect;
    *previous_rect = rect;
    if !dialogue.is_changed() && !fonts_changed && !layout_changed {
        return;
    }

    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }
    if !dialogue.active || rect.is_none() {
        return;
    }

    let title = format!(
        "{} · {}",
        default_title(&dialogue),
        dialog_owner_label(dialogue.input_owner)
    );
    let options = &dialogue.option_labels;
    let selected = dialogue.selected_option;

    let selected_marker = ui_fonts
        .as_deref()
        .map(UiFonts::selected_marker)
        .unwrap_or(">");

    if !*logged_font_state {
        let marker_codepoints = selected_marker
            .chars()
            .map(|ch| format!("U+{:04X}", ch as u32))
            .collect::<Vec<_>>()
            .join(" ");

        let font_state = ui_fonts
            .as_deref()
            .map(|fonts| {
                format!(
                    "regular={}, semibold={}",
                    fonts.regular.is_some(),
                    fonts.semibold.is_some()
                )
            })
            .unwrap_or_else(|| "UiFonts resource missing".to_string());

        info!(
            "dialog UI font state: {font_state}; selected_marker='{selected_marker}' ({marker_codepoints})"
        );

        *logged_font_state = true;
    }

    let dialog_font = |font_size: f32, weight: UiFontWeight| {
        ui_fonts
            .as_deref()
            .map(|fonts| fonts.text_font(font_size, weight))
            .unwrap_or(TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            })
    };

    commands
        .spawn((
            // Spawned into the reading rect at once so the first frame is
            // right; `fit_to_reading_rect` keeps it right across resizes.
            {
                let mut node = Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Percent(15.0),
                    bottom: Val::Px(0.0),
                    padding: UiRect::axes(Val::Px(18.0), Val::Px(18.0)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    ..default()
                };
                surface.place(&mut node, rect.unwrap());
                node
            },
            ZIndex(45),
            Name::new("Default Dialogue Overlay Root"),
            DialogOverlayRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(72.0),
                    min_width: Val::Px(120.0),
                    max_width: Val::Px(960.0),
                    max_height: Val::Percent(94.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(20.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.025, 0.030, 0.045, 0.95)),
                BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.86)),
                Name::new("Default Dialogue Panel"),
                DefaultDialogPanel,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(title),
                    dialog_font(24.0, UiFontWeight::Semibold),
                    TextColor(Color::srgb(0.82, 0.94, 1.00)),
                ));
                parent.spawn((
                    Text::new(dialogue.body.clone()),
                    dialog_font(20.0, UiFontWeight::Regular),
                    TextColor(Color::srgb(0.93, 0.96, 1.00)),
                    Node {
                        width: Val::Percent(100.0),
                        ..default()
                    },
                ));
                for (idx, label) in options.iter().enumerate() {
                    spawn_default_dialog_choice_row(
                        parent,
                        idx,
                        label,
                        idx == selected,
                        selected_marker,
                        &dialog_font,
                    );
                }

                let hint = default_hint(&dialogue);
                parent
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            min_height: Val::Px(24.0),
                            justify_content: JustifyContent::FlexEnd,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        DefaultDialogFooter,
                    ))
                    .with_children(|footer| {
                        footer.spawn((
                            Text::new(hint),
                            dialog_font(14.0, UiFontWeight::Regular),
                            TextColor(Color::srgba(0.66, 0.76, 0.88, 0.96)),
                            TextLayout::new(Justify::Right, LineBreak::WordBoundary),
                            Node {
                                width: Val::Percent(100.0),
                                min_width: Val::Px(0.0),
                                max_width: Val::Percent(100.0),
                                flex_shrink: 1.0,
                                ..default()
                            },
                        ));
                    });
            });
        });
}

fn default_title(dialogue: &DialogView) -> String {
    if dialogue.speaker_label == dialogue.conversation_label {
        format!("{} — dialogue", dialogue.speaker_label)
    } else {
        format!(
            "{} — {}",
            dialogue.speaker_label, dialogue.conversation_label
        )
    }
}

fn spawn_default_dialog_choice_row(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    label: &str,
    selected: bool,
    selected_marker: &str,
    dialog_font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    let bg = if selected {
        Color::srgba(0.95, 0.78, 0.32, 0.96)
    } else {
        Color::srgba(0.10, 0.13, 0.20, 0.74)
    };
    let fg = if selected {
        Color::srgb(0.18, 0.06, 0.04)
    } else {
        Color::srgb(0.82, 0.90, 1.0)
    };
    let marker = if selected { selected_marker } else { " " };
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(46.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(bg),
            DialogChoiceSlot { index },
            Name::new(format!("Dialogue choice {index}: {label}")),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{marker} {label}")),
                dialog_font(20.0, UiFontWeight::Regular),
                TextColor(fg),
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OtherPresenter;

    impl Plugin for OtherPresenter {
        fn build(&self, app: &mut App) {
            claim_dialog_presentation(app, "test::OtherPresenter");
        }
    }

    #[test]
    #[should_panic(expected = "dialogue presentation already claimed")]
    fn two_presenters_cannot_own_the_same_overlay() {
        let mut app = App::new();
        app.add_plugins(DefaultDialogUiPlugin);
        app.add_plugins(OtherPresenter);
    }

    #[test]
    fn default_presenter_is_opt_in_and_uses_the_shared_root() {
        let mut app = App::new();
        app.init_resource::<DialogView>();
        app.add_plugins(DefaultDialogUiPlugin);
        {
            let mut view = app.world_mut().resource_mut::<DialogView>();
            view.active = true;
            view.dialogue_id = "example".to_string();
            view.speaker_label = "Speaker".to_string();
            view.conversation_label = "Speaker".to_string();
            view.body = "Hello".to_string();
        }
        app.update();

        let mut roots = app
            .world_mut()
            .query_filtered::<Entity, With<DialogOverlayRoot>>();
        assert_eq!(roots.iter(app.world()).count(), 1);
    }

    #[test]
    fn dialogue_follows_its_participant_through_split_merge_and_view_removal() {
        use ambition_characters::control::PlayerSlot;
        use ambition_input::InputOwner;
        use ambition_input::ParticipantId;
        let mut app = App::new();
        app.insert_resource(DialogView {
            active: true,
            input_owner: Some(InputOwner::Participant(ParticipantId(1))),
            body: "A line in progress".to_owned(),
            selected_option: 2,
            ..default()
        });
        app.add_plugins(DefaultDialogUiPlugin);
        let first = app.world_mut().spawn((LocalView, LocalViewId(0), ViewParticipant(PlayerSlot(0)), ViewPlacement::column(0, 2))).id();
        let second = app.world_mut().spawn((LocalView, LocalViewId(1), ViewParticipant(PlayerSlot(1)), ViewPlacement::column(1, 2))).id();
        app.update();
        let rect = app.world().resource::<ambition_ui_nav::DialogPointerRegion>().0.unwrap();
        assert_eq!(rect, Rect::from_corners(Vec2::new(640.0, 0.0), Vec2::new(1280.0, 720.0)));
        let mut roots = app.world_mut().query_filtered::<&Node, With<DialogOverlayRoot>>();
        let root = roots.single(app.world()).unwrap();
        assert_eq!(root.left, Val::Px(640.0));
        assert_eq!(root.width, Val::Px(640.0));
        assert_eq!(root.overflow, Overflow::clip());
        app.world_mut().despawn(second);
        app.world_mut().entity_mut(first).remove::<ViewParticipant>();
        app.world_mut().entity_mut(first).remove::<ViewPlacement>();
        app.update();
        assert_eq!(app.world().resource::<ambition_ui_nav::DialogPointerRegion>().0.unwrap().size(), Vec2::new(1280.0, 720.0));
        assert_eq!(app.world().resource::<DialogView>().body, "A line in progress");
        assert_eq!(app.world().resource::<DialogView>().selected_option, 2);
        app.world_mut().entity_mut(first).insert(ViewParticipant(PlayerSlot(0)));
        app.update();
        assert!(app.world().resource::<ambition_ui_nav::DialogPointerRegion>().0.is_none());
        assert_eq!(roots.iter(app.world()).count(), 0);
    }
}
