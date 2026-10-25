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
//! A terminal is a WHEEL of the ring's chevrons, laid out as the ring is
//! ([`chevron_angle`]: clockwise from the lower left over the top, the first
//! of the inscription at the lower left), with the ENGAGE press (a solid Δ)
//! in the slice at the bottom, beside a list of the known and recent
//! addresses. A gamepad points the stick at a chevron and presses to enter
//! it; arrow keys and the d-pad step round the wheel; a mouse or a finger
//! presses a chevron. There is no typing: one chevron of the ring can stand
//! for one letter twice (Λ, Λ₂, Λ₃), and only the wheel tells them apart.
//!
//! The presentation (`presentation::gate_terminal`) draws [`TerminalModel`]
//! and feeds presses back through [`TerminalUi`], so every device uses one
//! road. While a terminal is open it takes the seat's input
//! ([`GATE_TERMINAL_CONTEXT`]), so the body at the terminal stands still.

use ambition_platformer2d::world::rooms::{
    Chevron, GateNetworkCatalog, GateNetworkState, GateTerminalAction, GateTerminalIntent,
};
use bevy::prelude::*;

/// The most chevrons an address field holds. Longer than every authored
/// address, so a player can enter a wrong long one and read the refusal.
pub const ADDRESS_FIELD_LEN: usize = 12;

/// The slice of the wheel with no chevron, at the bottom: on the ring the
/// base it stands in, on a terminal the engage press. The ring's art draws
/// the same layout (`glyphs.legally_distinct.ARCH_GAP_DEG`).
pub const ARCH_GAP_DEG: f32 = 96.0;

/// The input context of an open gate terminal: it takes the seat's input
/// from gameplay, above a dialogue and below the inventory.
pub const GATE_TERMINAL_CONTEXT: ambition_input::participant::InputContextId =
    ambition_input::participant::InputContextId("gate_terminal");
/// Its claim priority, between `DIALOGUE` (150) and `INVENTORY` (160).
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

/// Which part of the terminal the menu keys drive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TerminalFocus {
    /// The wheel: point and press to enter chevrons.
    #[default]
    Wheel,
    /// The list beside it: known and recent addresses, and the actions.
    List,
}

/// One row of the list beside the wheel.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ListRow {
    /// An address: a press dials it.
    Address { address: Vec<Chevron>, word: String, name: String, recent: bool },
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
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct TerminalUi {
    /// The ring the state belongs to. A different ring starts a fresh state.
    pub ring: Option<String>,
    pub focus: TerminalFocus,
    /// The chevrons entered.
    pub entered: Vec<Chevron>,
    /// Where the wheel points.
    pub pointed: Option<WheelSlot>,
    /// The row of the list the cursor is on.
    pub list_cursor: usize,
    /// The report serial this terminal already acted on.
    pub seen_serial: Option<u64>,
}

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
    pub list_cursor: usize,
}

/// The addresses a terminal lists as known, as chevrons: those written on it
/// first, then each the traveller knows (`GateNetworkCatalog::knows`: what
/// the save records, or every address while the catalog starts with all
/// known), in catalog order, words and glyph addresses alike. `flag` reads a
/// save flag.
pub fn known_addresses(catalog: &GateNetworkCatalog, ring: &str, flag: impl Fn(&str) -> bool) -> Vec<Vec<Chevron>> {
    let mut listed: Vec<Vec<Chevron>> = catalog
        .ring(ring)
        .map(|ring| ring.posted.iter().filter_map(|word| catalog.spell(word).ok()).collect())
        .unwrap_or_default();
    for candidate in &catalog.rings {
        let Some(address) = catalog.address_of(candidate) else {
            continue;
        };
        if candidate.zone != ring && catalog.knows(candidate, &flag) && !listed.contains(&address) {
            listed.push(address);
        }
    }
    listed
}

fn address_row(catalog: &GateNetworkCatalog, address: Vec<Chevron>, recent: bool) -> ListRow {
    let ring = catalog.ring_at(&address);
    ListRow::Address {
        word: ring.and_then(|ring| ring.address.clone()).unwrap_or_default(),
        name: ring.map_or_else(|| "No ring answered".to_string(), |ring| ring.name.clone()),
        address,
        recent,
    }
}

impl TerminalUi {
    /// Start a fresh state for `ring`, on the wheel, pointing at nothing.
    pub fn open(&mut self, ring: &str, seen_serial: Option<u64>) {
        *self = Self { ring: Some(ring.to_string()), seen_serial, ..default() };
    }

