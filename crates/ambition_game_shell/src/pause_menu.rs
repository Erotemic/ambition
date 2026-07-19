//! The universal in-session pause menu the host offers every experience.
//!
//! A hosted experience that brings no pause chrome of its own (Sanic, Mary-O,
//! the pocket demo) still needs the three-verb minimum: **Resume**, **Quit to
//! Title**, **Quit to Desktop**. Rather than each demo hand-rolling a menu, the
//! shell offers ONE — opened with Escape / Start while a gameplay session is
//! live, drawn with the same `ambition_menu` Bevy-UI renderer the launcher uses,
//! and dispatched to the same host-relative [`ShellCommand`]s (`QuitToHome`,
//! `ExitProcess`) the launcher and F10 already fire. Because it rides
//! [`MinimalShellPlugins`](crate::MinimalShellPlugins), the standalone demo apps
//! AND the multi-game host get it for free.
//!
//! ## Coexistence with a game's own pause menu
//!
//! Ambition's gameplay has a richer pause menu (the kaleidoscope), so this shell
//! menu must yield there. It does, via [`ShellPauseMenuSuppressed`]: the host
//! sets it while Ambition's own rooms are active (its `in_base_mode` signal), so
//! the shell menu runs for exactly the sessions the kaleidoscope does NOT — the
//! two partition every live session with no overlap. In a standalone demo app the
//! flag stays `false` and the shell menu is the pause menu.

use ambition_menu::render::bevy_ui::{
    install_bevy_ui_menu_actions, spawn_bevy_ui_menu_with_assets, BevyUiMenuInteractionSet,
    BevyUiMenuRoot, BevyUiMenuTabSpec, BevyUiMenuView,
};
use ambition_menu::{
    MenuActionActivated, MenuColor, MenuControlKind, MenuPageModel, MenuRect, MenuTextAlign,
};
use ambition_platformer_primitives::schedule::GameMode;
use ambition_sfx::{ids, OwnedSfxMessage, SfxMessage, SfxWriter};
use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;

use crate::{shell_action_edges, ActiveGameplaySession, ShellAnalogLatch, ShellCommand};

/// The three universal entries, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PauseEntry {
    Resume,
    QuitToTitle,
    QuitToDesktop,
}

impl PauseEntry {
    const ALL: [PauseEntry; 3] = [
        PauseEntry::Resume,
        PauseEntry::QuitToTitle,
        PauseEntry::QuitToDesktop,
    ];

    fn label(self) -> &'static str {
        match self {
            PauseEntry::Resume => "Resume",
            PauseEntry::QuitToTitle => "Quit to Title",
            PauseEntry::QuitToDesktop => "Quit to Desktop",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            PauseEntry::Resume => "Return to the game.",
            PauseEntry::QuitToTitle => "Leave this session and return to the title screen.",
            PauseEntry::QuitToDesktop => "Exit the game.",
        }
    }
}

/// The pause menu's open state + cursor. Cursor indexes [`PauseEntry::ALL`].
#[derive(Resource, Default)]
pub struct ShellPauseMenu {
    pub open: bool,
    cursor: usize,
}

/// Host-set gate: when `true`, the shell pause menu yields to the active
/// experience's OWN pause chrome (e.g. Ambition's kaleidoscope). Defaults to
/// `false`, so a standalone demo app — which has no other pause menu — always
/// gets it. The multi-game host drives this from its `in_base_mode` signal.
#[derive(Resource, Default)]
pub struct ShellPauseMenuSuppressed(pub bool);

/// Page id for the single-page pause menu model.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PausePage {
    Root,
}

/// Marks the pause menu's UI root so its rebuild teardown never claims another
/// `BevyUiMenuRoot` producer (the launcher, a game's own menu).
#[derive(Component)]
struct ShellPauseMenuRoot;

/// Adds the universal in-session pause menu. Rides [`MinimalShellPlugins`], so
/// every host and standalone demo app gets it.
pub struct ShellPauseMenuPlugin;

impl Plugin for ShellPauseMenuPlugin {
    fn build(&self, app: &mut App) {
        install_bevy_ui_menu_actions::<PauseEntry>(app);
        app.init_resource::<ShellPauseMenu>()
            .init_resource::<ShellPauseMenuSuppressed>()
            .add_message::<OwnedSfxMessage>()
            .init_resource::<ambition_sfx::SfxEmissionContext>()
            .add_systems(
                Update,
                (
                    drive_shell_pause_menu,
                    shell_pause_menu_pointer.after(BevyUiMenuInteractionSet),
                    render_shell_pause_menu,
                )
                    .chain(),
            );
    }
}

