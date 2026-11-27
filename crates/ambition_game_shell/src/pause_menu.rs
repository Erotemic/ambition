//! The universal shell/system menu the host offers every experience.
//!
//! Experiences without their own system chrome (Sanic, Mary-O, the pocket demo,
//! Smash's character select) get global exit and audio controls here. A host
//! can add settings pages. The menu opens with Escape or Start and uses the
//! same `ambition_menu` Bevy-UI renderer as
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
use ambition_platformer2d_shared_tangle::lifecycle::{LocalMenuPauseVotes, MenuPauseHold, MenuPauseRoster, NetworkMenuPause, SessionScopeId};
use ambition_input::ParticipantId;
use std::collections::BTreeMap;
use ambition_sfx::{ids, OwnedSfxMessage, SfxMessage, SfxWriter};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::abandon::{ShellAbandonOffer, ShellAbandonRequested};
use crate::audio_controls::ShellAudioControl;
use crate::pause_pages::{PublishShellPausePages, ShellPauseOption, ShellPausePageId, ShellPausePages};
use ambition_persistence::settings::UserSettings;
use crate::{
    shell_action_edges, ActiveGameplaySession, PauseRequested, ShellCommand,
    ShellHostConfiguration, ShellRouter,
};


/// The universal menu entries, in display order.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PauseEntry {
    Resume,
    /// The row the active experience contributes, e.g. Smash's *Exit Match*.
    /// See [`ShellAbandonOffer`].
    Abandon,
    Audio(ShellAudioControl),
    Page(ShellPausePageId),
    Option(ShellPausePageId, String),
    PageStep(ShellPausePageId, i32),
    Back,
    QuitToTitle,
    QuitToDesktop,
    /// Close the menu when there is no session to resume.
    Close,
}

#[derive(Component)]
#[require(ambition_menu::MenuInteraction<PauseEntry>)]
struct ShellPauseMenuInstance;

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

    fn presentation(
        &self,
        settings: &UserSettings,
        context: &PauseMenuContext,
        menu: &ShellPauseMenu,
    ) -> (String, String) {
        match self {
            Self::Resume => ("Resume".into(), "Return to the game.".into()),
            Self::Abandon => context.abandon().map(|offer| (offer.label.clone(), offer.detail.clone()))
                .unwrap_or_else(|| ("Exit".into(), "Leave the current game.".into())),
            Self::Audio(control) => (control.label().into(), control.value(settings)),
            Self::Page(id) => (context.pages.get(id).expect("registered pause page").title.into(), String::new()),
            Self::Option(_, id) => context.page(menu).and_then(|page| page.rows.iter().find(|row| row.id == *id))
                .map(|row| (row.label.clone(), row.value.clone())).unwrap_or_default(),
            Self::Back => ("Back".into(), String::new()),
            Self::PageStep(_, _) => unreachable!("scroll controls have their own presentation"),
            Self::QuitToTitle => ("Quit to Title".into(), "Return to the title screen.".into()),
            Self::QuitToDesktop => ("Quit to Desktop".into(), "Exit the game.".into()),
            Self::Close => ("Close".into(), "Close this menu.".into()),
        }
    }
}

#[derive(Default, Clone, Copy)]
struct PauseLocation {
    page: Option<ShellPausePageId>,
    cursor: usize,
}

/// The pause menu's open state and cursor. The cursor indexes the current rows.
#[derive(Resource, Default)]
pub struct ShellPauseMenu {
    pub open: bool,
    cursor: usize,
    /// The seat that drives the currently displayed menu.
    owner: Option<u8>,
    /// Each seat with an open menu and its own saved cursor.
    seats: BTreeMap<u8, PauseLocation>,
    page: Option<ShellPausePageId>,
    scope: Option<SessionScopeId>,
}

impl ShellPauseMenu {
    /// The seat currently driving the pause menu, if one owns it.
    pub fn owner(&self) -> Option<u8> {
        self.owner
    }

    /// Return the foreground participant's page.
    pub fn page(&self) -> Option<ShellPausePageId> {
        self.page
    }

