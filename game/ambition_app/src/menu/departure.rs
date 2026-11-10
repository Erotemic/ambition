//! The System menu's departure rows: a joined local seat may leave, and the
//! primary player may remove one.

use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::inventory_ui::InventoryMenu;
use ambition_platformer2d::settings_menu::system::SystemMenuModel;
use bevy::prelude::*;

/// The seats a menu offers to end: its own seat, if it may leave, and the
/// other seats it may remove.
pub(crate) type Departures = (Option<u8>, Vec<u8>);

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MenuDepartures<'w, 's> {
    owners: Query<'w, 's, &'static InventoryMenu>,
    scope: ambition_platformer2d::platformer::lifecycle::LiveSessionScope<'w, 's>,
    roots: Query<
        'w,
        's,
        (
            &'static ambition_platformer2d::platformer::lifecycle::SessionRoot,
            &'static ambition_platformer2d::session::participants::SessionParticipants,
            &'static ambition_platformer2d::session::initial_body::InitialBodyPolicy,
        ),
    >,
    local: Option<Res<'w, ambition_platformer2d::rollback::LocalPlayers>>,
}

impl MenuDepartures<'_, '_> {
    /// Only a session that joins seats with home bodies lets one leave (a
    /// match owns its whole cast), only a local seat is ended from this
    /// machine, and the primary seat never leaves this way: Quit to Title is
    /// its way out.
    pub(crate) fn state(&self, menu: Entity) -> Departures {
        let Ok(owner) = self.owners.get(menu) else {
            return (None, Vec::new());
        };
        let Some(scope) = self.scope.get() else {
            return (None, Vec::new());
        };
        let Some((_, members, _)) = self.roots.iter().find(|(root, _, policy)| {
            root.0 == scope
                && matches!(
                    policy,
                    ambition_platformer2d::session::initial_body::InitialBodyPolicy::SpawnCharacter(_)
                )
        }) else {
            return (None, Vec::new());
        };
        let local = |slot: PlayerSlot| {
            self.local.as_ref().is_none_or(|local| local.0.contains(&usize::from(slot.0)))
        };
        let leavers = || members.iter().filter(|slot| slot.0 != 0 && local(*slot));
        let own = owner.owner.slot();
        if own == 0 {
            (None, leavers().map(|slot| slot.0).collect())
        } else {
            (leavers().any(|slot| slot.0 == own).then_some(own), Vec::new())
        }
    }

    pub(crate) fn attach(&self, model: SystemMenuModel, menu: Entity) -> SystemMenuModel {
        let (own, others) = self.state(menu);
        model.with_seat_departure(own, &others)
    }
}
