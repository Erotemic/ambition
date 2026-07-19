//! Plain Bevy UI reference presentation for launchers and shell sequences.
//!
//! Launcher content is translated into `ambition_menu`'s renderer-independent
//! page model and drawn by its flat Bevy-UI renderer. The shell keeps only the
//! host-relative route catalog and cursor; it does not introduce a competing
//! menu content or rendering model.

use ambition_menu::render::bevy_ui::{
    install_bevy_ui_menu_actions, spawn_bevy_ui_menu_with_assets, BevyUiMenuInteractionSet,
    BevyUiMenuRoot, BevyUiMenuTabSpec, BevyUiMenuView,
};
use ambition_menu::{
    MenuActionActivated, MenuColor, MenuControlKind, MenuPageModel, MenuRect, MenuTextAlign,
};
use ambition_sfx::{ids, OwnedSfxMessage, SfxMessage, SfxWriter};
use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;

use crate::{
    image_sequence_frame_at, shell_action_edges, ActiveShellSequence, FrontendOwnedEntity,
    FrontendPresentationKind, ShellAnalogLatch, ShellLaunchCatalog, ShellLauncherCommand,
    ShellLauncherPresentation, ShellLauncherState, ShellRouter, ShellSegmentPresentation,
    ShellSequenceCommand,
};

#[derive(Component)]
pub struct BasicSequenceRoot;

/// Marks the fade-able CONTENT of a vanity card (its text / image), distinct from
/// the opaque black backdrop. [`drive_basic_sequence_card`] ramps its alpha from
/// the sequence runtime's elapsed time so the card eases in from black and out
/// again, instead of snapping.
#[derive(Component)]
pub struct BasicSequenceCardContent;

/// Every frame handle of an animated sequence, resolved ONCE when the card
/// spawns and held on its image node.
///
/// Preloading matters here: the card is short, so resolving handles lazily per
/// frame would let a late-arriving image miss its own slot entirely. It also
/// keeps the node tree stable — the animation advances by swapping this node's
/// texture, never by rebuilding the card (see [`shell_frame_key`]).
#[derive(Component)]
pub struct BasicSequenceImages {
    handles: Vec<Handle<Image>>,
}

/// The per-frame "this picture is missing" notice.
///
/// Sequence payloads can be absent from a checkout (they are generated, and
/// git-ignored), so a frame that fails to load degrades to a visible label for
/// exactly its own slot rather than taking down the card. Timing is untouched:
/// the sequence still runs its full length and still hands off on schedule.
#[derive(Component)]
pub struct BasicSequenceMissingNotice;

/// Seconds the vanity card spends fading in, and (separately) fading out. The
/// card holds at full opacity in between; a card whose `auto_advance_after` is
/// shorter than `2 * FADE` still reads as a smooth in-then-out.
const CARD_FADE_SECONDS: f32 = 0.55;

#[derive(Default)]
struct BasicSequenceFrame {
    key: String,
    text: String,
    image_path: Option<String>,
    /// Every frame path, when this segment is an animated sequence. Empty for
    /// still cards. Drives preloading and the per-frame texture swap.
    sequence_paths: Vec<String>,
}

/// Marker on the basic shell presentation's own launcher menu root, so its
/// rebuild teardown never claims another producer's `BevyUiMenuRoot`.
#[derive(Component)]
pub struct BasicShellUiRoot;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BasicLauncherPage {
    Home,
}

/// Stable selectable index in the launcher's semantic selection space
/// (available routes first, then the optional Exit row).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BasicLauncherAction(usize);

#[derive(Default)]
pub struct BasicShellPresentationPlugin;

impl Plugin for BasicShellPresentationPlugin {
    fn build(&self, app: &mut App) {
        install_bevy_ui_menu_actions::<BasicLauncherAction>(app);
        app.add_message::<OwnedSfxMessage>()
            .init_resource::<ambition_sfx::SfxEmissionContext>()
            .add_systems(
                Update,
                (
                    basic_shell_keyboard,
                    basic_shell_pointer.after(BevyUiMenuInteractionSet),
                    render_basic_shell,
                    drive_basic_sequence_card,
                )
                    .chain(),
            );
    }
}

