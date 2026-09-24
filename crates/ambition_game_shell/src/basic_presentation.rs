//! Plain Bevy UI reference presentation for launchers and shell sequences.
//!
//! Launcher content is translated into `ambition_menu`'s renderer-independent
//! page model and drawn by its flat Bevy-UI renderer. The shell keeps only the
//! host-relative route catalog and cursor; it does not introduce a competing
//! menu content or rendering model.

use ambition_input::participant::context_priority;
use ambition_input::{
    ActiveUiCues, InputSet, UiCue, LAUNCHER_CONTEXT, STARTUP_ACKNOWLEDGE_CONTEXT,
};
use ambition_menu::render::bevy_ui::{
    install_bevy_ui_menu_actions, BevyUiMenuInteractionSet, BevyUiMenuRoot, BevyUiMenuTabSpec,
    BevyUiMenuView,
};
use ambition_menu::{
    AmbitionMenuControl, MenuActionActivated, MenuActionPreviewed, MenuColor, MenuControlKind,
    MenuFocusKey, MenuPageModel, MenuRect, MenuTextAlign,
};
use ambition_sfx::{ids, OwnedSfxMessage, SfxMessage, SfxWriter};
use bevy::prelude::*;

use crate::audio_controls::ShellAudioControl;

use crate::{
    image_sequence_frame_at, shell_action_edges, ActiveShellSequence, FrontendOwnedEntity,
    FrontendPresentationKind, LauncherTab, ShellLaunchCatalog, ShellLauncherCommand,
    ShellLauncherPresentation,
    ShellLauncherState, ShellRouter, ShellSegmentPresentation, ShellSequenceCommand,
};

#[derive(Component)]
pub struct BasicSequenceRoot;

/// Marks the content of a vanity card (text or image), not its black backdrop.
/// [`drive_basic_sequence_card`] ramps its alpha so the card fades in and out.
#[derive(Component)]
pub struct BasicSequenceCardContent;

/// Every frame handle of an animated sequence, resolved once when the card
/// spawns. The card is short, so lazy loading could miss a frame's slot. The
/// animation swaps this node's texture and does not rebuild the card (see
/// [`shell_frame_key`]).
#[derive(Component)]
pub struct BasicSequenceImages {
    handles: Vec<Handle<Image>>,
}

/// The per-frame "this picture is missing" notice.
///
/// Sequence payloads are generated and git-ignored, so they can be absent. A
/// frame that fails to load shows a label for its own slot only. Timing does
/// not change.
#[derive(Component)]
pub struct BasicSequenceMissingNotice;

/// Seconds the vanity card fades in, and again fades out. The card holds at
/// full opacity between the fades.
const CARD_FADE_SECONDS: f32 = 0.55;

#[derive(Default)]
struct BasicSequenceFrame {
    key: String,
    text: String,
    image_path: Option<String>,
    /// Every frame path of an animated sequence; empty for still cards.
    sequence_paths: Vec<String>,
}

/// Marker on the basic shell presentation's own launcher menu root, so its
/// rebuild teardown never claims another producer's `BevyUiMenuRoot`.
#[derive(Component)]
pub struct BasicShellUiRoot;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BasicLauncherPage {
    Home,
    Settings,
}

/// Stable selectable index in the launcher's semantic selection space
/// (available routes first, then the optional Exit row).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BasicLauncherAction(usize);

/// The full-screen tap surface of a startup card. It acknowledges or skips the
/// card, the same command as keyboard or controller confirm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ShellCardAction;

#[derive(Default)]
pub struct BasicShellPresentationPlugin;

impl Plugin for BasicShellPresentationPlugin {
    fn build(&self, app: &mut App) {
        install_bevy_ui_menu_actions::<BasicLauncherAction>(app);
        // Installs `publish_bevy_ui_menu_tabs`, which turns a tab press into
        // `MenuTabActivated`. Without it, pointer presses on the tab strip do
        // nothing. Guarded by `the_shell_plugin_installs_the_tab_pointer_road`.
        ambition_menu::render::bevy_ui::install_bevy_ui_menu_tabs(app);
        install_bevy_ui_menu_actions::<ShellCardAction>(app);
        app.add_message::<OwnedSfxMessage>()
            .init_resource::<ambition_sfx::SfxEmissionContext>()
            .init_resource::<ActiveUiCues>()
            // This presentation owns its surface text, so it also publishes
            // the submit cues ("Continue", "Play", the exit label).
            .add_systems(Update, publish_shell_ui_cues.in_set(InputSet::PublishCues))
            // Input consumers: after every producer, in the same frame.
            .add_systems(
                Update,
                (
                    basic_shell_menu_intent,
                    basic_shell_pointer.after(BevyUiMenuInteractionSet),
                    basic_shell_card_tap.after(BevyUiMenuInteractionSet),
                    render_basic_shell,
                    // After the render: a rebuild spawns the correct cursor;
                    // on other frames only this system moves it.
                    follow_the_launcher_cursor,
                    drive_basic_sequence_card,
                )
                    .chain()
                    .in_set(InputSet::Consume),
            );
    }
}