    /// Fold the menu shut and release its input owner.
    pub fn close(&mut self) {
        self.open = false;
        self.page = None;
        self.cursor = 0;
        self.owner = None;
        self.seats.clear();
    }

    fn select_owner(&mut self, owner: u8) {
        if self.owner == Some(owner) { return; }
        if let Some(previous) = self.owner {
            if let Some(location) = self.seats.get_mut(&previous) {
                *location = PauseLocation { page: self.page, cursor: self.cursor };
            }
        }
        self.owner = Some(owner);
        let location = self.seats.get(&owner).copied().unwrap_or_default();
        self.page = location.page;
        self.cursor = location.cursor;
    }

    fn open_for(&mut self, slot: u8) {
        self.seats.entry(slot).or_default();
        self.open = true;
        self.select_owner(slot);
    }

    fn close_for(&mut self, slot: u8) {
        self.seats.remove(&slot);
        if self.owner == Some(slot) {
            self.owner = None;
            self.page = None;
            self.cursor = 0;
            if let Some(next) = self.seats.keys().next().copied() { self.select_owner(next); }
        }
        self.open = !self.seats.is_empty();
    }

    fn open_page(&mut self, page: Option<ShellPausePageId>) {
        self.page = page;
        self.cursor = 0;
    }

    fn location(&self, owner: u8) -> Option<ShellPausePageId> {
        if self.owner == Some(owner) { self.page } else { self.seats.get(&owner)?.page }
    }

    fn close_foreground(&mut self) {
        if let Some(owner) = self.owner { self.close_for(owner); } else { self.close(); }
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
    pages: Res<'w, ShellPausePages>,
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

    fn page(&self, menu: &ShellPauseMenu) -> Option<&super::pause_pages::PausePage> {
        self.pages.get(menu.page()?)
    }

    fn captures(&self, menu: &ShellPauseMenu, owner: u8) -> bool {
        menu.location(owner)
            .and_then(|id| self.pages.get(id))
            .is_some_and(|page| page.owner == Some(ParticipantId(owner)) && page.capture_hint.is_some())
    }

    fn rows(&self, menu: &ShellPauseMenu) -> Vec<PauseEntry> {
        if let Some(page) = self.page(menu) {
            let mut rows = vec![PauseEntry::Back];
            if page.owner.map(ParticipantId::slot) == menu.owner() {
                rows.extend(page.rows.iter().map(|row| PauseEntry::Option(page.id, row.id.clone())));
            }
            return rows;
        }
        let mut rows = PauseEntry::rows(self.in_session(), self.can_quit_to_title(), self.abandon());
        let index = rows.iter().position(|row| *row == PauseEntry::Audio(ShellAudioControl::Mute))
            .expect("audio rows in the pause menu");
        rows.splice(index..index, self.pages.0.iter().map(|page| PauseEntry::Page(page.id)));
        rows
    }
}

/// Where the footer line of the open menu starts (percent of the panel).
const FOOTER_TOP: f32 = 90.0;
/// Where the first row starts, how much of the panel the rows and their gaps
/// have, and the gap between two rows (percent of the panel). The rows end 2
/// above the footer.
const ROWS_TOP: f32 = 30.0;
const ROWS_SPAN: f32 = FOOTER_TOP - 2.0 - ROWS_TOP;
const ROW_GAP: f32 = 2.0;
const PAGE_OPTIONS_VISIBLE: usize = 6;

/// The height of each of `count` rows that share `span` with `gap` between
/// two of them, and no more than `most`.
///
/// `span / count` is not it: `count` rows of that height and their gaps are
/// `span + gap * (count - 1)`, and the last rows go past the span.
pub(crate) fn row_height_that_fits(span: f32, gap: f32, count: usize, most: f32) -> f32 {
    ((span + gap) / count.max(1) as f32 - gap).min(most)
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

/// The primary local input can operate a session with no human participants.
fn local_spectator(roster: &MenuPauseRoster) -> Option<u8> {
    (roster.scope.is_some() && roster.members.is_empty())
        .then_some(ParticipantId::PRIMARY.slot())
}

/// Adds the universal shell/system menu. Rides [`MinimalShellPlugins`], so
/// every host and standalone demo app gets it.
pub struct ShellPauseMenuPlugin;

impl Plugin for ShellPauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().spawn(ShellPauseMenuInstance);
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
            .init_resource::<ShellPausePages>()
            .add_message::<ShellPauseOption>()
            .add_message::<PauseRequested>()
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
            .configure_sets(Update, PublishShellPausePages
                .in_set(ambition_input::InputSet::Consume)
                .after(shell_pause_menu_pointer)
                .before(render_shell_pause_menu))
            // The open pause menu owns input, so a surface under it does not
            // act on the same presses.
            .add_systems(
                Update,
                declare_pause_context.in_set(ambition_input::InputSet::ResolveContext),
            )
            .add_systems(PostUpdate, aggregate_shell_menu_pause);
    }
}

