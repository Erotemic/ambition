//! Contextual button sampling and physical press history.

use std::collections::{BTreeMap, HashMap};

use bevy::prelude::{Entity, Resource, Vec2};
use leafwing_input_manager::prelude::{ActionState, Buttonlike};
use leafwing_input_manager::user_input::updating::CentralInputStore;

use crate::{BindingTable, InputContextId, MenuControlFrame, ParticipantContexts,
    PhysicalControl, InputAction as A};
use crate::bindings::physical_control_of;

/// Interaction policy, separate from the identity of the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InputProfile {
    Gameplay,
    SessionUi,
    Paused,
    Frontend,
    Blocked,
}

impl InputProfile {
    pub const ACTIVE: [Self; 4] = [Self::Gameplay, Self::SessionUi, Self::Paused, Self::Frontend];

    /// Whether this profile has a consumer for the action. This does not bind any device.
    pub fn accepts(self, action: A) -> bool {
        match self {
            Self::Gameplay => !action.is_menu_only(),
            Self::SessionUi | Self::Paused => action.is_menu_only() || matches!(action, A::Start | A::Inventory | A::Map),
            Self::Frontend => action.is_menu_only() || action == A::Start,
            Self::Blocked => false,
        }
    }
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
    Up,
    Down,
    Left,
    Right,
    PageLeft,
    PageRight,
}

impl MenuButtonIntent {
    pub fn action(self) -> A {
        match self {
            Self::Pause => A::Start,
            Self::Confirm => A::MenuSelect,
            Self::Back => A::MenuBack,
            Self::Inventory => A::Inventory,
            Self::Map => A::Map,
            Self::Up => A::MenuNavigateUp,
            Self::Down => A::MenuNavigateDown,
            Self::Left => A::MenuNavigateLeft,
            Self::Right => A::MenuNavigateRight,
            Self::PageLeft => A::MenuPageLeft,
            Self::PageRight => A::MenuPageRight,
        }
    }