/// Publish the shell surfaces' submit cues, keyed by their input contexts.
/// The startup cards say "Continue"; the launcher says the focused row's
/// verb ("Play" for an experience, the exit label for the Exit row).
fn publish_shell_ui_cues(
    launcher: Res<ShellLauncherState>,
    catalog: Res<ShellLaunchCatalog>,
    presentation: Res<ShellLauncherPresentation>,
    sequence: Res<ActiveShellSequence>,
    mut cues: ResMut<ActiveUiCues>,
) {
    let sequence_active = sequence.activation_id.is_some() && sequence.runtime.is_some();
    cues.sync(
        UiCue {
            context: STARTUP_ACKNOWLEDGE_CONTEXT,
            priority: context_priority::STARTUP_ACKNOWLEDGE,
            submit_label: "Continue".to_owned(),
        },
        sequence_active,
    );

    let available = catalog.entries.iter().filter(|e| e.available).count();
    let on_exit_row = presentation.exit_label.is_some() && launcher.selected >= available;
    let label = if on_exit_row {
        presentation
            .exit_label
            .clone()
            .unwrap_or_else(|| "Exit".to_owned())
    } else {
        "Play".to_owned()
    };
    cues.sync(
        UiCue {
            context: LAUNCHER_CONTEXT,
            priority: context_priority::LAUNCHER,
            submit_label: label,
        },
        launcher.active,
    );
}

/// Pointer/touch activation for launcher rows. The shared menu renderer turns
/// `Interaction::Pressed` into [`MenuActionActivated`]; this adapter routes the
/// selected row through the same [`ShellLauncherCommand`] processor used by
/// keyboard/controller confirmation.
fn basic_shell_pointer(
    launcher: Res<ShellLauncherState>,
    mut activated: MessageReader<MenuActionActivated<BasicLauncherAction>>,
    mut previewed: MessageReader<MenuActionPreviewed<BasicLauncherAction>>,
    mut tab_activated: MessageReader<ambition_menu::MenuTabActivated>,
    mut launcher_commands: MessageWriter<ShellLauncherCommand>,
    mut sfx: SfxWriter,
) {
    // Hover moves the cursor (`Focus`, not `Activate`). It is the same cursor
    // the keyboard moves, so hover then Enter launches the hovered row.
    for preview in previewed.read() {
        if !launcher.active {
            continue;
        }
        launcher_commands.write(ShellLauncherCommand::Focus(preview.action.0));
        // Same cue as a keyboard cursor move.
        sfx.write(SfxMessage::Play {
            id: ids::UI_MENU_MOVE,
            pos: Vec2::ZERO,
        });
    }
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

    // Pointer presses on the tab strip. `publish_bevy_ui_menu_tabs` writes
    // `MenuTabActivated`; this is its reader on the title screen. Keyboard tests
    // do not cover this path; `clicking_the_settings_tab_reaches_it` does.
    for tab in tab_activated.read() {
        if !launcher.active {
            continue;
        }
        launcher_commands.write(ShellLauncherCommand::SelectTab(tab.index));
        sfx.write(SfxMessage::Play {
            id: ids::UI_MENU_MOVE,
            pos: Vec2::ZERO,
        });
    }
}

/// Unified semantic menu input: keyboard, controller, and touch all arrive as
/// the same [`MenuControlFrame`] edges (populated from the persistent input
/// participant), so no downstream logic is duplicated per device and no raw
/// device is read here. A phone dismisses a startup card and picks a launcher
/// row with no keyboard attached; the launcher works before any gameplay
/// actor exists.
fn basic_shell_menu_intent(
    menu_frame: Option<Res<ambition_input::MenuControlFrame>>,
    launcher: Res<ShellLauncherState>,
    sequence: Res<ActiveShellSequence>,
    mut launcher_commands: MessageWriter<ShellLauncherCommand>,
    mut sequence_commands: MessageWriter<ShellSequenceCommand>,
    mut sfx: SfxWriter,
) {
    let actions = shell_action_edges(menu_frame.as_deref());
    let (up, down, confirm) = (actions.previous, actions.next, actions.confirm);
    // Bumpers cycle the tab strip, as on the kaleidoscope menu.
    let bump = menu_frame.as_deref().map_or(0, |frame| {
        (frame.page_right as i32) - (frame.page_left as i32)
    });
    if launcher.active {
        // Escape/Start must reach settings from the title screen (the pause
        // menu yields to the launcher, so this is the only way to mute here).
        // With two tabs, a cycle is a toggle. Guarded by
        // `shell_host_rendered::the_title_screen_menu_opens_and_mutes_the_game`.
        if actions.pause {
            launcher_commands.write(ShellLauncherCommand::CycleTab(1));
            sfx.write(SfxMessage::Play {
                id: ids::UI_MENU_MOVE,
                pos: Vec2::ZERO,
            });
            return;
        }
        if bump != 0 {
            launcher_commands.write(ShellLauncherCommand::CycleTab(bump));
            sfx.write(SfxMessage::Play {
                id: ids::UI_MENU_MOVE,
                pos: Vec2::ZERO,
            });
            return;
        }
        // On the settings tab, up/down move the cursor and left/right adjust
        // the focused control. Confirm does nothing, so it cannot launch a game.
        if launcher.tab == LauncherTab::Settings {
            let rows = ShellAudioControl::ALL.len();
            if up || down {
                let mut cursor = ambition_ui_nav::ListCursor::new(launcher.selected, rows);
                cursor.apply_directional(up, down);
                launcher_commands.write(ShellLauncherCommand::SelectRow(cursor.selected()));
                sfx.write(SfxMessage::Play {
                    id: ids::UI_MENU_MOVE,
                    pos: Vec2::ZERO,
                });
            }
            let adjust = menu_frame
                .as_deref()
                .map_or(0, |frame| (frame.right as i32) - (frame.left as i32));
            if adjust != 0 {
                launcher_commands.write(ShellLauncherCommand::AdjustSetting(adjust));
                sfx.write(SfxMessage::Play {
                    id: ids::UI_MENU_ACCEPT,
                    pos: Vec2::ZERO,
                });
            }
            return;
        }
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
    } else if confirm {
        advance_active_sequence(&sequence, &mut sequence_commands, &mut sfx);
    }
}

