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

use crate::{
    image_sequence_frame_at, shell_action_edges, ActiveShellSequence, FrontendOwnedEntity,
    FrontendPresentationKind, ShellLaunchCatalog, ShellLauncherCommand, ShellLauncherPresentation,
    ShellLauncherState, ShellRouter, ShellSegmentPresentation, ShellSequenceCommand,
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

/// The full-screen tap-anywhere surface of a startup/vanity card. One
/// semantic activation: acknowledge (or skip) the card — the same command
/// keyboard/controller confirm fires.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ShellCardAction;

#[derive(Default)]
pub struct BasicShellPresentationPlugin;

impl Plugin for BasicShellPresentationPlugin {
    fn build(&self, app: &mut App) {
        install_bevy_ui_menu_actions::<BasicLauncherAction>(app);
        install_bevy_ui_menu_actions::<ShellCardAction>(app);
        app.add_message::<OwnedSfxMessage>()
            .init_resource::<ambition_sfx::SfxEmissionContext>()
            .init_resource::<ActiveUiCues>()
            // This presentation owns the words on its surfaces, so it also
            // publishes their submit cues ("Continue", "Play", the exit
            // label) for the prompt fold and the touch confirm button.
            .add_systems(Update, publish_shell_ui_cues.in_set(InputSet::PublishCues))
            // Consumers of the routed input semantics: after every producer,
            // same frame.
            .add_systems(
                Update,
                (
                    basic_shell_menu_intent,
                    basic_shell_pointer.after(BevyUiMenuInteractionSet),
                    basic_shell_card_tap.after(BevyUiMenuInteractionSet),
                    render_basic_shell,
                    // AFTER the render: on a rebuild frame the tree spawns with
                    // the cursor already correct, and on every other frame this
                    // is the only thing that moves it.
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
    mut launcher_commands: MessageWriter<ShellLauncherCommand>,
    mut sfx: SfxWriter,
) {
    // HOVER moves the cursor.
    //
    // Hovering is not choosing, so this is `Focus` rather than `Activate`: a
    // launcher that started a game because the pointer crossed a row on its way
    // somewhere else would be unusable. It lands in the SAME cursor the keyboard
    // moves, so hover-then-Enter does what it looks like it will.
    for preview in previewed.read() {
        if !launcher.active {
            continue;
        }
        launcher_commands.write(ShellLauncherCommand::Focus(preview.action.0));
        // The same cue the cursor makes when a key moves it. A row that
        // highlights silently under the mouse and clicks under the keyboard is
        // two different menus.
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
    } else if confirm {
        advance_active_sequence(&sequence, &mut sequence_commands, &mut sfx);
    }
}

/// Acknowledge (or skip) the active card — the ONE semantic advance both the
/// confirm intent and a direct tap on the card converge on.
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

/// Tap-anywhere on a startup/vanity card: the card's full-screen surface is a
/// pressable control, and its activation advances the sequence through the
/// SAME semantic command as keyboard/controller confirm — not a special case.
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
    // The font menus draw with; `None` keeps Bevy's ASCII-only default.
    menu_font: Option<Res<ambition_menu::render::bevy_ui::MenuFont>>,
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
            menu_font.as_deref(),
            activation_id,
        );
        return;
    }

    let frame = sequence_frame(&sequence);
    if frame.text.is_empty() && frame.image_path.is_none() {
        return;
    }
    // Startup cards render AUTHORED prose, so they need the same typeface the
    // launcher below gets. Left at `TextFont::default()` they resolved Bevy's
    // built-in `FiraMono-subset.ttf`, which drew hollow boxes for `·` and `—`
    // in every menu until `MenuFont` existed. `None` still means that font; see
    // `ambition_menu::render::bevy_ui::MenuFont`, which records what about it is
    // proven and what is not.
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
            // The whole card is one tap-anywhere control: bevy's
            // `ui_focus_system` presses it from mouse OR touch, the shared
            // interaction bridge publishes the semantic activation, and
            // `basic_shell_card_tap` advances the sequence — the same command
            // path as keyboard/controller confirm.
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
    activation_id: crate::ShellActivationId,
) {
    let mut page = MenuPageModel::new(
        BasicLauncherPage::Home,
        presentation.title.clone(),
        MenuColor::rgba(0.015, 0.020, 0.055, 0.98),
    );
    // Sizes are PERCENTAGES OF VIEWPORT HEIGHT, like the `x`/`y` beside them —
    // see `MenuNode::Text`. These three were always authored that way and were
    // always right; the `bevy_ui` backend was reading them as pixels and
    // drawing this title FIVE PIXELS tall. It now spawns them as
    // `FontSize::Vh`, which is that unit, so the engine resolves them.
    page.text(
        50.0,
        8.0,
        5.6,
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
        // The cap only binds when there are FEW experiences, which is the case
        // that was too small: three games shared a budget sized for eight. With
        // many rows the divisor still wins and nothing overflows, so this makes
        // the common launcher bigger without making a full one break.
        let row_height = (66.0 / (catalog.entries.len() + exit_rows).max(1) as f32).min(16.0);
        let row_left = 12.0;
        let row_width = 76.0;
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

    // screen, where nothing is being played yet — the verb belongs on the
    // confirm button (which still says "Play" on an experience row), and the
    // heading should say what the screen is for.
    let tabs = [BevyUiMenuTabSpec::new(
        BasicLauncherPage::Home,
        "Choose Game",
    )];
    let view = BevyUiMenuView::<BasicLauncherPage, BasicLauncherAction> {
        tabs: &tabs,
        active_tab: 0,
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

/// Move the launcher highlight in place.
///
/// The rows already carry their selection index — `BasicLauncherAction(i)`, put
/// there so pointer activation lands in the same command the cursor produces —
/// so nothing new has to be tracked. This writes `MenuVisualState`, and the
/// menu crate's `restyle_bevy_ui_menu_controls` recolours what changed.
///
/// writes only on a real change. Bevy stamps the change tick on any `&mut`
/// deref, so touching every row every frame would defeat the `Changed<..>` query
/// this is paired with and restore the churn in a quieter form.
fn follow_the_launcher_cursor(
    mut commands: Commands,
    launcher: Res<ShellLauncherState>,
    // No extra marker: `AmbitionMenuControl<BasicLauncherAction>` is already
    // this presentation's own action type, so the query cannot reach another
    // menu's rows.
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
        // Nothing outside the menu crate reads it today, which is exactly why leaving it pointing
        // at the wrong row would be a trap rather than a bug — the first reader to trust it would
        // be wrong.
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
) -> String {
    if launcher.active {
        // `launcher.selected` is DELIBERATELY not here. It was, and an
        // arrow press therefore despawned and respawned every node in the
        // launcher — throwing away hover state and any per-frame animation, and
        // making a one-frame text defect visible as a whole-UI blink.
        //
        // The cursor is runtime state, not structure. `follow_the_launcher_cursor`
        // moves the highlight in place through `MenuVisualState`, and
        // `restyle_bevy_ui_menu_controls` recolours what changed. This key names
        // only what a REBUILD is actually needed for: which rows exist and what
        // they say.
        return format!("launcher:{}:{:?}", presentation.title, catalog.entries);
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

        // The tap path: the card surface's pointer press flows through the
        // shared bridge into the SAME consumer command.
        //
        // press THEN release. The bridge activates on the way up, so a tap is
        // two `Interaction` states — `Pressed`, then the `Hovered` Bevy reports
        // when a pointer comes up still over the control.
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
        app.init_resource::<ambition_sfx::SfxEmissionContext>();
        app.world_mut()
            .resource_mut::<ambition_sfx::SfxEmissionContext>()
            .set(ambition_sfx::AudioContextOwner::Frontend(9), "shell.test");
        app.init_resource::<ShellLauncherState>();
        app.add_systems(Update, basic_shell_pointer);
        app.world_mut().resource_mut::<ShellLauncherState>().active = active;
        app
    }

    fn drained(app: &mut App) -> Vec<ShellLauncherCommand> {
        app.world_mut()
            .resource_mut::<Messages<ShellLauncherCommand>>()
            .drain()
            .collect()
    }

    /// It did nothing because the renderer translated only `Interaction::Pressed`.
    /// `MenuActionPreviewed` was defined, documented as the hover message, and had
    /// no emitter and no reader anywhere in the tree — a vocabulary with no
    /// customer, which reads as a feature right up until somebody moves a mouse.
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

    /// Hovering is not choosing. A launcher that started a game because the
    /// pointer crossed a row on its way somewhere else would be unusable, and
    /// that is why hover is a separate command rather than a flag on activation.
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

    /// A press still launches — the point is to ADD hover, not to replace the
    /// click with it.
    #[test]
    fn pressing_a_launcher_row_still_activates_it() {
        let mut app = app_with_pointer(true);
        app.world_mut().write_message(MenuActionActivated {
            action: BasicLauncherAction(1),
        });
        app.update();
        assert_eq!(drained(&mut app), vec![ShellLauncherCommand::Activate(1)]);
    }

    /// The launcher is not the only surface on screen. A hover arriving while a
    /// startup card is up must not move a cursor nobody can see.
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

    /// Two launcher rows, as `render_basic_shell` spawns them: each carrying its
    /// SELECTION index in its action, which is what lets the cursor be applied
    /// without knowing anything about the page that built them.
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

    /// The cursor moves and the rows are the SAME entities.
    ///
    /// this is the whole row.
    ///
    /// Asserting the ENTITY IDS is the point.
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
        // The same two entities answered before and after. Bevy recycles indices,
        // so this is `get` on the original ids rather than a count.
        assert!(app.world().get::<MenuVisualState>(first).is_some());
        assert!(app.world().get::<MenuVisualState>(second).is_some());
    }

    /// And the REBUILD is what actually went away.
    ///
    /// If it comes back, the in-place path still works and the churn returns silently — so the key
    /// itself is the assertion.
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

        let at_row_0 = shell_frame_key(&launcher, &catalog, &presentation, &sequence);
        launcher.selected = 3;
        let at_row_3 = shell_frame_key(&launcher, &catalog, &presentation, &sequence);

        assert_eq!(
            at_row_0, at_row_3,
            "the cursor is runtime state, not structure — a frame key that moves \
             with it despawns and respawns every node in the launcher on every \
             arrow press"
        );
    }

    /// The control: the key DOES move when the rows themselves change, so the
    /// test above is not passing on a key that never changes at all.
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
        );
        let after = shell_frame_key(
            &launcher,
            &catalog,
            &ShellLauncherPresentation {
                title: "A different title".to_owned(),
                ..Default::default()
            },
            &sequence,
        );
        assert_ne!(before, after, "a real structural change still rebuilds");
    }

    /// The marker that says "this is the cursor" moves with the state.
    ///
    /// nothing outside the menu crate reads `BevyUiMenuFocused` today, which
    /// is exactly why a stale one would be a trap rather than a bug: the first
    /// reader to trust it would be wrong, and nothing would have told them.
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
