//! Open semantic action vocabulary between physical bindings and consumers.
//!
//! [`SemanticActionId`] lets capabilities register actions without extending the
//! closed leafwing device-action enum. [`ActionRegistry`] is authoritative for
//! each id and its control kind, and mints the [`ProviderAction`] key that binds
//! one — so a registered action is now describable AND bindable without touching
//! the device enum.
//!
//! ⚠ Not yet ROUTED: nothing installs an `InputMap<ProviderAction>` in production,
//! because two maps means two reader paths and a rule for which wins a conflict.

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

/// What SHAPE of input an action carries. Mirrors leafwing's control kinds,
/// because a binding UI and a prompt both need to know whether they are drawing
/// a button or a stick.
///
/// ⭐ `Hash` and `Reflect` are here for [`ProviderAction`], which carries this as
/// a FIELD because leafwing's `input_control_kind` takes `&self` — the key has to
/// know its own shape rather than look it up. They cost nothing on a fieldless
/// enum, and they are what keeps the provider key from needing a second copy of
/// this vocabulary beside it.
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
    /// One line, for a help screen or a rebind UI. Documentation metadata
    /// belongs with the registration or it is not documentation.
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
    /// Two owners for one id is refused HERE rather than at use, for the same
    /// reason the content compiler refuses an ambiguous schema: letting it
    /// through means the winner is decided by map iteration order.
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

    /// What can be pressed in this context? The question a prompt, a help
    /// screen and a rebind UI all ask, answered once.
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

    /// Mint the leafwing key for a registered action — the ONLY way to get one.
    ///
    /// ⭐ THE REGISTRY MINTS IT BECAUSE THE REGISTRY OWNS THE KIND. `ProviderAction`
    /// hashes on both id and kind, so two keys built by hand for one action could
    /// disagree about its shape and silently miss each other in an `InputMap`. The
    /// registry already enforces one kind per id ([`ActionRegistry::register`]),
    /// and routing every key through it is what extends that rule to the bindings.
    ///
    /// `None` for an unregistered id: a key for an action nobody declared is a
    /// binding to nothing, and the honest place to notice is here.
    #[cfg(feature = "input")]
    pub fn key(&self, id: SemanticActionId) -> Option<ProviderAction> {
        self.get(id).map(|def| ProviderAction {
            id: def.id.0.to_string(),
            kind: def.kind,
        })
    }
}

/// A registered action AS A LEAFWING KEY — the second map's keyspace.
///
/// ⭐⭐ THE POINT IS THAT IT IS NOT AN ENUM. `InputMap<A: Actionlike>` is already
/// generic, so a composition installs one of these beside the engine's map and a
/// capability binds an action the engine has never heard of — with no `Any`, no
/// `TypeId`, no service locator, and no edit to the 35-variant device enum. Every
/// previous escape from that enum reached for erasure and was refused twice.
///
/// ⛔ MINT IT WITH [`ActionRegistry::key`], never by hand. The kind is part of the
/// hash, so a hand-built key that guesses wrong binds into a slot nothing reads.
///
/// ⚠ WHAT THIS DOES NOT YET DO: nothing installs a map over it in production, so
/// a provider action is bindable and not yet routed. Two maps means two reader
/// paths and a rule for which wins a conflict, and that rule is the next slice —
/// see `docs/planning/engine/participant-action-system.md`.
#[cfg(feature = "input")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, bevy::prelude::Reflect)]
pub struct ProviderAction {
    /// The registered [`SemanticActionId`], owned because a key must outlive the
    /// registration's `&'static str` in a reflected, serialisable map.
    pub id: String,
    /// Carried, not looked up: leafwing's `input_control_kind` takes `&self`, so
    /// the key has to know its own shape with no registry in hand.
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
/// ⭐ SEPARATE FROM [`ActionRegistry`] ON PURPOSE. The registry DESCRIBES an
/// action — id, kind, contexts, doc — and describing is what a capability can do
/// alone. What key it sits on is the COMPOSITION's answer, the same split the
/// engine's own vocabulary already has between the registry and `BindingRecipe`.
/// A capability that shipped its own key would be deciding a thing the game it is
/// installed into owns.
///
/// Empty is the honest default: a composition that binds nothing routes nothing,
/// and every action stays reachable by whoever writes its message directly.
#[cfg(feature = "input")]
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct ProviderBindings(pub leafwing_input_manager::prelude::InputMap<ProviderAction>);

/// A registered provider action was pressed by a seat this frame.
///
/// ⛔ AN EDGE, NOT A LEVEL. `just_pressed`, so a held key is one message and not
/// one per frame — the same rule the menu and pointer channels beside it keep.
///
/// The consumer is the capability that registered the action: it reads this and
/// writes whatever its own request message is. That indirection is the point —
/// `ambition_input` never learns what a pulse is, and the capability never learns
/// what a keyboard is.
#[cfg(feature = "input")]
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticActionPressed {
    pub id: SemanticActionId,
    pub participant: crate::participant::ParticipantId,
}

/// Give every seat the composition's provider map, and keep it current.
///
/// ⭐ A SYNC RATHER THAN A SPAWN EDIT, because a capability may be installed after
/// the seats exist and a binding change must reach seats that are already sitting
/// there. The two spawn sites in `input_systems.rs` build their tuples from the
/// engine's `BindingRecipe`, which knows nothing about provider actions; making
/// them know would put a capability's concern in the seat constructor.
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
/// ⛔ THE REGISTRY IS THE FILTER, and that is what makes the id in the message a
/// `&'static str` rather than the key's owned `String`. A key whose id no longer
/// resolves is an action the composition stopped installing, and routing it would
/// deliver a press to a capability that is no longer there.
#[cfg(feature = "input")]
pub fn publish_provider_action_edges(
    registry: Option<bevy::prelude::Res<InstalledActions>>,
    seats: bevy::prelude::Query<(
        &crate::participant::InputParticipant,
        &leafwing_input_manager::prelude::ActionState<ProviderAction>,
    )>,
    mut pressed: bevy::prelude::MessageWriter<SemanticActionPressed>,
) {
    let Some(registry) = registry else {
        return;
    };
    // ⛔ SEAT ORDER IS NOT QUERY ORDER. Two seats pressing the same action on one
    // frame must publish in the same order every run, and Bevy's iteration order
    // is not a promise. Sorted by the seat's own id, which is the only stable
    // name a participant has.
    let mut rows: Vec<_> = seats.iter().collect();
    rows.sort_by_key(|(participant, _)| participant.id.0);
    for (participant, actions) in rows {
        for key in actions.get_just_pressed() {
            let Some(def) = registry
                .all()
                .find(|def| def.id.0 == key.id && def.kind == key.kind)
            else {
                continue;
            };
            pressed.write(SemanticActionPressed {
                id: def.id,
                participant: participant.id,
            });
        }
    }
}

/// The composition's action vocabulary, as a resource.
///
/// A registry is a value; this is where the running app keeps one. Built by the
/// facade's assembly pass from the engine's actions plus whatever the mounted
/// modules declared, so a prompt, a help screen or a rebind UI asks ONE
/// question and gets the game's actions beside the engine's.
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
/// One entry per `Platformer2dInputActionMonolith`. `every_device_action_is_registered` fails when
/// a variant is added without one, so this cannot quietly fall behind the enum —
/// which is the difference between a registry and a description of a registry.
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
    /// The whole point of the row: `Platformer2dInputActionMonolith` is a closed enum a capability
    /// cannot extend, and this is the half that is open.
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

