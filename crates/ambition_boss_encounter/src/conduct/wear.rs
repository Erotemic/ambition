//! The adapter of `ambition.boss.wear`: a body puts on or takes off a worn
//! gear (`WornGearManifest`). See the port card of
//! `ambition_boss_special_port::WearPort` and `docs/systems/worn-gear.md`.

use ambition_boss_special_port::{Wear, WearPort, Wearer};
use ambition_characters::equipment::{EquipmentRow, WornEquipment};
use ambition_combat::components::ActorTarget;
use ambition_extension_host::{ExtensionOutbox, Submitted};
use ambition_extension_sdk::phases::BOSS_CONDUCT;
use ambition_platformer2d_shared_tangle::worn_gear::{WornGearEntry, WornGearManifest};
use bevy::prelude::*;

/// The `ambition.boss.wear` requests of this tick, for [`wear_gear`].
pub fn drain_wears(mut outbox: ResMut<ExtensionOutbox>) -> Vec<Submitted<Wear>> {
    outbox.drain::<WearPort>(&BOSS_CONDUCT)
}

/// The equipment row of `gear`: its id, in its exclusive slot.
pub fn gear_row(gear: &WornGearEntry) -> EquipmentRow {
    EquipmentRow { id: gear.id.clone(), exclusive_slot: Some(gear.slot.clone()), ..Default::default() }
}

/// Put `gear` on `worn`, or take it off. The caller writes only when
/// [`WornEquipment::wears`] differs from `on`, so a request that changes
/// nothing does not mark the set as changed (its readers fold the body's
/// repertoire again on a change).
fn put(worn: &mut WornEquipment, gear: &WornGearEntry, on: bool) {
    if on {
        worn.equip(gear_row(gear));
    } else {
        worn.unequip(&gear.id);
    }
}

