//! The gate terminal's model: what a terminal shows and what a press on it
//! asks of the simulation.
//!
//! The simulation owns which terminal is open and what each dial answered
//! (`GateNetworkState`). This module owns the terminal's local state (the
//! chevrons entered, where the wheel points, the list cursor) and turns a
//! press into a [`GateTerminalIntent`], which enters the simulation through
//! the host intent ledger on a stamped tick. Nothing here moves a body or
//! opens a ring.
//!
//! The directory groups known gates by dimension. Recent addresses have
//! their own page. Local portal arches list destinations directly.
//! The manual dial uses the ring's chevrons ([`chevron_angle`]) and an
//! engage press (a solid Δ) at the bottom. A gamepad points its stick;
//! arrow keys and the d-pad step around the wheel. A mouse or finger can
//! press a chevron. The wheel distinguishes repeated letters (Λ, Λ₂, Λ₃).
//!
//! The presentation (`presentation::gate_terminal`) draws [`TerminalModel`]
//! and feeds presses back through [`TerminalUi`], so every device uses one
//! road. While a terminal is open it takes the seat's input
//! ([`GATE_TERMINAL_CONTEXT`]), so the body at the terminal stands still.

use ambition_content_schemas::gate_network::Chevron;
use crate::gate_inscription::address_fits;
use ambition_content_schemas::gate_network::GateWords;
use ambition_platformer2d::world::rooms::{
    GateNetworkCatalog, GateNetworkState, GateRingPositions, GateTerminalAction, GateTerminalIntent,
    RingKind,
};
use bevy::prelude::*;

/// The most chevrons an address field holds: the longest address a ring's
/// position grids can spell (`MAX_ADDRESS_LEN`).
pub const ADDRESS_FIELD_LEN: usize = crate::gate_inscription::MAX_ADDRESS_LEN;

/// The slice of the wheel with no chevron, at the bottom: on the ring the
/// base it stands in, on a terminal the engage press. The ring's art draws
/// the same layout (`glyphs.legally_distinct.ARCH_GAP_DEG`).
pub const ARCH_GAP_DEG: f32 = 96.0;

/// The input context of an open gate terminal: it takes the seat's input
/// from gameplay, above a dialogue and below the inventory.
pub const GATE_TERMINAL_CONTEXT: ambition_input::participant::InputContextId =
    ambition_input::participant::InputContextId("gate_terminal");
/// Its claim priority is above dialogue and below the unified pause menu.
pub const GATE_TERMINAL_PRIORITY: i32 = 155;

/// Where chevron `index` of `count` sits round the wheel: degrees clockwise
/// from 3 o'clock with y DOWN (screen space), from the lower left over the
/// top to the lower right, the bottom slice left for the engage press.
pub fn chevron_angle(index: usize, count: usize) -> f32 {
    let step = (360.0 - ARCH_GAP_DEG) / (count.max(2) - 1) as f32;
    90.0 + ARCH_GAP_DEG / 2.0 + index as f32 * step
}

/// One place on the wheel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WheelSlot {
    Chevron(Chevron),
    /// The engage press, in the slice at the bottom: it dials.
    Engage,
}

impl WheelSlot {
    /// The slot at the angle `degrees` (clockwise from 3 o'clock, y down):
    /// the nearest chevron, or the engage press when the bottom is nearer.
    pub fn at_angle(degrees: f32, count: usize) -> Self {
        let off = |target: f32| ((degrees - target).rem_euclid(360.0)).min((target - degrees).rem_euclid(360.0));
        let mut best = (off(90.0), Self::Engage);
        for index in 0..count {
            let distance = off(chevron_angle(index, count));
            if distance < best.0 {
                best = (distance, Self::Chevron(index as Chevron));
            }
        }
        best.1
    }

    /// The next slot clockwise (`step` 1) or anticlockwise (-1): round the
    /// chevrons in order, and through the engage press between the last and
    /// the first.
    pub fn stepped(self, step: i32, count: usize) -> Self {
        let ring = count as i32 + 1;
        let at = match self {
            Self::Chevron(chevron) => i32::from(chevron),
            Self::Engage => count as i32,
        };
        let next = (at + step).rem_euclid(ring);
        if next == count as i32 { Self::Engage } else { Self::Chevron(next as Chevron) }
    }
}

/// What the terminal shows, and what the menu keys drive. The list is the
/// terminal (Jon, 2026-10-25: the dial "should be secondary, you are almost
/// never going to use it"); the wheel is a view behind its "Dial an
/// address" row, and Back returns from it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TerminalFocus {
    /// The dial view: point and press to enter chevrons.
    Wheel,
    /// The list: known and recent addresses, and the actions.
    #[default]
    List,
}

/// The directory page. Opening a page does not send a simulation request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TerminalDirectory {
    #[default]
    Dimensions,
    Dimension(String),
    Recent,
}

