//! Open semantic action vocabulary between physical bindings and consumers.
//!
//! [`SemanticActionId`] lets capabilities register actions without extending
//! the closed leafwing device-action enum. [`ActionRegistry`] owns each id and
//! its control kind, and mints the [`ProviderAction`] key that binds it. A
//! registered action is thus describable and bindable without a change to the
//! device enum.
//!
//! Not yet routed: nothing installs an `InputMap<ProviderAction>` in
//! production. Two maps need two reader paths and a rule for which one wins a
//! conflict.

use std::collections::BTreeMap;

use crate::participant::{GAMEPLAY_CONTEXT, INVENTORY_CONTEXT, LAUNCHER_CONTEXT, SELECT_CONTEXT};
use crate::InputContextId;

/// An action's stable identity. Open: a capability mints its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticActionId(pub &'static str);

impl std::fmt::Display for SemanticActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// The shape of input an action carries. Mirrors leafwing's control kinds, so
/// a binding UI or a prompt knows whether it draws a button or a stick.
///
/// `Hash` and `Reflect` are for [`ProviderAction`], which carries this as a
/// field because leafwing's `input_control_kind` takes `&self`. They cost
/// nothing on a fieldless enum, and the provider key needs no second copy of
/// this vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, bevy::prelude::Reflect)]
pub enum ActionControlKind {
    Button,
    Axis,
    DualAxis,
}

/// One registered action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticActionDef {
    pub id: SemanticActionId,
    /// Which capability owns it. `"engine"` for the built-in vocabulary; a
    /// capability's own name for anything it adds.
    pub capability: &'static str,
    pub kind: ActionControlKind,
    /// The contexts it is meaningful in. A prompt asks this to decide what to
    /// show; a router asks it to decide whether a press means anything here.
    pub contexts: &'static [InputContextId],
    /// One line, for a help screen or a rebind UI. Keep it with the
    /// registration.
    pub doc: &'static str,
}

/// Every action a composition understands.
#[derive(Clone, Debug, Default)]
pub struct ActionRegistry {
    actions: BTreeMap<SemanticActionId, SemanticActionDef>,
}

/// Registering the same id twice, with the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionConflict {
    pub id: SemanticActionId,
    pub first_owner: &'static str,
    pub second_owner: &'static str,
}

impl std::fmt::Display for ActionConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "action `{}` is claimed by capability `{}` and capability `{}`",
            self.id, self.first_owner, self.second_owner
        )
    }
}

impl ActionRegistry {
    /// A registry with the engine's own vocabulary installed.
    pub fn with_engine_actions() -> Self {
        let mut registry = Self::default();
        for def in ENGINE_ACTIONS {
            registry
                .register(def.clone())
                .expect("the engine's own action table has no duplicates");
        }
        registry
    }

    /// Install one action.
    ///
    /// Refuse two owners for one id here, not at use. Otherwise map iteration
    /// order would decide the winner.
    pub fn register(&mut self, def: SemanticActionDef) -> Result<(), ActionConflict> {
        if let Some(existing) = self.actions.get(&def.id) {
            return Err(ActionConflict {
                id: def.id,
                first_owner: existing.capability,
                second_owner: def.capability,
            });
        }
        self.actions.insert(def.id, def);
        Ok(())
    }

    pub fn get(&self, id: SemanticActionId) -> Option<&SemanticActionDef> {
        self.actions.get(&id)
    }

    /// Every action, in canonical order.
    pub fn all(&self) -> impl Iterator<Item = &SemanticActionDef> {
        self.actions.values()
    }

    /// The actions that can be pressed in this context. Prompts, help screens and
    /// rebind UIs all use this.
    pub fn for_context(
        &self,
        context: InputContextId,
    ) -> impl Iterator<Item = &SemanticActionDef> + '_ {
        self.actions
            .values()
            .filter(move |def| def.contexts.contains(&context))
    }

    /// Every action a capability owns — what a game gets by installing it.
    pub fn owned_by<'a>(
        &'a self,
        capability: &'a str,
    ) -> impl Iterator<Item = &'a SemanticActionDef> + 'a {
        self.actions
            .values()
            .filter(move |def| def.capability == capability)
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Mint the leafwing key for a registered action. This is the only way to get
    /// one.
    ///
    /// The registry mints it because the registry owns the kind. `ProviderAction`
    /// hashes on id and kind, so two hand-built keys for one action could disagree
    /// on shape and miss each other in an `InputMap`. [`ActionRegistry::register`]
    /// enforces one kind per id; this extends that rule to the bindings.
    ///
    /// `None` for an unregistered id: a key for an undeclared action binds to
    /// nothing.
    #[cfg(feature = "input")]
    pub fn key(&self, id: SemanticActionId) -> Option<ProviderAction> {
        self.get(id).map(|def| ProviderAction {
            id: def.id.0.to_string(),
            kind: def.kind,
        })
    }
}

