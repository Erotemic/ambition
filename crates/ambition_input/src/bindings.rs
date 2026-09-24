//! One authority for "which physical control is this action on".
//!
//! Routing, remapping, controller glyphs, touch affordances, action prompts
//! and help displays all read this one fact.
//!
//! ## Derived, never parallel
//!
//! [`SeatBindings`] is projected from the same
//! `InputMap<Platformer2dInputActionMonolith>` the router reads. It is not a
//! second table. A rebind changes the map, and the projection follows on the
//! next frame.
//!
//! ## Per seat
//!
//! Two people at one machine can hold different presets. `SeatBindings` is
//! keyed by participant slot, like [`crate::SeatInputContexts`] and
//! `SeatMenuFrames`.

use std::collections::BTreeMap;

use bevy::prelude::*;
use leafwing_input_manager::prelude::{ActionState, InputMap};

use crate::presets::{KeyboardPreset, PresetId};
use crate::settings::{BindingOverride, OverrideControl, OverrideDeviceClass};
use crate::{InputParticipant, Platformer2dInputActionMonolith};

/// Which physical devices this seat hears.
///
/// This is separate from the preset, so a recipe can say "seat 1 is on a pad
/// and seat 2 is on the keyboard". Character select can produce that mix.
///
/// For a decided match, the frozen `LocalChannelPlan` is the authority; this
/// only records its answer. A surface with no plan (launcher, menus) is
/// [`Self::Unified`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BindingSources {
    /// Keyboard and gamepad. One person playing alone, and every menu surface.
    #[default]
    Unified,
    /// Keyboard only: a seat someone claimed with the keyboard.
    KeyboardOnly,
    /// Gamepad only: a seat someone claimed with a pad.
    GamepadOnly,
}

impl BindingSources {
    pub fn admits_keyboard(self) -> bool {
        matches!(self, Self::Unified | Self::KeyboardOnly)
    }

    pub fn admits_gamepad(self) -> bool {
        matches!(self, Self::Unified | Self::GamepadOnly)
    }

    /// Overrides obey the scope too. `apply_override` inserts a binding even
    /// when the map has no binding of the same class, so a keyboard remap on
    /// a pad-only seat would add a keyboard key back.
    pub fn admits(self, class: crate::OverrideDeviceClass) -> bool {
        match class {
            crate::OverrideDeviceClass::Keyboard => self.admits_keyboard(),
            crate::OverrideDeviceClass::Gamepad => self.admits_gamepad(),
        }
    }
}

/// The declared source of one participant's `InputMap`.
///
/// It lives on the participant entity beside the map, so a rebuild affects
/// one seat only.
///
/// The map is working state, not a source: the seat-device pass restricts it
/// to one pad, the touch overlay inserts its virtual controls, and a remap
/// changes it. The recipe is the starting point every rebuild returns to. The
/// other layers re-apply through their own `Changed<InputMap>` hooks.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct BindingRecipe {
    /// The keyboard layout this seat uses if it hears a keyboard. A pad-only
    /// seat keeps it too, so a player who switches devices keeps the preset.
    pub preset: PresetId,
    /// Which devices this seat hears. See [`BindingSources`].
    pub sources: BindingSources,
    /// Which game's pad layout this seat uses: the middle term of
    /// `device -> profile -> semantic action -> rules`.
    ///
    /// It is a fact about the mode, not the person. See
    /// [`crate::BindingLayout`] for why this is not more [`Self::overrides`].
    pub layout: crate::BindingLayout,
    /// What this seat moved off the base, in declared order.
    ///
    /// A layer on the base, not a replacement: after a preset change, every
    /// action except the remapped ones follows the new preset.
    pub overrides: Vec<BindingOverride>,
}

impl BindingRecipe {
    pub fn preset(id: PresetId) -> Self {
        Self {
            preset: id,
            sources: BindingSources::Unified,
            layout: crate::BindingLayout::default(),
            overrides: Vec::new(),
        }
    }

    pub fn gamepad_only() -> Self {
        // A pad seat still carries a preset. The scope filters out the
        // keyboard half, so the seat works on a keyboard if the plan changes.
        Self::preset(KeyboardPreset::by_index(0).id).with_sources(BindingSources::GamepadOnly)
    }

    /// The same recipe restricted to the devices this seat actually claimed.
    pub fn with_sources(mut self, sources: BindingSources) -> Self {
        self.sources = sources;
        self
    }