/// Claim input while the pause menu is open.
///
/// Surfaces underneath (e.g. character select, which reads `SeatMenuFrames`)
/// check the claim and stop acting on input. Each open seat claims only its own input.
fn declare_pause_context(
    menu: Res<ShellPauseMenu>,
    mut participants: Query<
        (&ambition_input::InputParticipant, &mut ambition_input::participant::ParticipantContexts),
    >,
) {
    for (participant, mut contexts) in &mut participants {
        let open = menu.seats.contains_key(&participant.id.slot());
        if contexts.is_declared(ambition_input::PAUSE_CONTEXT) != open {
            contexts.sync(
                ambition_input::participant::ContextClaim::capturing(
                    ambition_input::PAUSE_CONTEXT,
                    ambition_input::participant::context_priority::PAUSE,
                ),
                open,
            );
        }
    }
}

/// Open and close the pause menu on Start, navigate, and dispatch the selected
/// entry. Pausing the sim ([`GameMode::Paused`]) is best-effort: a demo without
/// the `GameMode` state keeps running behind the menu.
#[allow(clippy::too_many_arguments)]
fn drive_shell_pause_menu(
    access: ambition_input::participant::ParticipantContextAccess,
    // Device-independent menu input (includes the touch "Menu" button).
    // Absent in an app with no host input stack.
    menu_frame: Option<Res<ambition_input::MenuControlFrame>>,
    // Per-seat frames: any seat can pause, and that seat drives the menu.
    // Absent in a standalone demo; then the global frame is the only seat.
    seat_frames: Option<Res<ambition_input::SeatMenuFrames>>,
    context: PauseMenuContext,
    roster: Option<Res<MenuPauseRoster>>,
    network: Option<Res<NetworkMenuPause>>,
    suppressed: Res<ShellPauseMenuSuppressed>,
    // The menu must not open over the launcher. `ShellPauseMenuSuppressed`
    // does not cover this: it tracks Ambition's own session, and the title
    // screen has no session. Read the launcher here instead of adding a second
    // writer to that flag.
    launcher: Option<Res<crate::launcher::ShellLauncherState>>,
    mut menu: ResMut<ShellPauseMenu>,
    mut shell: MessageWriter<ShellCommand>,
    mut abandon: MessageWriter<ShellAbandonRequested>,
    mut settings: Option<ResMut<ambition_persistence::settings::UserSettings>>,
    mut options: MessageWriter<ShellPauseOption>,
    mut sfx: SfxWriter,
    mut pause_requests: MessageReader<PauseRequested>,
) {
    // Read on every frame, so a request made while another surface owns the
    // screen is not acted on later.
    let pause_requested = pause_requests.read().count() > 0;
    // The launcher owns the screen. Close this menu if it is open.
    if launcher.as_deref().is_some_and(|state| state.active) {
        if menu.open {
            menu.close();
        }
        return;
    }
    // The active experience has its own pause menu. Close this one if it is
    // open and resume the sim.
    if suppressed.0 {
        if menu.open {
            menu.close();
        }
        return;
    }
    let scope = context.session.0.as_ref().map(|session| session.scope);
    if menu.scope != scope { menu.close(); menu.scope = scope; }
    let in_session = context.in_session();
    // Every seat can open or close its own menu even while another is open.
    // The shared presentation displays one seat at a time. Navigation from
    // a paused seat can bring that seat's own cursor to the foreground.
    let roster = roster.as_deref().filter(|roster| in_session && roster.scope == scope);
    let spectator = roster.filter(|_| network.is_none()).and_then(local_spectator);
    let permits = |slot| {
        roster.is_none_or(|roster| {
            spectator == Some(slot)
                || roster.members.iter().any(|(admitted, _)| *admitted == slot)
        })
            && access.permits(ParticipantId(slot),
                ambition_input::ContextClaim::capturing(ambition_input::PAUSE_CONTEXT,
                    ambition_input::participant::context_priority::PAUSE))
    };
    let inputs: Vec<_> = match seat_frames.as_deref() {
        Some(frames) => frames.seats().filter(|(slot, _)| permits(*slot)).collect(),
        None if permits(0) => vec![(0, menu_frame.as_deref().copied().unwrap_or_default())],
        None => vec![],
    };
    if pause_requested && in_session && !menu.seats.contains_key(&0) {
        menu.open_for(0);
        return;
    }
    let mut toggled = false;
    for (slot, frame) in &inputs {
        let edges = shell_action_edges(Some(frame));
        let open = menu.seats.contains_key(slot);
        if context.captures(&menu, *slot) { continue; }
        if open && edges.back && menu.location(*slot).is_some() {
            menu.select_owner(*slot);
            menu.open_page(None);
            play(&mut sfx, ids::UI_MENU_BACK);
            toggled = true;
        } else if edges.pause || (open && edges.back) {
            if open { menu.close_for(*slot); play(&mut sfx, ids::UI_MENU_BACK); }
            else { menu.open_for(*slot); play(&mut sfx, ids::UI_MENU_ACCEPT); }
            toggled = true;
        }
    }
    if toggled { return; }
    let active = inputs.iter().find(|(slot, frame)| {
        menu.seats.contains_key(slot) && !context.captures(&menu, *slot) && (frame.select || frame.up || frame.down || frame.left || frame.right)
    });
    if let Some((slot, _)) = active { menu.select_owner(*slot); }
    let edges = menu.owner.and_then(|slot| inputs.iter().find(|(at, _)| *at == slot))
        .map(|(_, frame)| shell_action_edges(Some(frame))).unwrap_or_default();

    if !menu.open || menu.owner.is_some_and(|owner| context.captures(&menu, owner)) {
        return;
    }
    if context.page(&menu).is_some_and(|page| page.owner.map(ParticipantId::slot) != menu.owner()) {
        return;
    }
    let rows = context.rows(&menu);

    // Use the shared `ListCursor` (wraps at both ends), like the other menus.
    // Play the move cue only when the selection changed.
    let mut cursor = ambition_ui_nav::ListCursor::new(menu.cursor, rows.len());
    if cursor.apply_directional(edges.previous, edges.next) {
        play(&mut sfx, ids::UI_MENU_MOVE);
    }
    menu.cursor = cursor.selected();

    let focused = rows.get(menu.cursor).cloned().unwrap_or(PauseEntry::Close);
    let direction = i32::from(edges.increase) - i32::from(edges.decrease);
    if edges.confirm || direction != 0 {
        activate_pause_entry(
            focused,
            if edges.confirm { 0 } else { direction },
            &mut menu,
            &mut shell,
            &mut abandon,
            settings.as_deref_mut(),
            &mut options,
            &mut sfx,
        );
    }
}

