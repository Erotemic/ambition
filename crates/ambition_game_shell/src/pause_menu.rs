//! The universal shell/system menu the host offers every experience.
//!
//! Experiences without their own system chrome (Sanic, Mary-O, the pocket demo,
//! Smash's character select) get global exit and audio controls here. The menu
//! opens with Escape / Start, uses the same `ambition_menu` Bevy-UI renderer as
//! the launcher, and sends the same host-relative [`ShellCommand`]s
//! (`QuitToHome`, `ExitProcess`). A live gameplay session adds Resume. It is
//! part of [`MinimalShellPlugins`](crate::MinimalShellPlugins), so standalone
//! demo apps and the multi-game host both get it.
//!
//! ## Coexistence with a game's own pause menu
//!
//! Ambition's gameplay has its own pause menu (the kaleidoscope). The host sets
//! [`ShellPauseMenuSuppressed`] from its `in_base_mode` signal, so the two menus
//! never serve the same live session. In a standalone demo the flag stays
//! `false`. The menu also yields while the launcher is active.

use ambition_menu::render::bevy_ui::{
    install_bevy_ui_menu_actions, BevyUiMenuInteractionSet, BevyUiMenuRoot, BevyUiMenuTabSpec,
    BevyUiMenuView,
};
use ambition_menu::{
    MenuActionActivated, MenuColor, MenuControlKind, MenuPageModel, MenuRect, MenuTextAlign,
};
use ambition_platformer2d_shared_tangle::schedule::GameMode;
use ambition_sfx::{ids, OwnedSfxMessage, SfxMessage, SfxWriter};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::abandon::{ShellAbandonOffer, ShellAbandonRequested};
use crate::audio_controls::ShellAudioControl;
use crate::{
    shell_action_edges, ActiveGameplaySession, ShellCommand, ShellHostConfiguration, ShellRouter,
};


/// The universal menu entries, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PauseEntry {
    Resume,
    /// The row the active experience contributes, e.g. Smash's *Exit Match*.
    /// See [`ShellAbandonOffer`].
    Abandon,
    Audio(ShellAudioControl),
    QuitToTitle,
    QuitToDesktop,
    /// Close the menu when there is no session to resume.
    Close,
}

impl PauseEntry {
    /// Build rows from two independent facts. `Resume` needs a session.
    /// `Quit to Title` needs a route other than home: a frontend subroute such
    /// as Smash's character select has no session but can still quit to title.
    fn rows(
        in_session: bool,
        can_quit_to_title: bool,
        abandon: Option<&ShellAbandonOffer>,
    ) -> Vec<PauseEntry> {
        let mut rows = Vec::with_capacity(9);
        if in_session {
            rows.push(PauseEntry::Resume);
        }
        // Directly under Resume, so it is easier to reach than Quit to Title,
        // which ends the whole session.
        if abandon.is_some() {
            rows.push(PauseEntry::Abandon);
        }
        rows.extend(ShellAudioControl::ALL.map(PauseEntry::Audio));
        if !in_session {
            rows.push(PauseEntry::Close);
        }
        if can_quit_to_title {
            rows.push(PauseEntry::QuitToTitle);
        }
        rows.push(PauseEntry::QuitToDesktop);
        rows
    }

    fn label(self, abandon: Option<&ShellAbandonOffer>) -> String {
        match self {
            PauseEntry::Resume => "Resume".to_owned(),
            // The experience supplies the words; the shell does not know them.
            PauseEntry::Abandon => abandon
                .map(|offer| offer.label.clone())
                .unwrap_or_else(|| "Exit".to_owned()),
            PauseEntry::Audio(control) => control.label().to_owned(),
            PauseEntry::QuitToTitle => "Quit to Title".to_owned(),
            PauseEntry::QuitToDesktop => "Quit to Desktop".to_owned(),
            PauseEntry::Close => "Close".to_owned(),
        }
    }

    fn detail(
        self,
        settings: &ambition_persistence::settings::UserSettings,
        abandon: Option<&ShellAbandonOffer>,
    ) -> String {
        match self {
            PauseEntry::Resume => "Return to the game.".to_owned(),
            PauseEntry::Abandon => abandon
                .map(|offer| offer.detail.clone())
                .unwrap_or_else(|| "Leave what is running.".to_owned()),
            // The current value is the detail.
            PauseEntry::Audio(control) => control.value(settings),
            PauseEntry::QuitToTitle => "Return to the title screen.".to_owned(),
            PauseEntry::QuitToDesktop => "Exit the game.".to_owned(),
            PauseEntry::Close => "Close this menu.".to_owned(),
        }
    }
}

/// The pause menu's open state and cursor. The cursor indexes the current rows.
#[derive(Resource, Default)]
pub struct ShellPauseMenu {
    pub open: bool,
    cursor: usize,
    /// Seat that opened the pause menu and therefore drives it. Pause itself is
    /// global; only menu input ownership is per-seat. `None` while unowned.
    owner: Option<u8>,
}