/// Pointer/touch activation for launcher rows. The shared menu renderer turns
/// `Interaction::Pressed` into [`MenuActionActivated`]; this adapter routes the
/// selected row through the same [`ShellLauncherCommand`] processor used by
/// keyboard/controller confirmation.
fn basic_shell_pointer(
    launcher: Res<ShellLauncherState>,
    mut activated: MessageReader<MenuActionActivated<BasicLauncherAction>>,
    mut launcher_commands: MessageWriter<ShellLauncherCommand>,
    mut sfx: SfxWriter,
) {
    for activation in activated.read() {
        if !launcher.active {
            continue;
        }
        launcher_commands.write(ShellLauncherCommand::Activate(activation.action.0));
        sfx.write(SfxMessage::Play {
            id: ids::UI_MENU_ACCEPT,
            pos: Vec2::ZERO,
        });
    }
}

/// Unified menu input: keyboard AND controller drive the same neutral
/// navigation edges (up / down / confirm), so no downstream logic is duplicated
/// per device. The D-pad mirrors the arrow keys; South (A / cross) mirrors
/// Enter/Space. Touch and the on-screen HUD arrive through the optional
/// `MenuControlFrame` seam rather than a second device reader, so a phone can
/// dismiss a startup card and pick a launcher row with no keyboard attached.
fn basic_shell_keyboard(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    pads: Query<&Gamepad>,
    menu_frame: Option<Res<ambition_input::MenuControlFrame>>,
    launcher: Res<ShellLauncherState>,
    sequence: Res<ActiveShellSequence>,
    mut launcher_commands: MessageWriter<ShellLauncherCommand>,
    mut sequence_commands: MessageWriter<ShellSequenceCommand>,
    mut sfx: SfxWriter,
    mut analog: Local<ShellAnalogLatch>,
) {
    let actions = shell_action_edges(keys.as_deref(), &pads, menu_frame.as_deref(), &mut analog);
    let (up, down, confirm) = (actions.previous, actions.next, actions.confirm);
    if launcher.active {
        if up {
            launcher_commands.write(ShellLauncherCommand::Previous);
            sfx.write(SfxMessage::Play {
                id: ids::UI_MENU_MOVE,
                pos: Vec2::ZERO,
            });
        }
        if down {
            launcher_commands.write(ShellLauncherCommand::Next);
            sfx.write(SfxMessage::Play {
                id: ids::UI_MENU_MOVE,
                pos: Vec2::ZERO,
            });
        }
        if confirm {
            launcher_commands.write(ShellLauncherCommand::LaunchSelected);
            sfx.write(SfxMessage::Play {
                id: ids::UI_MENU_ACCEPT,
                pos: Vec2::ZERO,
            });
        }
    } else if let (Some(activation_id), Some(runtime)) =
        (sequence.activation_id, sequence.runtime.as_ref())
    {
        if confirm {
            sfx.write(SfxMessage::Play {
                id: ids::UI_MENU_ACCEPT,
                pos: Vec2::ZERO,
            });
            if runtime
                .current()
                .is_some_and(|segment| segment.policy.requires_acknowledgement)
            {
                sequence_commands.write(ShellSequenceCommand::Acknowledge { activation_id });
            } else {
                sequence_commands.write(ShellSequenceCommand::Skip { activation_id });
            }
        }
    }
}

