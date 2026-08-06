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
use leafwing_input_manager::prelude::{ActionState, InputMap};

use crate::presets::{KeyboardPreset, PresetId};
use crate::settings::{BindingOverride, OverrideControl, OverrideDeviceClass};
use crate::{InputParticipant, Platformer2dInputActionMonolith};

/// What a participant's `InputMap` is BUILT from.
///
/// The map itself is WORKING STATE, not a source: the seat-device pass
/// restricts it to one pad, the touch overlay inserts its virtual controls,
/// and a remap will mutate it live. None of that can be re-derived from the
/// map alone — which is why "apply the new preset" used to be a wholesale
/// replacement that exactly one caller (one app, one seat) knew how to
/// perform. The recipe is the declared starting point every rebuild returns
/// to; the layers on top re-apply themselves through their own
/// `Changed<InputMap>` hooks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingBase {
    /// A keyboard-and-gamepad preset — the primary seat's shape.
    Preset(PresetId),
    /// Gamepad bindings only. A second player on the same keyboard as the
    /// first is not a second player, so extra seats start here.
    GamepadOnly,
}

/// The declared source of one participant's `InputMap`.
///
/// Lives on the participant entity beside the map so a rebuild is a fact
/// about ONE seat: the settings menu changes the primary's recipe and the
/// primary's map follows; a couch seat's gamepad-only recipe does not care
/// what preset the keyboard player picked.
/// ⚠ **`Clone`, not `Copy`** — [`Self::overrides`] is a `Vec`. The two spawn
/// sites and the settings-sync system each take one `.clone()`; nothing else
/// held it by value.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct BindingRecipe {
    pub base: BindingBase,
    /// What this seat moved off the base, in the order it was declared.
    ///
    /// A LAYER on the base, never a replacement for it: the base still decides
    /// every action nobody remapped, so changing preset after a remap moves
    /// everything except the remapped action — which is what "I changed one
    /// binding" has to mean.
    pub overrides: Vec<BindingOverride>,
}

impl BindingRecipe {
    pub fn preset(id: PresetId) -> Self {
        Self {
            base: BindingBase::Preset(id),
            overrides: Vec::new(),
        }
    }

    pub fn gamepad_only() -> Self {
        Self {
            base: BindingBase::GamepadOnly,
            overrides: Vec::new(),
        }
    }

    /// The same recipe with these overrides layered on.
    pub fn with_overrides(mut self, overrides: Vec<BindingOverride>) -> Self {
        self.overrides = overrides;
        self
    }

    /// The map this recipe declares. Pure, so a spawn site and the rebuild
    /// system cannot drift: both call this.
    pub fn build(&self) -> InputMap<Platformer2dInputActionMonolith> {
        let mut map = match self.base {
            BindingBase::Preset(id) => KeyboardPreset::of(id).input_map(),
            BindingBase::GamepadOnly => KeyboardPreset::gamepad_only_map(),
        };
        for over in &self.overrides {
            apply_override(&mut map, over);
        }
        map
    }
}

