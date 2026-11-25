//! Contextual interpretation of button bindings. The InputMap is the source of
//! bindings; these rules give each physical button one menu or session intent.

use std::collections::{BTreeMap, HashMap};

use bevy::prelude::*;
use leafwing_input_manager::prelude::{ActionState, Buttonlike, InputMap};
use leafwing_input_manager::user_input::updating::CentralInputStore;

use crate::{ActionBindings, InputContextId, MenuControlFrame, ParticipantContexts,
    PhysicalControl, InputAction as A};

/// Interaction policy, separate from the identity of the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputProfile {
    Gameplay,
    SessionUi,
    Paused,
    Frontend,
    Blocked,
}

/// Use the claim owner to select the policy. The owner also prevents a held
/// button from transferring to a different surface with the same policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonRoute {
    pub profile: InputProfile,
    pub owner: Option<InputContextId>,
}

impl ButtonRoute {
    pub fn from_contexts(contexts: Option<&ParticipantContexts>) -> Self {
        use crate::{DEBUG_CONTEXT, GAMEPLAY_CONTEXT, INVENTORY_CONTEXT, LAUNCHER_CONTEXT,
            LOADING_CONTEXT, PAUSE_CONTEXT, SELECT_CONTEXT, STARTUP_ACKNOWLEDGE_CONTEXT};
        let owner = contexts.and_then(|c| c.resolved().first().copied());
        let session = contexts.is_some_and(|c| c.is_declared(GAMEPLAY_CONTEXT));
        let profile = match owner {
            Some(GAMEPLAY_CONTEXT) => InputProfile::Gameplay,
            Some(PAUSE_CONTEXT | INVENTORY_CONTEXT) => InputProfile::Paused,
            Some(DEBUG_CONTEXT | LOADING_CONTEXT) => InputProfile::Blocked,
            Some(LAUNCHER_CONTEXT | STARTUP_ACKNOWLEDGE_CONTEXT) => InputProfile::Frontend,
            Some(SELECT_CONTEXT) if !session => InputProfile::Frontend,
            Some(_) if session => InputProfile::SessionUi,
            Some(_) => InputProfile::Frontend,
            None => InputProfile::Blocked,
        };
        Self { profile, owner }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuButtonIntent {
    Pause,
    Confirm,
    Back,
    Inventory,
    Map,
}

impl MenuButtonIntent {
    pub fn action(self) -> A {
        match self {
            Self::Pause => A::Start,
            Self::Confirm => A::MenuSelect,
            Self::Back => A::MenuBack,
            Self::Inventory => A::Inventory,
            Self::Map => A::Map,
        }
    }
}

/// Pages that the active game lets a seat open with a shortcut.
#[derive(Resource, Clone, Copy, Default)]
pub struct MenuShortcutPages {
    pub inventory: bool,
    pub map: bool,
}

impl MenuShortcutPages {
    pub fn allows(self, intent: MenuButtonIntent) -> bool {
        match intent {
            MenuButtonIntent::Inventory => self.inventory,
            MenuButtonIntent::Map => self.map,
            _ => true,
        }
    }
}

/// Resolve one physical button's complete set of declared actions.
///
/// Start+Confirm means Enter: pause in gameplay, confirm in interfaces.
/// Start+Back means Escape: pause in a session, back in pause or frontend UI.
/// Start alone means a dedicated pause control. Back alone never opens pause.
/// Reset is a gameplay action; a touch position can also bind Back in UI.
/// Session shortcuts take priority over navigation on the same control.
pub fn resolve_menu_button(profile: InputProfile, actions: &[A]) -> Option<MenuButtonIntent> {
    use InputProfile::*;
    use MenuButtonIntent::*;
    let has = |a| actions.contains(&a);
    if profile == Blocked { return None; }
    if has(A::Reset) {
        return (profile != Gameplay && has(A::MenuBack)).then_some(Back);
    }
    if has(A::Start) {
        if has(A::MenuBack) {
            return Some(match profile {
                Gameplay | SessionUi => Pause,
                Paused | Frontend => Back,
                Blocked => unreachable!(),
            });
        }
        if has(A::MenuSelect) {
            return Some(if profile == Gameplay { Pause } else { Confirm });
        }
        return Some(Pause);
    }
    if profile != Frontend {
        if has(A::Inventory) { return Some(Inventory); }
        if has(A::Map) { return Some(Map); }
    }
    if profile != Gameplay {
        if has(A::MenuBack) { return Some(Back); }
        if has(A::MenuSelect) { return Some(Confirm); }
    }
    None
}

/// Static button census from the same map that feeds the runtime. The row
/// retains every declared action, including gameplay actions and remaps.
pub fn contextual_button_rows(bindings: &ActionBindings, profile: InputProfile)
    -> Vec<(PhysicalControl, Vec<A>, Option<MenuButtonIntent>)>
{
    let mut rows = BTreeMap::<PhysicalControl, Vec<A>>::new();
    for (name, controls) in bindings.all() {
        let Some(action) = crate::action_named(name) else { continue; };
        for control in controls {
            let actions = rows.entry(control.clone()).or_default();
            if !actions.contains(&action) { actions.push(action); }
        }
    }
    rows.into_iter().map(|(control, actions)| {
        let intent = resolve_menu_button(profile, &actions);
        (control, actions, intent)
    }).collect()
}

/// Physical held state, not action held state. Two buttons with the same
/// action must not hide each other's press edges. Track a button even when its
/// current context does not use it.
#[derive(Default)]
pub struct ContextualButtonHistory {
    held: HashMap<Box<dyn Buttonlike>, Option<(ButtonRoute, MenuButtonIntent)>>,
    pad: Option<Entity>,
    initialized: bool,
}

impl ContextualButtonHistory {
    /// Read the Leafwing input cache, including custom virtual buttons. Map
    /// changes or focus loss inhibit a new edge until the button is released.
    pub fn sample(&mut self, map: &InputMap<A>, actions: &ActionState<A>, store: &CentralInputStore,
        pad: Entity, route: ButtonRoute, map_changed: bool, enabled: bool, pages: MenuShortcutPages,
        frame: &mut MenuControlFrame)
    {
        let mut rows = HashMap::<&Box<dyn Buttonlike>, Vec<A>>::new();
        for (action, inputs) in map.iter_buttonlike() {
            for input in inputs {
                if input.pressed(store, pad) {
                    rows.entry(input).or_default().push(*action);
                }
            }
        }
        let resync = self.initialized && (map_changed || self.pad != Some(pad));
        self.initialized = true;
        self.pad = Some(pad);
        self.apply(rows, route, enabled && !actions.disabled(), resync, frame,
            |intent| pages.allows(intent) && !actions.action_disabled(&intent.action()));
    }

    fn apply(&mut self, rows: HashMap<&Box<dyn Buttonlike>, Vec<A>>,
        route: ButtonRoute, enabled: bool, resync: bool, frame: &mut MenuControlFrame,
        action_enabled: impl Fn(MenuButtonIntent) -> bool)
    {
        frame.start = false;
        frame.select = false;
        frame.select_held = false;
        frame.back = false;
        frame.back_held = false;
        frame.inventory = false;
        frame.map = false;
        if !enabled || matches!(route.profile, InputProfile::Gameplay | InputProfile::Blocked) {
            frame.consume_nav_edges();
            frame.analog = Vec2::ZERO;
        }
        let mut held = HashMap::new();
        for (control, actions) in rows {
            let intent = resolve_menu_button(route.profile, &actions).filter(|intent| action_enabled(*intent));
            let previous = self.held.get(control.as_ref());
            let edge = enabled && !resync && previous.is_none();
            let accepted = if edge {
                intent.map(|intent| (route, intent))
            } else if enabled && !resync {
                previous.copied().flatten().filter(|(old_route, old_intent)|
                    *old_route == route && Some(*old_intent) == intent)
            } else { None };
            held.insert(control.clone(), accepted);
            if let Some((_, intent)) = accepted {
                match intent {
                    MenuButtonIntent::Pause => frame.start |= edge,
                    MenuButtonIntent::Confirm => { frame.select |= edge; frame.select_held = true; }
                    MenuButtonIntent::Back => { frame.back |= edge; frame.back_held = true; }
                    MenuButtonIntent::Inventory => frame.inventory |= edge,
                    MenuButtonIntent::Map => frame.map |= edge,
                }
            }
        }
        self.held = held;
        // An open/close or page shortcut takes the frame before navigation.
        // This also covers simultaneous presses of different physical buttons.
        if frame.start || frame.inventory || frame.map {
            frame.consume_nav_edges();
            frame.select_held = false;
            frame.back_held = false;
        }
        if !enabled { *frame = MenuControlFrame::default(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BindingLayout, BindingRecipe, KeyboardPreset};

    #[test]
    fn common_controls_have_one_contextual_meaning_under_every_layout() {
        use InputProfile::*;
        use MenuButtonIntent::*;
        for preset in KeyboardPreset::presets() {
            for layout in [BindingLayout::Standard, BindingLayout::Smash] {
                let map = BindingRecipe::preset(preset.id).with_layout(layout).build();
                let bindings = ActionBindings::from_map(&map);
                for (control, expected) in [
                    (PhysicalControl::Key(KeyCode::Enter), [Some(Pause), Some(Confirm), Some(Confirm)]),
                    (PhysicalControl::Key(KeyCode::Escape), [Some(Pause), Some(Pause), Some(Back)]),
                    (PhysicalControl::Button(GamepadButton::Start), [Some(Pause); 3]),
                    (PhysicalControl::Button(GamepadButton::South), [None, Some(Confirm), Some(Confirm)]),
                    (PhysicalControl::Button(GamepadButton::East), [None, Some(Back), Some(Back)]),
                    (PhysicalControl::Button(GamepadButton::Select), [None; 3]),
                    (PhysicalControl::Key(KeyCode::Backspace), [None, Some(Back), Some(Back)]),
                    (PhysicalControl::Key(KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Android(4))), [None, Some(Back), Some(Back)]),
                ] {
                    for (profile, wanted) in [Gameplay, SessionUi, Paused].into_iter().zip(expected) {
                        let rows = contextual_button_rows(&bindings, profile);
                        assert_eq!(rows.iter().find(|row| row.0 == control).unwrap().2, wanted,
                            "{control:?} {profile:?} {layout:?}");
                    }
                }
                assert!(bindings.controls(&A::Reset).contains(&PhysicalControl::Button(GamepadButton::Select)));
                assert!(!bindings.controls(&A::Modifier).contains(&PhysicalControl::Button(GamepadButton::RightThumb)));
            }
        }
    }

    #[test]
    fn touch_reset_is_back_only_in_ui_and_never_pause() {
        use InputProfile::*;
        use MenuButtonIntent::*;
        let reset = [A::Reset, A::MenuBack];
        assert_eq!(resolve_menu_button(Gameplay, &reset), None);
        assert_eq!(resolve_menu_button(SessionUi, &reset), Some(Back));
        assert_eq!(resolve_menu_button(Paused, &reset), Some(Back));
        assert_eq!(resolve_menu_button(Blocked, &reset), None);
        assert_eq!(resolve_menu_button(Gameplay, &[A::Reset, A::Start]), None);
        assert_eq!(resolve_menu_button(SessionUi, &[A::Reset, A::Start]), None);
    }

    #[test]
    fn a_held_enter_does_not_confirm_after_it_opens_pause() {
        let mut history = ContextualButtonHistory::default();
        let mut frame = MenuControlFrame::default();
        let enter: Box<dyn Buttonlike> = Box::new(KeyCode::Enter);
        let rows = || HashMap::from([(&enter, vec![A::Start, A::MenuSelect])]);
        let route = |profile| ButtonRoute { profile, owner: None };
        history.apply(rows(), route(InputProfile::Gameplay), true, false, &mut frame, |_| true);
        assert!(frame.start && !frame.select);
        history.apply(rows(), route(InputProfile::Paused), true, false, &mut frame, |_| true);
        assert!(!frame.start && !frame.select && !frame.select_held);
        history.apply(HashMap::new(), route(InputProfile::Paused), true, false, &mut frame, |_| true);
        history.apply(rows(), route(InputProfile::Paused), true, false, &mut frame, |_| true);
        assert!(frame.select && !frame.start);
    }

    #[test]
    fn confirm_buttons_keep_separate_edges_and_held_buttons_resync() {
        let enter: Box<dyn Buttonlike> = Box::new(KeyCode::Enter);
        let space: Box<dyn Buttonlike> = Box::new(KeyCode::Space);
        let rows = || HashMap::from([(&enter, vec![A::Start, A::MenuSelect]), (&space, vec![A::MenuSelect])]);
        let route = ButtonRoute { profile: InputProfile::SessionUi, owner: None };
        let mut history = ContextualButtonHistory::default();
        let mut frame = MenuControlFrame::default();
        history.apply(HashMap::from([(&enter, vec![A::Start, A::MenuSelect])]), route, true, false, &mut frame, |_| true);
        assert!(frame.select);
        history.apply(rows(), route, true, false, &mut frame, |_| true);
        assert!(frame.select);
        history.apply(rows(), route, false, false, &mut frame, |_| true);
        history.apply(rows(), route, true, false, &mut frame, |_| true);
        assert!(!frame.select && !frame.select_held);
        history.apply(HashMap::new(), route, true, false, &mut frame, |_| true);
        history.apply(rows(), route, true, true, &mut frame, |_| true);
        assert!(!frame.select && !frame.select_held);
    }
}