/// A registered action as a leafwing key: the keyspace of the second map.
///
/// It is not an enum. `InputMap<A: Actionlike>` is already generic, so a
/// composition installs one of these beside the engine's map. A capability can
/// then bind an action the engine does not know, with no `Any`, no `TypeId`,
/// no service locator, and no change to the device enum.
///
/// Mint it with [`ActionRegistry::key`], not by hand. The kind is part of the
/// hash, so a hand-built key with the wrong kind binds into a slot nothing
/// reads.
///
/// Not yet routed in production: two maps need two reader paths and a rule for
/// which one wins a conflict. See
/// `docs/planning/engine/participant-action-system.md`.
#[cfg(feature = "input")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, bevy::prelude::Reflect)]
pub struct ProviderAction {
    /// The registered [`SemanticActionId`]. Owned, because a key must outlive the
    /// registration's `&'static str` in a reflected, serialisable map.
    pub id: String,
    /// Carried, not looked up: leafwing's `input_control_kind` takes `&self`, so
    /// the key must know its shape with no registry.
    pub kind: ActionControlKind,
}

#[cfg(feature = "input")]
impl leafwing_input_manager::Actionlike for ProviderAction {
    fn input_control_kind(&self) -> leafwing_input_manager::InputControlKind {
        match self.kind {
            ActionControlKind::Button => leafwing_input_manager::InputControlKind::Button,
            ActionControlKind::Axis => leafwing_input_manager::InputControlKind::Axis,
            ActionControlKind::DualAxis => leafwing_input_manager::InputControlKind::DualAxis,
        }
    }
}

/// The physical bindings a composition gives its provider actions.
///
/// Kept separate from [`ActionRegistry`]. The registry describes an action
/// (id, kind, contexts, doc), which a capability can do alone. The key it sits
/// on is the composition's choice, the same split the engine vocabulary has
/// between the registry and `BindingRecipe`.
///
/// Empty by default: a composition that binds nothing routes nothing, and each
/// action stays reachable by writing its message directly.
#[cfg(feature = "input")]
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct ProviderBindings(pub leafwing_input_manager::prelude::InputMap<ProviderAction>);

/// A registered provider action was pressed by a seat this frame.
///
/// An edge, not a level: `just_pressed`, so a held key gives one message. The
/// menu and pointer channels use the same rule.
///
/// The capability that registered the action reads this and writes its own
/// request message. So `ambition_input` does not know what the action does,
/// and the capability does not know what a keyboard is.
#[cfg(feature = "input")]
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticActionPressed {
    pub id: SemanticActionId,
    pub participant: crate::participant::ParticipantId,
}

/// Give every seat the composition's provider map, and keep it current.
///
/// A sync, not a spawn edit: a capability can be installed after the seats
/// exist, and a binding change must reach existing seats. The two spawn sites
/// in `input_systems.rs` build from the engine's `BindingRecipe`, which does
/// not know provider actions, and should stay that way.
#[cfg(feature = "input")]
pub fn install_provider_bindings_on_seats(
    mut commands: bevy::prelude::Commands,
    bindings: Option<bevy::prelude::Res<ProviderBindings>>,
    seats: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            Option<&leafwing_input_manager::prelude::InputMap<ProviderAction>>,
        ),
        bevy::prelude::With<crate::participant::InputParticipant>,
    >,
) {
    use bevy::prelude::DetectChanges;
    let Some(bindings) = bindings.as_ref() else {
        return;
    };
    if !bindings.is_changed() && seats.iter().all(|(_, map)| map.is_some()) {
        return;
    }
    for (seat, _) in &seats {
        commands.entity(seat).insert((
            bindings.0.clone(),
            leafwing_input_manager::prelude::ActionState::<ProviderAction>::default(),
        ));
    }
}

