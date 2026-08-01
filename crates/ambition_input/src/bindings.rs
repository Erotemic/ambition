//! **One authority for "which physical control is this action on".**
//!
//! Routing, remapping, controller glyphs, touch affordances, action prompts and
//! help displays are five readers of one fact. Before this they had none: the
//! prompt read-model carried the VERB ("Continue", "Equip") and
//! `control_prompt.rs` said so in a comment — *"per-slot glyphs (the physical
//! binding) land with the `ActiveBindings` source"* — while
//! `KeyboardPreset::action_label` built one big string for a debug overlay and
//! the gamepad map had no label producer at all.
//!
//! ## Derived, never parallel
//!
//! [`SeatBindings`] is PROJECTED from the very `InputMap<Platformer2dInputActionMonolith>` the
//! router reads. It is not a second table that has to be kept in step — there is
//! nothing to keep in step, because a rebind changes the map and the projection
//! follows on the next frame.
//!
//! That is the whole design, and it is the difference between this and the
//! thing it replaces. A hand-maintained "what does Jump say on screen" table is
//! correct on the day it is written and wrong the first time somebody remaps,
//! and the symptom — a prompt telling a player to press a key that does nothing
//! — is indistinguishable from a broken binding.
//!
//! ## Per seat, because a binding is
//!
//! Two people at one machine hold different presets. `SeatBindings` is keyed by
//! participant slot for the same reason [`crate::SeatInputContexts`] and
//! `SeatMenuFrames` are: one global answer is right for nobody once there are
//! two of them.

use std::collections::BTreeMap;

use bevy::prelude::*;
use leafwing_input_manager::prelude::InputMap;

use crate::{InputParticipant, Platformer2dInputActionMonolith};

/// A physical control, named well enough to draw.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhysicalControl {
    Key(KeyCode),
    Button(GamepadButton),
    /// Something the projection could not name.
    ///
    /// ⚠ **it carries the debug form rather than being dropped.** A prompt that
    /// silently omits an unrecognised binding tells a player the action has no
    /// control at all, which is a worse lie than an ugly label — and it hides
    /// the fact that this projection needs a new arm.
    Other(String),
}

impl PhysicalControl {
    /// What to print. Short, because it goes on a button.
    pub fn label(&self) -> String {
        match self {
            Self::Key(key) => key_label(*key),
            Self::Button(button) => button_label(*button).to_string(),
            Self::Other(raw) => raw.clone(),
        }
    }
}

/// One seat's bindings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionBindings {
    /// Sorted by the action's name, so two runs and two machines agree. A
    /// `HashMap` here would make a help screen list its rows in a different
    /// order every launch.
    actions: Vec<(String, Vec<PhysicalControl>)>,
}

impl ActionBindings {
    /// Project one live `InputMap`. The system below is a thin loop over this,
    /// so a pure caller (a test, a rebind preview, a docs generator) gets the
    /// same answer the running game gets without standing up a `World`.
    pub fn from_map(map: &InputMap<Platformer2dInputActionMonolith>) -> Self {
        let mut actions: Vec<(String, Vec<PhysicalControl>)> = map
            .iter_buttonlike()
            .map(|(action, inputs)| {
                (
                    action_name(action),
                    inputs.iter().map(|input| classify(input.as_ref())).collect(),
                )
            })
            .collect();
        actions.sort_by(|(a, _), (b, _)| a.cmp(b));
        Self { actions }
    }

    /// Every physical control bound to this action, in insertion-stable order.
    pub fn controls(&self, action: &Platformer2dInputActionMonolith) -> &[PhysicalControl] {
        let name = action_name(action);
        self.actions
            .iter()
            .find(|(bound, _)| *bound == name)
            .map(|(_, controls)| controls.as_slice())
            .unwrap_or_default()
    }

    /// The label a prompt should show — the FIRST binding, which is the one a
    /// preset lists first and therefore the one the author considered primary.
    pub fn label(&self, action: &Platformer2dInputActionMonolith) -> Option<String> {
        self.controls(action).first().map(PhysicalControl::label)
    }

