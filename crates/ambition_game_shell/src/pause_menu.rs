//! The universal shell/system menu the host offers every experience.
//!
//! A hosted experience that brings no system chrome of its own (Sanic, Mary-O,
//! the pocket demo, Smash's character select) still needs the same global exit
//! and audio controls. Rather than each route hand-rolling a menu, the shell
//! offers ONE — opened with Escape / Start, drawn with the same `ambition_menu`
//! Bevy-UI renderer the launcher uses, and dispatched to the same host-relative
//! [`ShellCommand`]s (`QuitToHome`, `ExitProcess`) the launcher and F10 already
//! fire. A live gameplay session additionally contributes Resume. Because it
//! rides [`MinimalShellPlugins`](crate::MinimalShellPlugins), the standalone demo
//! apps AND the multi-game host get it for free.
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
use crate::{
    shell_action_edges, ActiveGameplaySession, ShellCommand, ShellHostConfiguration, ShellRouter,
};

/// The universal audio controls the shell offers every experience.
///
/// This is shell presentation identity, not the full settings-menu IR. The mutation law stays
/// canonical in `ambition_persistence::settings::AudioSettings`; this enum only says which four
/// global audio fields belong on the universal shell menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShellAudioControl {
    Mute,
    MasterVolume,
    MusicVolume,
    SfxVolume,
}

impl ShellAudioControl {
    const ALL: [Self; 4] = [
        Self::Mute,
        Self::MasterVolume,
        Self::MusicVolume,
        Self::SfxVolume,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Mute => "Mute",
            Self::MasterVolume => "Master Volume",
            Self::MusicVolume => "Music Volume",
            Self::SfxVolume => "Sound Volume",
        }
    }

    fn value(self, settings: &ambition_persistence::settings::UserSettings) -> String {
        use ambition_persistence::settings::AudioSettings;

        match self {
            Self::Mute => if settings.audio.muted { "On" } else { "Off" }.to_owned(),
            Self::MasterVolume => {
                format!("{}%", AudioSettings::percent(settings.audio.master_volume))
            }
            Self::MusicVolume => {
                format!("{}%", AudioSettings::percent(settings.audio.music_volume))
            }
            Self::SfxVolume => {
                format!("{}%", AudioSettings::percent(settings.audio.sfx_volume))
            }
        }
    }

    fn adjust(self, direction: i32, settings: &mut ambition_persistence::settings::UserSettings) {
        use ambition_persistence::settings::AudioSettings;

        let step = if direction < 0 {
            -AudioSettings::VOLUME_STEP
        } else {
            AudioSettings::VOLUME_STEP
        };
        match self {
            Self::Mute => settings.audio.toggle_mute(),
            Self::MasterVolume => settings.audio.nudge_master(step),
            Self::MusicVolume => settings.audio.nudge_music(step),
            Self::SfxVolume => settings.audio.nudge_sfx(step),
        }
    }
}

/// The universal menu entries, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PauseEntry {
    Resume,
    /// The row the ACTIVE EXPERIENCE contributes — Smash's *Exit Match*. See
    /// [`ShellAbandonOffer`] for why the shell hosts a row it cannot describe.
    Abandon,
    Audio(ShellAudioControl),
    QuitToTitle,
    QuitToDesktop,
    /// Close the menu when there is no session to resume.
    Close,
}

impl PauseEntry {
    /// Build rows from the two independent facts the menu actually cares about.
    ///
    /// `Resume` is session-relative. `Quit to Title` is route-relative: a
    /// frontend subroute such as Smash's character select has no gameplay
    /// session, but it still has somewhere meaningful to quit *to*. Only the
    /// host's home route should omit that row.
    fn rows(
        in_session: bool,
        can_quit_to_title: bool,
        abandon: Option<&ShellAbandonOffer>,
    ) -> Vec<PauseEntry> {
        let mut rows = Vec::with_capacity(9);
        if in_session {
            rows.push(PauseEntry::Resume);
        }
        // Directly under Resume: it is the other answer to "I do not want to be
        // in this any more", and burying it under four volume sliders would make
        // Quit to Title the easier way out of a match — which loses the whole
        // session rather than the match.
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
            // The experience's words. A shell that spelled "Exit Match" itself
            // would be a shell that knows what a match is.
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
            // The VALUE is the detail. A settings row whose current state is
            // invisible is a switch with no indicator: you can only discover
            // what it does by changing it.
            PauseEntry::Audio(control) => control.value(settings),
            PauseEntry::QuitToTitle => "Return to the title screen.".to_owned(),
            PauseEntry::QuitToDesktop => "Exit the game.".to_owned(),
            PauseEntry::Close => "Close this menu.".to_owned(),
        }
    }
}

