//! The gate terminal's model: what a terminal shows and what a press on it
//! asks of the simulation.
//!
//! The simulation owns which terminal is open and what each dial answered
//! (`GateNetworkState`). This module owns the terminal's local state (the
//! tab, the symbols entered, the cursor) and turns a press into a
//! [`GateTerminalIntent`], which enters the simulation through the host
//! intent ledger on a stamped tick. Nothing here moves a body or opens a
//! ring.
//!
//! The presentation (`presentation::gate_terminal`) draws [`TerminalModel`]
//! and feeds presses back through [`TerminalUi::activate`] and
//! [`TerminalUi::navigate`], so keyboard, gamepad, mouse and touch use one
//! road. While a terminal is open it takes the seat's input
//! ([`GATE_TERMINAL_CONTEXT`]), so the body at the terminal stands still.

use ambition_platformer2d::world::rooms::{
    GateNetworkCatalog, GateNetworkState, GateTerminalAction, GateTerminalIntent,
};
use bevy::prelude::*;

/// The most symbols an address field holds. Longer than every authored
/// address, so a player can type a wrong long one and read the refusal.
pub const ADDRESS_FIELD_LEN: usize = 12;

/// The input context of an open gate terminal: it takes the seat's input
/// from gameplay, above a dialogue and below the inventory.
pub const GATE_TERMINAL_CONTEXT: ambition_input::participant::InputContextId =
    ambition_input::participant::InputContextId("gate_terminal");
/// Its claim priority, between `DIALOGUE` (150) and `INVENTORY` (160).
pub const GATE_TERMINAL_PRIORITY: i32 = 155;

/// The view a terminal shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TerminalTab {
    /// The addresses this traveller knows, and those written on this terminal.
    #[default]
    Known,
    /// The addresses the network linked last.
    Recent,
    /// The chevron keypad: any address, known or not.
    Manual,
}

impl TerminalTab {
    pub const ALL: [Self; 3] = [Self::Known, Self::Recent, Self::Manual];

    pub fn label(self) -> &'static str {
        match self {
            Self::Known => "Known",
            Self::Recent => "Recent",
            Self::Manual => "Manual",
        }
    }
}

/// One thing a terminal's cursor can stand on.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TerminalCell {
    Tab(TerminalTab),
    /// A listed address: a press dials it.
    Entry { address: String, label: String },
    /// A chevron symbol: a press adds it to the address field.
    Symbol(char),
    Erase,
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
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct TerminalUi {
    /// The ring the state belongs to. A different ring starts a fresh state.
    pub ring: Option<String>,
    pub tab: TerminalTab,
    /// The symbols entered, upper case.
    pub address: String,
    /// The cursor: row and column of [`TerminalModel::rows`].
    pub cursor: (usize, usize),
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
    pub address: String,
    /// The line under the address: the link, the last answer, or a hint.
    pub status: String,
    /// Whether the status line reports a refusal.
    pub refused: bool,
    pub tab: TerminalTab,
    pub rows: Vec<Vec<TerminalCell>>,
    pub cursor: (usize, usize),
}

/// The Greek letter a chevron draws for an address symbol (the ring's
/// inscription, ΛΕΓΑΛΛΥ ΔΙΣΤΙΝΧΤ).
pub fn chevron_glyph(symbol: char) -> char {
    match symbol {
        'A' => 'Α',
        'C' => 'Χ',
        'D' => 'Δ',
        'E' => 'Ε',
        'G' => 'Γ',
        'I' => 'Ι',
        'L' => 'Λ',
        'N' => 'Ν',
        'S' => 'Σ',
        'T' => 'Τ',
        'Y' => 'Υ',
        other => other,
    }
}

/// The addresses a terminal lists as known: those written on it first, then
/// each the traveller knows (`GateNetworkCatalog::knows`: what the save
/// records, or every address while the catalog starts with all known), in
/// catalog order. `flag` reads a save flag.
pub fn known_addresses(
    catalog: &GateNetworkCatalog,
    ring: &str,
    flag: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut listed: Vec<String> = catalog.ring(ring).map(|ring| ring.posted.clone()).unwrap_or_default();
    for candidate in &catalog.rings {
        let Some(address) = candidate.address.as_deref() else {
            continue;
        };
        if candidate.zone != ring && catalog.knows(address, &flag) && !listed.iter().any(|a| a == address) {
            listed.push(address.to_string());
        }
    }
    listed
}