/// Pointer and touch activation for the pause rows. Uses the same activation
/// function as keyboard or controller confirm.
#[allow(clippy::too_many_arguments)]
fn shell_pause_menu_pointer(
    instance: Single<Entity, With<ShellPauseMenuInstance>>,
    context: PauseMenuContext,
    suppressed: Res<ShellPauseMenuSuppressed>,
    mut activated: MessageReader<MenuActionActivated<PauseEntry>>,
    mut menu: ResMut<ShellPauseMenu>,
    mut shell: MessageWriter<ShellCommand>,
    mut abandon: MessageWriter<ShellAbandonRequested>,
    mut settings: Option<ResMut<ambition_persistence::settings::UserSettings>>,
    mut options: MessageWriter<ShellPauseOption>,
    mut sfx: SfxWriter,
) {
    for activation in activated.read() {
        // Session-independent, like the keyboard path.
        if suppressed.0 || !menu.open || activation.menu != *instance {
            continue;
        }
        if let PauseEntry::PageStep(page, direction) = activation.action {
            if menu.page() == Some(page) && !menu.owner.is_some_and(|owner| context.captures(&menu, owner)) {
                let last = context.rows(&menu).len().saturating_sub(1);
                let step = PAGE_OPTIONS_VISIBLE as isize * direction as isize;
                menu.cursor = menu.cursor.saturating_add_signed(step).clamp(1, last.max(1));
            }
            continue;
        }
        let Some(cursor) = context.rows(&menu).iter().position(|entry| *entry == activation.action) else {
            continue;
        };
        if menu.owner.is_some_and(|owner| context.captures(&menu, owner))
            && activation.action != PauseEntry::Back
        {
            continue;
        }
        menu.cursor = cursor;
        activate_pause_entry(
            activation.action.clone(),
            0,
            &mut menu,
            &mut shell,
            &mut abandon,
            settings.as_deref_mut(),
            &mut options,
            &mut sfx,
        );
    }
}