/// The neutral `(up, down, confirm)` navigation edges for this frame, unified
/// across keyboard and every connected controller. Kept as a free function so
/// the mapping is unit-testable without a live window.
fn render_basic_shell(
    mut commands: Commands,
    launcher: Res<ShellLauncherState>,
    catalog: Res<ShellLaunchCatalog>,
    launcher_presentation: Res<ShellLauncherPresentation>,
    sequence: Res<ActiveShellSequence>,
    router: Res<ShellRouter>,
    asset_server: Option<Res<AssetServer>>,
    sequence_roots: Query<Entity, With<BasicSequenceRoot>>,
    // Identity, not species: only THIS presentation's launcher tree. Other
    // `BevyUiMenuRoot` producers (a game's pause menu) coexist in the host.
    launcher_roots: Query<Entity, (With<BevyUiMenuRoot>, With<BasicShellUiRoot>)>,
    mut prior_key: Local<String>,
) {
    let frame_key = format!(
        "{:?}:{}",
        router.active.as_ref().map(|active| active.activation_id),
        shell_frame_key(&launcher, &catalog, &launcher_presentation, &sequence),
    );
    if *prior_key == frame_key {
        return;
    }
    *prior_key = frame_key;

    for entity in &sequence_roots {
        commands.entity(entity).despawn();
    }
    for entity in &launcher_roots {
        commands.entity(entity).despawn();
    }

    let Some(activation_id) = router.active.as_ref().map(|active| active.activation_id) else {
        return;
    };

    if launcher.active {
        spawn_launcher_menu(
            &mut commands,
            &launcher,
            &catalog,
            &launcher_presentation,
            asset_server.as_deref(),
            activation_id,
        );
        return;
    }

    let frame = sequence_frame(&sequence);
    if frame.text.is_empty() && frame.image_path.is_none() {
        return;
    }
    commands
        .spawn((
            BasicSequenceRoot,
            FrontendOwnedEntity::shell(activation_id, FrontendPresentationKind::StartupRoot),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.025, 0.03, 0.05)),
            GlobalZIndex(900),
            Name::new("basic shell sequence presentation"),
        ))
        .with_children(|root| {
            if let Some(handle) = frame
                .image_path
                .as_ref()
                .zip(asset_server.as_deref())
                .map(|(path, server)| server.load::<Image>(path.clone()))
            {
                // Start transparent; the fade system eases it in (matching the
                // text below, so neither content kind flashes for a frame).
                let mut image = ImageNode::new(handle);
                image.color.set_alpha(0.0);
                let mut node = root.spawn((
                    image,
                    // Width-driven with an automatic height so the picture keeps
                    // its own aspect ratio. Pinning both axes stretches whatever
                    // is loaded to the box — a 16:9 card would render squashed.
                    Node {
                        width: Val::Percent(70.0),
                        height: Val::Auto,
                        max_height: Val::Percent(80.0),
                        ..default()
                    },
                    BasicSequenceCardContent,
                    Name::new("basic shell sequence image"),
                ));
                // Resolve every frame up front so a short card never waits on a
                // texture mid-animation.
                if let Some(server) = asset_server.as_deref() {
                    if !frame.sequence_paths.is_empty() {
                        node.insert(BasicSequenceImages {
                            handles: frame
                                .sequence_paths
                                .iter()
                                .map(|path| server.load::<Image>(path.clone()))
                                .collect(),
                        });
                    }
                }
            }
            if !frame.sequence_paths.is_empty() {
                // Always present for a sequence, empty until a frame actually
                // fails to load — see `BasicSequenceMissingNotice`.
                root.spawn((
                    Text::default(),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.55, 0.55).with_alpha(0.0)),
                    TextLayout::new_with_justify(Justify::Center),
                    BasicSequenceCardContent,
                    BasicSequenceMissingNotice,
                    Name::new("basic shell sequence missing-frame notice"),
                ));
            }
            if !frame.text.is_empty() {
                root.spawn((
                    Text::new(frame.text),
                    TextFont {
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.92, 0.94, 1.0).with_alpha(0.0)),
                    TextLayout::new_with_justify(Justify::Center),
                    BasicSequenceCardContent,
                ));
            }
        });
}

