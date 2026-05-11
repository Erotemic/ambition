use super::*;

pub(super) fn apply_item_effect(
    kind: ItemKind,
    inventory: &mut PlayerInventory,
    runtime: &mut SandboxRuntime,
) {
    match kind {
        ItemKind::HealthPotion => {
            if inventory.remove(ItemKind::HealthPotion, 1) > 0 {
                runtime.player_health.heal(2);
            }
        }
        ItemKind::SpareBattery | ItemKind::DataChip => {
            // Reserved for future effects; intentionally no-op for now.
        }
    }
}
