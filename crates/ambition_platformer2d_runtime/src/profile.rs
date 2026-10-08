//! Named supported engine profiles: capability contracts, not plugin lists.
//!
//! ⭐ **A PROFILE IS A PROMISE ABOUT WHAT IS ABSENT, AND IT IS ONLY WORTH ANYTHING
//! IF SOMETHING CHECKS IT.** The engine group installs every capability by
//! default. A composition that omits one (`PluginGroupBuilder::disable`) was
//! always possible; what was missing was (1) a NAME for the omission a consumer
//! may rely on, (2) a proof that the engine still steps a real subject without
//! it, and (3) a proof that the omitted capability is not installed by some
//! other road. The probe that produced this list found three couplings that
//! made a promised omission fail on its first tick (the durable-room ledger
//! installed by held-use, a kernel system nested in a set only held-use
//! configured, the conversation UI bridge added from inside another plugin's
//! `build`, where `disable` cannot see it); those are repaired, and the ones
//! that cannot be (`dev_tools_sim`, the bag resource) are NOT in this list.
//!
//! ⚠ **WHAT A PROFILE DOES NOT CLAIM.** It claims what is not INSTALLED
//! (plugins, and a resource each capability owns). It does not claim a crate is
//! absent from the build: the engine crates behind these capabilities are
//! unconditional dependencies of this crate (`Q106`), so only the facade's
//! feature-gated edges (the renderer, the menu, …) can be absent from a
//! dependency closure. `scripts/check_engine_profiles.py` holds that half.
//!
//! The witnesses live in `ambition_platformer2d_host/tests/supported_profiles.rs`:
//! each profile is built, stepped over a real body, and checked against
//! [`Capability::is_installed`], with a control arm proving the probe can say yes.

use bevy::app::{App, PluginGroupBuilder};
use bevy::prelude::World;

/// A removable unit of the engine group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Authored cutscenes and their trigger queue.
    Cutscenes,
    /// The dialogue runtime, the conversation authority and its UI bridge.
    Dialogue,
    /// Named boss encounters (generic encounters remain).
    BossEncounters,
    /// Using a held item or a wielded ability.
    HeldUse,
    /// Touching an item to collect it, and the authored conditions that ask about the bag.
    /// ⚠ The bag itself (`OwnedItems`) is core state and is NOT removed (`Q106`).
    Inventory,
}

impl Capability {
    pub const ALL: [Capability; 5] = [
        Capability::Cutscenes,
        Capability::Dialogue,
        Capability::BossEncounters,
        Capability::HeldUse,
        Capability::Inventory,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Capability::Cutscenes => "cutscenes",
            Capability::Dialogue => "dialogue",
            Capability::BossEncounters => "boss-encounters",
            Capability::HeldUse => "held-use",
            Capability::Inventory => "inventory",
        }
    }

    /// Drop this capability's plugins from the engine group.
    pub fn omit(self, group: PluginGroupBuilder) -> PluginGroupBuilder {
        match self {
            Capability::Cutscenes => {
                group.disable::<ambition_platformer2d_actor_monolith::cutscene::CutsceneSchedulePlugin>()
            }
            Capability::Dialogue => group
                .disable::<ambition_dialog::DialogSimStatePlugin>()
                .disable::<ambition_conversation::ConversationPlugin>(),
            Capability::BossEncounters => group
                .disable::<ambition_boss_encounter::BossEncounterSimulationPlugin>()
                .disable::<ambition_boss_encounter::conditions::BossConditionsPlugin>(),
            Capability::HeldUse => group
                .disable::<ambition_held_items::HeldItemSimulationPlugin>()
                .disable::<ambition_abilities::AbilitySimulationPlugin>(),
            Capability::Inventory => group
                .disable::<ambition_platformer2d_actor_monolith::items::pickup::ItemPickupSimulationPlugin>()
                .disable::<ambition_world_items::WorldItemSimulationPlugin>()
                .disable::<ambition_platformer2d_actor_monolith::items::conditions::InventoryConditionsPlugin>(),
        }
    }

    /// Whether this capability is installed in `app`: any of its plugins was
    /// added, or a resource only it installs exists.
    ///
    /// ⛔ Both halves, because a plugin check alone is satisfied by the road
    /// that was disabled and a resource check alone by a plugin that installs
    /// nothing observable.
    pub fn is_installed(self, app: &App) -> bool {
        let world: &World = app.world();
        match self {
            Capability::Cutscenes => {
                app.is_plugin_added::<ambition_platformer2d_actor_monolith::cutscene::CutsceneSchedulePlugin>()
                    || world.contains_resource::<ambition_cutscene::ActiveCutscene>()
            }
            Capability::Dialogue => {
                app.is_plugin_added::<ambition_dialog::DialogSimStatePlugin>()
                    || app.is_plugin_added::<ambition_conversation::ConversationPlugin>()
                    || world.contains_resource::<ambition_dialog::DialogState>()
            }
            Capability::BossEncounters => {
                app.is_plugin_added::<ambition_boss_encounter::BossEncounterSimulationPlugin>()
                    || app.is_plugin_added::<ambition_boss_encounter::conditions::BossConditionsPlugin>()
                    || world.contains_resource::<ambition_boss_encounter::BossEncounterRegistry>()
            }
            Capability::HeldUse => {
                app.is_plugin_added::<ambition_held_items::HeldItemSimulationPlugin>()
                    || app.is_plugin_added::<ambition_abilities::AbilitySimulationPlugin>()
            }
            Capability::Inventory => {
                app.is_plugin_added::<ambition_platformer2d_actor_monolith::items::pickup::ItemPickupSimulationPlugin>()
                    || app.is_plugin_added::<ambition_world_items::WorldItemSimulationPlugin>()
                    || app.is_plugin_added::<ambition_platformer2d_actor_monolith::items::conditions::InventoryConditionsPlugin>()
            }
        }
    }
}

