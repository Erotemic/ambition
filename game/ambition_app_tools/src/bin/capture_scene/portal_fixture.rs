//! Fixed apertures for deterministic render witnesses.

use super::*;
use ambition_platformer2d::platformer::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, RoomInstanceRoot, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d::portal::{PlacedPortal, PortalChannel, PortalChannelColor, PortalHost};

#[derive(Clone, Debug)]
pub(super) struct PortalAt {
    tick: u64,
    portal: PlacedPortal,
}

impl PortalAt {
    pub(super) fn parse(text: &str) -> Result<Self, String> {
        let bad = || format!("--portal-at requires TICK:INDEX:X,Y:NX,NY:HALF_LENGTH, got '{text}'");
        let fields: Vec<_> = text.split(':').collect();
        let [tick, index, at, normal, half] = fields.as_slice() else { return Err(bad()); };
        let channel = PortalChannel::Authored(PortalChannelColor::Indexed(index.parse().map_err(|_| bad())?));
        let mut portal = PlacedPortal::fixed(channel, parse_vec2(at).ok_or_else(bad)?,
            parse_vec2(normal).ok_or_else(bad)?, half.parse().map_err(|_| bad())?)
            .map_err(|fault| format!("--portal-at: {fault:?}"))?;
        portal.host = PortalHost::Static;
        Ok(Self { tick: tick.parse().map_err(|_| bad())?, portal })
    }
}

pub(super) fn place_portals(
    config: Res<SceneCaptureConfig>,
    tick: Res<ambition_platformer2d::time::SimTick>,
    room: Single<&LiveRoomInstance, With<RoomInstanceRoot>>,
    active: Option<Res<ActiveSessionScope>>,
    mut commands: Commands,
) {
    let Some(scope) = SessionSpawnScope::for_optional_active_session(active.as_deref()) else { return; };
    for placement in config.portal_placements.iter().filter(|placement| placement.tick == tick.get()) {
        commands.spawn_session_scoped(scope.in_room(Some(**room)), (
            placement.portal.clone(), Name::new("Capture aperture"),
        ));
        eprintln!("capture_scene: placed {:?} at {:?} on tick {}", placement.portal.channel, placement.portal.pos, placement.tick);
    }
}