/// Publish this frame's provider-action presses as semantic edges.
///
/// The registry is the filter, so the message id is a `&'static str` and not
/// the key's `String`. A key whose id no longer resolves is an action the
/// composition no longer installs; do not route it.
#[cfg(feature = "input")]
pub fn publish_provider_action_edges(
    registry: Option<bevy::prelude::Res<InstalledActions>>,
    // Check the seat's context, as `SemanticActionDef::contexts` requires.
    // Without this check, a capability's gameplay action fires in the pause
    // menu, dialogue, the launcher or a cutscene, unlike every built-in control.
    //
    // Not `Option`: the input plugin inits this resource. If it is absent, the
    // composition must fail at assembly, not drop every provider edge silently.
    contexts: bevy::prelude::Res<crate::participant::SeatInputContexts>,
    seats: bevy::prelude::Query<(
        &crate::participant::InputParticipant,
        &leafwing_input_manager::prelude::ActionState<ProviderAction>,
    )>,
    mut pressed: bevy::prelude::MessageWriter<SemanticActionPressed>,
) {
    let Some(registry) = registry else {
        return;
    };
    // Sort by seat id, not query order. Two seats that press one action on the
    // same frame must publish in the same order every run, and Bevy's iteration
    // order is not stable.
    let mut rows: Vec<_> = seats.iter().collect();
    rows.sort_by_key(|(participant, _)| participant.id.0);
    for (participant, actions) in rows {
        // An unresolved seat (a couch slot with no pad) owns no context. That is the
        // same answer as "captured by a menu": no permission, not an error.
        let seat = contexts.for_seat(participant.id.slot());
        for key in actions.get_just_pressed() {
            let Some(def) = registry
                .all()
                .find(|def| def.id.0 == key.id && def.kind == key.kind)
            else {
                continue;
            };
            if !def.contexts.iter().any(|id| seat.allows(*id)) {
                continue;
            }
            pressed.write(SemanticActionPressed {
                id: def.id,
                participant: participant.id,
            });
        }
    }
}

/// The composition's action vocabulary, as a resource.
///
/// The running app's registry. The facade's assembly pass builds it from the
/// engine actions plus the actions the mounted modules declare, so prompts,
/// help screens and rebind UIs get game and engine actions from one place.
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct InstalledActions(pub ActionRegistry);