    pub fn from_action(action: A) -> Option<Self> {
        Some(match action {
            A::Start => Self::Pause,
            A::MenuSelect => Self::Confirm,
            A::MenuBack => Self::Back,
            A::Inventory => Self::Inventory,
            A::Map => Self::Map,
            A::MenuNavigateUp => Self::Up,
            A::MenuNavigateDown => Self::Down,
            A::MenuNavigateLeft => Self::Left,
            A::MenuNavigateRight => Self::Right,
            A::MenuPageLeft => Self::PageLeft,
            A::MenuPageRight => Self::PageRight,
            _ => return None,
        })
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

/// Print the actual effective rows, including unselected contexts and game overrides.
pub fn contextual_button_rows(table: &BindingTable, profile: InputProfile)
    -> Vec<(PhysicalControl, Option<A>)>
{
    let mut rows = BTreeMap::new();
    for row in table.rows() {
        let control = physical_control_of(row.control.as_ref());
        let action = rows.entry(control).or_insert(None);
        if row.profile == profile { *action = Some(row.action); }
    }
    rows.into_iter().collect()
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
    pub fn sample(&mut self, table: &BindingTable, actions: &ActionState<A>, store: &CentralInputStore,
        pad: Entity, route: ButtonRoute, map_changed: bool, enabled: bool, pages: MenuShortcutPages,
        frame: &mut MenuControlFrame)
    {
        let mut rows = HashMap::<&Box<dyn Buttonlike>, Option<MenuButtonIntent>>::new();
        for row in table.rows() {
            if row.control.pressed(store, pad) {
                let intent = rows.entry(&row.control).or_insert(None);
                if row.profile == route.profile {
                    *intent = MenuButtonIntent::from_action(row.action);
                }
            }
        }
        let resync = self.initialized && (map_changed || self.pad != Some(pad));
        self.initialized = true;
        self.pad = Some(pad);
        self.apply(rows, route, enabled && !actions.disabled(), resync, frame,
            |intent| pages.allows(intent) && !actions.action_disabled(&intent.action()));
    }

    fn apply(&mut self, rows: HashMap<&Box<dyn Buttonlike>, Option<MenuButtonIntent>>,
        route: ButtonRoute, enabled: bool, resync: bool, frame: &mut MenuControlFrame,
        action_enabled: impl Fn(MenuButtonIntent) -> bool)
    {
        frame.up = false;
        frame.down = false;
        frame.left = false;
        frame.right = false;
        frame.page_left = false;
        frame.page_right = false;
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
        for (control, intent) in rows {
            let intent = intent.filter(|intent| action_enabled(*intent));
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
                    MenuButtonIntent::Up => frame.up |= edge,
                    MenuButtonIntent::Down => frame.down |= edge,
                    MenuButtonIntent::Left => frame.left |= edge,
                    MenuButtonIntent::Right => frame.right |= edge,
                    MenuButtonIntent::PageLeft => frame.page_left |= edge,
                    MenuButtonIntent::PageRight => frame.page_right |= edge,
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
    use super::{ButtonRoute, ContextualButtonHistory, InputProfile, MenuButtonIntent};
    use crate::{BindingTable, InputAction as Action, MenuControlFrame, PhysicalControl};
    use bevy::prelude::KeyCode;
    use leafwing_input_manager::prelude::Buttonlike;
    use std::collections::HashMap;

    #[test]
    fn two_profiles_can_assign_different_actions_to_one_control() {
        let mut table = BindingTable::default();
        table.bind_button(InputProfile::Gameplay, KeyCode::Enter, Action::Start);
        table.bind_button(InputProfile::Paused, KeyCode::Enter, Action::MenuSelect);
        let enter = PhysicalControl::Key(KeyCode::Enter);
        assert_eq!(table.action_for(InputProfile::Gameplay, &enter), Some(Action::Start));
        assert_eq!(table.action_for(InputProfile::Paused, &enter), Some(Action::MenuSelect));
        assert_eq!(table.action_for(InputProfile::SessionUi, &enter), None);
    }

    #[test]
    fn a_held_enter_does_not_confirm_after_it_opens_pause() {
        let mut history = ContextualButtonHistory::default();
        let mut frame = MenuControlFrame::default();
        let enter: Box<dyn Buttonlike> = Box::new(KeyCode::Enter);
        let route = |profile| ButtonRoute { profile, owner: None };
        let gameplay = || HashMap::from([(&enter, Some(MenuButtonIntent::Pause))]);
        let menu = || HashMap::from([(&enter, Some(MenuButtonIntent::Confirm))]);
        history.apply(gameplay(), route(InputProfile::Gameplay), true, false, &mut frame, |_| true);
        assert!(frame.start && !frame.select);
        history.apply(menu(), route(InputProfile::Paused), true, false, &mut frame, |_| true);
        assert!(!frame.start && !frame.select && !frame.select_held);
        history.apply(HashMap::new(), route(InputProfile::Paused), true, false, &mut frame, |_| true);
        history.apply(menu(), route(InputProfile::Paused), true, false, &mut frame, |_| true);
        assert!(frame.select && !frame.start);
    }

    #[test]
    fn confirm_buttons_keep_separate_edges_and_held_buttons_resync() {
        let enter: Box<dyn Buttonlike> = Box::new(KeyCode::Enter);
        let space: Box<dyn Buttonlike> = Box::new(KeyCode::Space);
        let rows = || HashMap::from([(&enter, Some(MenuButtonIntent::Confirm)), (&space, Some(MenuButtonIntent::Confirm))]);
        let route = ButtonRoute { profile: InputProfile::SessionUi, owner: None };
        let mut history = ContextualButtonHistory::default();
        let mut frame = MenuControlFrame::default();
        history.apply(HashMap::from([(&enter, Some(MenuButtonIntent::Confirm))]), route, true, false, &mut frame, |_| true);
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