/// One row of the terminal directory.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ListRow {
    Dimension { id: String, name: String, destinations: usize },
    Recent { destinations: usize },
    /// Return to the dimension list.
    Dimensions,
    /// An address: a press dials it.
    Address { address: Vec<Chevron>, word: String, name: String, recent: bool },
    /// Open the dial view, to enter an address chevron by chevron.
    Dial,
    Hangup,
    Close,
}

/// What one press asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalEffect {
    /// Nothing for the simulation: the terminal's own state changed.
    None,
    /// Send this request to the simulation.
    Send(GateTerminalAction),
}

/// The terminal's local state, for the ring whose terminal is open.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct TerminalUi {
    /// The actor whose terminal this participant controls.
    pub subject: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId>,
    /// The ring the state belongs to. A different ring starts a fresh state.
    pub ring: Option<String>,
    pub focus: TerminalFocus,
    pub directory: TerminalDirectory,
    /// The cursor and window in the parent directory.
    pub(crate) parent_position: (usize, usize),
    /// The chevrons entered.
    pub entered: Vec<Chevron>,
    /// Where the wheel points.
    pub pointed: Option<WheelSlot>,
    /// The row of the list the cursor is on.
    pub list_cursor: usize,
    /// The first row of the list shown: the list shows [`LIST_ROWS`] rows at
    /// a time. The cursor's steps keep the cursor inside it; a mouse wheel or
    /// a drag moves it without moving the cursor.
    pub list_window: usize,
    /// The row the pointer is over. Presentation only (Q70): a hover is drawn,
    /// and never moves the cursor; a press selects.
    pub list_hover: Option<usize>,
    /// How many rows fit on the screen (0: [`LIST_ROWS`]); the presentation
    /// sets it from the screen's height, so a phone shows fewer.
    pub list_rows: usize,
    /// A short screen (a phone held sideways): the presentation draws the
    /// list with a one-line heading and no hint.
    pub compact: bool,
    /// A press on a row, armed until the pointer comes up on it: a tap
    /// activates, a drag (a finger scrolling the list) does not.
    pub row_press: ambition_ui_nav::RowPress,
    /// The report serial this terminal already acted on.
    pub seen_serial: Option<u64>,
}

/// How many rows of the list the terminal shows at once, when the
/// presentation has not measured the screen.
pub const LIST_ROWS: usize = 8;

/// What a terminal shows, built from the catalog, the network's state, the
/// save and [`TerminalUi`]. Pure data, so a test reads it without a window.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalModel {
    pub title: String,
    pub facility: String,
    pub note: String,
    /// How many chevrons the wheel has, and the letter each stands for.
    pub inscription: Vec<char>,
    pub entered: Vec<Chevron>,
    /// The word of the entered chevrons, when a ring has that address.
    pub entered_word: Option<String>,
    /// The line under the address: the link, the last answer, or a hint.
    pub status: String,
    /// Whether the status line reports a refusal.
    pub refused: bool,
    pub focus: TerminalFocus,
    pub pointed: Option<WheelSlot>,
    pub list: Vec<ListRow>,
    pub list_title: String,
    pub list_hint: String,
    pub list_cursor: usize,
    /// The first row shown, how many are shown, and the row the pointer is over.
    pub list_window: usize,
    pub list_rows: usize,
    pub list_hover: Option<usize>,
    pub compact: bool,
}

/// The addresses a terminal lists as known, as chevrons: those written on it
/// first, then each the traveller knows (`gate_network::knows`: what the
/// save records, or every address while the directory starts with all
/// known), in catalog order, words and glyph addresses alike. `flag` reads a
/// save flag.
///
/// A terminal lists only what its ring can reach: a gate lists gates; a
/// portal arch lists the arches of its own world within its reach on the flat
/// map (`positions`), so its list is where it can take the traveller.
pub fn known_addresses(
    catalog: &GateNetworkCatalog,
    words: &GateWords,
    directory: &ambition_content_schemas::gate_directory::GateDirectory,
    positions: &GateRingPositions,
    ring: &str,
    flag: impl Fn(&str) -> bool,
) -> Vec<Vec<Chevron>> {
    let mut listed: Vec<Vec<Chevron>> =
        directory.posted(ring).iter().filter_map(|word| words.spell(word).ok()).collect();
    let from = catalog.ring(ring);
    let reachable = |candidate: &ambition_platformer2d::world::rooms::GateRing| {
        let Some(from) = from else {
            return true;
        };
        if candidate.kind != from.kind {
            return false;
        }
        if from.kind != RingKind::Portal {
            return true;
        }
        let dimension = |ring: &ambition_platformer2d::world::rooms::GateRing| {
            catalog.facility(&ring.facility).map(|facility| facility.world.clone())
        };
        dimension(candidate) == dimension(from)
            && positions.distance(&from.zone, &candidate.zone).is_some_and(|distance| distance <= from.reach_px() as f32)
    };
    for candidate in &catalog.rings {
        if candidate.zone != ring && reachable(candidate) && crate::gate_network::knows(directory, words, candidate, &flag) && !listed.contains(&candidate.address) {
            listed.push(candidate.address.clone());
        }
    }
    listed
}