impl std::ops::Deref for InstalledActions {
    type Target = ActionRegistry;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The capability that owns the built-in vocabulary.
pub const ENGINE_CAPABILITY: &str = "engine";

const GAMEPLAY: &[InputContextId] = &[GAMEPLAY_CONTEXT];
const MENUS: &[InputContextId] = &[
    LAUNCHER_CONTEXT,
    SELECT_CONTEXT,
    INVENTORY_CONTEXT,
    crate::participant::DIALOGUE_CONTEXT,
];

const fn engine(
    id: &'static str,
    kind: ActionControlKind,
    contexts: &'static [InputContextId],
    doc: &'static str,
) -> SemanticActionDef {
    SemanticActionDef {
        id: SemanticActionId(id),
        capability: ENGINE_CAPABILITY,
        kind,
        contexts,
        doc,
    }
}

/// The engine's vocabulary, and the whole of it.
///
/// One entry per `Platformer2dInputActionMonolith` variant.
/// `every_device_action_is_registered` fails when a variant has no entry.
pub static ENGINE_ACTIONS: &[SemanticActionDef] = &[
    engine(
        "move",
        ActionControlKind::DualAxis,
        GAMEPLAY,
        "Walk / run / aim the body",
    ),
    engine(
        "move_left",
        ActionControlKind::Button,
        GAMEPLAY,
        "Walk left (edge-detectable)",
    ),
    engine(
        "move_right",
        ActionControlKind::Button,
        GAMEPLAY,
        "Walk right (edge-detectable)",
    ),
    engine(
        "move_up",
        ActionControlKind::Button,
        GAMEPLAY,
        "Up (doors, ladders, aim)",
    ),
    engine(
        "move_down",
        ActionControlKind::Button,
        GAMEPLAY,
        "Down (crouch, fast fall)",
    ),
    engine("jump", ActionControlKind::Button, GAMEPLAY, "Jump"),
    engine(
        "attack",
        ActionControlKind::Button,
        GAMEPLAY,
        "Primary melee",
    ),
    engine(
        "strong_attack",
        ActionControlKind::Button,
        GAMEPLAY,
        "Strong-attack hint; the sim classifies tilt vs smash",
    ),
    engine(
        "burst",
        ActionControlKind::Button,
        GAMEPLAY,
        "Dodge / dash — the one shared burst press",
    ),
    engine(
        "blink",
        ActionControlKind::Button,
        GAMEPLAY,
        "Blink / teleport",
    ),
    engine(
        "special",
        ActionControlKind::Button,
        GAMEPLAY,
        "Signature special",
    ),
    engine(
        "shield",
        ActionControlKind::Button,
        GAMEPLAY,
        "Hold to raise a guard, release to drop it",
    ),
    engine(
        "grab",
        ActionControlKind::Button,
        GAMEPLAY,
        "Press to catch hold of another body",
    ),
    engine(
        "taunt",
        ActionControlKind::Button,
        GAMEPLAY,
        "Press to taunt; it costs you your footing and buys nothing",
    ),
    engine(
        "interact",
        ActionControlKind::Button,
        GAMEPLAY,
        "Talk, open, use",
    ),
    engine(
        "walk",
        ActionControlKind::Button,
        GAMEPLAY,
        "Hold to walk instead of run",
    ),
    engine(
        "modifier",
        ActionControlKind::Button,
        GAMEPLAY,
        "Sustained modifier; content decides what holding it means",
    ),
    engine(
        "utility",
        ActionControlKind::Button,
        GAMEPLAY,
        "Fly / form toggle",
    ),
    engine("map", ActionControlKind::Button, GAMEPLAY, "Open the map"),
    engine(
        "inventory",
        ActionControlKind::Button,
        GAMEPLAY,
        "Open the inventory",
    ),
    engine(
        "pogo",
        ActionControlKind::Button,
        GAMEPLAY,
        "Pogo (down + attack on presets without a dedicated key)",
    ),
    engine(
        "reset",
        ActionControlKind::Button,
        GAMEPLAY,
        "Restart / soft reset",
    ),
    engine(
        "start",
        ActionControlKind::Button,
        GAMEPLAY,
        "Pause — the shell verb",
    ),
    engine(
        "projectile",
        ActionControlKind::Button,
        GAMEPLAY,
        "Fire a projectile",
    ),
    engine(
        "trail_toggle",
        ActionControlKind::Button,
        GAMEPLAY,
        "Toggle the trail drawing mode",
    ),
    engine(
        "menu_navigate_up",
        ActionControlKind::Button,
        MENUS,
        "Menu: up",
    ),
    engine(
        "menu_navigate_down",
        ActionControlKind::Button,
        MENUS,
        "Menu: down",
    ),
    engine(
        "menu_navigate_left",
        ActionControlKind::Button,
        MENUS,
        "Menu: left",
    ),
    engine(
        "menu_navigate_right",
        ActionControlKind::Button,
        MENUS,
        "Menu: right",
    ),
    engine(
        "menu_select",
        ActionControlKind::Button,
        MENUS,
        "Menu: confirm",
    ),
    engine("menu_back", ActionControlKind::Button, MENUS, "Menu: back"),
    engine(
        "menu_page_left",
        ActionControlKind::Button,
        MENUS,
        "Paged menu: previous page",
    ),
    engine(
        "menu_page_right",
        ActionControlKind::Button,
        MENUS,
        "Paged menu: next page",
    ),
    engine(
        "menu_stick",
        ActionControlKind::DualAxis,
        MENUS,
        "Menu navigation stick",
    ),
    engine(
        "burst_analog",
        ActionControlKind::Axis,
        GAMEPLAY,
        "Analog burst trigger",
    ),
    engine(
        "aim_stick",
        ActionControlKind::DualAxis,
        GAMEPLAY,
        "Aim / blink steer",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engine_vocabulary_installs_and_is_queryable_by_context() {
        let registry = ActionRegistry::with_engine_actions();
        assert_eq!(registry.len(), ENGINE_ACTIONS.len());

        let gameplay: Vec<_> = registry
            .for_context(GAMEPLAY_CONTEXT)
            .map(|def| def.id.0)
            .collect();
        assert!(gameplay.contains(&"jump") && gameplay.contains(&"attack"));
        assert!(
            !gameplay.contains(&"menu_select"),
            "a menu verb is not offered in gameplay — that is what `contexts` is for"
        );

        let menu: Vec<_> = registry
            .for_context(LAUNCHER_CONTEXT)
            .map(|def| def.id.0)
            .collect();
        assert!(menu.contains(&"menu_select") && !menu.contains(&"jump"));
    }

    /// A capability adds an action without editing the engine.
    ///
    /// `Platformer2dInputActionMonolith` is a closed enum that a capability cannot
    /// extend; the registry is the open half.
    #[test]
    fn a_capability_registers_its_own_action_without_touching_the_engine_enum() {
        const GRAPPLE: SemanticActionDef = SemanticActionDef {
            id: SemanticActionId("grapple"),
            capability: "traversal",
            kind: ActionControlKind::Button,
            contexts: GAMEPLAY,
            doc: "Fire the grapple",
        };

        let mut registry = ActionRegistry::with_engine_actions();
        registry.register(GRAPPLE).expect("a fresh id");

        assert_eq!(
            registry
                .get(SemanticActionId("grapple"))
                .map(|d| d.capability),
            Some("traversal")
        );
        assert!(
            registry
                .for_context(GAMEPLAY_CONTEXT)
                .any(|def| def.id.0 == "grapple"),
            "and it is OFFERED where it is meaningful, beside the engine's own"
        );
        assert_eq!(registry.owned_by("traversal").count(), 1);
        assert_eq!(
            registry.owned_by(ENGINE_CAPABILITY).count(),
            ENGINE_ACTIONS.len(),
            "the engine's vocabulary is not disturbed by a capability adding to it"
        );
    }

    #[test]
    fn two_capabilities_claiming_one_action_is_refused_and_names_both() {
        let mut registry = ActionRegistry::with_engine_actions();
        let conflict = registry
            .register(SemanticActionDef {
                id: SemanticActionId("jump"),
                capability: "traversal",
                kind: ActionControlKind::Button,
                contexts: GAMEPLAY,
                doc: "a second jump",
            })
            .expect_err("`jump` is the engine's");
        assert_eq!(conflict.first_owner, ENGINE_CAPABILITY);
        assert_eq!(conflict.second_owner, "traversal");
        assert!(
            conflict.to_string().contains("engine") && conflict.to_string().contains("traversal")
        );
    }

    /// The registry must not fall behind the enum. A variant without a semantic
    /// entry would be invisible to every prompt, help screen and rebind UI.
    #[cfg(feature = "input")]
    #[test]
    fn every_device_action_is_registered() {
        use crate::Platformer2dInputActionMonolith;
        use bevy::reflect::{TypeInfo, Typed};

        // `Actionlike` has no `variants()` in leafwing 0.20, but it requires
        // `Reflect + Typed`, so the enum's type info gives a list that cannot go stale.
        let TypeInfo::Enum(info) = Platformer2dInputActionMonolith::type_info() else {
            panic!("Platformer2dInputActionMonolith is an enum");
        };
        let registry = ActionRegistry::with_engine_actions();
        let missing: Vec<String> = (0..info.variant_len())
            .filter_map(|i| info.variant_at(i))
            .map(|variant| snake_case(variant.name()))
            .filter(|name| registry.get(SemanticActionId(leak(name))).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "these `Platformer2dInputActionMonolith` variants have no semantic entry, so nothing that asks the \
             registry can see them: {missing:?}\nadd them to `ENGINE_ACTIONS`"
        );
    }

    #[cfg(feature = "input")]
    fn snake_case(camel: &str) -> String {
        let mut out = String::new();
        for (i, ch) in camel.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 {
                    out.push('_');
                }
                out.extend(ch.to_lowercase());
            } else {
                out.push(ch);
            }
        }
        out
    }

