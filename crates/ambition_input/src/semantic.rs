//! **Semantic actions: the open vocabulary between a device and a consumer.**
//!
//! ```text
//! physical input → participant bindings → ordered contexts → SEMANTIC ACTIONS → consumers
//! ```
//!
//! `Platformer2dInputActionMonolith` is leafwing's concrete `Actionlike` enum, and it has to be
//! concrete — leafwing needs a real type to key an `InputMap`. That makes it a
//! closed vocabulary: a capability cannot add a variant without editing the
//! engine, which is the one central closed enum the content compiler exists to
//! avoid, one layer over.
//!
//! This is the open half. A [`SemanticActionId`] is a `&'static str` like
//! [`crate::InputContextId`] and like the content compiler's `SchemaId`, for the
//! same reason: a capability mints its own and nobody edits an enum.
//!
//! ## The registry is the VOCABULARY, not a description of it
//!
//! Every engine `Platformer2dInputActionMonolith` is registered here. That is what makes this
//! authoritative rather than a parallel list somebody has to remember to update
//! — and `every_device_action_is_registered` fails if one is added without a
//! semantic entry, so the two cannot drift.
//!
//! ## What is NOT here yet
//!
//! A capability-owned action can be DECLARED and looked up, and it can ride an
//! existing device action. It cannot yet have a device binding of its own,
//! because that needs `InputMap<SemanticActionId>` in place of
//! `InputMap<Platformer2dInputActionMonolith>`.
//!
//! ⛔ **and the blocker is NOT the call-site count, which is what this paragraph
//! used to say.** Measured 2026-08-02: 348 lines across 35 files name the device
//! enum, but only **21 `InputMap<…>` + 25 `ActionState<…>`** are structural — the
//! other ~225 are variant references that follow mechanically once the target
//! type exists. ~46 hard sites, not "hundreds".
//!
//! ⭐ **the real blocker is one line of leafwing's trait.** `Actionlike` requires
//! `Debug + Eq + Hash + Send + Sync + Clone + Reflect + Typed + TypePath +
//! FromReflect + 'static`, and [`SemanticActionId`] satisfies every one of them —
//! `bevy_reflect` implements `Reflect`/`Typed`/`FromReflect` for `&'static str`,
//! so a `#[derive(Reflect)]` newtype is enough. What it cannot satisfy is the
//! trait's single METHOD:
//!
//! ```ignore
//! fn input_control_kind(&self) -> InputControlKind;
//! ```
//!
//! It takes `&self` and nothing else, so an action must be **self-describing**
//! about whether it is a button, an axis or a dual axis. This design deliberately
//! puts that in the REGISTRY instead — [`SemanticActionDef::kind`] — where a
//! composition can own it. An id alone cannot answer, and a global lookup inside
//! the impl would reintroduce exactly the central mutable table the open
//! vocabulary exists to avoid.
//!
//! ▢ **and the registry already settles it.** `ActionRegistry` is
//! `BTreeMap<SemanticActionId, SemanticActionDef>` and [`ActionConflict`] refuses
//! a second owner for the same id — so **an id has exactly one kind, by
//! construction**. The "two ids with the same string, different kinds" worry
//! cannot arise here.
//!
//! ⭐ so the shape is: keep [`SemanticActionId`] as the identity and the registry
//! key, unchanged, and give leafwing a SEPARATE `SemanticAction { id, kind }`
//! that is only ever built by looking an id up in the registry. It cannot be
//! minted with a kind the registry disagrees with, and the docs' own wording —
//! `InputMap<SemanticAction>`, not `InputMap<SemanticActionId>` — was already
//! pointing at exactly that.
//!
//! ⚠ NOT built here. An unused type is a hypothetical; this belongs in the commit
//! that does the rename, which now has one fewer decision to make.

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

    /// **What can be pressed in this context?** The question a prompt, a help
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
}

/// **The composition's action vocabulary, as a resource.**
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

/// **The engine's vocabulary, and the whole of it.**
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
    engine("dash", ActionControlKind::Button, GAMEPLAY, "Dash"),
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
        "quick_action",
        ActionControlKind::Button,
        GAMEPLAY,
        "Shield / guard",
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
        "dash_analog",
        ActionControlKind::Axis,
        GAMEPLAY,
        "Analog dash trigger",
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

    /// **A capability adds an action without editing the engine.**
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

    /// ⛔ **The registry must not fall behind the enum.**
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
}
