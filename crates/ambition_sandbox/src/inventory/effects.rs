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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::PlayerHealRequested;
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct HealCount(usize);

    fn record_heals(mut reader: MessageReader<PlayerHealRequested>, mut log: ResMut<HealCount>) {
        log.0 += reader.read().count();
    }

    fn use_one_health_potion(
        mut inventory: ResMut<PlayerInventory>,
        mut heals: MessageWriter<PlayerHealRequested>,
    ) {
        apply_item_effect(ItemKind::HealthPotion, &mut inventory, &mut heals);
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<HealCount>();
        app.add_message::<PlayerHealRequested>();
        app.add_systems(Update, (use_one_health_potion, record_heals).chain());
        app
    }

    #[test]
    fn using_a_health_potion_decrements_it_and_emits_a_heal() {
        let mut app = app();
        let mut inv = PlayerInventory::default();
        inv.add(ItemKind::HealthPotion, 2);
        app.insert_resource(inv);
        app.update();
        assert_eq!(
            app.world()
                .resource::<PlayerInventory>()
                .count(ItemKind::HealthPotion),
            1,
            "using one potion leaves one"
        );
        assert_eq!(app.world().resource::<HealCount>().0, 1, "one heal emitted");
    }

    #[test]
    fn using_a_health_potion_with_none_left_is_a_noop() {
        let mut app = app();
        app.insert_resource(PlayerInventory::default()); // zero potions
        app.update();
        assert_eq!(
            app.world().resource::<HealCount>().0,
            0,
            "no potion to spend → no heal"
        );
    }
}