    /// Every bound action and its controls, in canonical order. For a help
    /// screen or a rebind UI.
    pub fn all(&self) -> impl Iterator<Item = (&str, &[PhysicalControl])> {
        self.actions
            .iter()
            .map(|(name, controls)| (name.as_str(), controls.as_slice()))
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// **Which action drives an ability slot.**
///
/// ⛔ **this did not exist, and two hand-maintained maps stood in for it.**
/// `ambition_touch_input` carried `TouchActionButton → ControlSlot` and
/// `TouchActionButton → Platformer2dInputActionMonolith` fifteen lines apart, agreeing only
/// because somebody kept them agreeing — the same shape as the gamepad glyph
/// table this module replaced. Anything else that wanted to ask "what is the
/// Attack SLOT bound to" would have needed a third.
///
/// `ControlSlot` is the ability vocabulary (`ambition_entity_catalog`, a pure
/// leaf) and `Platformer2dInputActionMonolith` is the input vocabulary; the correspondence is an
/// INPUT-layer fact, so it lives here, where both the prompt and the overlay can
/// reach it.
///
/// `None` for a slot with no single action behind it — today none, and the
/// `Option` is what will say so honestly when a slot becomes a chord.
pub fn action_for_slot(slot: ambition_entity_catalog::action_scheme::ControlSlot) -> Option<Platformer2dInputActionMonolith> {
    use ambition_entity_catalog::action_scheme::ControlSlot;
    Some(match slot {
        ControlSlot::Jump => Platformer2dInputActionMonolith::Jump,
        ControlSlot::Attack => Platformer2dInputActionMonolith::Attack,
        ControlSlot::Special => Platformer2dInputActionMonolith::Special,
        ControlSlot::Projectile => Platformer2dInputActionMonolith::Projectile,
        ControlSlot::Dash => Platformer2dInputActionMonolith::Dash,
        ControlSlot::Blink => Platformer2dInputActionMonolith::Blink,
        ControlSlot::Interact => Platformer2dInputActionMonolith::Interact,
        ControlSlot::Utility => Platformer2dInputActionMonolith::Utility,
        ControlSlot::QuickAction => Platformer2dInputActionMonolith::QuickAction,
        ControlSlot::Modifier => Platformer2dInputActionMonolith::Modifier,
    })
}

/// Every seat's bindings, keyed by participant slot.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct SeatBindings {
    seats: BTreeMap<u8, ActionBindings>,
}

impl SeatBindings {
    /// A seat with no map reads as bound to nothing, which is true — an
    /// unplugged pad is the ordinary state of a couch, not a fault.
    pub fn for_seat(&self, slot: u8) -> &ActionBindings {
        static NONE: std::sync::LazyLock<ActionBindings> =
            std::sync::LazyLock::new(ActionBindings::default);
        self.seats.get(&slot).unwrap_or(&NONE)
    }

    /// The label this seat's prompt should show for this action.
    pub fn label(&self, slot: u8, action: &Platformer2dInputActionMonolith) -> Option<String> {
        self.for_seat(slot).label(action)
    }

    /// The label for an ability SLOT — what a prompt actually wants to ask.
    /// Composes [`action_for_slot`] rather than carrying its own table.
    pub fn label_for_slot(
        &self,
        seat: u8,
        slot: ambition_entity_catalog::action_scheme::ControlSlot,
    ) -> Option<String> {
        self.label(seat, &action_for_slot(slot)?)
    }