    /// The same recipe with these overrides layered on.
    pub fn with_overrides(mut self, overrides: Vec<BindingOverride>) -> Self {
        self.overrides = overrides;
        self
    }

    /// The same recipe arranged for this game's layout.
    pub fn with_layout(mut self, layout: crate::BindingLayout) -> Self {
        self.layout = layout;
        self
    }

    /// The map this recipe declares. Pure, so spawn sites and the rebuild
    /// system give the same result.
    ///
    /// Layer order is precedence: base, then the game's layout, then the
    /// user's overrides. A user remap wins inside a mode that ships its own
    /// pad. Guarded by `a_user_remap_beats_the_modes_layout`.
    pub fn build(&self) -> InputMap<Platformer2dInputActionMonolith> {
        let mut map = KeyboardPreset::of(self.preset).map_for(self.sources);
        self.layout.apply(&mut map);
        for over in &self.overrides {
            // A remap cannot widen the seat. See `BindingSources::admits`.
            if self.sources.admits(over.control.device_class()) {
                apply_override(&mut map, over);
            }
        }
        map
    }
}

/// Layer one override onto a built map.
///
/// Do not use `clear_action`: it drops keyboard and gamepad bindings together,
/// so a keyboard remap would unbind the controller. Only bindings of the
/// override's device class are removed, found through [`ActionBindings`], the
/// same projection a prompt reads.
///
/// The override goes into the displaced binding's position, not at the end.
/// [`ActionBindings::label`] reads the first binding, so an appended override
/// would leave the prompt showing the old control.
///
/// Two cases are ignored, because a settings file can outlive its build:
/// * an action name this build does not have;
/// * an action that is not buttonlike (`Move`, `AimStick`). A button in an
///   axis action's map corrupts it, and nothing downstream checks.
fn apply_override(map: &mut InputMap<Platformer2dInputActionMonolith>, over: &BindingOverride) {
    use leafwing_input_manager::prelude::{Actionlike, Buttonlike};

    let Some(action) = action_named(&over.action) else {
        return;
    };
    if action.input_control_kind() != leafwing_input_manager::InputControlKind::Button {
        return;
    }
    // Positions of the same-class bindings, in map order. The projection uses
    // the same order, so an index here is the position a prompt reads.
    let class = over.control.device_class();
    let displaced: Vec<usize> = ActionBindings::from_map(map)
        .controls(&action)
        .iter()
        .enumerate()
        .filter(|(_, control)| device_class_of(control) == Some(class))
        .map(|(index, _)| index)
        .collect();

    let control: Box<dyn Buttonlike> = match over.control {
        OverrideControl::Key(key) => Box::new(key),
        OverrideControl::Button(button) => Box::new(button),
    };
    let Some(bindings) = map.get_buttonlike_mut(&action) else {
        // Nothing binds this action (for example, a preset with no gamepad
        // Special). The override is its first binding.
        map.insert_boxed(action, control);
        return;
    };
    let slot = displaced.first().copied().unwrap_or(bindings.len());
    // Back to front, so an earlier index is still the element it named.
    for index in displaced.iter().rev() {
        bindings.remove(*index);
    }
    bindings.insert(slot.min(bindings.len()), control);
}

/// Which device class a projected control belongs to. `None` for a control
/// the projection cannot name: an override must not displace a binding it
/// cannot restore.
fn device_class_of(control: &PhysicalControl) -> Option<OverrideDeviceClass> {
    match control {
        PhysicalControl::Key(_) => Some(OverrideDeviceClass::Keyboard),
        PhysicalControl::Button(_) => Some(OverrideDeviceClass::Gamepad),
        PhysicalControl::Other(_) => None,
    }
}

/// The action a settings-file name refers to, or `None` if this build has no
/// such action.
///
/// The name is the enum's variant spelling, the string [`action_name`]
/// publishes and a settings file stores. It resolves through `Reflect`.
pub fn action_named(name: &str) -> Option<Platformer2dInputActionMonolith> {
    use bevy::reflect::enums::{DynamicEnum, DynamicVariant, VariantInfo};
    use bevy::reflect::{FromReflect, TypeInfo, Typed};
    let TypeInfo::Enum(info) = Platformer2dInputActionMonolith::type_info() else {
        return None;
    };
    // Unit variants only: every action is a unit variant, and a
    // `DynamicVariant::Unit` for any other kind panics.
    if !matches!(info.variant(name), Some(VariantInfo::Unit(_))) {
        return None;
    }
    Platformer2dInputActionMonolith::from_reflect(&DynamicEnum::new(name, DynamicVariant::Unit))
}