fn spawn_launcher_menu(
    commands: &mut Commands,
    launcher: &ShellLauncherState,
    catalog: &ShellLaunchCatalog,
    presentation: &ShellLauncherPresentation,
    asset_server: Option<&AssetServer>,
    activation_id: crate::ShellActivationId,
) {
    let mut page = MenuPageModel::new(
        BasicLauncherPage::Home,
        presentation.title.clone(),
        MenuColor::rgba(0.015, 0.020, 0.055, 0.98),
    );
    page.text(
        50.0,
        8.0,
        5.2,
        presentation.title.clone(),
        MenuTextAlign::Center,
        MenuColor::WHITE,
    );
    if catalog.entries.is_empty() && presentation.exit_label.is_none() {
        page.text(
            50.0,
            48.0,
            3.6,
            presentation.empty_message.clone(),
            MenuTextAlign::Center,
            MenuColor::WHITE,
        );
    } else {
        // Every registered experience gets a row: available ones are selectable
        // Actions; unavailable ones are non-actionable Items showing the reason.
        // The navigation cursor addresses only available entries, so map that
        // cursor onto the full list when deciding what to highlight.
        let exit_rows = usize::from(presentation.exit_label.is_some());
        let row_height = (60.0 / (catalog.entries.len() + exit_rows).max(1) as f32).min(12.0);
        let mut available_index = 0usize;
        for (index, entry) in catalog.entries.iter().enumerate() {
            let (kind, action, detail, selected) = if entry.available {
                let selected = available_index == launcher.selected;
                // The row carries its SELECTION index, not its route: pointer
                // activation then lands in the same command the cursor produces.
                let action = BasicLauncherAction(available_index);
                available_index += 1;
                (
                    MenuControlKind::Action,
                    Some(action),
                    (!entry.description.is_empty()).then_some(entry.description.clone()),
                    selected,
                )
            } else {
                (
                    MenuControlKind::Item,
                    None,
                    Some(
                        entry
                            .unavailable_reason
                            .clone()
                            .unwrap_or_else(|| "Unavailable".to_owned()),
                    ),
                    false,
                )
            };
            page.control(
                MenuRect::new(
                    16.0,
                    18.0 + index as f32 * (row_height + 1.5),
                    68.0,
                    row_height,
                ),
                kind,
                entry.label.clone(),
                detail,
                selected,
                false,
                action,
            );
        }
        // The built-in Exit row after the experiences. The navigation cursor
        // addresses available entries then Exit, so Exit is selected when the
        // cursor equals the available count.
        if let Some(exit_label) = &presentation.exit_label {
            page.control(
                MenuRect::new(
                    16.0,
                    18.0 + catalog.entries.len() as f32 * (row_height + 1.5),
                    68.0,
                    row_height,
                ),
                MenuControlKind::Action,
                exit_label.clone(),
                Some("Leave the game".to_owned()),
                available_index == launcher.selected,
                false,
                // Exit sits after the experiences in the same selection space,
                // so it is pointer-activatable like any other row.
                Some(BasicLauncherAction(available_index)),
            );
        }
        if !presentation.footer.is_empty() {
            page.text(
                50.0,
                92.0,
                2.6,
                presentation.footer.clone(),
                MenuTextAlign::Center,
                MenuColor::WHITE,
            );
        }
    }

    let tabs = [BevyUiMenuTabSpec::new(BasicLauncherPage::Home, "Play")];
    let view = BevyUiMenuView::<BasicLauncherPage, BasicLauncherAction> {
        tabs: &tabs,
        active_tab: 0,
        page: &page,
        focused: None,
        focused_tab: None,
    };
    let root = spawn_bevy_ui_menu_with_assets(commands, &view, asset_server);
    commands.entity(root).insert((
        BasicShellUiRoot,
        FrontendOwnedEntity::shell(activation_id, FrontendPresentationKind::LauncherRoot),
    ));
}