    pub fn seats(&self) -> impl Iterator<Item = (u8, &ActionBindings)> {
        self.seats.iter().map(|(slot, bindings)| (*slot, bindings))
    }
}

/// Project every participant's live `InputMap` into [`SeatBindings`].
///
/// Runs in `InputSet::ResolveActions`, after any remap and before anything
/// draws a prompt. Change-detected: a quiet frame does not touch the resource,
/// so a prompt rebuilding on `is_changed()` rebuilds only on a real rebind.
pub fn publish_seat_bindings(
    participants: Query<(&InputParticipant, &InputMap<Platformer2dInputActionMonolith>)>,
    mut bindings: ResMut<SeatBindings>,
) {
    let mut next: BTreeMap<u8, ActionBindings> = BTreeMap::new();
    for (participant, map) in &participants {
        next.insert(participant.id.slot(), ActionBindings::from_map(map));
    }
    if bindings.seats != next {
        bindings.seats = next;
    }
}

/// The action's stable name. `Debug` is what leafwing's derive gives us and it
/// is the same string the trace and the settings file already use, so a rebind
/// UI keyed on it lines up with everything else.
fn action_name(action: &Platformer2dInputActionMonolith) -> String {
    format!("{action:?}")
}

/// Name a `dyn Buttonlike`.
///
/// `Buttonlike: Reflect`, so the concrete input downcasts. Anything the two
/// arms miss becomes [`PhysicalControl::Other`] carrying its `Debug` form —
/// visibly unhandled rather than silently absent.
fn classify(input: &dyn leafwing_input_manager::prelude::Buttonlike) -> PhysicalControl {
    let reflected = input.as_reflect();
    if let Some(key) = reflected.downcast_ref::<KeyCode>() {
        return PhysicalControl::Key(*key);
    }
    if let Some(button) = reflected.downcast_ref::<GamepadButton>() {
        return PhysicalControl::Button(*button);
    }
    PhysicalControl::Other(format!("{input:?}"))
}

fn button_label(button: GamepadButton) -> &'static str {
    match button {
        GamepadButton::South => "A",
        GamepadButton::East => "B",
        GamepadButton::North => "Y",
        GamepadButton::West => "X",
        GamepadButton::LeftTrigger => "LB",
        GamepadButton::LeftTrigger2 => "LT",
        GamepadButton::RightTrigger => "RB",
        GamepadButton::RightTrigger2 => "RT",
        GamepadButton::Select => "Select",
        GamepadButton::Start => "Start",
        GamepadButton::Mode => "Home",
        GamepadButton::LeftThumb => "L3",
        GamepadButton::RightThumb => "R3",
        GamepadButton::DPadUp => "D-Up",
        GamepadButton::DPadDown => "D-Down",
        GamepadButton::DPadLeft => "D-Left",
        GamepadButton::DPadRight => "D-Right",
        // Deliberately not `unreachable!`: a Bevy upgrade adding a variant must
        // print something odd, not panic a HUD. Same rule `key_label` follows.
        _ => "Button",
    }
}