/// Layer one override onto a built map.
///
/// ⛔ **not `clear_action`**, which drops an action's keyboard AND gamepad
/// bindings together — so remapping Jump to `J` on a keyboard would silently
/// unbind the controller, and the player would find out mid-jump. The removal
/// is restricted to the override's own device class, enumerated through
/// [`ActionBindings`] — the same projection a prompt reads, so "what this
/// override replaces" is by construction what the screen was showing.
///
/// ⚠ **the override lands IN THE DISPLACED BINDING'S PLACE, not on the end.**
/// [`ActionBindings::label`] is the first binding, so an appended override left
/// a remapped Jump still printing the gamepad button it did not touch: the
/// player remaps to `J` and the prompt keeps saying `A`. That is precisely the
/// lie this module exists to make impossible, and it survived the
/// "does the map bind the new key" check — only asking for the LABEL caught it.
///
/// Two things are quietly ignored rather than enforced, both because a settings
/// file outlives the build that wrote it:
/// * an action name this build does not have (a file from a newer build);
/// * an action that is not buttonlike (`Move`, `AimStick`) — inserting a button
///   into an axis action's map corrupts it in a way nothing downstream checks.
fn apply_override(map: &mut InputMap<Platformer2dInputActionMonolith>, over: &BindingOverride) {
    use leafwing_input_manager::prelude::{Actionlike, Buttonlike};

    let Some(action) = action_named(&over.action) else {
        return;
    };
    if action.input_control_kind() != leafwing_input_manager::InputControlKind::Button {
        return;
    }
    // WHERE the same-class bindings are, in the map's own order. The projection
    // iterates that same order, so an index here means the same position a
    // prompt would read.
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
        // Nothing bound this action at all — a preset that leaves gamepad
        // Special unassigned, say. The override is its first binding.
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

/// Which device class a projected control belongs to. `None` for a control the
/// projection could not name — an override must not displace a binding nobody
/// can identify, because it could not put it back.
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
/// **Derived, like everything else here.** The name is the enum's own variant
/// spelling — the string [`action_name`] publishes and a settings file stores —
/// and the resolution runs through `Reflect`, which the action enum already
/// derives. So a new action is nameable the moment it is declared; a
/// hand-written `"Jump" => Jump` table would have to be remembered, and the
/// symptom of forgetting is a remap that silently does nothing.
/// ⚠ **the variant is checked BEFORE it is built.** `FromReflect` on a
/// `DynamicEnum` naming a variant that does not exist PANICS rather than
/// answering `None` — so the obvious three-line version turned "a settings file
/// from a newer build" into a crash on load.
pub fn action_named(name: &str) -> Option<Platformer2dInputActionMonolith> {
    use bevy::reflect::{DynamicEnum, DynamicVariant, FromReflect, TypeInfo, Typed, VariantInfo};
    let TypeInfo::Enum(info) = Platformer2dInputActionMonolith::type_info() else {
        return None;
    };
    // Unit only: every action is a unit variant today, and building a
    // `DynamicVariant::Unit` for anything else panics the same way.
    if !matches!(info.variant(name), Some(VariantInfo::Unit(_))) {
        return None;
    }
    Platformer2dInputActionMonolith::from_reflect(&DynamicEnum::new(name, DynamicVariant::Unit))
}

/// Rebuild a participant's `InputMap` when its [`BindingRecipe`] changes.
///
/// Every seat, not "the" seat: the app-side system this replaces read its
/// participant with `single_mut()`, so the moment a second seat existed a
/// preset change silently reached nobody — and it existed only in Ambition's
/// own app, so no demo composition had a rebuild path at all.
///
/// Two properties the replacement preserves on purpose:
/// * **the seat's controller survives.** The seat-device pass owns WHICH pad
///   this map answers; a recipe change re-decides the bindings, never the
///   seat's controller — so the current gamepad association is carried into
///   the rebuilt map rather than dropping to leafwing's any-pad fallback for
///   a frame.
/// * **edges do not leak across bindings.** `ActionState` is reset when the
///   map actually changes: a press latched under the old bindings is not a
///   press under the new ones.
///
/// Touch virtual controls are NOT re-added here — the touch crate's
/// `Changed<InputMap>` hook re-binds them the same frame, exactly as it does
/// for every other wholesale map write.
pub fn rebuild_maps_from_recipes(
    mut participants: Query<
        (
            &BindingRecipe,
            &mut InputMap<Platformer2dInputActionMonolith>,
            &mut ActionState<Platformer2dInputActionMonolith>,
        ),
        Changed<BindingRecipe>,
    >,
) {
    for (recipe, mut map, mut actions) in &mut participants {
        let mut built = recipe.build();
        if let Some(pad) = map.gamepad() {
            built.set_gamepad(pad);
        }
        // `Changed` includes `Added`, and on the spawn frame the map IS the
        // recipe's output — comparing before writing keeps that frame from
        // resetting a fresh `ActionState` and from marking the map changed
        // for the touch re-bind hook over nothing.
        if *map != built {
            *map = built;
            actions.reset_all();
        }
    }
}

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
    /// What to print. Short, because it goes on a button. Gamepad buttons are
    /// spelled Xbox-style; a caller that knows the seat's pad uses
    /// [`Self::label_for`].
    pub fn label(&self) -> String {
        self.label_for(crate::GamepadStyle::XboxLike)
    }

    /// What to print, in the vocabulary of the seat's own pad — the SAME
    /// table the glyph path draws from, so a prompt and a glyph can never
    /// name one physical button two ways on one frame.
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
                    inputs
                        .iter()
                        .map(|input| classify(input.as_ref()))
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

    /// The label a prompt should show — the FIRST binding, which is the one a
    /// preset lists first and therefore the one the author considered primary.
    ///
    /// Gamepad buttons come out Xbox-style. A caller that knows which pad the
    /// seat is holding should say so with [`Self::label_for`].
    pub fn label(&self, action: &Platformer2dInputActionMonolith) -> Option<String> {
        self.controls(action).first().map(PhysicalControl::label)
    }

    /// The same label, in the vocabulary of the pad this seat is actually
    /// holding — so a DualSense reads "Cross" where an Xbox pad reads "A".
    pub fn label_for(
        &self,
        action: &Platformer2dInputActionMonolith,
        style: crate::GamepadStyle,
    ) -> Option<String> {
        self.controls(action)
            .first()
            .map(|control| control.label_for(style))
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
pub fn action_for_slot(
    slot: ambition_entity_catalog::action_scheme::ControlSlot,
) -> Option<Platformer2dInputActionMonolith> {
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
    ///
    /// ⚠ **`devices` is a PARAMETER, not a second call.** The seat's pad decides
    /// whether its jump button reads "A" or "Cross", so a caller that asked for
    /// the label and then re-spelled it from the device fact would be two steps
    /// where one is correct — and the second step is the one a new caller
    /// forgets. `None` (a headless sim with no device tracking) spells gamepad
    /// buttons Xbox-style, which is the documented default and not a guess about
    /// this seat.
    pub fn label_for_slot(
        &self,
        seat: u8,
        slot: ambition_entity_catalog::action_scheme::ControlSlot,
        devices: Option<&crate::SeatActiveDevices>,
    ) -> Option<String> {
        let style =
            devices.map_or_else(Default::default, |devices| devices.gamepad_style_for(seat));
        self.for_seat(seat)
            .label_for(&action_for_slot(slot)?, style)
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

/// The action's stable name. `Debug` is what leafwing's derive gives us and it
/// is the same string the trace and the settings file already use, so a rebind
/// UI keyed on it lines up with everything else. [`action_named`] is its
/// inverse.
pub fn action_name(action: &Platformer2dInputActionMonolith) -> String {
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
        // The round trip is the whole reason `action_named` goes through
        // `Reflect` instead of a hand-written table: a table is correct until
        // somebody adds an action, and the symptom of forgetting is a remap
        // that silently does nothing. Driving this from the map's OWN action
        // list means a new action is covered the day it is bound.
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
        // A settings file outlives the build that wrote it. One written by a
        // build with an action this one lacks must still load.
        assert_eq!(action_named("SummonKraken"), None);
        let recipe = BindingRecipe::preset(PresetId::ArrowsZxc)
            .with_overrides(vec![BindingOverride::key("SummonKraken", KeyCode::F13)]);
        assert_eq!(
            recipe.build(),
            BindingRecipe::preset(PresetId::ArrowsZxc).build(),
            "an unknown action leaves the map exactly as the preset built it"
        );
    }

    #[test]
    fn an_override_moves_the_key_and_leaves_the_pad_alone() {
        // ⛔ the reason this is not `clear_action`: that drops an action's
        // keyboard AND gamepad bindings together, so remapping Jump on a
        // keyboard would silently unbind the controller — and the player would
        // find out mid-jump.
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
        // The property that makes this a RECIPE and not a stored map: changing
        // preset after a remap moves every action EXCEPT the remapped one.
        // Storing the rebuilt map instead would freeze a player's controls at
        // whatever preset they were on when they first touched a binding.
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
        // ⚠ inserting a buttonlike into a dual-axis action's map corrupts it in
        // a way nothing downstream checks: the action reads as bound, and the
        // stick reads as dead.
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

    #[test]
    fn a_recipe_change_that_is_only_an_override_still_rebuilds_the_map() {
        // The wiring claim: an override reaches BEHAVIOUR through the same
        // rebuild a preset change proved, so nothing new has to remember to
        // re-apply it.
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