fn address_row(catalog: &GateNetworkCatalog, words: &GateWords, address: Vec<Chevron>, recent: bool) -> ListRow {
    let ring = catalog.ring_at(&address);
    ListRow::Address {
        word: ring.and_then(|ring| words.word(&ring.zone)).unwrap_or_default().to_string(),
        name: ring.map_or_else(|| "No ring answered".to_string(), |ring| ring.name.clone()),
        address,
        recent,
    }
}

fn address_dimension<'a>(catalog: &'a GateNetworkCatalog, address: &[Chevron]) -> Option<&'a str> {
    let ring = catalog.ring_at(address)?;
    Some(&catalog.facility(&ring.facility)?.world)
}

fn dimension_name(id: &str) -> String {
    id.split(['_', '-']).map(|word| {
        let mut letters = word.chars();
        letters.next().map_or_else(String::new, |first| first.to_uppercase().collect::<String>() + letters.as_str())
    }).collect::<Vec<_>>().join(" ")
}

impl TerminalUi {
    /// Open the directory for this ring with no address entered.
    pub fn open(&mut self, ring: &str, seen_serial: Option<u64>) {
        *self = Self { subject: self.subject.clone(), ring: Some(ring.to_string()), seen_serial, ..default() };
    }

    /// Group listed gates by their facility's dimension. Local arches keep a flat list.
    pub fn list(&self, catalog: &GateNetworkCatalog, words: &GateWords, state: &GateNetworkState, known: &[Vec<Chevron>]) -> Vec<ListRow> {
        let mut addresses = known.to_vec();
        for address in &state.recent {
            if !addresses.contains(address) {
                addresses.push(address.clone());
            }
        }
        let address_rows = |addresses: Vec<Vec<Chevron>>| addresses.into_iter()
            .map(|address| {
                let recent = state.recent.contains(&address);
                address_row(catalog, words, address, recent)
            }).collect::<Vec<_>>();
        let mut rows = if self.is_local_portal(catalog) {
            address_rows(addresses)
        } else {
            match &self.directory {
                TerminalDirectory::Dimensions => {
                    let mut dimensions = std::collections::BTreeMap::<&str, usize>::new();
                    for address in &addresses {
                        if let Some(dimension) = address_dimension(catalog, address) {
                            *dimensions.entry(dimension).or_default() += 1;
                        }
                    }
                    let mut rows: Vec<_> = dimensions.into_iter().map(|(id, destinations)| ListRow::Dimension {
                        id: id.to_string(), name: dimension_name(id), destinations,
                    }).collect();
                    if !state.recent.is_empty() {
                        rows.push(ListRow::Recent { destinations: state.recent.len() });
                    }
                    rows
                }
                TerminalDirectory::Dimension(id) => {
                    let mut rows = vec![ListRow::Dimensions];
                    rows.extend(address_rows(addresses.into_iter()
                        .filter(|address| address_dimension(catalog, address) == Some(id.as_str())).collect()));
                    rows
                }
                TerminalDirectory::Recent => {
                    let mut rows = vec![ListRow::Dimensions];
                    rows.extend(address_rows(state.recent.clone()));
                    rows
                }
            }
        };
        rows.push(ListRow::Dial);
        if state.route_from(self.ring.as_deref().unwrap_or_default()).is_some() {
            rows.push(ListRow::Hangup);
        }
        rows.push(ListRow::Close);
        rows
    }

    fn is_local_portal(&self, catalog: &GateNetworkCatalog) -> bool {
        self.ring.as_deref().and_then(|ring| catalog.ring(ring)).is_some_and(|ring| ring.kind == RingKind::Portal)
    }

    fn open_directory(&mut self, directory: TerminalDirectory) {
        self.parent_position = (self.list_cursor, self.list_window);
        self.directory = directory;
        self.list_cursor = 1;
        self.list_window = 0;
        self.list_hover = None;
        self.row_press.clear();
    }

    fn show_dimensions(&mut self) {
        self.directory = TerminalDirectory::Dimensions;
        (self.list_cursor, self.list_window) = self.parent_position;
        self.list_hover = None;
        self.row_press.clear();
    }

    /// Point the wheel at a slot.
    pub fn point(&mut self, slot: WheelSlot) {
        self.focus = TerminalFocus::Wheel;
        self.pointed = Some(slot);
    }

    /// Step the wheel's pointer round by `step`, from the top when it points
    /// nowhere yet.
    pub fn step_wheel(&mut self, step: i32, count: usize) {
        let from = self.pointed.unwrap_or(WheelSlot::at_angle(270.0, count));
        self.point(if self.pointed.is_some() { from.stepped(step, count) } else { from });
    }