fn activate_pause_entry(
    entry: PauseEntry,
    direction: i32,
    menu: &mut ShellPauseMenu,
    shell: &mut MessageWriter<ShellCommand>,
    // Written here; the experience acts on it. See [`ShellAbandonOffer`].
    abandon: &mut MessageWriter<ShellAbandonRequested>,
    settings: Option<&mut ambition_persistence::settings::UserSettings>,
    options: &mut MessageWriter<ShellPauseOption>,
    sfx: &mut SfxWriter,
) {
    match entry {
        // Confirm on an audio row is the positive direction: mute toggles and
        // volumes step up.
        PauseEntry::Audio(control) => {
            if let Some(settings) = settings {
                control.adjust(direction, settings);
            }
        }
        PauseEntry::Option(page, row) => {
            if let Some(owner) = menu.owner {
                options.write(ShellPauseOption { page, owner: ParticipantId(owner), row, direction });
            }
        }
        _ if direction != 0 => {}
        PauseEntry::Page(page) => menu.open_page(Some(page)),
        PauseEntry::Back => menu.open_page(None),
        PauseEntry::PageStep(_, _) => {}
        PauseEntry::Close => {
            menu.close_foreground();
            play(sfx, ids::UI_MENU_BACK);
        }
        PauseEntry::Resume => {
            menu.close_foreground();
            play(sfx, ids::UI_MENU_BACK);
        }
        PauseEntry::Abandon => {
            // Also resume the sim. Otherwise `GameMode::Paused` stays set and
            // the stage stays frozen.
            abandon.write(ShellAbandonRequested);
            menu.close();
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

#[derive(PartialEq)]
struct PausePresentation {
    title: &'static str,
    focused: usize,
    capture_hint: Option<&'static str>,
    scroll_page: Option<ShellPausePageId>,
    rows: Vec<(PauseEntry, String, String)>,
}

impl PausePresentation {
    fn read(menu: &ShellPauseMenu, context: &PauseMenuContext, settings: Option<&UserSettings>) -> Self {
        let default_settings;
        let settings = match settings {
            Some(settings) => settings,
            None => {
                default_settings = UserSettings::default();
                &default_settings
            }
        };
        let page = context.page(menu);
        let rows = context.rows(menu);
        let scroll_page = page.filter(|_| rows.len() > PAGE_OPTIONS_VISIBLE + 1).map(|page| page.id);
        let start = if scroll_page.is_some() { menu.cursor.saturating_sub(PAGE_OPTIONS_VISIBLE) } else { 0 };
        let rows = rows.into_iter().enumerate().filter_map(|(index, entry)| {
            (scroll_page.is_none() || index == 0 || (start + 1..=start + PAGE_OPTIONS_VISIBLE).contains(&index))
                .then_some(entry)
        }).map(|entry| {
            let (label, value) = entry.presentation(settings, context, menu);
            (entry, label, value)
        }).collect();
        let capture_hint = menu.owner.filter(|owner| context.captures(menu, *owner))
            .and_then(|_| page?.capture_hint);
        Self {
            title: page.map_or(if context.in_session() { "Paused" } else { "Settings" }, |page| page.title),
            focused: menu.cursor.saturating_sub(start),
            capture_hint,
            scroll_page,
            rows,
        }
    }
}

/// Rebuild or tear down the pause menu when its key changes (open, cursor,
/// route facts, audio values).
fn render_shell_pause_menu(
    mut commands: Commands,
    instance: Single<Entity, With<ShellPauseMenuInstance>>,
    menu: Res<ShellPauseMenu>,
    context: PauseMenuContext,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    asset_server: Option<Res<AssetServer>>,
    // `None` keeps Bevy's ASCII-only default font.
    menu_font: Option<Res<ambition_menu::render::bevy_ui::MenuFont>>,
    roots: Query<Entity, (With<BevyUiMenuRoot>, With<ShellPauseMenuRoot>)>,
    mut prior: Local<Option<PausePresentation>>,
) {
    let presentation = menu.open.then(|| PausePresentation::read(&menu, &context, settings.as_deref()));
    if *prior == presentation { return; }
    *prior = presentation;
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some(view_model) = prior.as_ref() else { return; };
    let heading = view_model.title;
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
    let row_span = if view_model.scroll_page.is_some() { ROWS_SPAN - 9.0 } else { ROWS_SPAN };
    let row_height = row_height_that_fits(row_span, ROW_GAP, view_model.rows.len(), 10.0);
    for (index, (entry, label, value)) in view_model.rows.iter().enumerate() {
        page.control(
            MenuRect::new(28.0, ROWS_TOP + index as f32 * (row_height + ROW_GAP), 44.0, row_height),
            MenuControlKind::Action,
            label.clone(),
            Some(value.clone()),
            view_model.focused == index,
            false,
            Some(entry.clone()),
        );
    }
    if let Some(id) = view_model.scroll_page {
        for (x, direction, label) in [(28.0, -1, "Previous options"), (51.0, 1, "More options")] {
            page.control(
                MenuRect::new(x, FOOTER_TOP - 9.0, 21.0, 7.0),
                MenuControlKind::Action, label, None, false, false, Some(PauseEntry::PageStep(id, direction)),
            );
        }
    }
    page.text(
        50.0,
        FOOTER_TOP,
        2.6,
        if let Some(hint) = view_model.capture_hint {
            hint
        } else if menu.page().is_some() {
            "Up / Down select · Left / Right adjust · Enter confirms · Esc goes back"
        } else if context.in_session() {
            "Up / Down select \u{b7} Left / Right adjust \u{b7} Enter confirms \u{b7} Esc resumes"
        } else {
            "Up / Down select \u{b7} Left / Right adjust \u{b7} Enter confirms \u{b7} Esc closes"
        },
        MenuTextAlign::Center,
        MenuColor::WHITE,
    );

    let tabs = [BevyUiMenuTabSpec::new(PausePage::Root, "Paused")];
    let view = BevyUiMenuView::<PausePage, PauseEntry> {
        menu: *instance,
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

/// Apply only a unanimous session-island vote. A shell without a gameplay
/// roster has no authority to infer multiplayer membership from devices.
fn aggregate_shell_menu_pause(
    menu: Res<ShellPauseMenu>,
    session: Res<ActiveGameplaySession>,
    suppressed: Res<ShellPauseMenuSuppressed>,
    roster: Option<Res<MenuPauseRoster>>,
    mut votes: Option<ResMut<LocalMenuPauseVotes>>,
    network: Option<Res<NetworkMenuPause>>,
    mode: Option<Res<State<GameMode>>>,
    next: Option<ResMut<NextState<GameMode>>>,
    mut hold: Local<MenuPauseHold>,
    mut previous_scope: Local<Option<SessionScopeId>>,
) {
    let scope = session.0.as_ref().map(|session| session.scope);
    if *previous_scope != scope {
        *hold = Default::default();
        *previous_scope = scope;
    }
    if !suppressed.0 {
        if let Some(votes) = votes.as_mut() {
            votes.publish(scope, menu.seats.keys().copied());
        }
    }
    let (Some(mode), Some(mut next)) = (mode, next) else { return; };
    let open = !suppressed.0 && network.is_none() && roster.as_deref().is_some_and(|roster| {
        let is_open = |slot| menu.seats.contains_key(&slot);
        roster.scope == scope
            && (local_spectator(roster).is_some_and(is_open) || roster.session_pause(is_open))
    });
    let pending = match &*next {
        NextState::Pending(mode) | NextState::PendingIfNeq(mode) => Some(*mode),
        NextState::Unchanged => None,
    };
    if let Some(target) = hold.update(open, *mode.get(), pending) {
        ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(target, "shell_menu_pause_vote");
        next.set(target);
    }
}

fn play(sfx: &mut SfxWriter, id: ambition_sfx::SfxId) {
    sfx.write(SfxMessage::Play {
        id,
        emitter: ambition_sfx::SfxEmitter::Global,
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

    #[test]
    fn each_seat_can_open_and_close_its_own_menu() {
        let mut app = couch_app();
        with_live_session(&mut app);
        seat_intent(&mut app, 0, |f| f.start = true);
        seat_intent(&mut app, 1, |f| f.start = true);
        assert_eq!(app.world().resource::<ShellPauseMenu>().seats.len(), 2);
        seat_intent(&mut app, 0, |f| f.back = true);
        assert!(app.world().resource::<ShellPauseMenu>().open);
        assert_eq!(app.world().resource::<ShellPauseMenu>().owner(), Some(1));
        seat_intent(&mut app, 1, |f| f.back = true);
        assert!(!app.world().resource::<ShellPauseMenu>().open);
    }

    #[test]
    fn a_local_spectator_can_pause_resume_and_leave_the_match() {
        for entry in [PauseEntry::Abandon, PauseEntry::QuitToTitle] {
            let mut app = couch_app();
            app.add_plugins(bevy::state::app::StatesPlugin)
                .insert_state(GameMode::Playing);
            with_live_session(&mut app);
            let scope = app.world().resource::<ActiveGameplaySession>().0.as_ref().unwrap().scope;
            app.insert_resource(MenuPauseRoster { scope: Some(scope), members: Vec::new() })
                .insert_resource(ShellAbandonOffer {
                    label: "Exit Match".into(),
                    detail: String::new(),
                });

            seat_intent(&mut app, 1, |frame| frame.start = true);
            assert!(!app.world().resource::<ShellPauseMenu>().open);

            let primary = ParticipantId::PRIMARY.slot();
            seat_intent(&mut app, primary, |frame| frame.start = true);
            app.update();
            assert_eq!(app.world().resource::<State<GameMode>>().get(), &GameMode::Paused);
            seat_intent(&mut app, primary, |frame| frame.back = true);
            app.update();
            assert_eq!(app.world().resource::<State<GameMode>>().get(), &GameMode::Playing);

            seat_intent(&mut app, primary, |frame| frame.start = true);
            let offer = app.world().resource::<ShellAbandonOffer>();
            let row = PauseEntry::rows(true, true, Some(offer))
                .iter().position(|row| *row == entry).unwrap();
            for _ in 0..row {
                seat_intent(&mut app, primary, |frame| frame.down = true);
            }
            seat_intent(&mut app, primary, |frame| frame.select = true);
            assert!(!app.world().resource::<ShellPauseMenu>().open);
            if entry == PauseEntry::Abandon {
                assert_eq!(app.world_mut().resource_mut::<Messages<ShellAbandonRequested>>().drain().count(), 1);
            } else {
                assert!(app.world_mut().resource_mut::<Messages<ShellCommand>>().drain().any(|command| command == ShellCommand::QuitToHome));
            }
        }
    }

    /// The Start intent (Escape, controller Start, touch "Menu"). Escape also
    /// maps to MenuBack, so both arrive together.
    fn press_start(app: &mut App) {
        intent(app, |f| {
            f.start = true;
            f.back = true;
        });
    }

    /// The platform's pause (the app went to the background) opens the menu
    /// over a live session, and another request does not close it.
    #[test]
    fn a_platform_pause_opens_the_menu_and_keeps_it_open() {
        let mut app = app();
        with_live_session(&mut app);
        app.update();
        app.world_mut().write_message(PauseRequested);
        app.update();
        assert!(app.world().resource::<ShellPauseMenu>().open);
        app.world_mut().write_message(PauseRequested);
        app.update();
        assert!(
            app.world().resource::<ShellPauseMenu>().open,
            "a request is not a toggle"
        );
    }

    /// With no session there is nothing to pause.
    #[test]
    fn a_platform_pause_without_a_session_opens_nothing() {
        let mut app = app();
        app.world_mut().write_message(PauseRequested);
        app.update();
        assert!(!app.world().resource::<ShellPauseMenu>().open);
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

    /// Each row of the open menu ends above the footer line, in a session
    /// (7 rows) and with no session.
    ///
    /// The rows were sized to share a span and then spaced with a gap the
    /// span did not count, so from 6 rows on the last row was under the
    /// footer (measured 2026-10-09 in Mary-O: "Quit to Desktop" under "Up /
    /// Down select").
    #[test]
    fn the_last_row_of_the_open_menu_ends_above_the_footer() {
        for in_session in [false, true] {
            let mut app = app();
            if in_session {
                with_live_session(&mut app);
            }
            press_start(&mut app);
            let mut controls = app
                .world_mut()
                .query::<(&Node, &ambition_menu::AmbitionMenuControl<PauseEntry>)>();
            let bottoms: Vec<f32> = controls
                .iter(app.world())
                .map(|(node, _)| match (node.top, node.height) {
                    (Val::Percent(top), Val::Percent(height)) => top + height,
                    other => panic!("a row is laid out in {other:?}, and this test reads percent"),
                })
                .collect();
            assert!(
                bottoms.len() >= 6,
                "premise: the open menu (in session: {in_session}) has {} rows",
                bottoms.len()
            );
            let lowest = bottoms.iter().copied().fold(0.0, f32::max);
            assert!(
                lowest <= FOOTER_TOP,
                "in session: {in_session}: the last of {} rows ends at {lowest:.1}% and the \
                 footer starts at {FOOTER_TOP}%",
                bottoms.len()
            );
        }
    }

    /// Rows of the height that fits, and their gaps, are in the span for
    /// each count, and few rows keep the cap. The launcher's five rows keep
    /// the height they had.
    #[test]
    fn rows_and_their_gaps_are_in_the_span() {
        for count in 1..=12usize {
            let height = row_height_that_fits(ROWS_SPAN, ROW_GAP, count, 10.0);
            let used = height * count as f32 + ROW_GAP * (count - 1) as f32;
            assert!(height > 0.0 && used <= ROWS_SPAN + 1e-3, "{count} rows use {used} of {ROWS_SPAN}");
        }
        assert_eq!(row_height_that_fits(ROWS_SPAN, ROW_GAP, 3, 10.0), 10.0);
        assert!((row_height_that_fits(72.0, 1.5, 5, 16.0) - 13.2).abs() < 1e-4);
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
        crate::basic_presentation::pointer_press(&mut app, pressed);
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
            crate::basic_presentation::pointer_press(app, pressed);
            let released = row(app);
            crate::basic_presentation::pointer_release(app, released);
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