impl ShellPauseMenu {
    /// The seat currently driving the pause menu, if one owns it.
    pub fn owner(&self) -> Option<u8> {
        self.owner
    }

    /// Fold the menu shut and release its input owner.
    pub fn close(&mut self) {
        self.open = false;
        self.cursor = 0;
        self.owner = None;
    }
}

/// Host-set gate: when `true`, the shell pause menu yields to the active
/// experience's own pause menu (e.g. Ambition's kaleidoscope). Defaults to
/// `false` for standalone demos. The multi-game host sets it from its
/// `in_base_mode` signal.
#[derive(Resource, Default)]
pub struct ShellPauseMenuSuppressed(pub bool);

/// Route and session facts that decide which menu rows to show. Shared by the
/// three menu systems.
#[derive(SystemParam)]
struct PauseMenuContext<'w> {
    session: Res<'w, ActiveGameplaySession>,
    router: Res<'w, ShellRouter>,
    host: Res<'w, ShellHostConfiguration>,
    /// What the active experience offers to leave, if it offers anything.
    abandon: Option<Res<'w, ShellAbandonOffer>>,
}

impl PauseMenuContext<'_> {
    fn in_session(&self) -> bool {
        self.session.0.is_some()
    }

    fn can_quit_to_title(&self) -> bool {
        if self.in_session() {
            return true;
        }
        let (Some(active), Some(host)) = (self.router.active.as_ref(), self.host.spec.as_ref())
        else {
            return false;
        };
        active.route_id != host.home_route
    }

    /// The standing offer, only while a session is live. A stale offer from a
    /// retired experience must not add a row on the title screen.
    fn abandon(&self) -> Option<&ShellAbandonOffer> {
        self.in_session().then(|| self.abandon.as_deref()).flatten()
    }

    fn rows(&self) -> Vec<PauseEntry> {
        PauseEntry::rows(self.in_session(), self.can_quit_to_title(), self.abandon())
    }
}

/// Page id for the single-page pause menu model.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PausePage {
    Root,
}

/// Marks the pause menu's UI root so its rebuild teardown never claims another
/// `BevyUiMenuRoot` producer (the launcher, a game's own menu).
#[derive(Component)]
struct ShellPauseMenuRoot;

/// Adds the universal shell/system menu. Rides [`MinimalShellPlugins`], so
/// every host and standalone demo app gets it.
pub struct ShellPauseMenuPlugin;

impl Plugin for ShellPauseMenuPlugin {
    fn build(&self, app: &mut App) {
        install_bevy_ui_menu_actions::<PauseEntry>(app);
        // The menu edits `UserSettings`, so it must supply them. Otherwise the
        // audio rows draw defaults but every adjust does nothing (as in the
        // Sanic and Mary-O binaries). Use `init_resource` so settings that a
        // composition already inserted or loaded are kept. Guarded by
        // `the_pause_menu_plugin_supplies_the_settings_it_edits`.
        app.init_resource::<ambition_persistence::settings::UserSettings>();
        // Rows a stray touch must not trigger. `MenuTapMode` needs two taps for
        // these. Reversible rows stay one tap.
        app.insert_resource(ambition_menu::MenuDestructiveActions::<PauseEntry>::new(
            |entry| {
                matches!(
                    entry,
                    PauseEntry::Abandon | PauseEntry::QuitToTitle | PauseEntry::QuitToDesktop
                )
            },
        ));
        app.init_resource::<ShellPauseMenu>()
            .init_resource::<ShellPauseMenuSuppressed>()
            .add_message::<OwnedSfxMessage>()
            .init_resource::<ambition_sfx::SfxEmissionContext>()
            // Input consumers: after every producer, in the same frame.
            .add_systems(
                Update,
                (
                    drive_shell_pause_menu,
                    shell_pause_menu_pointer.after(BevyUiMenuInteractionSet),
                    render_shell_pause_menu,
                )
                    .chain()
                    .in_set(ambition_input::InputSet::Consume),
            )
            // The open pause menu owns input, so a surface under it does not
            // act on the same presses.
            .add_systems(
                Update,
                declare_pause_context.in_set(ambition_input::InputSet::ResolveContext),
            );
    }
}

/// Claim input while the pause menu is open.
///
/// Surfaces underneath (e.g. character select, which reads `SeatMenuFrames`)
/// check the claim and stop acting on input. The claim goes to every
/// participant because the pause menu is global.
fn declare_pause_context(
    menu: Res<ShellPauseMenu>,
    mut participants: Query<
        &mut ambition_input::participant::ParticipantContexts,
        With<ambition_input::InputParticipant>,
    >,
) {
    for mut contexts in &mut participants {
        // Write only on a real change, to avoid false change detection.
        if contexts.is_declared(ambition_input::PAUSE_CONTEXT) != menu.open {
            contexts.sync(
                ambition_input::participant::ContextClaim::capturing(
                    ambition_input::PAUSE_CONTEXT,
                    ambition_input::participant::context_priority::PAUSE,
                ),
                menu.open,
            );
        }
    }
}

