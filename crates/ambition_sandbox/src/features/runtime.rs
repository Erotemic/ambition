use super::*;

mod build;
mod combat;
mod save;
mod settling;
mod spawn;
mod update;
mod views;

pub use settling::tick_chest_fall;

/// Legacy runtime shell for authored feature actors.
///
/// STRANGLER MIGRATION RULE: do not add new feature families or behavior here.
/// New simple gameplay actors should be Bevy entities with typed components
/// from `crate::features::components`, then owned by ECS systems and Bevy
/// messages. Existing vectors are migration targets that should shrink over
/// time.
#[derive(Clone, Debug)]
pub struct FeatureRuntime {
    pub hazards: Vec<HazardRuntime>,
    pub enemies: Vec<EnemyRuntime>,
    pub bosses: Vec<BossRuntime>,
    pub breakables: Vec<BreakableRuntime>,
    pub pickups: Vec<PickupRuntime>,
    pub chests: Vec<ChestRuntime>,
    pub npcs: Vec<NpcRuntime>,
    pub switches: Vec<SwitchRuntime>,
    pub banner: String,
    pub banner_timer: f32,
}

/// Runtime state of a `Switch` interactable. The custom payload comes
/// from the LDtk `Switch` entity via `entity_to_runtime`; the
/// encounter system parses it on activation.
#[derive(Clone, Debug)]
pub struct SwitchRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub interactable: ae::Interactable,
    /// The `Custom("switch:...")` payload string. Cached here so the
    /// activation event doesn't have to re-pattern-match `kind`.
    pub custom_payload: String,
    /// Live on/off state for color rendering. The encounter system
    /// keeps this in sync with the persisted save state + the live
    /// encounter phase: `on = true` means the encounter is `Cleared`
    /// or has been disabled by the user; `on = false` means the
    /// encounter is armed (will fire when the player enters).
    pub on: bool,
}

impl SwitchRuntime {
    pub(super) fn new(
        object: &ae::RoomObject,
        interactable: ae::Interactable,
        payload: String,
    ) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            interactable,
            custom_payload: payload,
            on: false,
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}

impl FeatureRuntime {
    pub(super) fn accept_events(&mut self, events: &FeatureEvents) {
        if let Some(message) = events.messages.last() {
            self.banner = message.clone();
            self.banner_timer = 2.6;
        }
    }
}