/// ⚠ **`presets::key_name` returns `"?"` for a key it does not list**, which on
/// a prompt tells a player nothing at all — and after a rebind to any unlisted
/// key, that is what they would see. So an unnamed key falls through to the
/// `KeyCode`'s own debug form (`F13`, `NumpadAdd`), which is ugly and TRUE.
/// Same rule as [`PhysicalControl::Other`]: visibly unhandled beats silently
/// wrong.
fn key_label(key: KeyCode) -> String {
    match key {
        KeyCode::Space => "Space".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Escape => "Esc".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "Shift".to_string(),
        KeyCode::ControlLeft | KeyCode::ControlRight => "Ctrl".to_string(),
        KeyCode::AltLeft | KeyCode::AltRight => "Alt".to_string(),
        KeyCode::ArrowUp => "Up".to_string(),
        KeyCode::ArrowDown => "Down".to_string(),
        KeyCode::ArrowLeft => "Left".to_string(),
        KeyCode::ArrowRight => "Right".to_string(),
        other => match crate::presets::key_name(other) {
            "?" => format!("{other:?}"),
            named => named.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::KeyboardPreset;
    use bevy::ecs::system::RunSystemOnce;

    fn publish(app: &mut App) {
        app.world_mut()
            .run_system_once(publish_seat_bindings)
            .expect("projection runs");
    }

    fn app_with_seats(seats: &[(u8, InputMap<Platformer2dInputActionMonolith>)]) -> App {
        let mut app = App::new();
        app.init_resource::<SeatBindings>();
        for (slot, map) in seats {
            app.world_mut().spawn((
                InputParticipant::with_id(crate::ParticipantId(*slot)),
                map.clone(),
            ));
        }
        app
    }

    #[test]
    fn two_seats_on_two_presets_report_two_different_labels() {
        // The reason this is per-seat: two people at one machine on different
        // halves of a keyboard. One global answer is right for neither.
        let arrows = KeyboardPreset::arrows_zxc();
        let wasd = KeyboardPreset::wasd_jkl();
        let mut app = app_with_seats(&[(0, arrows.input_map()), (1, wasd.input_map())]);
        publish(&mut app);

        let bindings = app.world().resource::<SeatBindings>();
        let seat0 = bindings.label(0, &Platformer2dInputActionMonolith::Jump);
        let seat1 = bindings.label(1, &Platformer2dInputActionMonolith::Jump);
        assert!(seat0.is_some() && seat1.is_some(), "both seats bind Jump");
        assert_ne!(
            seat0, seat1,
            "two presets, two answers — a global binding table could not say this"
        );
    }

    #[test]
    fn the_published_label_is_the_binding_the_router_reads() {
        // ⚠ the property that makes this ONE authority rather than a second
        // table: the projection is derived, so a rebind moves it with no
        // synchronisation step for anybody to forget.
        let preset = KeyboardPreset::arrows_zxc();
        let mut map = preset.input_map();
        let mut app = app_with_seats(&[(0, map.clone())]);
        publish(&mut app);
        let before = app
            .world()
            .resource::<SeatBindings>()
            .label(0, &Platformer2dInputActionMonolith::Jump);

        // Rebind Jump to a key nothing else uses, the way a remap screen would.
        map.clear_action(&Platformer2dInputActionMonolith::Jump);
        map.insert(Platformer2dInputActionMonolith::Jump, KeyCode::F13);
        let mut app = app_with_seats(&[(0, map)]);
        publish(&mut app);
        let after = app
            .world()
            .resource::<SeatBindings>()
            .label(0, &Platformer2dInputActionMonolith::Jump);

        assert_ne!(before, after, "the projection followed the rebind");
        assert_eq!(
            after.as_deref(),
            Some("F13"),
            "and it names the key the map actually holds"
        );
    }

    #[test]
    fn a_gamepad_binding_is_named_as_a_button_not_as_a_debug_blob() {
        let mut app = app_with_seats(&[(3, crate::presets::KeyboardPreset::gamepad_only_map())]);
        publish(&mut app);
        let bindings = app.world().resource::<SeatBindings>();
        let jump = bindings.for_seat(3).controls(&Platformer2dInputActionMonolith::Jump);
        assert!(
            jump.iter().any(|control| matches!(control, PhysicalControl::Button(_))),
            "the gamepad map binds Jump to a button: {jump:?}"
        );
        assert_eq!(
            bindings.label(3, &Platformer2dInputActionMonolith::Jump).as_deref(),
            Some("A"),
            "and it prints as a face button, which is what goes on a glyph"
        );
    }

    #[test]
    fn a_seat_with_no_map_is_bound_to_nothing_rather_than_missing() {
        let app = app_with_seats(&[]);
        let bindings = app.world().resource::<SeatBindings>();
        assert!(bindings.for_seat(2).is_empty());
        assert_eq!(bindings.label(2, &Platformer2dInputActionMonolith::Jump), None);
    }

    #[test]
    fn the_listing_is_in_a_stable_order() {
        // A help screen that lists its rows differently every launch reads as a
        // bug in the help screen.
        let map = KeyboardPreset::arrows_zxc().input_map();
        let mut first = app_with_seats(&[(0, map.clone())]);
        let mut second = app_with_seats(&[(0, map)]);
        publish(&mut first);
        publish(&mut second);
        let names = |app: &App| -> Vec<String> {
            app.world()
                .resource::<SeatBindings>()
                .for_seat(0)
                .all()
                .map(|(name, _)| name.to_string())
                .collect()
        };
        let listing = names(&first);
        assert_eq!(listing, names(&second));
        let mut sorted = listing.clone();
        sorted.sort();
        assert_eq!(listing, sorted, "canonical order, not hash order");
    }
}