    /// The registry is keyed by `&'static str`. Leak a few short test names
    /// instead of widening the key type.
    #[cfg(feature = "input")]
    fn leak(name: &str) -> &'static str {
        Box::leak(name.to_string().into_boxed_str())
    }

    /// A registered action reaches a seat's press, end to end in one app.
    ///
    /// Register an action the engine does not know, bind it to a key, press the
    /// key, and get a semantic edge back, with no `Any`, no `TypeId`, and no new
    /// device-enum variant.
    ///
    /// The press is held across two frames: a one-frame press cannot tell an edge
    /// channel from a level channel. A held key must give one message.
    #[cfg(feature = "input")]
    #[test]
    fn a_registered_action_bound_to_a_key_comes_back_as_a_seat_press() {
        use bevy::prelude::*;
        use leafwing_input_manager::prelude::*;

        const PULSE: SemanticActionDef = SemanticActionDef {
            id: SemanticActionId("pulse"),
            capability: "pulse",
            kind: ActionControlKind::Button,
            contexts: GAMEPLAY,
            doc: "Fire a shockwave",
        };
        let mut registry = ActionRegistry::with_engine_actions();
        registry.register(PULSE).expect("a fresh id");
        let key = registry.key(PULSE.id).expect("just registered");

        let mut bindings = ProviderBindings::default();
        bindings.0.insert(key, KeyCode::KeyG);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::input::InputPlugin)
            .add_plugins(InputManagerPlugin::<ProviderAction>::default())
            .insert_resource(InstalledActions(registry))
            .insert_resource(bindings)
            .add_message::<SemanticActionPressed>()
            .add_systems(
                bevy::app::PreUpdate,
                install_provider_bindings_on_seats
                    .before(leafwing_input_manager::plugin::InputManagerSystem::Update),
            )
            .add_systems(Update, publish_provider_action_edges);
        // The seat claims gameplay through the real resolver.
        app.init_resource::<crate::participant::SeatInputContexts>()
            .add_systems(
                bevy::app::PreUpdate,
                crate::participant::resolve_active_input_context,
            );
        let mut claims = crate::participant::ParticipantContexts::default();
        claims.declare(crate::participant::ContextClaim::capturing(
            GAMEPLAY_CONTEXT,
            0,
        ));
        app.world_mut()
            .spawn((crate::participant::InputParticipant::primary(), claims));

        // A frame with nothing pressed: the seat acquires the map, and an
        // unpressed binding must not announce itself.
        app.update();
        assert!(
            drain_presses(&mut app).is_empty(),
            "a bound action nobody pressed published an edge"
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.update();
        assert_eq!(
            drain_presses(&mut app),
            vec![SemanticActionPressed {
                id: SemanticActionId("pulse"),
                participant: crate::participant::ParticipantId::PRIMARY,
            }],
            "a key bound to a provider-minted action did not reach the seat"
        );

        // Still held. Bevy's `ButtonInput` keeps it pressed across the frame, so this
        // checks edge against level.
        app.update();
        assert!(
            drain_presses(&mut app).is_empty(),
            "holding the key republished the press every frame"
        );
    }

    /// A captured seat does not fire a gameplay action.
    ///
    /// The test first proves the same press fires in gameplay, so a failure means
    /// the context refused, not that the binding is broken. It presses `KeyG` in
    /// gameplay, opens the pause context and presses again, then returns to
    /// gameplay and checks that presses resume.
    #[cfg(feature = "input")]
    #[test]
    fn a_seat_captured_by_a_menu_publishes_no_gameplay_edge() {
        use crate::participant::PAUSE_CONTEXT;
        use bevy::prelude::*;
        use leafwing_input_manager::prelude::*;

        const PULSE: SemanticActionDef = SemanticActionDef {
            id: SemanticActionId("pulse"),
            capability: "pulse",
            kind: ActionControlKind::Button,
            contexts: GAMEPLAY,
            doc: "Fire a shockwave",
        };
        let mut registry = ActionRegistry::with_engine_actions();
        registry.register(PULSE).expect("a fresh id");
        let key = registry.key(PULSE.id).expect("just registered");
        let mut bindings = ProviderBindings::default();
        bindings.0.insert(key, KeyCode::KeyG);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::input::InputPlugin)
            .add_plugins(InputManagerPlugin::<ProviderAction>::default())
            .insert_resource(InstalledActions(registry))
            .insert_resource(bindings)
            .init_resource::<crate::participant::SeatInputContexts>()
            .add_message::<SemanticActionPressed>()
            .add_systems(
                bevy::app::PreUpdate,
                (
                    install_provider_bindings_on_seats
                        .before(leafwing_input_manager::plugin::InputManagerSystem::Update),
                    crate::participant::resolve_active_input_context,
                ),
            )
            .add_systems(Update, publish_provider_action_edges);
        let seat = app
            .world_mut()
            .spawn((
                crate::participant::InputParticipant::primary(),
                crate::participant::ParticipantContexts::default(),
            ))
            .id();

        // Through the real resolver: a surface declares a capturing claim and
        // retracts it, as a pause menu does.
        let own = |app: &mut App, context: InputContextId| {
            let mut claims = app
                .world_mut()
                .get_mut::<crate::participant::ParticipantContexts>(seat)
                .expect("the seat keeps its claims");
            claims.retract(GAMEPLAY_CONTEXT);
            claims.retract(PAUSE_CONTEXT);
            claims.declare(crate::participant::ContextClaim::capturing(context, 0));
        };

        own(&mut app, GAMEPLAY_CONTEXT);
        app.update();
        let _ = drain_presses(&mut app);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.update();
        assert_eq!(
            drain_presses(&mut app).len(),
            1,
            "the permitted case must work, or the refusal below proves nothing"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyG);
        app.update();
        let _ = drain_presses(&mut app);

        own(&mut app, PAUSE_CONTEXT);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.update();
        assert!(
            drain_presses(&mut app).is_empty(),
            "a gameplay action fired while the seat was captured by the pause menu"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyG);
        app.update();
        let _ = drain_presses(&mut app);

        own(&mut app, GAMEPLAY_CONTEXT);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.update();
        assert_eq!(
            drain_presses(&mut app).len(),
            1,
            "the edge did not come back when the seat got gameplay again"
        );
    }

    /// Drain the semantic edges published this frame.
    #[cfg(feature = "input")]
    fn drain_presses(app: &mut bevy::prelude::App) -> Vec<SemanticActionPressed> {
        app.world_mut()
            .resource_mut::<bevy::prelude::Messages<SemanticActionPressed>>()
            .drain()
            .collect()
    }

    /// A provider's action can be a leafwing key.
    ///
    /// `InputMap` and `ActionState` are keyed by the engine's closed enum, but
    /// `InputMap<A: Actionlike>` is generic. So a composition can install a
    /// second map, if a key that a provider mints satisfies `Actionlike`, without
    /// erasure (`Any`, `TypeId`, a service locator).
    ///
    /// `input_control_kind(&self)` takes `&self`, so the key carries its kind.
    /// The registry mints the key, and its one-kind-per-id rule keeps `Hash` and
    /// `Eq` consistent for one action.
    ///
    /// This test checks the key and one map entry only. It does not check routing
    /// between two maps.
    #[cfg(feature = "input")]
    #[test]
    fn a_registry_minted_key_satisfies_leafwing_without_erasure() {
        use bevy::prelude::KeyCode;
        use leafwing_input_manager::prelude::*;

        // Minted from a registration, exactly as a provider would reach it.
        const GRAPPLE: SemanticActionDef = SemanticActionDef {
            id: SemanticActionId("grapple"),
            capability: "traversal",
            kind: ActionControlKind::Button,
            contexts: GAMEPLAY,
            doc: "Fire the grapple",
        };
        let mut registry = ActionRegistry::with_engine_actions();
        registry.register(GRAPPLE).expect("a fresh id");
        let key = registry
            .key(SemanticActionId("grapple"))
            .expect("the registry mints a key for what it registered");

        let mut map = InputMap::default();
        map.insert(key.clone(), KeyCode::KeyG);
        assert!(
            map.get(&key).is_some_and(|bindings| !bindings.is_empty()),
            "a provider-minted key bound nothing, so the second-map route does \
             not reach `InputMap` after all"
        );
        // An unregistered id mints nothing: its key would bind to a slot no reader
        // polls.
        assert!(registry.key(SemanticActionId("nonesuch")).is_none());
        assert_eq!(key.input_control_kind(), InputControlKind::Button);
    }
}