fn entry(catalog: &GateNetworkCatalog, address: &str) -> TerminalCell {
    let label = catalog.ring_at(address).map_or_else(String::new, |ring| ring.name.clone());
    TerminalCell::Entry { address: address.to_string(), label }
}

impl TerminalUi {
    /// Start a fresh state for `ring`: the Known tab when it lists anything,
    /// else the keypad.
    pub fn open(&mut self, ring: &str, known: usize, seen_serial: Option<u64>) {
        *self = Self {
            ring: Some(ring.to_string()),
            tab: if known > 0 { TerminalTab::Known } else { TerminalTab::Manual },
            address: String::new(),
            cursor: (1, 0),
            seen_serial,
        };
    }

    /// The rows the cursor walks for the current tab.
    pub fn rows(&self, catalog: &GateNetworkCatalog, state: &GateNetworkState, known: &[String]) -> Vec<Vec<TerminalCell>> {
        let mut rows = vec![TerminalTab::ALL.iter().map(|tab| TerminalCell::Tab(*tab)).collect()];
        match self.tab {
            TerminalTab::Known => rows.extend(known.iter().map(|address| vec![entry(catalog, address)])),
            TerminalTab::Recent => rows.extend(state.recent.iter().map(|address| vec![entry(catalog, address)])),
            TerminalTab::Manual => {
                let symbols: Vec<char> = catalog.alphabet.chars().collect();
                let half = symbols.len().div_ceil(2);
                for chunk in symbols.chunks(half.max(1)) {
                    rows.push(chunk.iter().map(|symbol| TerminalCell::Symbol(*symbol)).collect());
                }
                rows.push(vec![TerminalCell::Erase]);
            }
        }
        let ring = self.ring.as_deref().unwrap_or_default();
        let mut actions = vec![TerminalCell::Dial];
        if state.route_from(ring).is_some() {
            actions.push(TerminalCell::Hangup);
        }
        actions.push(TerminalCell::Close);
        rows.push(actions);
        rows
    }

    /// Move the cursor by one step, kept inside the rows.
    pub fn navigate(&mut self, rows: &[Vec<TerminalCell>], dx: i32, dy: i32) {
        if rows.is_empty() {
            return;
        }
        let (row, col) = self.cursor;
        let row = (row as i32 + dy).clamp(0, rows.len() as i32 - 1) as usize;
        let width = rows[row].len().max(1);
        let col = if dy != 0 { col.min(width - 1) } else { (col as i32 + dx).clamp(0, width as i32 - 1) as usize };
        self.cursor = (row, col);
    }

    /// Clamp the cursor after the rows changed under it.
    pub fn clamp(&mut self, rows: &[Vec<TerminalCell>]) {
        self.navigate(rows, 0, 0);
    }

    /// Carry out a press on `cell`. A press of a symbol adds it, again each
    /// time it is pressed; a listed address dials at once.
    pub fn activate(&mut self, cell: &TerminalCell) -> TerminalEffect {
        match cell {
            TerminalCell::Tab(tab) => {
                self.tab = *tab;
                TerminalEffect::None
            }
            TerminalCell::Entry { address, .. } => {
                self.address = address.clone();
                TerminalEffect::Send(GateTerminalAction::Dial(address.clone()))
            }
            TerminalCell::Symbol(symbol) => {
                self.type_symbol(*symbol);
                TerminalEffect::None
            }
            TerminalCell::Erase => {
                self.address.pop();
                TerminalEffect::None
            }
            TerminalCell::Dial => TerminalEffect::Send(GateTerminalAction::Dial(self.address.clone())),
            TerminalCell::Hangup => TerminalEffect::Send(GateTerminalAction::Hangup),
            TerminalCell::Close => TerminalEffect::Send(GateTerminalAction::Close),
        }
    }

    /// Add one symbol to the address field. A symbol that is not on the
    /// chevrons is still added: the dial then names it in its refusal.
    pub fn type_symbol(&mut self, symbol: char) {
        if self.address.chars().count() < ADDRESS_FIELD_LEN {
            self.address.extend(symbol.to_uppercase());
        }
    }

    /// The back press: erase the last symbol, or leave the terminal when the
    /// field is empty.
    pub fn back(&mut self) -> TerminalEffect {
        if self.address.pop().is_some() {
            TerminalEffect::None
        } else {
            TerminalEffect::Send(GateTerminalAction::Close)
        }
    }