/// Rebuild a participant's `InputMap` when its [`BindingRecipe`] changes.
///
/// It handles every seat.
///
/// * The seat's controller survives. The seat-device pass owns which pad the
///   map answers, so the current gamepad association is carried into the
///   rebuilt map. It does not fall back to leafwing's any-pad for a frame.
/// * Edges do not leak across bindings. `ActionState` is reset when the map
///   changes, because a press under the old bindings is not a press under the
///   new ones.
///
/// Touch virtual controls are not re-added here. The touch crate's
/// `Changed<InputMap>` hook re-binds them the same frame.
pub fn rebuild_maps_from_recipes(
    mut commands: Commands,
    mut participants: Query<
        (
            Entity,
            &BindingRecipe,
            &mut InputMap<Platformer2dInputActionMonolith>,
            &mut ActionState<Platformer2dInputActionMonolith>,
        ),
        Changed<BindingRecipe>,
    >,
) {
    for (seat, recipe, mut map, mut actions) in &mut participants {
        let mut built = recipe.build();
        if let Some(pad) = map.gamepad() {
            built.set_gamepad(pad);
        }
        // `Changed` includes `Added`. On the spawn frame the map already is
        // the recipe's output. The comparison stops that frame from resetting
        // a fresh `ActionState` and from triggering the touch re-bind hook.
        if *map != built {
            *map = built;
            actions.reset_all();
            // The reset is only half the fix. See [`Rebound`].
            commands.entity(seat).insert(Rebound);
        }
    }
}

/// A seat whose map was rewritten on the previous frame.
///
/// `reset_all()` alone creates a false press. Example: leaving Smash retracts
/// its `BindingLayout`, which rebuilds every seat's map on the frame the shell
/// routes home. With Enter held:
///
/// ```text
/// f0  select=true   route=smash_gameplay     <- the real confirm, on "Quit to Title"
/// f1  select=false  route=ambition_launcher  <- home, launcher cursor reset to row 0
/// f2  select=false  route=ambition_launcher  <- map rewritten, ActionState reset: NOT pressed
/// f3  select=true   route=ambition_launcher  <- the key never moved. leafwing sees
///                                              Released -> down and calls it a PRESS
/// f5  ...           route=ambition_gameplay  <- the launcher launched row 0
/// ```
///
/// `reset_all` leaves the action `Released` while the key is still down, so
/// leafwing's next `Update` reads a rising edge. The launcher took it as a
/// confirm and started a game.
///
/// The reset is still correct: a press under the old bindings must end. This
/// marker lets [`swallow_the_rebinds_own_edges`] remove the false edge one
/// frame later.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rebound;

/// Age out every press edge born of the rebind, not of a finger.
///
/// Runs in `PreUpdate` after leafwing re-reads the devices through the new
/// map. `JustPressed` becomes `Pressed`, so a control held across the rebind
/// reads held but not newly pressed. Only actions that leafwing reads as down
/// are changed.
///
/// On that one frame it cannot tell the false edge from a real press. A real
/// press on the frame after a rebind loses its edge and must be repeated.
/// This costs one frame, on a preset change or mode exit.
pub fn swallow_the_rebinds_own_edges(
    mut commands: Commands,
    mut seats: Query<(Entity, &mut ActionState<Platformer2dInputActionMonolith>), With<Rebound>>,
) {
    use leafwing_input_manager::buttonlike::ButtonState;

    for (seat, mut actions) in &mut seats {
        for action in actions.get_just_pressed() {
            if let Some(data) = actions.button_data_mut(&action) {
                data.state = ButtonState::Pressed;
                data.update_state = ButtonState::Pressed;
            }
        }
        commands.entity(seat).remove::<Rebound>();
    }
}

/// A physical control, named well enough to draw.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhysicalControl {
    Key(KeyCode),
    Button(GamepadButton),
    /// Something the projection could not name.
    ///
    /// It keeps the debug form. If a prompt drops an unknown binding, the
    /// action looks unbound, and the missing arm stays hidden.
    Other(String),
}

impl PhysicalControl {
    /// What to print. Short, because it goes on a button. Gamepad buttons use
    /// Xbox names; a caller that knows the seat's pad uses [`Self::label_for`].
    pub fn label(&self) -> String {
        self.label_for(crate::GamepadStyle::XboxLike)
    }