/// Lower `ambition.boss.wear`: the wearer puts the gear on or takes it off. A
/// body with no `WornEquipment` gets one. A gear id that no content declares,
/// or a wearer that is not there, is refused with a warning.
pub fn wear_gear(
    In(requests): In<Vec<Submitted<Wear>>>,
    manifest: Option<Res<WornGearManifest>>,
    riders: Query<&ambition_mount::RidingOn>,
    targets: Query<&ActorTarget>,
    bodies: Query<()>,
    mut worn: Query<&mut WornEquipment>,
    mut commands: Commands,
) {
    // The new sets of bodies with no `WornEquipment`: two requests of one
    // tick for one such body go into one set, and the last insert does not
    // hide the first.
    let mut fresh: Vec<(Entity, WornEquipment)> = Vec::new();
    for submitted in requests {
        let wear = &submitted.value;
        let Some(gear) = manifest.as_deref().and_then(|manifest| manifest.get(&wear.gear)) else {
            warn!(
                "extension entry {} asked {:?} of {:?} to wear `{}`, which no content declares (`worn_gear`); refused",
                submitted.entry, wear.who, submitted.scope, wear.gear
            );
            continue;
        };
        let wearer = match wear.who {
            Wearer::Boss => Some(submitted.scope),
            Wearer::Mount => riders.get(submitted.scope).ok().map(|riding| riding.mount),
            Wearer::Target => targets.get(submitted.scope).ok().and_then(|target| target.entity),
        };
        let Some(body) = wearer.filter(|body| bodies.contains(*body)) else {
            warn!(
                "extension entry {} asked {:?} of {:?} to wear `{}`, and there is no such body; refused",
                submitted.entry, wear.who, submitted.scope, wear.gear
            );
            continue;
        };
        if let Ok(mut set) = worn.get_mut(body) {
            if set.wears(&gear.id) != wear.on {
                put(&mut set, gear, wear.on);
            }
        } else if let Some((_, set)) = fresh.iter_mut().find(|(fresh, _)| *fresh == body) {
            if set.wears(&gear.id) != wear.on {
                put(set, gear, wear.on);
            }
        } else if wear.on {
            fresh.push((body, WornEquipment::new(vec![gear_row(gear)])));
        }
    }
    for (body, set) in fresh {
        commands.entity(body).insert(set);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::worn_gear::{GearAnchor, WornGearAppExt};
    use bevy::ecs::system::RunSystemOnce as _;

    fn gear(id: &str, slot: &str) -> WornGearEntry {
        WornGearEntry {
            id: id.into(),
            anchor: GearAnchor::Head,
            slot: slot.into(),
            back: Some(format!("sprites/{id}_back.png")),
            front: None,
            size: Vec2::splat(64.0),
            socket: Vec2::new(0.5, 0.58),
            fits: Default::default(),
        }
    }

    fn wear(app: &mut App, scope: Entity, who: Wearer, gear: &str, on: bool) {
        let request = Submitted {
            scope,
            invocation: Default::default(),
            occurrence: None,
            entry: "test::gnu/conduct".into(),
            value: Wear { who, gear: gear.into(), on },
        };
        app.world_mut().run_system_once_with(wear_gear, vec![request]).unwrap();
    }

    fn ids(app: &App, body: Entity) -> Option<Vec<(String, Option<String>)>> {
        app.world()
            .get::<WornEquipment>(body)
            .map(|worn| worn.rows.iter().map(|row| (row.id.clone(), row.exclusive_slot.clone())).collect())
    }

    fn row(id: &str, slot: &str) -> (String, Option<String>) {
        (id.to_owned(), Some(slot.to_owned()))
    }

    /// ⭐ The boss puts the helmet on its mount: the mount gets a worn set
    /// with the helmet's row in the head slot, and its rider gets nothing. A
    /// crown in the same slot replaces it; the take-off empties the set. The
    /// controls: a gear no content declares, and a target the boss does not
    /// have, change nothing.
    #[test]
    fn a_boss_puts_gear_on_its_mount_and_takes_it_off() {
        let mut app = App::new();
        app.register_worn_gear([gear("space_helmet", "head"), gear("crown", "head")]);
        let gnu = app.world_mut().spawn_empty().id();
        let scholar = app.world_mut().spawn((ambition_mount::RidingOn { mount: gnu }, ActorTarget::default())).id();

        wear(&mut app, scholar, Wearer::Mount, "space_helmet", true);
        assert_eq!(ids(&app, gnu), Some(vec![row("space_helmet", "head")]));
        assert_eq!(ids(&app, scholar), None, "the rider wears nothing");

        wear(&mut app, scholar, Wearer::Mount, "space_helmet", true);
        assert_eq!(ids(&app, gnu), Some(vec![row("space_helmet", "head")]), "a second put-on is one row");

        wear(&mut app, scholar, Wearer::Mount, "crown", true);
        assert_eq!(ids(&app, gnu), Some(vec![row("crown", "head")]), "one gear in a slot");

        wear(&mut app, scholar, Wearer::Mount, "fish_bowl", true);
        assert_eq!(ids(&app, gnu), Some(vec![row("crown", "head")]), "an unknown gear changes nothing");
        wear(&mut app, scholar, Wearer::Target, "space_helmet", true);
        assert_eq!(ids(&app, scholar), None, "a boss with no target puts gear on nobody");

        wear(&mut app, scholar, Wearer::Mount, "crown", false);
        assert_eq!(ids(&app, gnu), Some(Vec::new()), "the take-off empties the set");
    }

    /// A request that changes nothing does not mark the set as changed: the
    /// body's repertoire is folded again only on a change.
    #[test]
    fn a_request_that_changes_nothing_leaves_the_set_unchanged() {
        let mut app = App::new();
        app.register_worn_gear([gear("space_helmet", "head")]);
        let boss = app.world_mut().spawn_empty().id();
        wear(&mut app, boss, Wearer::Boss, "space_helmet", true);
        let changed = |app: &App| app.world().entity(boss).get_ref::<WornEquipment>().unwrap().last_changed();
        let first = changed(&app);
        app.world_mut().increment_change_tick();
        wear(&mut app, boss, Wearer::Boss, "space_helmet", true);
        assert_eq!(changed(&app), first, "the second put-on wrote the set");
        wear(&mut app, boss, Wearer::Boss, "space_helmet", false);
        assert_ne!(changed(&app), first, "the take-off did not write the set");
        assert_eq!(ids(&app, boss), Some(Vec::new()));
    }
}