    /// What the terminal shows now.
    pub fn model(&self, catalog: &GateNetworkCatalog, state: &GateNetworkState, known: &[String]) -> TerminalModel {
        let ring_id = self.ring.as_deref().unwrap_or_default();
        let ring = catalog.ring(ring_id);
        let facility = ring
            .and_then(|ring| catalog.facility(&ring.facility))
            .map_or_else(String::new, |facility| facility.name.clone());
        let far_name = |zone: &str| catalog.ring(zone).map_or_else(|| zone.to_string(), |ring| ring.name.clone());
        let (status, refused) = if let Some(link) = state.route_from(ring_id) {
            (
                format!("LINK OPEN to {} ({}). Walk into the ring.", far_name(&link.far), link.address),
                false,
            )
        } else if state.holds_open(ring_id) {
            ("INCOMING: the ring is receiving.".to_string(), false)
        } else if let Some(report) = state.reports.get(ring_id) {
            match &report.outcome {
                Ok(far) => (format!("The link to {} has closed.", far_name(far)), false),
                Err(refusal) => (format!("{}: {}", report.address, refusal.message()), true),
            }
        } else {
            ("Choose an address, or enter one on the chevrons.".to_string(), false)
        };
        TerminalModel {
            title: ring.map_or_else(|| ring_id.to_string(), |ring| ring.name.clone()),
            facility,
            note: ring.map_or_else(String::new, |ring| ring.note.clone()),
            address: self.address.clone(),
            status,
            refused,
            tab: self.tab,
            rows: self.rows(catalog, state, known),
            cursor: self.cursor,
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

    /// A repeated symbol is entered each time it is pressed, and the back
    /// press erases before it leaves.
    #[test]
    fn a_repeated_symbol_is_entered_each_time_and_back_erases_first() {
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", 0, None);
        assert_eq!(ui.tab, TerminalTab::Manual, "nothing known: the keypad");
        for symbol in ['L', 'E', 'G', 'A', 'L', 'L'] {
            assert_eq!(ui.activate(&TerminalCell::Symbol(symbol)), TerminalEffect::None);
        }
        assert_eq!(ui.address, "LEGALL");
        assert_eq!(ui.back(), TerminalEffect::None);
        assert_eq!(ui.activate(&TerminalCell::Dial), TerminalEffect::Send(GateTerminalAction::Dial("LEGAL".into())));
        for _ in 0..5 {
            ui.back();
        }
        assert_eq!(ui.back(), TerminalEffect::Send(GateTerminalAction::Close));
    }

    /// The keypad has each chevron once, and the cursor stays in the rows.
    #[test]
    fn the_keypad_is_the_alphabet_and_the_cursor_stays_inside() {
        let catalog = catalog();
        let state = GateNetworkState::default();
        let mut ui = TerminalUi::default();
        ui.open("hub_public_gate_zone", 0, None);
        let rows = ui.rows(&catalog, &state, &[]);
        let symbols: String = rows
            .iter()
            .flatten()
            .filter_map(|cell| match cell {
                TerminalCell::Symbol(symbol) => Some(*symbol),
                _ => None,
            })
            .collect();
        assert_eq!(symbols, catalog.alphabet);
        for _ in 0..20 {
            ui.navigate(&rows, 1, 1);
        }
        assert_eq!(ui.cursor.0, rows.len() - 1);
        assert!(ui.cursor.1 < rows[ui.cursor.0].len());
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
            assert_eq!(all.len(), catalog.rings.iter().filter(|r| r.address.is_some()).count() - 1, "{all:?}");
        }
        catalog.start_known = GateStartKnowledge::Discovered;
        let known = known_addresses(&catalog, "math_gate_return_zone", |_| false);
        assert_eq!(known, ["LEGAL"]);
        let flag = gate_address_known_flag("GENESIS");
        let known = known_addresses(&catalog, "math_gate_return_zone", |id| id == flag);
        assert_eq!(known, ["LEGAL", "GENESIS"]);
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
        ui.open("hub_public_gate_zone", 0, None);
        state.open_link("hub_public_gate_zone", "math_gate_hall", "math_gate_return_zone", "TANGENT");
        let model = ui.model(&catalog, &state, &[]);
        assert!(model.status.starts_with("LINK OPEN to Math Town gate hall"), "{}", model.status);
        assert!(model.rows.last().unwrap().contains(&TerminalCell::Hangup));
        state.links.clear();
        state.report("hub_public_gate_zone", "SILENT".into(), Err(DialRefusal::Offline("The far ring is not yet built.".into())));
        let model = ui.model(&catalog, &state, &[]);
        assert!(model.refused && model.status.contains("OFFLINE"), "{}", model.status);
    }
}