    /// The list beside the wheel: the known addresses (by word), the recent
    /// ones (by chevrons, a known one only once), then the actions.
    pub fn list(&self, catalog: &GateNetworkCatalog, state: &GateNetworkState, known: &[Vec<Chevron>]) -> Vec<ListRow> {
        let mut rows: Vec<ListRow> = known.iter().map(|address| address_row(catalog, address.clone(), false)).collect();
        for address in &state.recent {
            let listed = rows.iter().any(|row| matches!(row, ListRow::Address { address: a, .. } if a == address));
            if !listed {
                rows.push(address_row(catalog, address.clone(), true));
            }
        }
        if state.route_from(self.ring.as_deref().unwrap_or_default()).is_some() {
            rows.push(ListRow::Hangup);
        }
        rows.push(ListRow::Close);
        rows
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

    /// Move the list cursor by `step`, kept inside the list.
    pub fn step_list(&mut self, step: i32, rows: usize) {
        self.focus = TerminalFocus::List;
        self.list_cursor = (self.list_cursor as i32 + step).clamp(0, rows.saturating_sub(1) as i32) as usize;
    }

    /// Carry out a press of a wheel slot: a chevron is entered, again each
    /// time it is pressed; the engage press dials what was entered.
    pub fn press_slot(&mut self, slot: WheelSlot) -> TerminalEffect {
        self.point(slot);
        match slot {
            WheelSlot::Chevron(chevron) => {
                if self.entered.len() < ADDRESS_FIELD_LEN {
                    self.entered.push(chevron);
                }
                TerminalEffect::None
            }
            WheelSlot::Engage => TerminalEffect::Send(GateTerminalAction::Dial(self.entered.clone())),
        }
    }

    /// Carry out a press of a list row: an address is entered and dialed.
    pub fn press_row(&mut self, row: &ListRow) -> TerminalEffect {
        match row {
            ListRow::Address { address, .. } => {
                self.entered = address.clone();
                TerminalEffect::Send(GateTerminalAction::Dial(address.clone()))
            }
            ListRow::Hangup => TerminalEffect::Send(GateTerminalAction::Hangup),
            ListRow::Close => TerminalEffect::Send(GateTerminalAction::Close),
        }
    }

    /// The back press: take back the last chevron, or leave the terminal
    /// when none is entered.
    pub fn back(&mut self) -> TerminalEffect {
        if self.entered.pop().is_some() {
            TerminalEffect::None
        } else {
            TerminalEffect::Send(GateTerminalAction::Close)
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
    pub fn model(&self, catalog: &GateNetworkCatalog, state: &GateNetworkState, known: &[Vec<Chevron>]) -> TerminalModel {
        let ring_id = self.ring.as_deref().unwrap_or_default();
        let ring = catalog.ring(ring_id);
        let facility = ring
            .and_then(|ring| catalog.facility(&ring.facility))
            .map_or_else(String::new, |facility| facility.name.clone());
        let far_name = |zone: &str| catalog.ring(zone).map_or_else(|| zone.to_string(), |ring| ring.name.clone());
        let word = |address: &[Chevron]| catalog.word_of(address).map(str::to_string);
        let (status, refused) = if let Some(link) = state.route_from(ring_id) {
            (format!("LINK OPEN to {}. Walk into the ring.", far_name(&link.far)), false)
        } else if state.holds_open(ring_id) {
            ("INCOMING: the ring is receiving.".to_string(), false)
        } else if let Some(report) = state.reports.get(ring_id) {
            match &report.outcome {
                Ok(far) => (format!("The link to {} has closed.", far_name(far)), false),
                Err(refusal) => (refusal.message(), true),
            }
        } else {
            ("Point at a chevron and press to enter it; press \u{0394} to dial.".to_string(), false)
        };
        let list = self.list(catalog, state, known);
        TerminalModel {
            title: ring.map_or_else(|| ring_id.to_string(), |ring| ring.name.clone()),
            facility,
            note: ring.map_or_else(String::new, |ring| ring.note.clone()),
            inscription: catalog.inscription.chars().collect(),
            entered: self.entered.clone(),
            entered_word: word(&self.entered),
            status,
            refused,
            focus: self.focus,
            pointed: self.pointed,
            list_cursor: self.list_cursor.min(list.len().saturating_sub(1)),
            list,
        }
    }
}

/// Declare [`GATE_TERMINAL_CONTEXT`] while a terminal is open, for every
/// participant (the terminal is one shared surface, as the inventory is).
pub fn declare_gate_terminal_context(
    state: Option<Res<GateNetworkState>>,
    mut participants: Query<
        &mut ambition_input::participant::ParticipantContexts,
        With<ambition_input::participant::InputParticipant>,
    >,
) {
    use ambition_input::participant::ContextClaim;
    let open = state.is_some_and(|state| state.terminal.is_some());
    for mut contexts in &mut participants {
        if contexts.is_declared(GATE_TERMINAL_CONTEXT) != open {
            contexts.sync(ContextClaim::capturing(GATE_TERMINAL_CONTEXT, GATE_TERMINAL_PRIORITY), open);
        }
    }
}

/// Whether a gate terminal is open now.
pub fn a_gate_terminal_is_open(state: Option<Res<GateNetworkState>>) -> bool {
    state.is_some_and(|state| state.terminal.is_some())
}

/// The run condition of [`declare_gate_terminal_context`]: a terminal is
/// open, or was open on the last frame (the claim is taken back once).
pub fn a_gate_terminal_is_or_was_open(state: Option<Res<GateNetworkState>>, mut was: Local<bool>) -> bool {
    let open = state.is_some_and(|state| state.terminal.is_some());
    let run = open || *was;
    *was = open;
    run
}

/// Install the terminal's input claim and its local state.
pub(super) fn register(app: &mut App) {
    app.init_resource::<TerminalUi>().add_systems(
        Update,
        declare_gate_terminal_context
            .run_if(a_gate_terminal_is_or_was_open)
            .in_set(ambition_input::InputSet::ResolveContext),
    );
}

/// A request the terminal sends, as the host intent it is.
pub fn intent(ring: &str, action: GateTerminalAction) -> GateTerminalIntent {
    GateTerminalIntent { ring: ring.to_string(), action }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> GateNetworkCatalog {
        crate::gate_network::lowered_gate_network(crate::pack::shipped())
    }

    /// The wheel is laid out as the ring: the first chevron at the lower
    /// left, the middle one (Δ) at the top, the last at the lower right, and
    /// the engage press at the bottom.
    #[test]
    fn the_wheel_is_laid_out_as_the_ring() {
        let n = catalog().chevrons();
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
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", None);
        let tangent = catalog.spell("TANGENT").unwrap();
        for chevron in &tangent {
            assert_eq!(ui.press_slot(WheelSlot::Chevron(*chevron)), TerminalEffect::None);
        }
        ui.press_slot(WheelSlot::Chevron(0));
        assert_eq!(ui.back(), TerminalEffect::None);
        assert_eq!(ui.press_slot(WheelSlot::Engage), TerminalEffect::Send(GateTerminalAction::Dial(tangent.clone())));
        let model = ui.model(&catalog, &GateNetworkState::default(), &[]);
        assert_eq!(model.entered_word.as_deref(), Some("TANGENT"));
        for _ in 0..tangent.len() {
            ui.back();
        }
        assert_eq!(ui.back(), TerminalEffect::Send(GateTerminalAction::Close));
    }

    /// A listed address dials its chevrons; a recent one that is not known
    /// is listed by its chevrons; the list ends with leaving.
    #[test]
    fn the_list_dials_known_and_recent_addresses() {
        let catalog = catalog();
        let mut state = GateNetworkState::default();
        let wrong = vec![0, 1, 2, 3, 0];
        state.recent = vec![catalog.spell("GENESIS").unwrap(), wrong.clone()];
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", None);
        let rows = ui.list(&catalog, &state, &[catalog.spell("GENESIS").unwrap()]);
        assert!(matches!(&rows[0], ListRow::Address { word, recent: false, .. } if word == "GENESIS"));
        assert!(matches!(&rows[1], ListRow::Address { address, word, recent: true, .. } if *address == wrong && word.is_empty()), "{rows:?}");
        assert_eq!(rows.last(), Some(&ListRow::Close));
        assert_eq!(
            ui.press_row(&rows[0]),
            TerminalEffect::Send(GateTerminalAction::Dial(catalog.spell("GENESIS").unwrap()))
        );
    }

    /// Under discovery, a terminal away from home lists its way home with no
    /// knowledge, and a learned address joins the list. With every address
    /// known (the development default), each is listed.
    #[test]
    fn a_terminal_lists_its_posted_address_and_what_the_save_knows() {
        use ambition_platformer2d::world::rooms::{gate_address_known_flag, GateStartKnowledge};
        let mut catalog = catalog();
        let all = known_addresses(&catalog, "hub_public_gate_zone", |_| false);
        if catalog.start_known == GateStartKnowledge::All {
            assert_eq!(all.len(), catalog.rings.len() - 1, "every other ring is listed: {all:?}");
        }
        catalog.start_known = GateStartKnowledge::Discovered;
        let spell = |word: &str| catalog.spell(word).unwrap();
        let known = known_addresses(&catalog, "math_gate_return_zone", |_| false);
        assert_eq!(known, [spell("LEGAL")]);
        let flag = gate_address_known_flag("GENESIS");
        let known = known_addresses(&catalog, "math_gate_return_zone", |id| id == flag);
        assert_eq!(known, [spell("LEGAL"), spell("GENESIS")]);
        assert!(known_addresses(&catalog, "hub_public_gate_zone", |_| false).is_empty(), "the hub posts nothing");
    }

    /// The status line reads the network: an open link, then a refusal by
    /// its own kind.
    #[test]
    fn the_status_line_reads_the_link_and_the_refusal() {
        use ambition_platformer2d::world::rooms::DialRefusal;
        let catalog = catalog();
        let mut state = GateNetworkState::default();
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", None);
        state.open_link("hub_public_gate_zone", "math_gate_hall", "math_gate_return_zone", &catalog.spell("TANGENT").unwrap());
        let model = ui.model(&catalog, &state, &[]);
        assert!(model.status.starts_with("LINK OPEN to Math Town gate hall"), "{}", model.status);
        assert!(model.list.contains(&ListRow::Hangup));
        state.links.clear();
        state.report(
            "hub_public_gate_zone",
            catalog.spell("SILENT").unwrap(),
            Err(DialRefusal::Offline("The far ring is not yet built.".into())),
        );
        let model = ui.model(&catalog, &state, &[]);
        assert!(model.refused && model.status.contains("OFFLINE"), "{}", model.status);
    }
}