    /// What to print, in the names of the seat's own pad. It uses the same
    /// table as the glyph path, so a prompt and a glyph always agree.
    pub fn label_for(&self, style: crate::GamepadStyle) -> String {
        match self {
            Self::Key(key) => key_label(*key),
            Self::Button(button) => crate::glyphs::button_label(*button, style).to_string(),
            Self::Other(raw) => raw.clone(),
        }
    }
}

/// One seat's bindings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionBindings {
    /// Sorted by action name, so the order is stable across runs and machines.
    actions: Vec<(String, Vec<PhysicalControl>)>,
}

impl ActionBindings {
    /// Project one live `InputMap`. The system below is a thin loop over this,
    /// so a pure caller (test, rebind preview, docs generator) gets the same
    /// answer without a `World`.
    pub fn from_map(map: &InputMap<Platformer2dInputActionMonolith>) -> Self {
        let mut actions: Vec<(String, Vec<PhysicalControl>)> = map
            .iter_buttonlike()
            .map(|(action, inputs)| {
                (
                    action_name(action),
                    inputs
                        .iter()
                        .map(|input| physical_control_of(input.as_ref()))
                        .collect(),
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

    /// The selection primitive: which bound control this seat shows, given
    /// the device in its hands.
    ///
    /// Choose the control first. Re-spelling a `KeyCode` in a controller
    /// vocabulary still gives `Z`, not `Cross`.
    ///
    /// Returns `None` when the seat has no binding of the device's class.
    /// Callers pick their miss policy: a glyph draws nothing, and a label
    /// falls back to the other class.
    pub fn control_for(
        &self,
        action: &Platformer2dInputActionMonolith,
        device: crate::ActiveDevice,
    ) -> Option<&PhysicalControl> {
        let want_key = device.draws_keyboard_glyphs();
        self.controls(action).iter().find(|control| {
            matches!(
                (control, want_key),
                (PhysicalControl::Key(_), true) | (PhysicalControl::Button(_), false)
            )
        })
    }

    /// The label a prompt shows: the first binding, which the preset lists as
    /// primary.
    ///
    /// It ignores the device: gamepad buttons use Xbox names and a mixed map
    /// gives its keyboard half. Use it only with no seat (docs generator,
    /// rebind list). A prompt for a seat uses [`Self::label_for`].
    pub fn label(&self, action: &Platformer2dInputActionMonolith) -> Option<String> {
        self.controls(action).first().map(PhysicalControl::label)
    }

    /// The label for the device this seat holds. From one map, a DualSense
    /// reads "Cross", an Xbox pad "A", and a keyboard "Z".
    ///
    /// It falls back to the other device class, not `None`: a blank prompt
    /// reads as "this action has no control".
    pub fn label_for(
        &self,
        action: &Platformer2dInputActionMonolith,
        device: crate::ActiveDevice,
    ) -> Option<String> {
        self.control_for(action, device)
            .or_else(|| self.controls(action).first())
            .map(|control| control.label_for(device.gamepad_style()))
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

/// Which action drives an ability slot.
///
/// This replaces two hand-kept maps in `ambition_touch_input`
/// (`TouchActionButton -> ControlSlot` and
/// `TouchActionButton -> Platformer2dInputActionMonolith`).
///
/// `ControlSlot` is the ability vocabulary (`ambition_entity_catalog`) and
/// `Platformer2dInputActionMonolith` is the input vocabulary. The mapping is
/// an input-layer fact, so it lives here, where prompts and the overlay can
/// reach it.
///
/// `None` is for a slot with no single action (for example, a future chord).
pub fn action_for_slot(
    slot: ambition_entity_catalog::action_scheme::ControlSlot,
) -> Option<Platformer2dInputActionMonolith> {
    use ambition_entity_catalog::action_scheme::ControlSlot;
    Some(match slot {
        ControlSlot::Jump => Platformer2dInputActionMonolith::Jump,
        ControlSlot::Attack => Platformer2dInputActionMonolith::Attack,
        ControlSlot::Special => Platformer2dInputActionMonolith::Special,
        ControlSlot::Projectile => Platformer2dInputActionMonolith::Projectile,
        ControlSlot::Burst => Platformer2dInputActionMonolith::Burst,
        ControlSlot::Blink => Platformer2dInputActionMonolith::Blink,
        ControlSlot::Interact => Platformer2dInputActionMonolith::Interact,
        ControlSlot::Utility => Platformer2dInputActionMonolith::Utility,
        ControlSlot::Shield => Platformer2dInputActionMonolith::Shield,
        ControlSlot::Grab => Platformer2dInputActionMonolith::Grab,
        ControlSlot::Taunt => Platformer2dInputActionMonolith::Taunt,
        ControlSlot::Modifier => Platformer2dInputActionMonolith::Modifier,
    })
}

/// Every seat's bindings, keyed by participant slot.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct SeatBindings {
    seats: BTreeMap<u8, ActionBindings>,
}

impl SeatBindings {
    /// A seat with no map is bound to nothing. An unplugged pad is normal.
    pub fn for_seat(&self, slot: u8) -> &ActionBindings {
        static NONE: std::sync::LazyLock<ActionBindings> =
            std::sync::LazyLock::new(ActionBindings::default);
        self.seats.get(&slot).unwrap_or(&NONE)
    }

    /// The label this seat's prompt should show for this action.
    pub fn label(&self, slot: u8, action: &Platformer2dInputActionMonolith) -> Option<String> {
        self.for_seat(slot).label(action)
    }

    /// The label for an ability slot. Uses [`action_for_slot`].
    ///
    /// `devices` is a parameter because the seat's device decides both which
    /// binding to name and how to spell it. `None` (a headless sim with no
    /// device tracking) means the keyboard, the documented cold-start answer.
    pub fn label_for_slot(
        &self,
        seat: u8,
        slot: ambition_entity_catalog::action_scheme::ControlSlot,
        devices: Option<&crate::SeatActiveDevices>,
    ) -> Option<String> {
        let device = devices.map_or_else(Default::default, |devices| devices.for_seat(seat));
        self.for_seat(seat)
            .label_for(&action_for_slot(slot)?, device)
    }

    pub fn seats(&self) -> impl Iterator<Item = (u8, &ActionBindings)> {
        self.seats.iter().map(|(slot, bindings)| (*slot, bindings))
    }
}

/// Project every participant's live `InputMap` into [`SeatBindings`].
///
/// Runs in `InputSet::ResolveActions`, after any remap and before prompts
/// draw. A quiet frame does not touch the resource, so `is_changed()` fires
/// only on a real rebind.
pub fn publish_seat_bindings(
    participants: Query<(
        &InputParticipant,
        &InputMap<Platformer2dInputActionMonolith>,
    )>,
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

/// The action's stable name: its `Debug` form, which the trace and the
/// settings file also use. [`action_named`] is its inverse.
pub fn action_name(action: &Platformer2dInputActionMonolith) -> String {
    format!("{action:?}")
}

/// Name one bound control.
///
/// `Buttonlike: Reflect`, so the concrete input downcasts. Any other input
/// becomes [`PhysicalControl::Other`] with its `Debug` form. This is the one
/// place a `dyn Buttonlike` becomes comparable. [`crate::layout`] uses it to
/// remove a binding, so a layout displaces what a prompt shows on that button.
pub(crate) fn physical_control_of(
    input: &dyn leafwing_input_manager::prelude::Buttonlike,
) -> PhysicalControl {
    let reflected = input.as_reflect();
    if let Some(key) = reflected.downcast_ref::<KeyCode>() {
        return PhysicalControl::Key(*key);
    }
    if let Some(button) = reflected.downcast_ref::<GamepadButton>() {
        return PhysicalControl::Button(*button);
    }
    PhysicalControl::Other(format!("{input:?}"))
}

/// `presets::key_name` returns `"?"` for a key it does not list. For those
/// keys, use the `KeyCode` debug form (`F13`, `NumpadAdd`), as with
/// [`PhysicalControl::Other`].
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
        // Two people on two halves of one keyboard need different answers.
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
        // The projection is derived, so a rebind moves it with no sync step.
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
        let mut app = app_with_seats(&[(
            3,
            crate::presets::KeyboardPreset::by_index(0).map_for(crate::BindingSources::GamepadOnly),
        )]);
        publish(&mut app);
        let bindings = app.world().resource::<SeatBindings>();
        let jump = bindings
            .for_seat(3)
            .controls(&Platformer2dInputActionMonolith::Jump);
        assert!(
            jump.iter()
                .any(|control| matches!(control, PhysicalControl::Button(_))),
            "the gamepad map binds Jump to a button: {jump:?}"
        );
        assert_eq!(
            bindings
                .label(3, &Platformer2dInputActionMonolith::Jump)
                .as_deref(),
            Some("A"),
            "and it prints as a face button, which is what goes on a glyph"
        );
    }

    /// A prompt names the device that is actually in the seat's hands.
    ///
    /// The primary seat's map is mixed: `input_map()` adds the keyboard half
    /// first, then the gamepad half. So `.first()` is a key for every action
    /// both halves bind. The gamepad-only fixture above cannot catch this.
    ///
    /// Both directions are checked. Always preferring the gamepad binding
    /// would pass the first assertion; the keyboard case catches it.
    #[test]
    fn a_prompt_names_the_device_the_seat_is_actually_holding() {
        use crate::active_input::{ActiveDevice, SeatActiveDevices};
        use ambition_entity_catalog::action_scheme::ControlSlot;

        // The primary seat's real map: keyboard first, then gamepad.
        let mut app = app_with_seats(&[(0, KeyboardPreset::arrows_zxc().input_map())]);
        publish(&mut app);
        let bindings = app.world().resource::<SeatBindings>().clone();

        let mut on_a_dualsense = SeatActiveDevices::default();
        on_a_dualsense.mark(0, ActiveDevice::Gamepad(crate::GamepadStyle::PlayStation));
        assert_eq!(
            bindings
                .label_for_slot(0, ControlSlot::Jump, Some(&on_a_dualsense))
                .as_deref(),
            Some("Cross"),
            "a seat holding a DualSense is told to press Cross — naming the key \
             `Z` that the same action also binds is a prompt for a device that \
             is not in their hands"
        );

        let mut on_the_keyboard = SeatActiveDevices::default();
        on_the_keyboard.mark(0, ActiveDevice::Keyboard);
        assert_eq!(
            bindings
                .label_for_slot(0, ControlSlot::Jump, Some(&on_the_keyboard))
                .as_deref(),
            Some("Z"),
            "and the same seat back on the keyboard is told to press Z, not the \
             button it is no longer holding"
        );
    }

    #[test]
    fn a_seat_with_no_map_is_bound_to_nothing_rather_than_missing() {
        let app = app_with_seats(&[]);
        let bindings = app.world().resource::<SeatBindings>();
        assert!(bindings.for_seat(2).is_empty());
        assert_eq!(
            bindings.label(2, &Platformer2dInputActionMonolith::Jump),
            None
        );
    }

    #[test]
    fn a_recipe_change_rebuilds_the_map_and_keeps_the_seats_pad() {
        let mut app = App::new();
        app.add_systems(Update, rebuild_maps_from_recipes);
        let pad = app.world_mut().spawn_empty().id();
        let recipe = BindingRecipe::preset(PresetId::ArrowsZxc);
        let mut map = recipe.build();
        map.set_gamepad(pad);
        let seat = app
            .world_mut()
            .spawn((
                InputParticipant::with_id(crate::ParticipantId(0)),
                recipe,
                map,
                ActionState::<Platformer2dInputActionMonolith>::default(),
            ))
            .id();
        app.update();

        app.world_mut()
            .entity_mut(seat)
            .insert(BindingRecipe::preset(PresetId::WasdJkl));
        app.update();

        let map = app
            .world()
            .entity(seat)
            .get::<InputMap<Platformer2dInputActionMonolith>>()
            .expect("the participant keeps its map");
        assert_eq!(
            ActionBindings::from_map(map)
                .label(&Platformer2dInputActionMonolith::Jump)
                .as_deref(),
            Some("Space"),
            "the rebuilt map is the new preset's (WASD binds Jump to Space)"
        );
        assert_eq!(
            map.gamepad(),
            Some(pad),
            "a recipe change re-decides bindings, never the seat's controller"
        );
    }

    #[test]
    fn every_published_action_name_resolves_back_to_its_action() {
        // The list comes from the map, so a new action is covered once bound.
        let map = KeyboardPreset::arrows_zxc().input_map();
        let listing = ActionBindings::from_map(&map);
        assert!(!listing.is_empty(), "the preset binds something");
        for (name, _) in listing.all() {
            let resolved = action_named(name).unwrap_or_else(|| panic!("`{name}` resolves"));
            assert_eq!(
                action_name(&resolved),
                name,
                "and it resolves to the action that published the name"
            );
        }
    }

    #[test]
    fn an_action_name_this_build_does_not_have_is_ignored_not_fatal() {
        // A settings file can come from a build with an action this one lacks.
        // It must still load.
        assert_eq!(action_named("SummonKraken"), None);
        let recipe = BindingRecipe::preset(PresetId::ArrowsZxc)
            .with_overrides(vec![BindingOverride::key("SummonKraken", KeyCode::F13)]);
        assert_eq!(
            recipe.build(),
            BindingRecipe::preset(PresetId::ArrowsZxc).build(),
            "an unknown action leaves the map exactly as the preset built it"
        );
    }

    /// Precedence: base, then the game's layout, then the user's overrides.
    ///
    /// A mode's pad is a better default, not an override of the player. A
    /// player who put Special on Y keeps it on Y in a smash match, where the
    /// layout says X.
    #[test]
    fn a_user_remap_beats_the_modes_layout() {
        let smash = BindingRecipe::gamepad_only().with_layout(crate::BindingLayout::Smash);
        let layout_only = ActionBindings::from_map(&smash.build());
        assert_eq!(
            layout_only.controls(&Platformer2dInputActionMonolith::Special),
            [PhysicalControl::Button(GamepadButton::West)],
            "the mode's layout is what an unremapped player gets"
        );

        let remapped = smash.clone().with_overrides(vec![BindingOverride::button(
            "Special",
            GamepadButton::North,
        )]);
        let after = ActionBindings::from_map(&remapped.build());
        assert_eq!(
            after.controls(&Platformer2dInputActionMonolith::Special),
            [PhysicalControl::Button(GamepadButton::North)],
            "the USER's remap wins over the mode's layout"
        );
        // The remap displaces; it does not add a second button.
        assert!(
            !after
                .controls(&Platformer2dInputActionMonolith::Special)
                .contains(&PhysicalControl::Button(GamepadButton::West)),
            "the layout's West binding is gone, not doubled"
        );
    }

    #[test]
    fn an_override_moves_the_key_and_leaves_the_pad_alone() {
        // `clear_action` would drop keyboard and gamepad bindings together and
        // unbind the controller.
        let base = BindingRecipe::preset(PresetId::ArrowsZxc);
        let before = ActionBindings::from_map(&base.build());
        let pad_before: Vec<_> = before
            .controls(&Platformer2dInputActionMonolith::Jump)
            .iter()
            .filter(|control| matches!(control, PhysicalControl::Button(_)))
            .cloned()
            .collect();
        assert!(!pad_before.is_empty(), "the preset binds Jump on a pad too");

        let remapped = base
            .clone()
            .with_overrides(vec![BindingOverride::key("Jump", KeyCode::F13)]);
        let after = ActionBindings::from_map(&remapped.build());
        let jump = after.controls(&Platformer2dInputActionMonolith::Jump);

        assert_eq!(
            jump.iter()
                .filter(|control| matches!(control, PhysicalControl::Key(_)))
                .collect::<Vec<_>>(),
            vec![&PhysicalControl::Key(KeyCode::F13)],
            "exactly one key, and it is the override's — the preset's key was displaced"
        );
        let pad_after: Vec<_> = jump
            .iter()
            .filter(|control| matches!(control, PhysicalControl::Button(_)))
            .cloned()
            .collect();
        assert_eq!(pad_after, pad_before, "the pad half is untouched");
    }

    #[test]
    fn an_override_layers_on_the_base_rather_than_freezing_it() {
        // Changing preset after a remap moves every action except the
        // remapped one. A stored map would freeze the old preset.
        let overrides = vec![BindingOverride::key("Jump", KeyCode::F13)];
        let arrows = BindingRecipe::preset(PresetId::ArrowsZxc)
            .with_overrides(overrides.clone())
            .build();
        let wasd = BindingRecipe::preset(PresetId::WasdJkl)
            .with_overrides(overrides)
            .build();

        let jump = |map: &InputMap<Platformer2dInputActionMonolith>| {
            ActionBindings::from_map(map).label(&Platformer2dInputActionMonolith::Jump)
        };
        let attack = |map: &InputMap<Platformer2dInputActionMonolith>| {
            ActionBindings::from_map(map).label(&Platformer2dInputActionMonolith::Attack)
        };
        assert_eq!(jump(&arrows).as_deref(), Some("F13"));
        assert_eq!(
            jump(&wasd).as_deref(),
            Some("F13"),
            "the remapped action stays where the player put it"
        );
        assert_ne!(
            attack(&arrows),
            attack(&wasd),
            "and everything nobody remapped still follows the preset"
        );
    }

    #[test]
    fn an_override_on_an_axis_action_is_refused() {
        // A buttonlike in a dual-axis action's map corrupts it: the action
        // reads as bound, and the stick reads as dead.
        let recipe = BindingRecipe::preset(PresetId::ArrowsZxc)
            .with_overrides(vec![BindingOverride::key("Move", KeyCode::F13)]);
        let built = recipe.build();
        assert_eq!(
            built,
            BindingRecipe::preset(PresetId::ArrowsZxc).build(),
            "the axis action's map is exactly what the preset built"
        );
        assert!(
            !ActionBindings::from_map(&built)
                .all()
                .any(|(_, controls)| controls.contains(&PhysicalControl::Key(KeyCode::F13))),
            "and the override's key went nowhere at all"
        );
    }

    /// A key held across a rebind is not a new press.
    ///
    /// `rebuild_maps_from_recipes` clears `ActionState`. Leafwing then reads
    /// the held key off a `Released` state and reports a rising edge. See
    /// `Rebound`.
    ///
    /// The override moves a different action, so the rebuild is real
    /// (`*map != built`) while `Jump` stays on the held key. Leaving Smash has
    /// the same shape.
    ///
    /// Without `swallow_the_rebinds_own_edges`, the last assertion fails.
    #[test]
    fn a_key_held_across_a_rebind_is_not_a_new_press() {
        use leafwing_input_manager::prelude::{Buttonlike, InputManagerPlugin};

        const HELD: KeyCode = KeyCode::KeyJ;
        let jump = Platformer2dInputActionMonolith::Jump;

        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins)
            .add_plugins(bevy::input::InputPlugin)
            .add_plugins(InputManagerPlugin::<Platformer2dInputActionMonolith>::default())
            .add_systems(Update, rebuild_maps_from_recipes)
            .add_systems(
                bevy::app::PreUpdate,
                swallow_the_rebinds_own_edges
                    .after(leafwing_input_manager::plugin::InputManagerSystem::Update),
            );

        let recipe = BindingRecipe::preset(PresetId::ArrowsZxc)
            .with_overrides(vec![BindingOverride::key("Jump", HELD)]);
        let seat = app
            .world_mut()
            .spawn((
                InputParticipant::with_id(crate::ParticipantId(0)),
                recipe.clone(),
                recipe.build(),
                ActionState::<Platformer2dInputActionMonolith>::default(),
            ))
            .id();
        app.update();

        let state = |app: &App| {
            let actions = app
                .world()
                .entity(seat)
                .get::<ActionState<Platformer2dInputActionMonolith>>()
                .expect("the seat keeps its action state");
            (actions.pressed(&jump), actions.just_pressed(&jump))
        };

        // The fixture must see a real press first, or the assertions below
        // pass for the wrong reason.
        Buttonlike::press(&HELD, app.world_mut());
        app.update();
        assert_eq!(state(&app), (true, true), "the real press is an edge");
        app.update();
        assert_eq!(state(&app), (true, false), "and only on its own frame");

        // The rebind. `Interact` moves; `Jump` stays on the held key.
        app.world_mut().entity_mut(seat).insert(
            BindingRecipe::preset(PresetId::ArrowsZxc).with_overrides(vec![
                BindingOverride::key("Jump", HELD),
                BindingOverride::key("Interact", KeyCode::F13),
            ]),
        );
        app.update();

        // The frame where the false edge would appear.
        app.update();
        let (pressed, just_pressed) = state(&app);
        assert!(
            pressed,
            "the key never moved, so the action is still held after the rebind"
        );
        assert!(
            !just_pressed,
            "a rebind is not a press: the key was already down when the map \
             changed and must not read as newly pressed"
        );
    }

    #[test]
    fn a_recipe_change_that_is_only_an_override_still_rebuilds_the_map() {
        // An override reaches behaviour through the same rebuild as a preset
        // change.
        let mut app = App::new();
        app.add_systems(Update, rebuild_maps_from_recipes);
        let recipe = BindingRecipe::preset(PresetId::ArrowsZxc);
        let seat = app
            .world_mut()
            .spawn((
                InputParticipant::with_id(crate::ParticipantId(0)),
                recipe.clone(),
                recipe.build(),
                ActionState::<Platformer2dInputActionMonolith>::default(),
            ))
            .id();
        app.update();

        app.world_mut()
            .entity_mut(seat)
            .insert(recipe.with_overrides(vec![BindingOverride::key("Jump", KeyCode::F13)]));
        app.update();

        let map = app
            .world()
            .entity(seat)
            .get::<InputMap<Platformer2dInputActionMonolith>>()
            .expect("the participant keeps its map");
        assert_eq!(
            ActionBindings::from_map(map)
                .label(&Platformer2dInputActionMonolith::Jump)
                .as_deref(),
            Some("F13"),
            "the live map the router reads is the overridden one"
        );
    }

    #[test]
    fn the_listing_is_in_a_stable_order() {
        // A help screen must list rows in the same order every launch.
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