/// The vanity card's content alpha at `elapsed` seconds into a segment lasting
/// `duration` seconds: ease in over the first [`CARD_FADE_SECONDS`], hold, then
/// ease out over the last [`CARD_FADE_SECONDS`]. A segment with no auto-advance
/// (`duration = None`) never fades out (it holds until skipped).
fn card_alpha(elapsed: f32, duration: Option<f32>) -> f32 {
    let fade = CARD_FADE_SECONDS.max(f32::EPSILON);
    let fade_in = (elapsed / fade).clamp(0.0, 1.0);
    let fade_out = match duration {
        Some(d) if d > 0.0 => ((d - elapsed) / fade).clamp(0.0, 1.0),
        _ => 1.0,
    };
    fade_in.min(fade_out)
}

/// Ease the vanity card's content (text / image) in and out each frame from the
/// sequence runtime's elapsed time, so the "Powered by Ambition" card no longer
/// snaps on and off. The opaque black backdrop is untouched — only the content
/// alpha ramps, so the card fades up from and back down to black.
fn drive_basic_sequence_card(
    sequence: Res<ActiveShellSequence>,
    asset_server: Option<Res<AssetServer>>,
    mut texts: Query<&mut TextColor, With<BasicSequenceCardContent>>,
    mut images: Query<
        (&mut ImageNode, Option<&BasicSequenceImages>),
        With<BasicSequenceCardContent>,
    >,
    mut notices: Query<&mut Text, With<BasicSequenceMissingNotice>>,
) {
    let Some(runtime) = sequence.runtime.as_ref() else {
        return;
    };
    let elapsed = runtime.elapsed.as_secs_f32();
    let duration = runtime
        .current()
        .and_then(|segment| segment.policy.auto_advance_after)
        .map(|d| d.as_secs_f32());
    let alpha = card_alpha(elapsed, duration);
    for mut color in &mut texts {
        color.0.set_alpha(alpha);
    }

    let active = active_sequence_frame(&sequence);
    let mut missing = None;
    for (mut image, frames) in &mut images {
        image.color.set_alpha(alpha);
        let (Some((index, count)), Some(frames)) = (active, frames) else {
            continue;
        };
        let Some(handle) = frames.handles.get(index) else {
            continue;
        };
        // A frame whose file is absent hides its own slot and names itself; the
        // rest of the sequence is unaffected.
        let failed = asset_server
            .as_deref()
            .is_some_and(|server| server.get_load_state(handle).is_some_and(|s| s.is_failed()));
        if failed {
            image.color.set_alpha(0.0);
            missing = Some((index, count));
        } else {
            image.image = handle.clone();
        }
    }

    for mut text in &mut notices {
        let wanted = match missing {
            Some((index, count)) => format!("missing frame {} of {count}", index + 1),
            None => String::new(),
        };
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}

fn shell_frame_key(
    launcher: &ShellLauncherState,
    catalog: &ShellLaunchCatalog,
    presentation: &ShellLauncherPresentation,
    sequence: &ActiveShellSequence,
) -> String {
    if launcher.active {
        return format!(
            "launcher:{}:{}:{:?}",
            launcher.selected, presentation.title, catalog.entries
        );
    }
    sequence_frame(sequence).key
}

fn sequence_frame(sequence: &ActiveShellSequence) -> BasicSequenceFrame {
    let Some(runtime) = sequence.runtime.as_ref() else {
        return BasicSequenceFrame::default();
    };
    let Some(segment) = runtime.current() else {
        return BasicSequenceFrame::default();
    };
    match &segment.presentation {
        ShellSegmentPresentation::TextCard { title, subtitle } => {
            let text = format!(
                "{}{}",
                title,
                subtitle
                    .as_ref()
                    .map(|item| format!("\n\n{item}"))
                    .unwrap_or_default()
            );
            BasicSequenceFrame {
                key: format!("text:{}:{text}", segment.id),
                text,
                image_path: None,
                sequence_paths: Vec::new(),
            }
        }
        ShellSegmentPresentation::StaticImage {
            asset_path,
            alt_text,
        } => BasicSequenceFrame {
            key: format!("image:{}:{asset_path}", segment.id),
            text: alt_text.clone(),
            image_path: Some(asset_path.clone()),
            sequence_paths: Vec::new(),
        },
        // Keyed on segment IDENTITY, deliberately not on the current frame: the
        // card spawns once and animates by swapping its texture. Folding the
        // frame index in here would rebuild the entire node tree every frame.
        ShellSegmentPresentation::ImageSequence { frames, alt_text } => BasicSequenceFrame {
            key: format!("sequence:{}:{}", segment.id, frames.len()),
            text: alt_text.clone(),
            image_path: frames.first().map(|frame| frame.asset_path.clone()),
            sequence_paths: frames
                .iter()
                .map(|frame| frame.asset_path.clone())
                .collect(),
        },
        ShellSegmentPresentation::Registered(_) => BasicSequenceFrame::default(),
    }
}

/// The frame index showing right now, and how many frames the sequence has.
fn active_sequence_frame(sequence: &ActiveShellSequence) -> Option<(usize, usize)> {
    let runtime = sequence.runtime.as_ref()?;
    let segment = runtime.current()?;
    let ShellSegmentPresentation::ImageSequence { frames, .. } = &segment.presentation else {
        return None;
    };
    if frames.is_empty() {
        return None;
    }
    Some((
        image_sequence_frame_at(frames, runtime.elapsed),
        frames.len(),
    ))
}

#[cfg(test)]
mod fade_tests {
    use super::card_alpha;

    #[test]
    fn vanity_card_eases_in_holds_then_eases_out() {
        let duration = 3.6;
        // Starts fully transparent, reaches opaque by the end of the fade-in.
        assert_eq!(card_alpha(0.0, Some(duration)), 0.0);
        assert!(card_alpha(super::CARD_FADE_SECONDS * 0.5, Some(duration)) > 0.0);
        assert_eq!(card_alpha(super::CARD_FADE_SECONDS, Some(duration)), 1.0);
        // Holds at full opacity through the middle.
        assert_eq!(card_alpha(duration * 0.5, Some(duration)), 1.0);
        // Fully faded out by the end.
        assert_eq!(card_alpha(duration, Some(duration)), 0.0);
        assert!(card_alpha(duration - super::CARD_FADE_SECONDS * 0.5, Some(duration)) < 1.0);
    }

    #[test]
    fn a_card_with_no_auto_advance_never_fades_out() {
        // Only the fade-in applies; it holds at full opacity indefinitely.
        assert_eq!(card_alpha(0.0, None), 0.0);
        assert_eq!(card_alpha(super::CARD_FADE_SECONDS, None), 1.0);
        assert_eq!(card_alpha(1_000.0, None), 1.0);
    }
}

#[cfg(test)]
mod raw_input_tests {
    use super::*;
    use crate::{ActiveShellSequence, ShellLauncherState, ShellSequenceCommand};
    use bevy::input::ButtonInput;
    use bevy::prelude::{App, KeyCode, Messages, Update};

    fn app_with_launcher(active: bool) -> App {
        let mut app = App::new();
        app.add_message::<ShellLauncherCommand>();
        app.add_message::<ShellSequenceCommand>();
        app.add_message::<OwnedSfxMessage>();
        app.init_resource::<ambition_sfx::SfxEmissionContext>();
        app.world_mut()
            .resource_mut::<ambition_sfx::SfxEmissionContext>()
            .set(ambition_sfx::AudioContextOwner::Frontend(9));
        app.init_resource::<ShellLauncherState>();
        app.init_resource::<ActiveShellSequence>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Update, basic_shell_keyboard);
        app.world_mut().resource_mut::<ShellLauncherState>().active = active;
        app
    }

    /// Simulate one discrete key tap: press, run a frame, then clear the
    /// per-frame `just_pressed` edge (no bevy InputPlugin does it here).
    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }

    fn drained(app: &mut App) -> Vec<ShellLauncherCommand> {
        app.world_mut()
            .resource_mut::<Messages<ShellLauncherCommand>>()
            .drain()
            .collect()
    }

    fn drained_sfx(app: &mut App) -> Vec<OwnedSfxMessage> {
        app.world_mut()
            .resource_mut::<Messages<OwnedSfxMessage>>()
            .drain()
            .collect()
    }

    #[test]
    fn arrow_keys_move_the_launcher_cursor() {
        let mut app = app_with_launcher(true);
        tap(&mut app, KeyCode::ArrowDown);
        assert_eq!(drained(&mut app), vec![ShellLauncherCommand::Next]);
        let sfx = drained_sfx(&mut app);
        assert!(matches!(
            sfx.as_slice(),
            [OwnedSfxMessage {
                owner: Some(ambition_sfx::AudioContextOwner::Frontend(9)),
                request: SfxMessage::Play { id, .. },
            }] if *id == ids::UI_MENU_MOVE
        ));
        tap(&mut app, KeyCode::ArrowUp);
        assert_eq!(drained(&mut app), vec![ShellLauncherCommand::Previous]);
        let _ = drained_sfx(&mut app);
    }

    #[test]
    fn enter_and_space_confirm_the_selection() {
        let mut app = app_with_launcher(true);
        tap(&mut app, KeyCode::Enter);
        assert_eq!(
            drained(&mut app),
            vec![ShellLauncherCommand::LaunchSelected]
        );
        assert!(matches!(
            drained_sfx(&mut app).as_slice(),
            [OwnedSfxMessage {
                request: SfxMessage::Play { id, .. },
                ..
            }] if *id == ids::UI_MENU_ACCEPT
        ));
        tap(&mut app, KeyCode::Space);
        assert_eq!(
            drained(&mut app),
            vec![ShellLauncherCommand::LaunchSelected]
        );
        let _ = drained_sfx(&mut app);
    }

    #[test]
    fn keyboard_is_inert_when_launcher_is_not_active() {
        let mut app = app_with_launcher(false);
        tap(&mut app, KeyCode::ArrowDown);
        assert!(
            drained(&mut app).is_empty(),
            "keyboard drives no launcher command when the launcher is not focused"
        );
    }

    /// Simulate one discrete controller button tap against a spawned `Gamepad`
    /// component (`digital_mut` is Bevy's documented input-mocking seam).
    fn pad_tap(app: &mut App, pad: Entity, button: bevy::input::gamepad::GamepadButton) {
        {
            let mut entity = app.world_mut().entity_mut(pad);
            let mut gamepad = entity.get_mut::<bevy::input::gamepad::Gamepad>().unwrap();
            gamepad.digital_mut().press(button);
        }
        app.update();
        let mut entity = app.world_mut().entity_mut(pad);
        let mut gamepad = entity.get_mut::<bevy::input::gamepad::Gamepad>().unwrap();
        gamepad.digital_mut().clear();
    }

    #[test]
    fn controller_dpad_and_south_drive_the_same_launcher_commands_as_the_keyboard() {
        use bevy::input::gamepad::Gamepad;
        let mut app = app_with_launcher(true);
        let pad = app.world_mut().spawn(Gamepad::default()).id();

        pad_tap(&mut app, pad, GamepadButton::DPadDown);
        assert_eq!(
            drained(&mut app),
            vec![ShellLauncherCommand::Next],
            "D-pad down navigates like ArrowDown"
        );
        pad_tap(&mut app, pad, GamepadButton::DPadUp);
        assert_eq!(drained(&mut app), vec![ShellLauncherCommand::Previous]);
        pad_tap(&mut app, pad, GamepadButton::South);
        assert_eq!(
            drained(&mut app),
            vec![ShellLauncherCommand::LaunchSelected],
            "South (A / cross) confirms like Enter"
        );
    }

    #[test]
    fn controller_is_inert_when_launcher_is_not_active() {
        use bevy::input::gamepad::Gamepad;
        let mut app = app_with_launcher(false);
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        pad_tap(&mut app, pad, GamepadButton::DPadDown);
        assert!(
            drained(&mut app).is_empty(),
            "controller drives no launcher command when the launcher is not focused"
        );
    }
}