/// Input + state for the pause menu: open/close on Escape/Start, navigate, and
/// dispatch the selected entry. Pausing the sim ([`GameMode::Paused`]) is
/// best-effort — a demo that does not register the `GameMode` state simply keeps
/// running behind the menu, which stays fully functional either way.
#[allow(clippy::too_many_arguments)]
fn drive_shell_pause_menu(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    pads: Query<&Gamepad>,
    // The device-agnostic menu seam. Absent in an app with no host input stack;
    // present (and touch-fed) in every windowed host, which is what lets the
    // on-screen "Menu" button open this menu on a phone.
    menu_frame: Option<Res<ambition_input::MenuControlFrame>>,
    session: Res<ActiveGameplaySession>,
    suppressed: Res<ShellPauseMenuSuppressed>,
    mut menu: ResMut<ShellPauseMenu>,
    mut shell: MessageWriter<ShellCommand>,
    game_mode: Option<Res<State<GameMode>>>,
    mut next_mode: Option<ResMut<NextState<GameMode>>>,
    mut sfx: SfxWriter,
    mut analog: Local<ShellAnalogLatch>,
) {
    // No live session, or the active experience owns its own pause chrome: the
    // shell menu is inert. If it was open (e.g. the session just retired), fold
    // it shut and hand the sim back.
    if session.0.is_none() || suppressed.0 {
        if menu.open {
            menu.open = false;
            menu.cursor = 0;
            resume_sim(&game_mode, &mut next_mode);
        }
        return;
    }

    let edges = shell_action_edges(keys.as_deref(), &pads, menu_frame.as_deref(), &mut analog);
    // Escape / Start toggle; the controller B (`back`) also closes an open menu.
    let toggle = edges.pause || (menu.open && edges.back);

    if toggle {
        menu.open = !menu.open;
        menu.cursor = 0;
        if menu.open {
            pause_sim(&game_mode, &mut next_mode);
            play(&mut sfx, ids::UI_MENU_ACCEPT);
        } else {
            resume_sim(&game_mode, &mut next_mode);
            play(&mut sfx, ids::UI_MENU_BACK);
        }
        return;
    }

    if !menu.open {
        return;
    }

    if edges.previous {
        menu.cursor = menu.cursor.saturating_sub(1);
        play(&mut sfx, ids::UI_MENU_MOVE);
    }
    if edges.next {
        menu.cursor = (menu.cursor + 1).min(PauseEntry::ALL.len() - 1);
        play(&mut sfx, ids::UI_MENU_MOVE);
    }
    if edges.confirm {
        activate_pause_entry(
            PauseEntry::ALL[menu.cursor],
            &mut menu,
            &mut shell,
            &game_mode,
            &mut next_mode,
            &mut sfx,
        );
    }
}

/// Pointer/touch activation for the universal pause rows. The shared Bevy-UI
/// interaction bridge publishes the row's semantic [`PauseEntry`], then this
/// adapter calls the same activation function as keyboard/controller confirm.
#[allow(clippy::too_many_arguments)]
fn shell_pause_menu_pointer(
    session: Res<ActiveGameplaySession>,
    suppressed: Res<ShellPauseMenuSuppressed>,
    mut activated: MessageReader<MenuActionActivated<PauseEntry>>,
    mut menu: ResMut<ShellPauseMenu>,
    mut shell: MessageWriter<ShellCommand>,
    game_mode: Option<Res<State<GameMode>>>,
    mut next_mode: Option<ResMut<NextState<GameMode>>>,
    mut sfx: SfxWriter,
) {
    for activation in activated.read() {
        if session.0.is_none() || suppressed.0 || !menu.open {
            continue;
        }
        menu.cursor = PauseEntry::ALL
            .iter()
            .position(|entry| *entry == activation.action)
            .unwrap_or(menu.cursor);
        activate_pause_entry(
            activation.action,
            &mut menu,
            &mut shell,
            &game_mode,
            &mut next_mode,
            &mut sfx,
        );
    }
}