    /// Move the list cursor by `step`, with wrap at both ends. Move the
    /// window only as far as needed to show the cursor.
    pub fn step_list(&mut self, step: i32, rows: usize) {
        self.focus = TerminalFocus::List;
        let mut cursor = ambition_ui_nav::ListCursor::new(self.list_cursor, rows);
        for _ in 0..step.unsigned_abs() {
            cursor.apply_directional(step < 0, step > 0);
        }
        self.list_cursor = cursor.selected();
        self.list_window = ambition_ui_nav::scroll_into_view(self.list_window, self.list_cursor, rows, self.shown_rows());
    }

    /// How many rows the list shows at once.
    pub fn shown_rows(&self) -> usize {
        if self.list_rows == 0 { LIST_ROWS } else { self.list_rows }
    }

    /// Move the window of the list by `steps` rows (a mouse wheel, a drag),
    /// leaving the cursor where it is.
    pub fn scroll_list(&mut self, steps: i32, rows: usize) {
        let last = rows.saturating_sub(self.shown_rows()) as i32;
        self.list_window = (self.list_window as i32 + steps).clamp(0, last) as usize;
    }

    /// Carry out a press of a wheel slot: a chevron is entered, again each
    /// time it is pressed, while the address keeps the limits (`address_fits`:
    /// a fifth use of a chevron has no row on its plaque); the engage press
    /// dials what was entered.
    pub fn press_slot(&mut self, slot: WheelSlot) -> TerminalEffect {
        self.point(slot);
        match slot {
            WheelSlot::Chevron(chevron) => {
                self.entered.push(chevron);
                if !address_fits(&self.entered) {
                    self.entered.pop();
                }
                TerminalEffect::None
            }
            WheelSlot::Engage => TerminalEffect::Send(GateTerminalAction::Dial(self.entered.clone())),
        }
    }

    /// Open a directory page or send the selected address or action.
    pub fn press_row(&mut self, row: &ListRow) -> TerminalEffect {
        match row {
            ListRow::Dimension { id, .. } => {
                self.open_directory(TerminalDirectory::Dimension(id.clone()));
                TerminalEffect::None
            }
            ListRow::Recent { .. } => {
                self.open_directory(TerminalDirectory::Recent);
                TerminalEffect::None
            }
            ListRow::Dimensions => {
                self.show_dimensions();
                TerminalEffect::None
            }
            ListRow::Address { address, .. } => {
                self.entered = address.clone();
                TerminalEffect::Send(GateTerminalAction::Dial(address.clone()))
            }
            ListRow::Dial => {
                self.entered.clear();
                self.focus = TerminalFocus::Wheel;
                TerminalEffect::None
            }
            ListRow::Hangup => TerminalEffect::Send(GateTerminalAction::Hangup),
            ListRow::Close => TerminalEffect::Send(GateTerminalAction::Close),
        }
    }

    /// The back press. In the dial view: take back the last chevron, then
    /// return to the list. In a directory page: return to dimensions, then leave.
    pub fn back(&mut self) -> TerminalEffect {
        match self.focus {
            TerminalFocus::Wheel => {
                if self.entered.pop().is_none() {
                    self.focus = TerminalFocus::List;
                }
                TerminalEffect::None
            }
            TerminalFocus::List if self.directory != TerminalDirectory::Dimensions => {
                self.show_dimensions();
                TerminalEffect::None
            }
            TerminalFocus::List => TerminalEffect::Send(GateTerminalAction::Close),
        }
    }

    /// Swap what the menu keys drive: the wheel or the list.
    pub fn swap_focus(&mut self) {
        self.focus = match self.focus {
            TerminalFocus::Wheel => TerminalFocus::List,
            TerminalFocus::List => TerminalFocus::Wheel,
        };
    }

