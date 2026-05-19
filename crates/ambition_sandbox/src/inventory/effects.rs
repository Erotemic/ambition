use super::*;

pub(super) fn apply_item_effect(
    kind: ItemKind,
    inventory: &mut PlayerInventory,
    heals: &mut MessageWriter<crate::player::PlayerHealRequested>,
) {
    match kind {
        ItemKind::HealthPotion => {
            if inventory.remove(ItemKind::HealthPotion, 1) > 0 {
                heals.write(crate::player::PlayerHealRequested::new(2));
            }
        }
        ItemKind::SpareBattery | ItemKind::DataChip => {
            // Reserved for future effects; intentionally no-op for now.
        }
    }
}