/// Open and close the pause menu on Start, navigate, and dispatch the selected
/// entry. Pausing the sim ([`GameMode::Paused`]) is best-effort: a demo without
/// the `GameMode` state keeps running behind the menu.
#[allow(clippy::too_many_arguments)]
fn drive_shell_pause_menu(
    // Device-independent menu input (includes the touch "Menu" button).
    // Absent in an app with no host input stack.
    menu_frame: Option<Res<ambition_input::MenuControlFrame>>,
    // Per-seat frames: any seat can pause, and that seat drives the menu.
    // Absent in a standalone demo; then the global frame is the only seat.
    seat_frames: Option<Res<ambition_input::SeatMenuFrames>>,
    context: PauseMenuContext,
    suppressed: Res<ShellPauseMenuSuppressed>,
    // The menu must not open over the launcher. `ShellPauseMenuSuppressed`
    // does not cover this: it tracks Ambition's own session, and the title
    // screen has no session. Read the launcher here instead of adding a second
    // writer to that flag.
    launcher: Option<Res<crate::launcher::ShellLauncherState>>,
    mut menu: ResMut<ShellPauseMenu>,
    mut shell: MessageWriter<ShellCommand>,
    mut abandon: MessageWriter<ShellAbandonRequested>,
    game_mode: Option<Res<State<GameMode>>>,
    mut next_mode: Option<ResMut<NextState<GameMode>>>,
    mut settings: Option<ResMut<ambition_persistence::settings::UserSettings>>,
    mut sfx: SfxWriter,
) {
    // The launcher owns the screen. Close this menu if it is open.
    if launcher.as_deref().is_some_and(|state| state.active) {
        if menu.open {
            menu.close();
            resume_sim(&game_mode, &mut next_mode);
        }
        return;
    }
    // The active experience has its own pause menu. Close this one if it is
    // open and resume the sim.
    if suppressed.0 {
        if menu.open {
            menu.close();
            resume_sim(&game_mode, &mut next_mode);
        }
        return;
    }
    let in_session = context.in_session();
    let rows = context.rows();

    // Open: read only the seat that opened the menu. Closed: any seat can
    // open it; the first seat in slot order that pressed Start wins, so a
    // simultaneous press is deterministic. Without `seat_frames`, use the
    // global frame.
    let (edges, presser) = match seat_frames.as_deref() {
        Some(frames) => match menu.owner {
            Some(owner) => (shell_action_edges(Some(&frames.for_seat(owner))), None),
            None => {
                let opener = frames
                    .seats()
                    .find(|(_, frame)| shell_action_edges(Some(frame)).pause)
                    .map(|(slot, _)| slot);
                (
                    opener
                        .map(|slot| shell_action_edges(Some(&frames.for_seat(slot))))
                        .unwrap_or_default(),
                    opener,
                )
            }
        },
        None => (shell_action_edges(menu_frame.as_deref()), None),
    };
    // Escape / Start toggle; the controller B (`back`) also closes an open menu.
    let toggle = edges.pause || (menu.open && edges.back);

    if toggle {
        menu.open = !menu.open;
        menu.cursor = 0;
        menu.owner = menu.open.then_some(presser).flatten();
        if menu.open {
            // Pause only when a session is live.
            if in_session {
                pause_sim(&game_mode, &mut next_mode);
            }
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

    // Use the shared `ListCursor` (wraps at both ends), like the other menus.
    // Play the move cue only when the selection changed.
    let mut cursor = ambition_ui_nav::ListCursor::new(menu.cursor, rows.len());
    if cursor.apply_directional(edges.previous, edges.next) {
        play(&mut sfx, ids::UI_MENU_MOVE);
    }
    menu.cursor = cursor.selected();

    // Left/right edit the focused row's value. Only settings rows have one.
    let focused = rows.get(menu.cursor).copied().unwrap_or(PauseEntry::Close);
    if let (PauseEntry::Audio(control), Some(settings)) = (focused, settings.as_deref_mut()) {
        let direction = i32::from(edges.increase) - i32::from(edges.decrease);
        if direction != 0 {
            control.adjust(direction, settings);
        }
    }

    if edges.confirm {
        activate_pause_entry(
            focused,
            &mut menu,
            &mut shell,
            &mut abandon,
            &game_mode,
            &mut next_mode,
            settings.as_deref_mut(),
            &mut sfx,
        );
    }
}

/// Pointer and touch activation for the pause rows. Uses the same activation
/// function as keyboard or controller confirm.
#[allow(clippy::too_many_arguments)]
fn shell_pause_menu_pointer(
    context: PauseMenuContext,
    suppressed: Res<ShellPauseMenuSuppressed>,
    mut activated: MessageReader<MenuActionActivated<PauseEntry>>,
    mut menu: ResMut<ShellPauseMenu>,
    mut shell: MessageWriter<ShellCommand>,
    mut abandon: MessageWriter<ShellAbandonRequested>,
    game_mode: Option<Res<State<GameMode>>>,
    mut next_mode: Option<ResMut<NextState<GameMode>>>,
    mut settings: Option<ResMut<ambition_persistence::settings::UserSettings>>,
    mut sfx: SfxWriter,
) {
    let rows = context.rows();
    for activation in activated.read() {
        // Session-independent, like the keyboard path.
        if suppressed.0 || !menu.open {
            continue;
        }
        menu.cursor = rows
            .iter()
            .position(|entry| *entry == activation.action)
            .unwrap_or(menu.cursor);
        activate_pause_entry(
            activation.action,
            &mut menu,
            &mut shell,
            &mut abandon,
            &game_mode,
            &mut next_mode,
            settings.as_deref_mut(),
            &mut sfx,
        );
    }
}

fn activate_pause_entry(
    entry: PauseEntry,
    menu: &mut ShellPauseMenu,
    shell: &mut MessageWriter<ShellCommand>,
    // Written here; the experience acts on it. See [`ShellAbandonOffer`].
    abandon: &mut MessageWriter<ShellAbandonRequested>,
    game_mode: &Option<Res<State<GameMode>>>,
    next_mode: &mut Option<ResMut<NextState<GameMode>>>,
    settings: Option<&mut ambition_persistence::settings::UserSettings>,
    sfx: &mut SfxWriter,
) {
    match entry {
        // Confirm on an audio row is the positive direction: mute toggles and
        // volumes step up.
        PauseEntry::Audio(control) => {
            if let Some(settings) = settings {
                control.adjust(1, settings);
            }
        }
        PauseEntry::Close => {
            menu.close();
            play(sfx, ids::UI_MENU_BACK);
        }
        PauseEntry::Resume => {
            menu.close();
            resume_sim(game_mode, next_mode);
            play(sfx, ids::UI_MENU_BACK);
        }
        PauseEntry::Abandon => {
            // Also resume the sim. Otherwise `GameMode::Paused` stays set and
            // the stage stays frozen.
            abandon.write(ShellAbandonRequested);
            menu.close();
            resume_sim(game_mode, next_mode);
            play(sfx, ids::UI_MENU_ACCEPT);
        }
        PauseEntry::QuitToTitle => {
            // Same path as F10. Session retirement resets the game mode
            // (`translate_shell_session_lifecycle`).
            shell.write(ShellCommand::QuitToHome);
            menu.close();
            play(sfx, ids::UI_MENU_ACCEPT);
        }
        PauseEntry::QuitToDesktop => {
            // The host sends `AppExit` (`exit_on_shell_request`).
            shell.write(ShellCommand::ExitProcess);
            play(sfx, ids::UI_MENU_ACCEPT);
        }
    }
}

/// Rebuild or tear down the pause menu when its key changes (open, cursor,
/// route facts, audio values).
fn render_shell_pause_menu(
    mut commands: Commands,
    menu: Res<ShellPauseMenu>,
    context: PauseMenuContext,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    asset_server: Option<Res<AssetServer>>,
    // `None` keeps Bevy's ASCII-only default font.
    menu_font: Option<Res<ambition_menu::render::bevy_ui::MenuFont>>,
    roots: Query<Entity, (With<BevyUiMenuRoot>, With<ShellPauseMenuRoot>)>,
    mut prior: Local<Option<(bool, usize, bool, bool, u64)>>,
) {
    let in_session = context.in_session();
    let can_quit_to_title = context.can_quit_to_title();
    let settings = settings.map(|s| s.clone()).unwrap_or_default();
    // The key includes the displayed audio values, so a change redraws. It
    // hashes the display text, so steps too small to show do not rebuild.
    let audio_key = ShellAudioControl::ALL.iter().fold(0u64, |acc, control| {
        let value = match control {
            ShellAudioControl::Mute => u64::from(settings.audio.muted),
            other => other
                .value(&settings)
                .bytes()
                .fold(0u64, |a, b| a.wrapping_mul(131).wrapping_add(u64::from(b))),
        };
        acc.wrapping_mul(1_000_003).wrapping_add(value)
    });
    let key = (
        menu.open,
        menu.cursor,
        in_session,
        can_quit_to_title,
        audio_key,
    );
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

    // Nothing is paused without a session, so the heading says "Settings".
    let heading = if in_session { "Paused" } else { "Settings" };
    let mut page = MenuPageModel::new(
        PausePage::Root,
        heading,
        MenuColor::rgba(0.02, 0.03, 0.07, 0.94),
    );
    page.text(
        50.0,
        14.0,
        5.0,
        heading,
        MenuTextAlign::Center,
        MenuColor::WHITE,
    );
    let rows = context.rows();
    let abandon = context.abandon();
    // Shrink rows to fit.
    let row_height = (52.0 / rows.len().max(1) as f32).min(10.0);
    for (index, entry) in rows.iter().enumerate() {
        page.control(
            MenuRect::new(
                28.0,
                30.0 + index as f32 * (row_height + 2.0),
                44.0,
                row_height,
            ),
            MenuControlKind::Action,
            entry.label(abandon),
            Some(entry.detail(&settings, abandon)),
            index == menu.cursor,
            false,
            Some(*entry),
        );
    }
    page.text(
        50.0,
        90.0,
        2.6,
        if in_session {
            "Up / Down select \u{b7} Left / Right adjust \u{b7} Enter confirms \u{b7} Esc resumes"
        } else {
            "Up / Down select \u{b7} Left / Right adjust \u{b7} Enter confirms \u{b7} Esc closes"
        },
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
    let root = ambition_menu::render::bevy_ui::spawn_bevy_ui_menu_with_font(
        &mut commands,
        &view,
        asset_server.as_deref(),
        menu_font.as_deref().and_then(|font| font.0.as_ref()),
    );
    commands.entity(root).insert(ShellPauseMenuRoot);
}

fn pause_sim(mode: &Option<Res<State<GameMode>>>, next: &mut Option<ResMut<NextState<GameMode>>>) {
    // Pause only from Playing, so Dialogue, RoomTransition, and Cutscene stay.
    if let (Some(mode), Some(next)) = (mode, next) {
        if *mode.get() == GameMode::Playing {
            ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
                GameMode::Paused,
                "shell_pause_menu",
            );
            next.set(GameMode::Paused);
        }
    }
}

fn resume_sim(mode: &Option<Res<State<GameMode>>>, next: &mut Option<ResMut<NextState<GameMode>>>) {
    if let (Some(mode), Some(next)) = (mode, next) {
        if *mode.get() == GameMode::Paused {
            ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
                GameMode::Playing,
                "shell_pause_menu",
            );
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

    /// The plugin supplies the `UserSettings` it edits. Without them the audio
    /// rows draw defaults, but adjusts do nothing.
    #[test]
    fn the_pause_menu_plugin_supplies_the_settings_it_edits() {
        let mut app = App::new();
        app.add_plugins(ShellPauseMenuPlugin);
        assert!(
            app.world()
                .contains_resource::<ambition_persistence::settings::UserSettings>(),
            "the pause menu draws audio rows but does not supply `UserSettings`, so \
             in any composition that does not install it separately those rows show \
             defaults and cannot be changed"
        );
    }

    use super::*;
    use ambition_input::MenuControlFrame;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<MenuControlFrame>()
            .insert_resource(ActiveGameplaySession(None))
            .init_resource::<ShellRouter>()
            .init_resource::<ShellHostConfiguration>()
            .add_plugins(ShellPauseMenuPlugin)
            .add_message::<ShellCommand>()
            // `ShellGamePlugin` registers this in production. The menu writes
            // it, so a fixture without it panics.
            .add_message::<crate::abandon::ShellAbandonRequested>();
        app
    }

    /// Inject one semantic intent for exactly one frame, then reset it.
    fn intent(app: &mut App, set: impl Fn(&mut MenuControlFrame)) {
        {
            let mut frame = app.world_mut().resource_mut::<MenuControlFrame>();
            *frame = MenuControlFrame::default();
            set(&mut frame);
        }
        app.update();
        *app.world_mut().resource_mut::<MenuControlFrame>() = MenuControlFrame::default();
    }

    /// A couch: two seated players, each with their own menu frame.
    fn couch_app() -> App {
        let mut app = app();
        app.init_resource::<ambition_input::SeatMenuFrames>();
        {
            let mut frames = app
                .world_mut()
                .resource_mut::<ambition_input::SeatMenuFrames>();
            frames.set(0, MenuControlFrame::default());
            frames.set(1, MenuControlFrame::default());
        }
        app
    }

    /// One seat's intent for exactly one frame; the per-seat form of [`intent`].
    fn seat_intent(app: &mut App, slot: u8, set: impl Fn(&mut MenuControlFrame)) {
        {
            let mut frames = app
                .world_mut()
                .resource_mut::<ambition_input::SeatMenuFrames>();
            let mut frame = MenuControlFrame::default();
            set(&mut frame);
            frames.set(slot, frame);
        }
        app.update();
        let mut frames = app
            .world_mut()
            .resource_mut::<ambition_input::SeatMenuFrames>();
        frames.set(slot, MenuControlFrame::default());
    }

    /// Start on the title screen must not open this menu over the launcher.
    /// `ShellPauseMenuSuppressed` does not cover this case: the title screen
    /// has no session.
    #[test]
    fn the_shell_menu_does_not_open_over_the_launcher() {
        let mut app = app();
        app.insert_resource(crate::launcher::ShellLauncherState {
            active: true,
            ..default()
        });
        intent(&mut app, |f| f.start = true);
        assert!(
            !app.world().resource::<ShellPauseMenu>().open,
            "the launcher owns the screen; the shell menu must not open over it"
        );
    }

    /// An open menu closes when the launcher comes up (e.g. after quit to home).
    #[test]
    fn an_open_shell_menu_folds_shut_when_the_launcher_comes_up() {
        let mut app = app();
        with_live_session(&mut app);
        intent(&mut app, |f| f.start = true);
        assert!(
            app.world().resource::<ShellPauseMenu>().open,
            "precondition: the menu opens during a live session"
        );

        app.insert_resource(crate::launcher::ShellLauncherState {
            active: true,
            ..default()
        });
        app.update();
        assert!(
            !app.world().resource::<ShellPauseMenu>().open,
            "the launcher came up; the menu must fold shut rather than stack"
        );
    }

    /// Control: a standalone demo has no launcher, and this is its only pause
    /// menu, so it must still open.
    #[test]
    fn without_a_launcher_the_shell_menu_still_opens() {
        let mut app = app();
        with_live_session(&mut app);
        intent(&mut app, |f| f.start = true);
        assert!(
            app.world().resource::<ShellPauseMenu>().open,
            "no launcher resource means a standalone demo, where this IS the menu"
        );
    }

    /// Player two can pause, and the menu answers to player two.
    ///
    /// `MenuControlFrame` holds only the primary seat, so this needs
    /// `SeatMenuFrames`.
    #[test]
    fn the_seat_that_paused_is_the_seat_that_drives_the_menu() {
        let mut app = couch_app();
        with_live_session(&mut app);

        // Seat ONE opens it. Seat zero never presses anything in this test.
        seat_intent(&mut app, 1, |f| f.start = true);
        assert!(
            app.world().resource::<ShellPauseMenu>().open,
            "player two's Start opened the pause menu"
        );
        assert_eq!(
            app.world().resource::<ShellPauseMenu>().owner(),
            Some(1),
            "and the menu belongs to the seat that opened it"
        );

        // The PRIMARY seat's navigation is ignored: this menu is not theirs.
        let cursor_before = app.world().resource::<ShellPauseMenu>().cursor;
        seat_intent(&mut app, 0, |f| f.down = true);
        assert_eq!(
            app.world().resource::<ShellPauseMenu>().cursor,
            cursor_before,
            "seat zero does not drive a menu seat one opened"
        );

        // The owner's navigation moves it.
        seat_intent(&mut app, 1, |f| f.down = true);
        assert_ne!(
            app.world().resource::<ShellPauseMenu>().cursor,
            cursor_before,
            "the seat that paused moves the cursor"
        );

        // And closing releases the seat, so the next player can open their own.
        seat_intent(&mut app, 1, |f| f.start = true);
        let menu = app.world().resource::<ShellPauseMenu>();
        assert!(!menu.open);
        assert_eq!(
            menu.owner(),
            None,
            "a closed menu owns nobody — otherwise seat one keeps the pause \
             button away from everyone else forever"
        );
    }

    /// The Start intent (Escape, controller Start, touch "Menu"). Escape also
    /// maps to MenuBack, so both arrive together.
    fn press_start(app: &mut App) {
        intent(app, |f| {
            f.start = true;
            f.back = true;
        });
    }

    fn with_live_session(app: &mut App) {
        // The drive system reads only `session.0.is_some()`.
        app.insert_resource(ActiveGameplaySession(Some(
            crate::GameplaySessionInstance::stub_live(),
        )));
    }

    /// Move the cursor onto `wanted` by pressing Down. The press count comes
    /// from the row list, so tests do not depend on the layout.
    fn navigate_to(app: &mut App, in_session: bool, can_quit_to_title: bool, wanted: PauseEntry) {
        let index = PauseEntry::rows(in_session, can_quit_to_title, None)
            .iter()
            .position(|entry| *entry == wanted)
            .expect("the row is in this menu");
        for _ in 0..index {
            intent(app, |f| f.down = true);
        }
    }

    #[test]
    fn the_start_intent_opens_the_menu_with_or_without_a_session() {
        let mut app = app();
        press_start(&mut app);
        assert!(
            app.world().resource::<ShellPauseMenu>().open,
            "the Start intent must open the shell menu on the title screen"
        );
        press_start(&mut app);
        assert!(!app.world().resource::<ShellPauseMenu>().open);

        with_live_session(&mut app);
        press_start(&mut app);
        assert!(
            app.world().resource::<ShellPauseMenu>().open,
            "the Start intent opens the menu during a live session"
        );

        press_start(&mut app);
        assert!(
            !app.world().resource::<ShellPauseMenu>().open,
            "the Start intent again closes it"
        );
    }

    /// Session presence and route position answer different questions. A
    /// frontend subroute has nothing to Resume but still needs Quit to Title.
    #[test]
    fn row_sets_distinguish_home_frontend_and_gameplay() {
        let home = PauseEntry::rows(false, false, None);
        assert!(!home.contains(&PauseEntry::Resume));
        assert!(!home.contains(&PauseEntry::QuitToTitle));
        assert!(home.contains(&PauseEntry::Close));
        assert!(home.contains(&PauseEntry::QuitToDesktop));

        let frontend = PauseEntry::rows(false, true, None);
        assert!(!frontend.contains(&PauseEntry::Resume));
        assert!(frontend.contains(&PauseEntry::Close));
        assert!(frontend.contains(&PauseEntry::QuitToTitle));
        assert!(frontend.contains(&PauseEntry::QuitToDesktop));

        let in_game = PauseEntry::rows(true, true, None);
        assert!(in_game.contains(&PauseEntry::Resume));
        assert!(in_game.contains(&PauseEntry::QuitToTitle));
        assert!(!in_game.contains(&PauseEntry::Close));

        // Audio is global and therefore present on all three surfaces.
        for control in ShellAudioControl::ALL {
            for (name, rows) in [
                ("title", &home),
                ("frontend", &frontend),
                ("gameplay", &in_game),
            ] {
                assert!(
                    rows.contains(&PauseEntry::Audio(control)),
                    "{control:?} missing from {name} menu"
                );
            }
        }
    }

    /// Left / right edit the focused setting through the shared settings code.
    #[test]
    fn adjusting_a_volume_row_writes_the_persisted_setting() {
        use ambition_persistence::settings::UserSettings;
        let mut app = app();
        // Do not insert `UserSettings` here. `ShellPauseMenuPlugin` must supply
        // it, and this test must fail if it does not.
        press_start(&mut app);
        navigate_to(
            &mut app,
            false,
            false,
            PauseEntry::Audio(ShellAudioControl::MasterVolume),
        );

        let before = app.world().resource::<UserSettings>().audio.master_volume;
        intent(&mut app, |f| f.right = true);
        let after = app.world().resource::<UserSettings>().audio.master_volume;
        assert!(
            after > before,
            "right on Master Volume did not raise it ({before} -> {after})"
        );

        intent(&mut app, |f| f.left = true);
        assert!(
            app.world().resource::<UserSettings>().audio.master_volume < after,
            "left did not lower it back"
        );
    }

    /// Confirm must toggle mute; direction-only toggles are not controller-accessible.
    #[test]
    fn confirming_the_mute_row_toggles_mute() {
        use ambition_persistence::settings::UserSettings;
        let mut app = app();
        // Supplied by `ShellPauseMenuPlugin`; see the volume-row test above.
        press_start(&mut app);
        navigate_to(
            &mut app,
            false,
            false,
            PauseEntry::Audio(ShellAudioControl::Mute),
        );

        assert!(!app.world().resource::<UserSettings>().audio.muted);
        intent(&mut app, |f| f.select = true);
        assert!(
            app.world().resource::<UserSettings>().audio.muted,
            "confirm on the Mute row did not mute"
        );
        intent(&mut app, |f| f.select = true);
        assert!(!app.world().resource::<UserSettings>().audio.muted);
    }

    #[test]
    fn suppressed_menu_never_opens_and_folds_if_open() {
        let mut app = app();
        with_live_session(&mut app);
        press_start(&mut app);
        assert!(app.world().resource::<ShellPauseMenu>().open);

        // The host raises suppression (Ambition's own mode took over): the menu
        // folds and stays inert.
        app.insert_resource(ShellPauseMenuSuppressed(true));
        app.update();
        assert!(!app.world().resource::<ShellPauseMenu>().open);
        press_start(&mut app);
        assert!(
            !app.world().resource::<ShellPauseMenu>().open,
            "a suppressed menu ignores the open input"
        );
    }

    fn put_on_frontend_route(app: &mut App, route: &str, home: &str) {
        app.world_mut()
            .resource_mut::<ShellHostConfiguration>()
            .spec = Some(crate::ShellHostSpec::new(home, home));
        app.world_mut().resource_mut::<ShellRouter>().active = Some(crate::ActiveShellExperience {
            activation_id: crate::ShellActivationId(1),
            route_id: crate::ShellRouteId::new(route),
            experience_id: crate::ShellExperienceId::new("test-frontend"),
            parameters: Default::default(),
            load_authorization: None,
            prepared_session: None,
        });
    }

    /// Regression for the Smash character-select gap: frontend routes have no
    /// gameplay session, but that must not erase the system menu's route home.
    #[test]
    fn frontend_subroute_can_quit_to_title_without_a_session() {
        let mut app = app();
        put_on_frontend_route(&mut app, "smash-character-select", "title");
        press_start(&mut app);
        navigate_to(&mut app, false, true, PauseEntry::QuitToTitle);
        intent(&mut app, |f| f.select = true);

        let sent: Vec<ShellCommand> = app
            .world_mut()
            .resource_mut::<Messages<ShellCommand>>()
            .drain()
            .collect();
        assert!(
            sent.iter().any(|c| matches!(c, ShellCommand::QuitToHome)),
            "a frontend subroute's Quit to Title uses the host-relative home command"
        );
        assert!(!app.world().resource::<ShellPauseMenu>().open);
    }

    #[test]
    fn quit_to_title_fires_quit_to_home_and_closes() {
        let mut app = app();
        with_live_session(&mut app);
        press_start(&mut app); // open
        navigate_to(&mut app, true, true, PauseEntry::QuitToTitle);
        intent(&mut app, |f| f.select = true); // confirm

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
    fn a_touch_that_comes_up_on_a_pause_row_dispatches_that_rows_action() {
        let mut app = app();
        with_live_session(&mut app);
        press_start(&mut app);

        // Look the row up again after each frame: the menu rebuilds, so the
        // entity changes. The bridge is keyed on the action.
        let row = |app: &mut App| {
            let mut q = app
                .world_mut()
                .query::<(Entity, &ambition_menu::AmbitionMenuControl<PauseEntry>)>();
            q.iter(app.world())
                .find_map(|(entity, control)| {
                    (control.action == Some(PauseEntry::QuitToTitle)).then_some(entity)
                })
                .expect("open pause menu renders a Quit to Title row")
        };

        let pressed = row(&mut app);
        app.world_mut()
            .entity_mut(pressed)
            .insert(Interaction::Pressed);
        app.update();
        assert!(
            app.world_mut()
                .resource_mut::<Messages<ShellCommand>>()
                .drain()
                .next()
                .is_none(),
            "a finger resting on Quit has not quit",
        );

        // `QuitToTitle` is in `MenuDestructiveActions`, and
        // `MenuTapMode::SingleTapWithDestructiveGuard` makes the first tap arm
        // the row. Only the second tap sends `QuitToHome`.
        let tap = |app: &mut App| {
            let pressed = row(app);
            app.world_mut()
                .entity_mut(pressed)
                .insert(Interaction::Pressed);
            app.update();
            let released = row(app);
            app.world_mut()
                .entity_mut(released)
                .insert(Interaction::Hovered);
            app.update();
            app.world_mut()
                .resource_mut::<Messages<ShellCommand>>()
                .drain()
                .collect::<Vec<ShellCommand>>()
        };

        let arming = tap(&mut app);
        assert!(
            !arming.iter().any(|c| matches!(c, ShellCommand::QuitToHome)),
            "the FIRST tap on a destructive row arms it; only the second spends it",
        );
        assert!(
            app.world().resource::<ShellPauseMenu>().open,
            "an armed Quit row leaves the menu open to be confirmed or abandoned",
        );

        let sent = tap(&mut app);
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
        press_start(&mut app);
        navigate_to(&mut app, true, true, PauseEntry::QuitToDesktop);
        intent(&mut app, |f| f.select = true);

        let sent: Vec<ShellCommand> = app
            .world_mut()
            .resource_mut::<Messages<ShellCommand>>()
            .drain()
            .collect();
        assert!(sent.iter().any(|c| matches!(c, ShellCommand::ExitProcess)));
    }

    /// The pause cursor wraps at both ends, like `ListCursor` in other menus.
    /// The test does not need the row count: up from the top must not stay at
    /// the top.
    #[test]
    fn the_pause_cursor_wraps_at_both_ends_like_every_other_list() {
        let mut app = app();
        intent(&mut app, |f| f.start = true);
        assert!(
            app.world().resource::<ShellPauseMenu>().open,
            "the menu never opened, so nothing below measures a cursor"
        );
        assert_eq!(app.world().resource::<ShellPauseMenu>().cursor, 0);

        intent(&mut app, |f| f.up = true);
        assert_ne!(
            app.world().resource::<ShellPauseMenu>().cursor,
            0,
            "pressing up on the first row stopped dead instead of wrapping to \
             the last, which is what the dialogue picker beside it does"
        );

        intent(&mut app, |f| f.down = true);
        assert_eq!(
            app.world().resource::<ShellPauseMenu>().cursor,
            0,
            "pressing down from the last row did not come back to the first"
        );
    }
}