fn activate_pause_entry(
    entry: PauseEntry,
    menu: &mut ShellPauseMenu,
    shell: &mut MessageWriter<ShellCommand>,
    game_mode: &Option<Res<State<GameMode>>>,
    next_mode: &mut Option<ResMut<NextState<GameMode>>>,
    sfx: &mut SfxWriter,
) {
    match entry {
        PauseEntry::Resume => {
            menu.open = false;
            menu.cursor = 0;
            resume_sim(game_mode, next_mode);
            play(sfx, ids::UI_MENU_BACK);
        }
        PauseEntry::QuitToTitle => {
            // Retire the session and return to the host's title screen — the
            // same leak-free path F10 fires. The menu folds; the sim is handed
            // back so the next session starts unpaused.
            shell.write(ShellCommand::QuitToHome);
            menu.open = false;
            menu.cursor = 0;
            resume_sim(game_mode, next_mode);
            play(sfx, ids::UI_MENU_ACCEPT);
        }
        PauseEntry::QuitToDesktop => {
            // Semantic process-exit request: the HOST actuates the actual
            // `AppExit` (`exit_on_shell_request`), keeping process policy
            // host-owned.
            shell.write(ShellCommand::ExitProcess);
            play(sfx, ids::UI_MENU_ACCEPT);
        }
    }
}

/// Draw (or tear down) the pause menu whenever its open/cursor state changes.
/// Clear-and-rebuild keyed on `(open, cursor)` — a three-row menu is cheap and a
/// rebuild only on change avoids per-frame churn.
fn render_shell_pause_menu(
    mut commands: Commands,
    menu: Res<ShellPauseMenu>,
    asset_server: Option<Res<AssetServer>>,
    roots: Query<Entity, (With<BevyUiMenuRoot>, With<ShellPauseMenuRoot>)>,
    mut prior: Local<Option<(bool, usize)>>,
) {
    let key = (menu.open, menu.cursor);
    if *prior == Some(key) {
        return;
    }
    *prior = Some(key);

    for root in &roots {
        commands.entity(root).despawn();
    }
    if !menu.open {
        return;
    }

    let mut page = MenuPageModel::new(
        PausePage::Root,
        "Paused",
        MenuColor::rgba(0.02, 0.03, 0.07, 0.94),
    );
    page.text(
        50.0,
        14.0,
        5.0,
        "Paused",
        MenuTextAlign::Center,
        MenuColor::WHITE,
    );
    let row_height = 10.0;
    for (index, entry) in PauseEntry::ALL.iter().enumerate() {
        page.control(
            MenuRect::new(
                28.0,
                34.0 + index as f32 * (row_height + 3.0),
                44.0,
                row_height,
            ),
            MenuControlKind::Action,
            entry.label(),
            Some(entry.detail().to_owned()),
            index == menu.cursor,
            false,
            Some(*entry),
        );
    }
    page.text(
        50.0,
        90.0,
        2.6,
        "Up / Down select \u{b7} Enter confirms \u{b7} Esc resumes",
        MenuTextAlign::Center,
        MenuColor::WHITE,
    );

    let tabs = [BevyUiMenuTabSpec::new(PausePage::Root, "Paused")];
    let view = BevyUiMenuView::<PausePage, PauseEntry> {
        tabs: &tabs,
        active_tab: 0,
        page: &page,
        focused: None,
        focused_tab: None,
    };
    let root = spawn_bevy_ui_menu_with_assets(&mut commands, &view, asset_server.as_deref());
    commands.entity(root).insert(ShellPauseMenuRoot);
}

fn pause_sim(mode: &Option<Res<State<GameMode>>>, next: &mut Option<ResMut<NextState<GameMode>>>) {
    // Only latch a pause when actually playing, so we do not stomp Dialogue /
    // RoomTransition / Cutscene modes a game may already be in.
    if let (Some(mode), Some(next)) = (mode, next) {
        if *mode.get() == GameMode::Playing {
            next.set(GameMode::Paused);
        }
    }
}

fn resume_sim(mode: &Option<Res<State<GameMode>>>, next: &mut Option<ResMut<NextState<GameMode>>>) {
    if let (Some(mode), Some(next)) = (mode, next) {
        if *mode.get() == GameMode::Paused {
            next.set(GameMode::Playing);
        }
    }
}