    /// What the terminal shows now.
    pub fn model(&self, catalog: &GateNetworkCatalog, words: &GateWords, state: &GateNetworkState, known: &[Vec<Chevron>]) -> TerminalModel {
        let ring_id = self.ring.as_deref().unwrap_or_default();
        let ring = catalog.ring(ring_id);
        let facility = ring
            .and_then(|ring| catalog.facility(&ring.facility))
            .map_or_else(String::new, |facility| facility.name.clone());
        let far_name = |zone: &str| catalog.ring(zone).map_or_else(|| zone.to_string(), |ring| ring.name.clone());
        let word = |address: &[Chevron]| words.word_of(catalog, address).map(str::to_string);
        let (list_title, list_status, list_hint) = if self.is_local_portal(catalog) {
            ("Destinations".to_string(), "Choose a destination.", "Select a destination to dial it  ·  drag or scroll for more  ·  Back leaves")
        } else {
            match &self.directory {
                TerminalDirectory::Dimensions => ("Dimensions".to_string(), "Choose a dimension.", "Select a dimension, then a gate  ·  drag or scroll for more  ·  Back leaves"),
                TerminalDirectory::Dimension(id) => (format!("Dimensions  /  {}", dimension_name(id)), "Choose a gate.", "Select a gate to dial it  ·  drag or scroll for more  ·  Back returns to dimensions"),
                TerminalDirectory::Recent => ("Dimensions  /  Recent addresses".to_string(), "Choose a recent address.", "Select an address to dial it  ·  Back returns to dimensions"),
            }
        };
        let (status, refused) = if let Some(link) = state.route_from(ring_id) {
            (format!("LINK OPEN to {}. Walk into the ring.", far_name(&link.far)), false)
        } else if state.holds_open(ring_id) {
            ("INCOMING: the ring is receiving.".to_string(), false)
        } else if let Some(report) = self.subject.as_ref().and_then(|subject| state.report_for(subject, ring_id)) {
            match &report.outcome {
                Ok(far) => (format!("The link to {} has closed.", far_name(far)), false),
                Err(refusal) => (crate::gate_inscription::refusal_words(refusal), true),
            }
        } else if self.focus == TerminalFocus::List {
            (list_status.to_string(), false)
        } else {
            ("Point at a chevron and press to enter it; press \u{0394} to dial.".to_string(), false)
        };
        let list = self.list(catalog, words, state, known);
        TerminalModel {
            title: ring.map_or_else(|| ring_id.to_string(), |ring| ring.name.clone()),
            facility,
            note: ring.map_or_else(String::new, |ring| ring.note.clone()),
            inscription: words.inscription.clone(),
            entered: self.entered.clone(),
            entered_word: word(&self.entered),
            status,
            refused,
            focus: self.focus,
            pointed: self.pointed,
            list_cursor: self.list_cursor.min(list.len().saturating_sub(1)),
            list_window: self.list_window.min(list.len().saturating_sub(self.shown_rows())),
            list_rows: self.shown_rows(),
            list_hover: self.list_hover.filter(|row| *row < list.len()),
            compact: self.compact,
            list_title,
            list_hint: list_hint.to_string(),
            list,
        }
    }
}

/// The unique actor controlled by a participant at this boundary.
pub(super) fn terminal_subject(
    participant: ambition_input::ParticipantId,
    drivers: &Query<(Entity, &ambition_platformer2d::characters::control::DrivingParticipant)>,
    bodies: &ambition_platformer2d_shared_tangle::lifecycle::LiveBodies,
) -> Option<ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId> {
    let mut matches = drivers.iter().filter(|(_, driver)| driver.0.0 == participant.slot());
    let subject = matches.next().and_then(|(entity, _)| bodies.id_of(entity));
    matches.next().is_none().then_some(subject).flatten()
}

/// Capture only the input of a participant whose actor uses a terminal.
pub fn declare_gate_terminal_context(
    state: Option<Res<GateNetworkState>>,
    drivers: Query<(Entity, &ambition_platformer2d::characters::control::DrivingParticipant)>,
    bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies,
    mut participants: Query<(&ambition_input::participant::InputParticipant,
        &mut ambition_input::participant::ParticipantContexts)>,
) {
    use ambition_input::participant::ContextClaim;
    for (participant, mut contexts) in &mut participants {
        let open = terminal_subject(participant.id, &drivers, &bodies)
            .is_some_and(|subject| state.as_ref().is_some_and(|state| state.terminals.contains_key(&subject)));
        if contexts.is_declared(GATE_TERMINAL_CONTEXT) != open {
            contexts.sync(ContextClaim::capturing(GATE_TERMINAL_CONTEXT, GATE_TERMINAL_PRIORITY), open);
        }
    }
}

/// Whether a gate terminal is open now.
pub fn a_gate_terminal_is_open(state: Option<Res<GateNetworkState>>) -> bool {
    state.is_some_and(|state| !state.terminals.is_empty())
}

/// The run condition of [`declare_gate_terminal_context`]: a terminal is
/// open, or was open on the last frame (the claim is taken back once).
pub fn a_gate_terminal_is_or_was_open(state: Option<Res<GateNetworkState>>, mut was: Local<bool>) -> bool {
    let open = state.is_some_and(|state| !state.terminals.is_empty());
    let run = open || *was;
    *was = open;
    run
}

/// Install the terminal's input claim and its local state.
pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        declare_gate_terminal_context
            .run_if(a_gate_terminal_is_or_was_open)
            .in_set(ambition_input::InputSet::ResolveContext),
    );
}