/// Which face the host presents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostFace {
    /// No renderer: tests, tools, RL, trace replay.
    Headless,
    /// The drawing host (`ambition_platformer2d_host` with its `render` feature).
    Windowed,
}

/// A named, supported composition.
///
/// `profile-contract: <name>` markers below are what `scripts/check_engine_profiles.py`
/// reads; a profile with no marker, or a marker with no profile, is red.
#[derive(Clone, Copy, Debug)]
pub struct EngineProfile {
    pub name: &'static str,
    pub summary: &'static str,
    pub face: HostFace,
    /// The capabilities this profile promises are NOT installed.
    pub omits: &'static [Capability],
}

// profile-contract: headless-body-world
// profile-contract: windowed-body-world
// profile-contract: combat-without-inventory-boss-dialogue
// profile-contract: collection-without-held-use
// profile-contract: encounters-without-named-bosses

/// A body in a world with no renderer and nothing omitted.
pub const HEADLESS_BODY_WORLD: EngineProfile = EngineProfile {
    name: "headless-body-world",
    summary: "a body stepping in a room with no renderer",
    face: HostFace::Headless,
    omits: &[],
};

/// The same, behind the drawing host's wiring.
pub const WINDOWED_BODY_WORLD: EngineProfile = EngineProfile {
    name: "windowed-body-world",
    summary: "a body stepping in a room under the drawing host",
    face: HostFace::Windowed,
    omits: &[],
};

/// A fighting game: bodies, hits and rooms; no bag, no held use, no bosses, no talking.
pub const COMBAT_WITHOUT_INVENTORY_BOSS_DIALOGUE: EngineProfile = EngineProfile {
    name: "combat-without-inventory-boss-dialogue",
    summary: "combat with no collectibles, held items, named bosses or dialogue",
    face: HostFace::Headless,
    omits: &[
        Capability::Inventory,
        Capability::HeldUse,
        Capability::BossEncounters,
        Capability::Dialogue,
    ],
};

/// Pick things up and keep them, but never use one.
pub const COLLECTION_WITHOUT_HELD_USE: EngineProfile = EngineProfile {
    name: "collection-without-held-use",
    summary: "collectibles and the bag, with no held item or wielded ability",
    face: HostFace::Headless,
    omits: &[Capability::HeldUse],
};

/// Generic encounters (waves, gates) with no named boss encounter.
pub const ENCOUNTERS_WITHOUT_NAMED_BOSSES: EngineProfile = EngineProfile {
    name: "encounters-without-named-bosses",
    summary: "generic encounters with no named boss encounter",
    face: HostFace::Headless,
    omits: &[Capability::BossEncounters],
};

/// Every supported profile, in the order the witnesses run them.
pub const SUPPORTED_PROFILES: [EngineProfile; 5] = [
    HEADLESS_BODY_WORLD,
    WINDOWED_BODY_WORLD,
    COMBAT_WITHOUT_INVENTORY_BOSS_DIALOGUE,
    COLLECTION_WITHOUT_HELD_USE,
    ENCOUNTERS_WITHOUT_NAMED_BOSSES,
];

/// Whether the session-edge parameter bundles validate in `world`.
///
/// ⭐ STEPPING A STEADY STATE PROVES NOTHING ABOUT THE EDGES. The session teardown
/// and the room-transition finalizer take capability-owned state as parameters,
/// and neither runs while a fixture merely ticks: the first probe of the
/// `Dialogue` omission stepped cleanly and would have failed parameter validation
/// at the first room transition and the first session end. Asking Bevy whether the
/// parameters validate is what a witness can do without staging either event.
pub fn session_edge_params_validate(world: &mut World) -> Result<(), String> {
    use bevy::ecs::system::SystemState;
    let mut teardown = SystemState::<ambition_platformer2d_actor_monolith::session::teardown::SessionScopedResources>::new(world);
    teardown
        .get_mut(world)
        .map(|_| ())
        .map_err(|error| format!("the session teardown bundle: {error}"))?;
    let mut finalize = SystemState::<crate::room_transition::RoomTransitionFinalize>::new(world);
    finalize
        .get_mut(world)
        .map(|_| ())
        .map_err(|error| format!("the room-transition finalizer: {error}"))?;
    Ok(())
}