/// The pause menu's open state + cursor. Cursor indexes [`PauseEntry::ALL`].
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
/// experience's OWN pause chrome (e.g. Ambition's kaleidoscope). Defaults to
/// `false`, so a standalone demo app — which has no other pause menu — always
/// gets it. The multi-game host drives this from its `in_base_mode` signal.
#[derive(Resource, Default)]
pub struct ShellPauseMenuSuppressed(pub bool);

/// Route/session facts needed to decide which universal menu rows make sense.
///
/// These resources form one coherent question: *where did the system menu open?*
/// Keeping that question here avoids growing three menu systems with parallel
/// route/session parameter lists.
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

    /// The standing offer, and only while there is a SESSION to leave: an offer
    /// left behind by a retired experience must not put a row on the title
    /// screen's menu.
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
        // The three rows a stray touch must not spend. `MenuTapMode`'s shipped
        // default guards exactly these and nothing else — Resume and the audio
        // rows stay one tap, because a guard on a reversible row is only a tax.
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
            // Consumers of the routed input semantics: after every producer
            // (participant populate, touch folds, pointer bridge), same frame.
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
            // The pause menu OWNS input while it is open — the same shape the
            // launcher surface already uses, and for the same reason: a surface
            // underneath must not act on the presses driving the menu.
            .add_systems(
                Update,
                declare_pause_context.in_set(ambition_input::InputSet::ResolveContext),
            );
    }
}