/// A request the terminal sends, as the host intent it is.
pub fn intent(subject: &ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId, ring: &str, action: GateTerminalAction) -> GateTerminalIntent {
    GateTerminalIntent { subject: subject.clone(), ring: ring.to_string(), action }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> GateNetworkCatalog {
        crate::gate_network::lowered_gate_network(crate::pack::shipped())
    }

    fn words() -> GateWords {
        crate::gate_network::lowered_gate_words(crate::pack::shipped())
    }

    fn directory() -> ambition_content_schemas::gate_directory::GateDirectory {
        crate::gate_network::lowered_directory(crate::pack::shipped())
    }

    /// The wheel is laid out as the ring: the first chevron at the lower
    /// left, the middle one (Δ) at the top, the last at the lower right, and
    /// the engage press at the bottom.
    #[test]
    fn the_wheel_is_laid_out_as_the_ring() {
        let n = catalog().symbols();
        assert_eq!(n, 15);
        assert!((chevron_angle(7, n) - 270.0).abs() < 1e-3, "Δ at 12 o'clock");
        assert_eq!(WheelSlot::at_angle(270.0, n), WheelSlot::Chevron(7));
        assert_eq!(WheelSlot::at_angle(90.0, n), WheelSlot::Engage, "straight down is the engage press");
        assert_eq!(WheelSlot::at_angle(180.0, n), WheelSlot::at_angle(chevron_angle(2, n), n), "left");
        assert_eq!(WheelSlot::at_angle(chevron_angle(0, n) + 3.0, n), WheelSlot::Chevron(0));
        assert_eq!(WheelSlot::Chevron(14).stepped(1, n), WheelSlot::Engage);
        assert_eq!(WheelSlot::Engage.stepped(1, n), WheelSlot::Chevron(0));
        assert_eq!(WheelSlot::Chevron(0).stepped(-1, n), WheelSlot::Engage);
    }

    /// A pressed chevron is entered each time, the back press takes the last
    /// one back before it leaves, and the engage press dials the chevrons.
    #[test]
    fn chevrons_are_entered_by_the_wheel_and_engaged_by_the_delta() {
        let catalog = catalog();
        let words = words();
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", None);
        let tangent = words.spell("TANGENT").unwrap();
        for chevron in &tangent {
            assert_eq!(ui.press_slot(WheelSlot::Chevron(*chevron)), TerminalEffect::None);
        }
        ui.press_slot(WheelSlot::Chevron(0));
        assert_eq!(ui.back(), TerminalEffect::None);
        assert_eq!(ui.press_slot(WheelSlot::Engage), TerminalEffect::Send(GateTerminalAction::Dial(tangent.clone())));
        let model = ui.model(&catalog, &words, &GateNetworkState::default(), &[]);
        assert_eq!(model.entered_word.as_deref(), Some("TANGENT"));
        for _ in 0..tangent.len() {
            ui.back();
        }
        assert_eq!(ui.back(), TerminalEffect::None, "an empty dial returns to the list");
        assert_eq!(ui.focus, TerminalFocus::List);
        assert_eq!(ui.back(), TerminalEffect::Send(GateTerminalAction::Close), "and the list's back leaves");
    }

    /// The field keeps the ring's limits: a fifth use of a chevron and a
    /// seventeenth chevron are not entered.
    #[test]
    fn the_field_keeps_the_rings_limits() {
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", None);
        for _ in 0..5 {
            ui.press_slot(WheelSlot::Chevron(3));
        }
        assert_eq!(ui.entered, [3, 3, 3, 3]);
        for chevron in 0..14 {
            ui.press_slot(WheelSlot::Chevron(chevron));
        }
        assert_eq!(ui.entered.len(), ADDRESS_FIELD_LEN);
    }

    /// Dimension selection changes only the page. A gate selection dials its address.
    #[test]
    fn the_list_dials_known_and_recent_addresses() {
        let catalog = catalog();
        let words = words();
        let mut state = GateNetworkState::default();
        let wrong = vec![0, 1, 2, 3, 0];
        state.recent = vec![words.spell("GENESIS").unwrap(), wrong.clone()];
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", None);
        let known = [words.spell("GENESIS").unwrap()];
        let rows = ui.list(&catalog, &words, &state, &known);
        assert!(matches!(&rows[0], ListRow::Dimension { id, destinations: 1, .. } if id == "jungle"));
        assert_eq!(rows.last(), Some(&ListRow::Close));
        assert_eq!(rows[rows.len() - 2], ListRow::Dial, "the dial is a row of the list, after the addresses");
        assert_eq!(ui.press_row(&rows[0]), TerminalEffect::None);
        let gates = ui.list(&catalog, &words, &state, &known);
        assert_eq!(gates[0], ListRow::Dimensions);
        assert!(matches!(&gates[1], ListRow::Address { word, recent: true, .. } if word == "GENESIS"));
        assert_eq!(
            ui.press_row(&gates[1]),
            TerminalEffect::Send(GateTerminalAction::Dial(words.spell("GENESIS").unwrap()))
        );
        assert_eq!(ui.back(), TerminalEffect::None);
        let recent = rows.iter().find(|row| matches!(row, ListRow::Recent { .. })).unwrap();
        assert_eq!(ui.press_row(recent), TerminalEffect::None);
        let recent_rows = ui.list(&catalog, &words, &state, &known);
        assert!(matches!(&recent_rows[2], ListRow::Address { address, word, recent: true, .. } if *address == wrong && word.is_empty()), "{recent_rows:?}");
        assert_eq!(ui.press_row(&recent_rows[2]), TerminalEffect::Send(GateTerminalAction::Dial(wrong)));
    }

    #[test]
    fn the_directory_groups_only_listed_gates_and_back_restores_its_position() {
        let catalog = catalog();
        let words = words();
        let state = GateNetworkState::default();
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", None);
        let known = known_addresses(&catalog, &words, &directory(), &GateRingPositions::default(), "hub_public_gate_zone", |_| false);
        let rows = ui.list(&catalog, &words, &state, &known);
        assert!(!rows.iter().any(|row| matches!(row, ListRow::Address { .. })));
        let science = rows.iter().position(|row| matches!(row, ListRow::Dimension { id, .. } if id == "science")).unwrap();
        ui.step_list(science as i32, rows.len());
        let parent_position = (ui.list_cursor, ui.list_window);
        assert_eq!(ui.press_row(&rows[science]), TerminalEffect::None);
        let gates = ui.list(&catalog, &words, &state, &known);
        let count = gates.iter().filter_map(|row| match row {
            ListRow::Address { address, .. } => {
                assert_eq!(address_dimension(&catalog, address), Some("science"));
                Some(address)
            }
            _ => None,
        }).count();
        assert!(count > 1, "multiple facilities in Science share one directory page");
        assert!(matches!(rows[science], ListRow::Dimension { destinations, .. } if destinations == count));
        assert_eq!(ui.list_cursor, 1, "the first gate is selected, after the back row");
        assert_eq!(ui.model(&catalog, &words, &state, &known).list_title, "Dimensions  /  Science");
        ui.press_row(&ListRow::Dial);
        assert_eq!(ui.back(), TerminalEffect::None);
        assert_eq!(ui.directory, TerminalDirectory::Dimension("science".into()));
        ui.step_list(2, gates.len());
        ui.list_hover = Some(2);
        assert_eq!(ui.back(), TerminalEffect::None);
        assert_eq!(ui.directory, TerminalDirectory::Dimensions);
        assert_eq!((ui.list_cursor, ui.list_window), parent_position);
        assert_eq!(ui.list_hover, None);
        assert_eq!(ui.back(), TerminalEffect::Send(GateTerminalAction::Close));

        let posted = [words.spell("LEGAL").unwrap()];
        let rows = ui.list(&catalog, &words, &state, &posted);
        assert_eq!(rows.iter().filter(|row| matches!(row, ListRow::Dimension { .. })).count(), 1);
        // LEGAL is the Gate Authority capital's (Q185).
        assert!(matches!(&rows[0], ListRow::Dimension { id, destinations: 1, .. } if id == "authority"));
    }

    #[test]
    fn local_portal_arches_keep_a_direct_destination_list() {
        let catalog = catalog();
        let words = words();
        let mut ui = TerminalUi::default();
        ui.open("magical_heights_arch", None);
        let castle = words.spell("CASTLE").unwrap();
        let state = GateNetworkState::default();
        let model = ui.model(&catalog, &words, &state, &[castle.clone()]);
        assert_eq!(model.list_title, "Destinations");
        assert!(matches!(&model.list[0], ListRow::Address { address, .. } if *address == castle));
        assert_eq!(ui.press_row(&model.list[0]), TerminalEffect::Send(GateTerminalAction::Dial(castle)));
    }

    /// Under discovery, a terminal away from home lists its way home with no
    /// knowledge, and a learned address joins the list. With every address
    /// known (the development default), each gate is listed.
    #[test]
    fn a_terminal_lists_its_posted_address_and_what_the_save_knows() {
        use crate::gate_network::gate_address_known_flag;
        use ambition_content_schemas::gate_directory::GateStartKnowledge;
        let catalog = catalog();
        let words = words();
        let mut directory = directory();
        let all = known_addresses(&catalog, &words, &directory, &GateRingPositions::default(), "hub_public_gate_zone", |_| false);
        if directory.start_known == GateStartKnowledge::All {
            // A gate lists gates: the portal arches are not on its list.
            let gates = catalog.rings.iter().filter(|ring| ring.kind == ambition_platformer2d::world::rooms::RingKind::Gate).count();
            assert_eq!(all.len(), gates - 1, "every other gate is listed: {all:?}");
        }
        directory.start_known = GateStartKnowledge::Discovered;
        let spell = |word: &str| words.spell(word).unwrap();
        let known = known_addresses(&catalog, &words, &directory, &GateRingPositions::default(), "math_gate_return_zone", |_| false);
        assert_eq!(known, [spell("LEGAL")]);
        let flag = gate_address_known_flag("GENESIS");
        let known = known_addresses(&catalog, &words, &directory, &GateRingPositions::default(), "math_gate_return_zone", |id| id == flag);
        assert_eq!(known, [spell("LEGAL"), spell("GENESIS")]);
        // The sandbox hub posts the capital, and the capital posts the hub (Q185).
        assert_eq!(known_addresses(&catalog, &words, &directory, &GateRingPositions::default(), "hub_public_gate_zone", |_| false), [spell("LEGAL")]);
        assert_eq!(known_addresses(&catalog, &words, &directory, &GateRingPositions::default(), "authority_concourse_gate", |_| false), [spell("TESTING")]);
    }

    /// The status line reads the network: an open link, then a refusal by
    /// its own kind.
    #[test]
    fn the_status_line_reads_the_link_and_the_refusal() {
        use ambition_platformer2d::world::rooms::DialRefusal;
        let catalog = catalog();
        let words = words();
        let mut state = GateNetworkState::default();
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", None);
        state.open_link(&catalog, "hub_public_gate_zone", "math_gate_return_zone").unwrap();
        let model = ui.model(&catalog, &words, &state, &[]);
        assert!(model.status.starts_with("LINK OPEN to Math Town gate hall"), "{}", model.status);
        assert!(model.list.contains(&ListRow::Hangup));
        ui.open("math_gate_return_zone", None);
        let returning = ui.model(&catalog, &words, &state, &[]);
        assert!(returning.status.contains(&catalog.ring("hub_public_gate_zone").unwrap().name));
        assert!(returning.list.contains(&ListRow::Hangup));
        state.close_link("math_gate_return_zone");
        assert!(!ui.model(&catalog, &words, &state, &[]).list.contains(&ListRow::Hangup));
        ui.open("hub_public_gate_zone", None);
        let subject = ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(
            ambition_platformer2d_shared_tangle::sim_id::SimId::from_snapshot("terminal_user".into()), None);
        ui.subject = Some(subject.clone());
        state.report(
            &subject, "hub_public_gate_zone",
            words.spell("SILENT").unwrap(),
            Err(DialRefusal::Offline("The far ring is not yet built.".into())),
        );
        let model = ui.model(&catalog, &words, &state, &[]);
        assert!(model.refused && model.status.contains("OFFLINE"), "{}", model.status);
    }

    /// A long list shows a window of [`LIST_ROWS`] rows. The cursor's steps
    /// carry the window only as far as they must; a wheel moves the window
    /// and leaves the cursor; a hover moves neither (Q70).
    #[test]
    fn a_long_list_scrolls_a_window_that_follows_the_cursor() {
        let rows = LIST_ROWS + 6;
        let mut ui = TerminalUi::default();
        for _ in 0..LIST_ROWS - 1 {
            ui.step_list(1, rows);
        }
        assert_eq!((ui.list_cursor, ui.list_window), (LIST_ROWS - 1, 0), "the last shown row: no scroll yet");
        ui.step_list(1, rows);
        assert_eq!((ui.list_cursor, ui.list_window), (LIST_ROWS, 1), "one step past the window moves it one row");
        ui.scroll_list(-5, rows);
        assert_eq!((ui.list_cursor, ui.list_window), (LIST_ROWS, 0), "a wheel moves the window, not the cursor");
        ui.scroll_list(100, rows);
        assert_eq!(ui.list_window, rows - LIST_ROWS, "the window stops at the last row");
        ui.scroll_list(-100, rows);
        assert_eq!((ui.list_cursor, ui.list_window), (LIST_ROWS, 0), "the cursor is now below the window");
        ui.list_hover = Some(2);
        ui.step_list(1, rows);
        assert_eq!(ui.list_cursor, LIST_ROWS + 1, "a hover does not take the cursor: the arrow moves it from where it was");
        assert_eq!(ui.list_window, 2, "and the window comes back to it, to its near edge");
    }

    #[test]
    fn the_list_wraps_at_both_ends_and_keeps_the_cursor_visible() {
        let rows = LIST_ROWS + 6;
        let mut ui = TerminalUi::default();
        ui.step_list(-1, rows);
        assert_eq!((ui.list_cursor, ui.list_window), (rows - 1, rows - LIST_ROWS));
        ui.step_list(1, rows);
        assert_eq!((ui.list_cursor, ui.list_window), (0, 0));

        for rows in [0, 1] {
            ui.step_list(-1, rows);
            assert_eq!((ui.list_cursor, ui.list_window), (0, 0));
            ui.step_list(1, rows);
            assert_eq!((ui.list_cursor, ui.list_window), (0, 0));
        }
    }

    /// The terminal opens on its list; the dial is a row of it, and an empty
    /// dial's back returns to the list (Jon, 2026-10-25: the dial is
    /// secondary).
    #[test]
    fn the_terminal_opens_on_the_list_and_the_dial_is_behind_a_row() {
        let catalog = catalog();
        let words = words();
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", None);
        assert_eq!(ui.focus, TerminalFocus::List);
        let rows = ui.list(&catalog, &words, &GateNetworkState::default(), &[]);
        let dial = rows.iter().position(|row| *row == ListRow::Dial).expect("a dial row");
        assert_eq!(ui.press_row(&rows[dial]), TerminalEffect::None);
        assert_eq!(ui.focus, TerminalFocus::Wheel);
        ui.press_slot(WheelSlot::Chevron(3));
        assert_eq!(ui.back(), TerminalEffect::None, "back erases first");
        assert_eq!(ui.focus, TerminalFocus::Wheel);
        assert_eq!(ui.back(), TerminalEffect::None, "then returns to the list");
        assert_eq!(ui.focus, TerminalFocus::List);
    }
}