/// Acknowledge or skip the active card. Both confirm and a card tap use this.
fn advance_active_sequence(
    sequence: &ActiveShellSequence,
    sequence_commands: &mut MessageWriter<ShellSequenceCommand>,
    sfx: &mut SfxWriter,
) {
    let (Some(activation_id), Some(runtime)) = (sequence.activation_id, sequence.runtime.as_ref())
    else {
        return;
    };
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

/// A tap anywhere on a startup card advances the sequence with the same
/// command as keyboard or controller confirm.
fn basic_shell_card_tap(
    launcher: Res<ShellLauncherState>,
    sequence: Res<ActiveShellSequence>,
    mut activated: MessageReader<MenuActionActivated<ShellCardAction>>,
    mut sequence_commands: MessageWriter<ShellSequenceCommand>,
    mut sfx: SfxWriter,
) {
    for _tap in activated.read() {
        if launcher.active {
            continue;
        }
        advance_active_sequence(&sequence, &mut sequence_commands, &mut sfx);
    }
}

/// Rebuild the launcher or startup card when [`shell_frame_key`] changes.
fn render_basic_shell(
    mut commands: Commands,
    launcher: Res<ShellLauncherState>,
    catalog: Res<ShellLaunchCatalog>,
    launcher_presentation: Res<ShellLauncherPresentation>,
    sequence: Res<ActiveShellSequence>,
    router: Res<ShellRouter>,
    asset_server: Option<Res<AssetServer>>,
    // The font menus draw with; `None` keeps Bevy's ASCII-only default.
    menu_font: Option<Res<ambition_menu::render::bevy_ui::MenuFont>>,
    // `Option`: a thin composition may have no user settings.
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    sequence_roots: Query<Entity, With<BasicSequenceRoot>>,
    // Identity, not species: only THIS presentation's launcher tree. Other
    // `BevyUiMenuRoot` producers (a game's pause menu) coexist in the host.
    launcher_roots: Query<Entity, (With<BevyUiMenuRoot>, With<BasicShellUiRoot>)>,
    mut prior_key: Local<String>,
) {
    let frame_key = format!(
        "{:?}:{}",
        router.active.as_ref().map(|active| active.activation_id),
        shell_frame_key(
            &launcher,
            &catalog,
            &launcher_presentation,
            &sequence,
            settings.as_deref(),
        ),
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
            menu_font.as_deref(),
            settings.as_deref(),
            activation_id,
        );
        return;
    }

    let frame = sequence_frame(&sequence);
    if frame.text.is_empty() && frame.image_path.is_none() {
        return;
    }
    // Startup cards use the menu font. Bevy's default font draws boxes for
    // `·` and `—`. See `ambition_menu::render::bevy_ui::MenuFont`.
    let card_font = menu_font
        .as_deref()
        .and_then(|font| font.0.clone())
        .unwrap_or_default();
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
            // The whole card is one tap control (mouse or touch).
            // `basic_shell_card_tap` advances the sequence.
            Button,
            Interaction::default(),
            AmbitionMenuControl::<ShellCardAction> {
                kind: MenuControlKind::Action,
                action: Some(ShellCardAction),
                focus: MenuFocusKey {
                    row: 0,
                    col: 0,
                    order: 0,
                },
            },
            Name::new("basic shell sequence presentation"),
        ))
        .with_children(|root| {
            if let Some(handle) = frame
                .image_path
                .as_ref()
                .zip(asset_server.as_deref())
                .map(|(path, server)| server.load::<Image>(path.clone()))
            {
                // Start transparent; the fade system eases it in.
                let mut image = ImageNode::new(handle);
                image.color.set_alpha(0.0);
                let mut node = root.spawn((
                    image,
                    // Automatic height keeps the image aspect ratio.
                    Node {
                        width: Val::Percent(70.0),
                        height: Val::Auto,
                        max_height: Val::Percent(80.0),
                        ..default()
                    },
                    BasicSequenceCardContent,
                    Name::new("basic shell sequence image"),
                ));
                // Resolve every frame up front.
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
                // Empty until a frame fails to load.
                root.spawn((
                    Text::default(),
                    TextFont {
                        font: card_font.clone().into(),
                        font_size: FontSize::Px(24.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.55, 0.55).with_alpha(0.0)),
                    TextLayout::justify(Justify::Center),
                    BasicSequenceCardContent,
                    BasicSequenceMissingNotice,
                    Name::new("basic shell sequence missing-frame notice"),
                ));
            }
            if !frame.text.is_empty() {
                root.spawn((
                    Text::new(frame.text),
                    TextFont {
                        font: card_font.clone().into(),
                        font_size: FontSize::Px(28.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.92, 0.94, 1.0).with_alpha(0.0)),
                    TextLayout::justify(Justify::Center),
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
    menu_font: Option<&ambition_menu::render::bevy_ui::MenuFont>,
    settings: Option<&ambition_persistence::settings::UserSettings>,
    activation_id: crate::ShellActivationId,
) {
    let mut page = MenuPageModel::new(
        match launcher.tab {
            LauncherTab::Home => BasicLauncherPage::Home,
            LauncherTab::Settings => BasicLauncherPage::Settings,
        },
        presentation.title.clone(),
        MenuColor::rgba(0.015, 0.020, 0.055, 0.98),
    );
    // Text sizes are percentages of viewport height, like `x`/`y` (see
    // `MenuNode::Text`).
    page.text(
        50.0,
        8.0,
        5.6,
        presentation.title.clone(),
        MenuTextAlign::Center,
        MenuColor::WHITE,
    );
    // The settings tab is a different page body in the same menu. Both tabs
    // share the tab strip, view, and spawn below.
    if launcher.tab == LauncherTab::Settings {
        page.text(
            50.0,
            15.0,
            2.6,
            "Audio".to_owned(),
            MenuTextAlign::Center,
            MenuColor::rgba(0.75, 0.80, 0.95, 1.0),
        );
        for (index, control) in ShellAudioControl::ALL.iter().enumerate() {
            let focused = index == launcher.selected;
            page.control(
                MenuRect::new(22.0, 24.0 + index as f32 * 9.0, 56.0, 7.0),
                // Same kind as the pause menu uses for these rows.
                ambition_menu::MenuControlKind::Action,
                control.label().to_owned(),
                // Read the value from `UserSettings`, as the pause menu does.
                // Do not cache a copy.
                settings.map(|s| control.value(s)),
                focused,
                false,
                // The action index is the row position, as for game rows.
                Some(BasicLauncherAction(index)),
            );
        }
        page.text(
            50.0,
            86.0,
            1.9,
            "Left/Right adjusts · Bumpers change tab".to_owned(),
            MenuTextAlign::Center,
            MenuColor::rgba(0.65, 0.70, 0.85, 1.0),
        );
    } else if catalog.entries.is_empty() && presentation.exit_label.is_none() {
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
        // The cap applies only with few rows; many rows still share the height.
        let row_height = (66.0 / (catalog.entries.len() + exit_rows).max(1) as f32).min(16.0);
        let row_left = 12.0;
        let row_width = 76.0;
        let mut available_index = 0usize;
        for (index, entry) in catalog.entries.iter().enumerate() {
            let (kind, action, detail, selected) = if entry.available {
                let selected = available_index == launcher.selected;
                // The row carries its selection index, not its route, so a
                // pointer activation gives the same command as the cursor.
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
                    row_left,
                    18.0 + index as f32 * (row_height + 1.5),
                    row_width,
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
                    row_left,
                    18.0 + catalog.entries.len() as f32 * (row_height + 1.5),
                    row_width,
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
            // A footer stays smaller than the rows; it is not supposed to compete with them.
            page.text(
                50.0,
                92.0,
                2.2,
                presentation.footer.clone(),
                MenuTextAlign::Center,
                MenuColor::WHITE,
            );
        }
    }

    // Tab labels name the screen. The verb ("Play") is on the confirm button.
    let tabs = [
        BevyUiMenuTabSpec::new(BasicLauncherPage::Home, LauncherTab::Home.label()),
        BevyUiMenuTabSpec::new(BasicLauncherPage::Settings, LauncherTab::Settings.label()),
    ];
    let view = BevyUiMenuView::<BasicLauncherPage, BasicLauncherAction> {
        tabs: &tabs,
        active_tab: match launcher.tab {
            LauncherTab::Home => 0,
            LauncherTab::Settings => 1,
        },
        page: &page,
        focused: None,
        focused_tab: None,
    };
    let root = ambition_menu::render::bevy_ui::spawn_bevy_ui_menu_with_font(
        commands,
        &view,
        asset_server,
        menu_font.and_then(|font| font.0.as_ref()),
    );
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

/// Fade the vanity card content in and out from the sequence elapsed time, and
/// swap animated frames. The black backdrop does not fade.
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
        // A missing frame hides its own slot and shows the notice.
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

/// Move the launcher highlight in place.
///
/// Rows carry their selection index in `BasicLauncherAction(i)`. This writes
/// `MenuVisualState`, and `restyle_bevy_ui_menu_controls` recolours what
/// changed. It writes only on a real change, because any `&mut` deref marks
/// the row changed for the `Changed<..>` query.
fn follow_the_launcher_cursor(
    mut commands: Commands,
    launcher: Res<ShellLauncherState>,
    // `BasicLauncherAction` is private to this presentation, so the query
    // cannot reach another menu's rows.
    mut rows: Query<(
        Entity,
        &ambition_menu::AmbitionMenuControl<BasicLauncherAction>,
        &mut ambition_menu::MenuVisualState,
    )>,
) {
    if !launcher.active {
        return;
    }
    for (entity, control, mut visual) in &mut rows {
        let Some(BasicLauncherAction(index)) = control.action else {
            continue;
        };
        let selected = index == launcher.selected;
        if visual.selected == selected && visual.focused == selected {
            continue;
        }
        visual.selected = selected;
        visual.focused = selected;
        // Keep `BevyUiMenuFocused` on the selected row so future readers can
        // trust it.
        if selected {
            commands
                .entity(entity)
                .insert(ambition_menu::render::bevy_ui::BevyUiMenuFocused);
        } else {
            commands
                .entity(entity)
                .remove::<ambition_menu::render::bevy_ui::BevyUiMenuFocused>();
        }
    }
}

fn shell_frame_key(
    launcher: &ShellLauncherState,
    catalog: &ShellLaunchCatalog,
    presentation: &ShellLauncherPresentation,
    sequence: &ActiveShellSequence,
    settings: Option<&ambition_persistence::settings::UserSettings>,
) -> String {
    if launcher.active {
        // The key names only what needs a rebuild: which rows exist and what
        // they say. Include a field only if it changes which nodes exist or
        // their text.
        //
        // - `launcher.selected` is not included. The cursor is runtime state;
        //   `follow_the_launcher_cursor` moves the highlight in place.
        // - `launcher.tab` is included. It selects the game rows or the
        //   settings rows. Guarded by
        //   `switching_the_tab_redraws_the_menu_the_player_sees`.
        // - The audio values are included, because settings rows read their
        //   text at build time. They change only when the player adjusts one.
        //
        // This key is built from selected inputs, so a new field on
        // `ShellLauncherState` has no effect until it is added here.
        let audio = settings
            .map(|s| {
                ShellAudioControl::ALL
                    .iter()
                    .map(|c| c.value(s))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        return format!(
"launcher:{:?}:{}:{:?}:{audio}",
            launcher.tab, presentation.title, catalog.entries
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
        // Keyed on segment identity, not the current frame: the card animates
        // by swapping its texture, not by a rebuild.
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
mod semantic_input_tests {
    use super::*;
    use crate::{
        ActiveShellSequence, ShellActivationId, ShellLauncherState, ShellSequenceCommand,
        ShellSequenceRuntime, ShellSequenceSpec,
    };
    use ambition_input::MenuControlFrame;
    use bevy::prelude::{App, Messages, Update};

    fn app_with_launcher(active: bool) -> App {
        let mut app = App::new();
        app.add_message::<ShellLauncherCommand>();
        app.add_message::<ShellSequenceCommand>();
        app.add_message::<OwnedSfxMessage>();
        app.init_resource::<ambition_sfx::SfxEmissionContext>();
        app.world_mut()
            .resource_mut::<ambition_sfx::SfxEmissionContext>()
            .set(ambition_sfx::AudioContextOwner::Frontend(9), "shell.test");
        app.init_resource::<ShellLauncherState>();
        app.init_resource::<ActiveShellSequence>();
        app.init_resource::<MenuControlFrame>();
        app.add_systems(Update, basic_shell_menu_intent);
        app.world_mut().resource_mut::<ShellLauncherState>().active = active;
        app
    }

    /// Inject one semantic intent for exactly one frame — what keyboard,
    /// gamepad, and touch all reduce to before the shell reads input.
    fn intent(app: &mut App, set: impl Fn(&mut MenuControlFrame)) {
        {
            let mut frame = app.world_mut().resource_mut::<MenuControlFrame>();
            *frame = MenuControlFrame::default();
            set(&mut frame);
        }
        app.update();
        *app.world_mut().resource_mut::<MenuControlFrame>() = MenuControlFrame::default();
    }

    /// The title screen has a game tab and a settings tab. The bumpers cycle
    /// them, as on the kaleidoscope menu.
    #[test]
    fn the_bumpers_cycle_the_title_screen_tabs() {
        let mut app = app_with_launcher(true);
        intent(&mut app, |f| f.page_right = true);
        assert!(
            drained(&mut app)
                .iter()
                .any(|c| matches!(c, ShellLauncherCommand::CycleTab(1))),
            "the right bumper did not cycle the tab strip"
        );
        intent(&mut app, |f| f.page_left = true);
        assert!(
            drained(&mut app)
                .iter()
                .any(|c| matches!(c, ShellLauncherCommand::CycleTab(-1))),
            "the left bumper did not cycle the tab strip"
        );
    }

    /// Start/Escape on the title screen shows the settings tab. The pause menu
    /// yields to the launcher, so this is the only way to reach audio settings
    /// here. See also
    /// `shell_host_rendered::the_title_screen_menu_opens_and_mutes_the_game`.
    #[test]
    fn start_on_the_title_screen_reaches_the_settings_tab() {
        let mut app = app_with_launcher(true);
        intent(&mut app, |f| f.start = true);
        assert!(
            drained(&mut app)
                .iter()
                .any(|c| matches!(c, ShellLauncherCommand::CycleTab(1))),
            "Start on the title screen must reach settings, not do nothing"
        );
    }

    /// Start toggles: with two tabs, a cycle returns to the game list.
    #[test]
    fn start_toggles_back_to_the_game_list() {
        let mut app = app_with_launcher(true);
        {
            let mut state = app.world_mut().resource_mut::<ShellLauncherState>();
            state.tab = LauncherTab::Settings;
        }
        // `cycled` is the arithmetic the command applies; assert the round trip
        // rather than re-implementing it here.
        assert_eq!(LauncherTab::Settings.cycled(1), LauncherTab::Home);
        intent(&mut app, |f| f.start = true);
        assert!(drained(&mut app)
            .iter()
            .any(|c| matches!(c, ShellLauncherCommand::CycleTab(1))));
    }

    /// Confirm on the settings tab must not start a game.
    #[test]
    fn confirm_on_the_settings_tab_does_not_launch_a_game() {
        let mut app = app_with_launcher(true);
        app.world_mut().resource_mut::<ShellLauncherState>().tab = LauncherTab::Settings;
        intent(&mut app, |f| f.select = true);
        let commands = drained(&mut app);
        assert!(
            !commands
                .iter()
                .any(|c| matches!(c, ShellLauncherCommand::LaunchSelected)),
            "confirm on the settings tab tried to launch: {commands:?}"
        );
    }

    /// Left/right adjust on the settings tab and do nothing on the game list.
    #[test]
    fn left_and_right_adjust_only_on_the_settings_tab() {
        let mut app = app_with_launcher(true);
        app.world_mut().resource_mut::<ShellLauncherState>().tab = LauncherTab::Settings;
        intent(&mut app, |f| f.right = true);
        assert!(
            drained(&mut app)
                .iter()
                .any(|c| matches!(c, ShellLauncherCommand::AdjustSetting(1))),
            "right did not adjust the focused control"
        );

        let mut home = app_with_launcher(true);
        intent(&mut home, |f| f.right = true);
        assert!(
            !drained(&mut home)
                .iter()
                .any(|c| matches!(c, ShellLauncherCommand::AdjustSetting(_))),
            "the game list has nothing to adjust"
        );
    }

    /// A bumper during a startup card must not move a hidden tab.
    #[test]
    fn the_bumpers_do_nothing_while_the_launcher_is_closed() {
        let mut app = app_with_launcher(false);
        intent(&mut app, |f| f.page_right = true);
        assert!(drained(&mut app).is_empty());
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

    fn with_active_card(app: &mut App) {
        *app.world_mut().resource_mut::<ActiveShellSequence>() = ActiveShellSequence {
            activation_id: Some(ShellActivationId(1)),
            runtime: Some(ShellSequenceRuntime::new(ShellSequenceSpec {
                segments: vec![crate::ShellSegmentSpec::text("card", "Card")],
            })),
        };
    }

    #[test]
    fn nav_intent_moves_the_launcher_cursor() {
        let mut app = app_with_launcher(true);
        intent(&mut app, |f| f.down = true);
        assert_eq!(drained(&mut app), vec![ShellLauncherCommand::Next]);
        let sfx = drained_sfx(&mut app);
        assert!(matches!(
            sfx.as_slice(),
            [OwnedSfxMessage {
                owner: Some(ambition_sfx::AudioContextOwner::Frontend(9)),
                source,
                request: SfxMessage::Play { id, .. },
            }] if source.as_str() == "shell.test" && *id == ids::UI_MENU_MOVE
        ));
        intent(&mut app, |f| f.up = true);
        assert_eq!(drained(&mut app), vec![ShellLauncherCommand::Previous]);
        let _ = drained_sfx(&mut app);
    }

    #[test]
    fn the_select_intent_confirms_the_selection() {
        let mut app = app_with_launcher(true);
        intent(&mut app, |f| f.select = true);
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
    }

    #[test]
    fn intent_is_inert_when_launcher_is_not_active() {
        let mut app = app_with_launcher(false);
        intent(&mut app, |f| f.down = true);
        assert!(
            drained(&mut app).is_empty(),
            "no launcher command when the launcher is not focused"
        );
    }

    fn drained_sequence(app: &mut App) -> Vec<ShellSequenceCommand> {
        app.world_mut()
            .resource_mut::<Messages<ShellSequenceCommand>>()
            .drain()
            .collect()
    }

    /// Input parity on the startup card: the semantic confirm intent and a
    /// direct tap on the card surface produce the SAME sequence command.
    #[test]
    fn confirm_and_direct_card_tap_advance_the_card_identically() {
        let mut app = app_with_launcher(false);
        with_active_card(&mut app);
        intent(&mut app, |f| f.select = true);
        let confirmed = drained_sequence(&mut app);
        assert!(
            matches!(confirmed.as_slice(), [ShellSequenceCommand::Skip { .. }]),
            "confirm on a card with no acknowledgement requirement skips it"
        );

        // The tap path must give the same command. The bridge activates on
        // release, so a tap is `Pressed` and then `Hovered`.
        with_active_card(&mut app);
        install_bevy_ui_menu_actions::<ShellCardAction>(&mut app);
        app.add_systems(Update, basic_shell_card_tap.after(BevyUiMenuInteractionSet));
        let card = app
            .world_mut()
            .spawn((
                Button,
                Interaction::Pressed,
                AmbitionMenuControl::<ShellCardAction> {
                    kind: MenuControlKind::Action,
                    action: Some(ShellCardAction),
                    focus: MenuFocusKey {
                        row: 0,
                        col: 0,
                        order: 0,
                    },
                },
            ))
            .id();
        app.update();
        assert!(
            drained_sequence(&mut app).is_empty(),
            "the finger going down on a card has not advanced it yet"
        );
        app.world_mut()
            .entity_mut(card)
            .insert(Interaction::Hovered);
        app.update();
        let tapped = drained_sequence(&mut app);
        assert_eq!(
            tapped, confirmed,
            "a direct tap emits the same semantic command as confirm"
        );
    }

    #[test]
    fn cues_name_the_focused_verb_per_surface() {
        let mut app = App::new();
        app.init_resource::<ShellLauncherState>();
        app.init_resource::<ShellLaunchCatalog>();
        app.init_resource::<ShellLauncherPresentation>();
        app.init_resource::<ActiveShellSequence>();
        app.init_resource::<ActiveUiCues>();
        app.add_systems(Update, publish_shell_ui_cues);

        // Nothing active: no cues.
        app.update();
        assert!(app.world().resource::<ActiveUiCues>().top().is_none());

        // An active card publishes "Continue" for the startup context.
        with_active_card(&mut app);
        app.update();
        assert_eq!(
            app.world()
                .resource::<ActiveUiCues>()
                .for_context(STARTUP_ACKNOWLEDGE_CONTEXT)
                .map(|c| c.submit_label.as_str()),
            Some("Continue")
        );

        // The launcher publishes "Play" on an experience row and the exit
        // label on the Exit row.
        *app.world_mut().resource_mut::<ActiveShellSequence>() = ActiveShellSequence::default();
        app.world_mut().resource_mut::<ShellLauncherState>().active = true;
        app.world_mut()
            .resource_mut::<ShellLauncherPresentation>()
            .exit_label = Some("Exit Ambition".to_owned());
        app.update();
        assert_eq!(
            app.world()
                .resource::<ActiveUiCues>()
                .for_context(LAUNCHER_CONTEXT)
                .map(|c| c.submit_label.as_str()),
            Some("Exit Ambition"),
            "an empty catalog leaves only the Exit row selected"
        );
        assert!(
            app.world()
                .resource::<ActiveUiCues>()
                .for_context(STARTUP_ACKNOWLEDGE_CONTEXT)
                .is_none(),
            "the retired card retracted its cue"
        );
    }
}

#[cfg(test)]
mod pointer_hover_tests {
    use super::*;
    use crate::{ShellLauncherCommand, ShellLauncherState};
    use bevy::prelude::{App, Messages, Update};

    fn app_with_pointer(active: bool) -> App {
        let mut app = App::new();
        app.add_message::<ShellLauncherCommand>();
        app.add_message::<MenuActionActivated<BasicLauncherAction>>();
        app.add_message::<MenuActionPreviewed<BasicLauncherAction>>();
        app.add_message::<OwnedSfxMessage>();
        // `basic_shell_pointer` reads this; a missing `Messages<T>` panics.
        // The plugin installs it through `install_bevy_ui_menu_tabs`.
        app.add_message::<ambition_menu::MenuTabActivated>();
        app.init_resource::<ambition_sfx::SfxEmissionContext>();
        app.world_mut()
            .resource_mut::<ambition_sfx::SfxEmissionContext>()
            .set(ambition_sfx::AudioContextOwner::Frontend(9), "shell.test");
        app.init_resource::<ShellLauncherState>();
        app.add_systems(Update, basic_shell_pointer);
        app.world_mut().resource_mut::<ShellLauncherState>().active = active;
        app
    }

    /// The real plugin installs the tab pointer path. `app_with_pointer` wires
    /// the handler itself, so the tests below do not prove this.
    #[test]
    fn the_shell_plugin_installs_the_tab_pointer_road() {
        let mut app = App::new();
        app.add_plugins(BasicShellPresentationPlugin);
        assert!(
            app.world()
                .contains_resource::<bevy::ecs::message::Messages<ambition_menu::MenuTabActivated>>(),
            "the shell plugin does not install the tab pointer road, so its tab \
             strip is drawn as buttons that reach no system — reachable only by \
             Escape/Start"
        );
    }

    /// A click or tap on the Settings tab selects it.
    ///
    /// Keyboard and controller tests use `MenuControlFrame` and cannot see the
    /// pointer path. This test sends only what a pointer sends.
    #[test]
    fn clicking_the_settings_tab_reaches_it() {
        let mut app = app_with_pointer(true);
        // What `publish_bevy_ui_menu_tabs` writes when a tab is pressed and
        // released on itself. Index 1 is Settings (`LauncherTab::ALL`).
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ambition_menu::MenuTabActivated>>()
            .write(ambition_menu::MenuTabActivated { index: 1 });
        app.update();
        let commands = drained(&mut app);
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, ShellLauncherCommand::SelectTab(1))),
            "a pointer activation of the Settings tab produced no command, so the \
             tab is reachable only by Escape/Start: {commands:?}"
        );
    }

    /// A click names the tab (`SelectTab`), not a `CycleTab` step. Tab
    /// arithmetic stays on `LauncherTab`. A click on the current tab does not
    /// move the strip.
    #[test]
    fn clicking_the_tab_you_are_on_does_not_move_the_strip() {
        let mut app = app_with_pointer(true);
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ambition_menu::MenuTabActivated>>()
            .write(ambition_menu::MenuTabActivated { index: 0 });
        app.update();
        let commands = drained(&mut app);
        assert!(
            !commands
                .iter()
                .any(|c| matches!(c, ShellLauncherCommand::CycleTab(_))),
            "a click was answered with a CYCLE, so the strip steps from wherever \
             the cursor happened to be instead of going where it was pointed"
        );
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, ShellLauncherCommand::SelectTab(0))),
            "clicking the active tab published nothing at all"
        );
    }

    fn drained(app: &mut App) -> Vec<ShellLauncherCommand> {
        app.world_mut()
            .resource_mut::<Messages<ShellLauncherCommand>>()
            .drain()
            .collect()
    }

    /// Hover (`MenuActionPreviewed`) moves the cursor to the row.
    #[test]
    fn hovering_a_launcher_row_moves_the_cursor_to_it() {
        let mut app = app_with_pointer(true);
        app.world_mut().write_message(MenuActionPreviewed {
            action: BasicLauncherAction(2),
        });
        app.update();
        assert_eq!(
            drained(&mut app),
            vec![ShellLauncherCommand::Focus(2)],
            "hovering a row published nothing, so the highlight stays wherever the \
             keyboard last left it and the pointer is decoration"
        );
    }

    /// Hovering is not choosing: hover must not launch a game.
    #[test]
    fn hovering_a_launcher_row_does_not_launch_it() {
        let mut app = app_with_pointer(true);
        app.world_mut().write_message(MenuActionPreviewed {
            action: BasicLauncherAction(1),
        });
        app.update();
        let commands = drained(&mut app);
        assert!(
            !commands
                .iter()
                .any(|c| matches!(c, ShellLauncherCommand::Activate(_))),
            "a hover launched a game: {commands:?}"
        );
    }

    /// A press still launches.
    #[test]
    fn pressing_a_launcher_row_still_activates_it() {
        let mut app = app_with_pointer(true);
        app.world_mut().write_message(MenuActionActivated {
            action: BasicLauncherAction(1),
        });
        app.update();
        assert_eq!(drained(&mut app), vec![ShellLauncherCommand::Activate(1)]);
    }

    /// A hover while a startup card shows must not move the hidden cursor.
    #[test]
    fn a_hover_while_the_launcher_is_inactive_is_ignored() {
        let mut app = app_with_pointer(false);
        app.world_mut().write_message(MenuActionPreviewed {
            action: BasicLauncherAction(2),
        });
        app.world_mut().write_message(MenuActionActivated {
            action: BasicLauncherAction(2),
        });
        app.update();
        assert!(drained(&mut app).is_empty());
    }
}

#[cfg(test)]
mod cursor_moves_without_a_rebuild_tests {
    use super::*;
    use crate::ShellLauncherState;
    use ambition_menu::{AmbitionMenuControl, MenuControlKind, MenuFocusKey, MenuVisualState};
    use bevy::prelude::{App, Entity, Update};

    /// Two launcher rows, as `render_basic_shell` spawns them, each with its
    /// selection index in its action.
    fn app_with_two_rows() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<ShellLauncherState>();
        app.add_systems(Update, follow_the_launcher_cursor);
        app.world_mut().resource_mut::<ShellLauncherState>().active = true;

        let row = |app: &mut App, index: usize, selected: bool| {
            app.world_mut()
                .spawn((
                    AmbitionMenuControl {
                        kind: MenuControlKind::Action,
                        action: Some(BasicLauncherAction(index)),
                        focus: MenuFocusKey::default(),
                    },
                    MenuVisualState {
                        selected,
                        focused: selected,
                        ..Default::default()
                    },
                ))
                .id()
        };
        let first = row(&mut app, 0, true);
        let second = row(&mut app, 1, false);
        (app, first, second)
    }

    fn selected(app: &App, entity: Entity) -> bool {
        app.world()
            .get::<MenuVisualState>(entity)
            .expect("the row still exists")
            .selected
    }

    /// The cursor moves and the rows stay the same entities.
    #[test]
    fn moving_the_cursor_restyles_the_existing_rows_instead_of_respawning_them() {
        let (mut app, first, second) = app_with_two_rows();
        app.update();
        assert!(selected(&app, first), "the cursor starts on row 0");
        assert!(!selected(&app, second));

        app.world_mut()
            .resource_mut::<ShellLauncherState>()
            .selected = 1;
        app.update();

        assert!(!selected(&app, first), "the cursor left row 0");
        assert!(selected(&app, second), "and arrived at row 1");
        // Bevy recycles indices, so check the original ids, not a count.
        assert!(app.world().get::<MenuVisualState>(first).is_some());
        assert!(app.world().get::<MenuVisualState>(second).is_some());
    }

    /// A cursor move does not change the frame key, so it does not rebuild.
    #[test]
    fn the_frame_key_does_not_change_when_only_the_cursor_moves() {
        use crate::{ActiveShellSequence, ShellLaunchCatalog, ShellLauncherPresentation};

        let catalog = ShellLaunchCatalog::default();
        let presentation = ShellLauncherPresentation::default();
        let sequence = ActiveShellSequence::default();
        let mut launcher = ShellLauncherState {
            active: true,
            ..Default::default()
        };

        let at_row_0 = shell_frame_key(&launcher, &catalog, &presentation, &sequence, None);
        launcher.selected = 3;
        let at_row_3 = shell_frame_key(&launcher, &catalog, &presentation, &sequence, None);

        assert_eq!(
            at_row_0, at_row_3,
            "the cursor is runtime state, not structure — a frame key that moves \
             with it despawns and respawns every node in the launcher on every \
             arrow press"
        );
    }

    /// Control for the test above: the key changes when the rows change.
    #[test]
    fn the_frame_key_still_changes_when_the_rows_do() {
        use crate::{ActiveShellSequence, ShellLaunchCatalog, ShellLauncherPresentation};

        let catalog = ShellLaunchCatalog::default();
        let sequence = ActiveShellSequence::default();
        let launcher = ShellLauncherState {
            active: true,
            ..Default::default()
        };
        let before = shell_frame_key(
            &launcher,
            &catalog,
            &ShellLauncherPresentation::default(),
            &sequence,
            None,
        );
        let after = shell_frame_key(
            &launcher,
            &catalog,
            &ShellLauncherPresentation {
                title: "A different title".to_owned(),
                ..Default::default()
            },
            &sequence,
            None,
        );
        assert_ne!(before, after, "a real structural change still rebuilds");
    }

    /// The `BevyUiMenuFocused` marker moves with the highlight.
    #[test]
    fn the_cursor_marker_moves_with_the_highlight() {
        let (mut app, first, second) = app_with_two_rows();
        app.world_mut()
            .resource_mut::<ShellLauncherState>()
            .selected = 1;
        app.update();
        assert!(
            app.world()
                .get::<ambition_menu::render::bevy_ui::BevyUiMenuFocused>(second)
                .is_some(),
            "the marker followed the cursor to row 1"
        );
        assert!(
            app.world()
                .get::<ambition_menu::render::bevy_ui::BevyUiMenuFocused>(first)
                .is_none(),
            "and left row 0"
        );
    }
}