/// Claim input while the pause menu is open.
///
///  the gap this closes was visible: with the pause menu open over the
/// character-select screen, the arrows drove BOTH — the menu's cursor and the
/// CPU count. Neither could consume the other's edge, because they read
/// different channels (`MenuControlFrame` here, `SeatMenuFrames` there).
///
/// The claim system is the seam that was already built for this: this side declares, the
/// surface underneath asks whether it still owns its seat, and neither names the other.
///
/// The claim goes to every participant because the pause menu is global — one
/// menu, opened by whoever pressed Start. A per-seat surface is a separate
/// question and is recorded as one.
fn declare_pause_context(
    menu: Res<ShellPauseMenu>,
    mut participants: Query<
        &mut ambition_input::participant::ParticipantContexts,
        With<ambition_input::InputParticipant>,
    >,
) {
    for mut contexts in &mut participants {
        // Touch the component only when the claim actually moves, so a quiet
        // frame is not a change-detection event for every reader downstream.
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

/// Input + state for the pause menu: open/close on the Start intent,
/// navigate, and dispatch the selected entry. Pausing the sim
/// ([`GameMode::Paused`]) is best-effort — a demo that does not register the
/// `GameMode` state simply keeps running behind the menu, which stays fully
/// functional either way.
#[allow(clippy::too_many_arguments)]
fn drive_shell_pause_menu(
    // The device-agnostic menu seam, populated from the persistent input
    // participant. Absent in an app with no host input stack; present (and
    // touch-fed) in every windowed host, which is what lets the on-screen
    // "Menu" button open this menu on a phone.
    menu_frame: Option<Res<ambition_input::MenuControlFrame>>,
    // The PER-SEAT frames, so any seat can pause and the seat that paused drives
    // the menu. Optional: a standalone demo composes this shell without the
    // participant pipeline, and there the global frame above is the only seat.
    seat_frames: Option<Res<ambition_input::SeatMenuFrames>>,
    context: PauseMenuContext,
    suppressed: Res<ShellPauseMenuSuppressed>,
    mut menu: ResMut<ShellPauseMenu>,
    mut shell: MessageWriter<ShellCommand>,
    mut abandon: MessageWriter<ShellAbandonRequested>,
    game_mode: Option<Res<State<GameMode>>>,
    mut next_mode: Option<ResMut<NextState<GameMode>>>,
    mut settings: Option<ResMut<ambition_persistence::settings::UserSettings>>,
    mut sfx: SfxWriter,
) {
    // The active experience owns its own pause chrome: the shell menu yields
    // entirely. If it was open (e.g. that experience just took over), fold it
    // shut and hand the sim back.
    if suppressed.0 {
        if menu.open {
            menu.close();
            resume_sim(&game_mode, &mut next_mode);
        }
        return;
    }
    let in_session = context.in_session();
    let rows = context.rows();

    // Whose presses is this menu reading?
    //
    // Open: the seat that opened it, and only that seat — you pressed the
    // button, so the cursor answers to you. Closed: EVERY seat is a candidate,
    // because any of them may pause. The first seat in slot order that pressed
    // Start wins the frame, which makes a simultaneous press deterministic
    // rather than dependent on iteration luck.
    //
    //  `seat_frames` is optional and the global frame is the fallback, for the
    // same reason it is optional everywhere else in this crate: a standalone
    // demo composes the shell without the participant pipeline, and there the
    // one global frame IS the only seat.
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
            // Pausing is a no-op without a session, and asking for it anyway
            // would pause a title screen that has no sim to pause.
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

    // ⭐⭐ THE SHARED CURSOR, and this crate's pause menu is the FIRST NAME IN ITS
    // OWN DOC: *"the boring but easy-to-drift rules shared by pause-menu, radio,
    // settings, inventory"*. It hand-rolled them anyway, and they HAD drifted —
    // `ambition_dialog`'s picker runs on `ListCursor`, whose only movement verbs
    // WRAP, while this one CLAMPED. Two menus in one game disagreed about what
    // the end of a list does.
    //
    // ⭐ AND THE MOVE CUE FOLLOWS THE MOVE NOW. `apply_directional` answers
    // whether the selection actually changed, so pressing down on the last row
    // is silent instead of playing a move sound for a cursor that did not move —
    // which under the clamp was every press at either end.
    let mut cursor = ambition_ui_nav::ListCursor::new(menu.cursor, rows.len());
    if cursor.apply_directional(edges.previous, edges.next) {
        play(&mut sfx, ids::UI_MENU_MOVE);
    }
    menu.cursor = cursor.selected();

    // LEFT/RIGHT edit the focused row's value. Only a settings row has one, so
    // this is inert everywhere else rather than being a second confirm.
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

/// Pointer/touch activation for the universal pause rows. The shared Bevy-UI
/// interaction bridge publishes the row's semantic [`PauseEntry`], then this
/// adapter calls the same activation function as keyboard/controller confirm.
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
        // Session-independent, like the keyboard path: the title screen's menu
        // is real, so its rows are pointer- and touch-activatable too.
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
    // The contributed row's channel. Written and NOT acted on here — see
    // [`ShellAbandonOffer`].
    abandon: &mut MessageWriter<ShellAbandonRequested>,
    game_mode: &Option<Res<State<GameMode>>>,
    next_mode: &mut Option<ResMut<NextState<GameMode>>>,
    settings: Option<&mut ambition_persistence::settings::UserSettings>,
    sfx: &mut SfxWriter,
) {
    match entry {
        // Confirm on an audio row is the positive direction: mute toggles and
        // volume sliders step up. The field-specific mutation still lives on
        // `AudioSettings`; the shell owns only which four controls it exposes.
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
            // ⛔ AND THE SIM IS RESUMED, which is the part that is easy to
            // forget: the experience ends its own mode on a running clock, and a
            // menu that folded away leaving `GameMode::Paused` behind would hand
            // back a frozen stage nobody can un-freeze.
            abandon.write(ShellAbandonRequested);
            menu.close();
            resume_sim(game_mode, next_mode);
            play(sfx, ids::UI_MENU_ACCEPT);
        }
        PauseEntry::QuitToTitle => {
            // Retire the session and return to the host's title screen — the
            // same leak-free path F10 fires.
            //
            // Session retirement resets the mode now (`translate_shell_session_lifecycle`), because
            // the lifecycle that ended the session is the one place that cannot forget.
            shell.write(ShellCommand::QuitToHome);
            menu.close();
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
    context: PauseMenuContext,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    asset_server: Option<Res<AssetServer>>,
    // The font menus draw with. `None` keeps Bevy's default, which is an
    // ASCII-only subset — see `ambition_menu::render::bevy_ui::MenuFont`.
    menu_font: Option<Res<ambition_menu::render::bevy_ui::MenuFont>>,
    roots: Query<Entity, (With<BevyUiMenuRoot>, With<ShellPauseMenuRoot>)>,
    mut prior: Local<Option<(bool, usize, bool, bool, u64)>>,
) {
    let in_session = context.in_session();
    let can_quit_to_title = context.can_quit_to_title();
    let settings = settings.map(|s| s.clone()).unwrap_or_default();
    // The rebuild key has to include the VALUES, or a volume that changed
    // without moving the cursor would keep drawing its old percentage — a
    // settings row whose number lags the setting is worse than no number.
    // Quantised to whole percent because that is all the row displays; a float
    // key would rebuild the page on every inaudible step.
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

    // "Paused" is wrong on the title screen — there is nothing to pause. The
    // heading names what the surface IS, and the two cases are genuinely
    // different surfaces sharing rows rather than one surface with a lie on it.
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
    // Seven rows do not fit the three-row spacing this menu was built for.
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
    // Only latch a pause when actually playing, so we do not stomp Dialogue /
    // RoomTransition / Cutscene modes a game may already be in.
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
            // ⛔⛔ THE ABANDON CHANNEL IS THE SHELL PLUGIN'S, NOT THIS ONE'S, and
            // leaving it out made this whole fixture a composition that cannot
            // exist: `drive_shell_pause_menu` writes `ShellAbandonRequested`, so
            // every test here died on *"Message not initialized"* the moment
            // anyone built them. `ShellGamePlugin` registers it in production
            // (`plugin.rs`); a fixture that installs only the menu has to say so.
            .add_message::<crate::abandon::ShellAbandonRequested>();
        app
    }

    /// Inject one semantic intent for exactly one frame — the shape every
    /// device (keyboard, gamepad, touch) reduces to before the shell reads
    /// input. The frame resets afterwards so edges never linger.
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

    /// One seat's intent for exactly one frame — the per-seat twin of
    /// [`intent`], and the shape a couch actually produces.
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

    /// Player two can pause, and the menu answers to player two.
    ///
    ///  neither half was true. `drive_shell_pause_menu` read
    /// `MenuControlFrame`, which `populate_menu_control_frame_from_actions`
    /// fills from the PRIMARY seat ALONE — so a second player's Start went
    /// nowhere and their D-pad moved nothing. From the couch that reads as a
    /// broken button, which is why the handoff's claim that "any seat pauses"
    /// was worth checking rather than believing.
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

    /// The Start intent — what keyboard Escape, controller Start, and the
    /// touch "Menu" button all become. Escape additionally maps to MenuBack
    /// in the bindings, so the pair arrives together like the real device.
    fn press_start(app: &mut App) {
        intent(app, |f| {
            f.start = true;
            f.back = true;
        });
    }

    fn with_live_session(app: &mut App) {
        // The drive system only reads `session.0.is_some()`; a minimal stub is
        // enough to mark gameplay live.
        app.insert_resource(ActiveGameplaySession(Some(
            crate::GameplaySessionInstance::stub_live(),
        )));
    }

    /// Move the cursor onto `wanted` by pressing Down, the way a player would.
    ///
    /// Derived from the row list rather than a hand-counted number of presses:
    /// the row set grew audio rows and every hardcoded count in these tests went
    /// stale at once, which is a test suite pinning a layout instead of a claim.
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

    /// Left / right edit the focused setting, through the shared IR rather
    /// than through a shell-local opinion about what a volume step is.
    #[test]
    fn adjusting_a_volume_row_writes_the_persisted_setting() {
        use ambition_persistence::settings::UserSettings;
        let mut app = app();
        app.init_resource::<UserSettings>();
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
        app.init_resource::<UserSettings>();
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

        // The row is looked up again after the press frame: the menu republishes
        // its controls, so the entity a finger lands on is routinely not the one
        // it lifts from. The bridge is keyed on the action for that reason.
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

        let released = row(&mut app);
        app.world_mut()
            .entity_mut(released)
            .insert(Interaction::Hovered);
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

    /// THE END OF THE LIST IS THE SAME PLACE IN BOTH MENUS.
    ///
    /// ⛔⛔ IT WAS NOT. This menu hand-rolled `(cursor + 1).min(len - 1)` while
    /// `ambition_dialog`'s picker ran on `ListCursor`, whose only movement verbs
    /// WRAP — so one menu in this game wrapped and the other stopped dead, and
    /// `ListCursor`'s own doc names *"pause-menu"* FIRST among the callers whose
    /// rules it exists to own.
    ///
    /// ⭐ ASKED WITHOUT THE ROW COUNT, because the row list belongs to the
    /// CONTEXT and this fixture holds only the resource. *"Up from the top does
    /// not stay at the top"* is the whole claim, and the clamp fails it.
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