fn play(sfx: &mut SfxWriter, id: ambition_sfx::SfxId) {
    sfx.write(SfxMessage::Play {
        id,
        pos: Vec2::ZERO,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(ActiveGameplaySession(None))
            .add_plugins(ShellPauseMenuPlugin)
            .add_message::<ShellCommand>();
        app
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }

    /// Fully reset the key state between simulated presses. `ButtonInput::clear`
    /// alone keeps the `pressed` set, so a second `press` of an already-held key
    /// would NOT re-raise `just_pressed`; release everything first so the next
    /// `press` is a fresh edge (there is no input plugin advancing state here).
    fn clear_keys(app: &mut App) {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release_all();
        input.clear();
    }

    fn with_live_session(app: &mut App) {
        // The drive system only reads `session.0.is_some()`; a minimal stub is
        // enough to mark gameplay live.
        app.insert_resource(ActiveGameplaySession(Some(
            crate::GameplaySessionInstance::stub_live(),
        )));
    }

    #[test]
    fn escape_opens_and_closes_only_during_a_live_session() {
        let mut app = app();
        // No session: Escape does nothing.
        press(&mut app, KeyCode::Escape);
        app.update();
        assert!(!app.world().resource::<ShellPauseMenu>().open);
        clear_keys(&mut app);

        with_live_session(&mut app);
        press(&mut app, KeyCode::Escape);
        app.update();
        assert!(
            app.world().resource::<ShellPauseMenu>().open,
            "Escape opens the menu during a live session"
        );
        clear_keys(&mut app);

        press(&mut app, KeyCode::Escape);
        app.update();
        assert!(
            !app.world().resource::<ShellPauseMenu>().open,
            "Escape again closes it"
        );
    }

    #[test]
    fn suppressed_menu_never_opens_and_folds_if_open() {
        let mut app = app();
        with_live_session(&mut app);
        press(&mut app, KeyCode::Escape);
        app.update();
        assert!(app.world().resource::<ShellPauseMenu>().open);
        clear_keys(&mut app);

        // The host raises suppression (Ambition's own mode took over): the menu
        // folds and stays inert.
        app.insert_resource(ShellPauseMenuSuppressed(true));
        app.update();
        assert!(!app.world().resource::<ShellPauseMenu>().open);
        press(&mut app, KeyCode::Escape);
        app.update();
        assert!(
            !app.world().resource::<ShellPauseMenu>().open,
            "a suppressed menu ignores the open input"
        );
    }

    #[test]
    fn quit_to_title_fires_quit_to_home_and_closes() {
        let mut app = app();
        with_live_session(&mut app);
        press(&mut app, KeyCode::Escape); // open
        app.update();
        clear_keys(&mut app);
        press(&mut app, KeyCode::ArrowDown); // cursor -> Quit to Title
        app.update();
        clear_keys(&mut app);
        press(&mut app, KeyCode::Enter); // confirm
        app.update();

        let sent: Vec<ShellCommand> = app
            .world_mut()
            .resource_mut::<Messages<ShellCommand>>()
            .drain()
            .collect();
        assert!(
            sent.iter().any(|c| matches!(c, ShellCommand::QuitToHome)),
            "Quit to Title fires QuitToHome"
        );
        assert!(!app.world().resource::<ShellPauseMenu>().open);
    }

    #[test]
    fn touch_press_on_pause_row_dispatches_that_rows_action() {
        let mut app = app();
        with_live_session(&mut app);
        press(&mut app, KeyCode::Escape);
        app.update();
        clear_keys(&mut app);

        let quit_to_title = {
            let mut q = app
                .world_mut()
                .query::<(Entity, &ambition_menu::AmbitionMenuControl<PauseEntry>)>();
            q.iter(app.world())
                .find_map(|(entity, control)| {
                    (control.action == Some(PauseEntry::QuitToTitle)).then_some(entity)
                })
                .expect("open pause menu renders a Quit to Title row")
        };
        app.world_mut()
            .entity_mut(quit_to_title)
            .insert(Interaction::Pressed);
        app.update();

        let sent: Vec<ShellCommand> = app
            .world_mut()
            .resource_mut::<Messages<ShellCommand>>()
            .drain()
            .collect();
        assert!(sent.iter().any(|c| matches!(c, ShellCommand::QuitToHome)));
        assert!(
            !app.world().resource::<ShellPauseMenu>().open,
            "the touch-selected row follows the same close policy as keyboard confirm",
        );
    }

    #[test]
    fn quit_to_desktop_requests_process_exit() {
        let mut app = app();
        with_live_session(&mut app);
        press(&mut app, KeyCode::Escape);
        app.update();
        clear_keys(&mut app);
        press(&mut app, KeyCode::ArrowDown);
        app.update();
        clear_keys(&mut app);
        press(&mut app, KeyCode::ArrowDown); // cursor -> Quit to Desktop
        app.update();
        clear_keys(&mut app);
        press(&mut app, KeyCode::Enter);
        app.update();

        let sent: Vec<ShellCommand> = app
            .world_mut()
            .resource_mut::<Messages<ShellCommand>>()
            .drain()
            .collect();
        assert!(sent.iter().any(|c| matches!(c, ShellCommand::ExitProcess)));
    }
}