    ///  The registry must not fall behind the enum.
    ///
    /// This is what makes it the vocabulary rather than a description of one. A
    /// `Platformer2dInputActionMonolith` added without a semantic entry would be invisible to
    /// every prompt, help screen and rebind UI that asks the registry — and
    /// invisible is exactly how a parallel list rots.
    #[cfg(feature = "input")]
    #[test]
    fn every_device_action_is_registered() {
        use crate::Platformer2dInputActionMonolith;
        use bevy::reflect::{TypeInfo, Typed};

        // `Actionlike` has no `variants()` in leafwing 0.20, but it requires
        // `Reflect + Typed` — so the enum's own type info is the honest list,
        // and it cannot go stale the way a hand-written one would.
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

    /// The registry is keyed by `&'static str`; a test-built name is not one.
    /// Leaking a handful of short strings inside one test is cheaper than
    /// widening the key type for it.
    #[cfg(feature = "input")]
    fn leak(name: &str) -> &'static str {
        Box::leak(name.to_string().into_boxed_str())
    }

    /// A REGISTERED ACTION REACHES A SEAT'S PRESS — the whole road, in one app.
    ///
    /// ⭐⭐ WHAT THIS PINS is the claim the plan could previously only argue:
    /// register an action the engine has never heard of, bind it to a key, press
    /// the key, and get a semantic edge back — with no `Any`, no `TypeId`, and no
    /// variant added to the 35-variant device enum. The capability demo's module
    /// doc named the missing half of exactly this.
    ///
    /// ⛔ THE PRESS IS HELD ACROSS TWO FRAMES on purpose. A one-frame press cannot
    /// tell an edge channel from a level one, and this channel owes the edge rule:
    /// a held key is one message, not one per frame.
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
        app.world_mut()
            .spawn(crate::participant::InputParticipant::primary());

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

        // Still held. `Bevy`'s `ButtonInput` keeps it pressed across the frame,
        // so this is the level-vs-edge question asked honestly.
        app.update();
        assert!(
            drain_presses(&mut app).is_empty(),
            "holding the key republished the press every frame"
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

    /// A PROVIDER'S ACTION CAN BE A LEAFWING KEY — checked, not argued.
    ///
    /// ⭐⭐ THE OPEN QUESTION THIS ANSWERS is why a registered action is
    /// describable and neither bindable nor readable: `InputMap` and
    /// `ActionState` are keyed by the engine's CLOSED enum, and every previous
    /// escape reached for erasure (`Any`, `TypeId`, a service locator), which the
    /// reviews refused twice. ⛔ this is not that. `InputMap<A: Actionlike>` is
    /// already generic, so a composition may install a SECOND map beside the
    /// engine's — and the only question is whether a key a provider can MINT can
    /// satisfy `Actionlike`.
    ///
    /// ⭐ IT CAN, and the one part that is not derivable is what shapes the type:
    /// `input_control_kind(&self)` takes `&self`, so the key must CARRY its kind
    /// rather than look it up. `SemanticActionDef` already holds that kind, so
    /// the registry mints the key and its own one-kind-per-id rule keeps `Hash`
    /// and `Eq` from ever disagreeing about the same action.
    ///
    /// ⚠ this test compiles the KEY and one map entry. It does not claim the
    /// carve is done — two maps means two reader paths and a rule for which wins
    /// — only that the bound is satisfiable without erasure, which is the thing
    /// that had never been checked.
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
        // An id nobody registered mints nothing: a key for an undeclared action
        // is a binding to a slot no reader will ever poll.
        assert!(registry.key(SemanticActionId("nonesuch")).is_none());
        assert_eq!(key.input_control_kind(), InputControlKind::Button);
    }
}
