//! Gameplay banner ticking and deferred-request application.

use super::*;

/// The banner's request drain, then its timer. A composition that orders the
/// banner against its own systems names this set, not the two functions.
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameplayBannerSet;

/// The banner's two systems in [`GameplayBannerSet`], each skipped when it has
/// nothing to do.
pub fn gameplay_banner_systems() -> bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::ScheduleSystem> {
    (
        apply_gameplay_banner_requests.run_if(ae::a_message_waits::<GameplayBannerRequested>),
        tick_gameplay_banner.run_if(the_banner_is_up),
    )
        .chain()
        .in_set(GameplayBannerSet)
}

/// Run condition of [`tick_gameplay_banner`]: the banner shows or holds a
/// timer to run down.
pub fn the_banner_is_up(banner: Res<GameplayBanner>) -> bool {
    banner.is_up()
}

/// Tick the gameplay banner resource once a tick while it is up
/// ([`the_banner_is_up`]); a resting banner is left unwritten.
pub fn tick_gameplay_banner(world_time: Res<WorldTime>, mut banner: ResMut<GameplayBanner>) {
    // Sim clock: the gameplay banner displays gameplay-driven
    // messages (quest hints, encounter intros) so its dismissal
    // timer should pause alongside the sim — otherwise the banner
    // burns its display window during bullet-time / pause.
    banner.tick(world_time.sim_dt());
}

/// Apply deferred banner requests from high-param systems.
pub fn apply_gameplay_banner_requests(
    mut banner: ResMut<GameplayBanner>,
    mut requests: MessageReader<GameplayBannerRequested>,
) {
    for request in requests.read() {
        banner.show_in(request.room, request.text.clone(), request.duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The banner ticks while it is up and clears when its timer runs out;
    /// after that it is left unwritten ([`the_banner_is_up`]).
    #[test]
    fn a_banner_runs_down_and_then_rests_unwritten() {
        let mut app = App::new();
        app.insert_resource(WorldTime::new(1.0, 1.0));
        app.insert_resource(GameplayBanner::default());
        app.add_systems(Update, tick_gameplay_banner.run_if(the_banner_is_up));
        app.world_mut().resource_mut::<GameplayBanner>().show("Hello", 1.5);
        app.update();
        assert!(app.world().resource::<GameplayBanner>().visible());
        app.update();
        assert!(!app.world().resource::<GameplayBanner>().visible());
        let cleared = app.world().resource_ref::<GameplayBanner>().last_changed();
        app.update();
        assert_eq!(app.world().resource_ref::<GameplayBanner>().last_changed(), cleared);
    }

    /// A room's banner shows only to the players of that room, and runs down
    /// like the banner every player sees.
    #[test]
    fn a_room_s_banner_shows_only_in_that_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
        let (here, there) = (LiveRoomInstance::from_ordinal(1), LiveRoomInstance::from_ordinal(2));
        let mut banner = GameplayBanner::default();
        banner.show_in(Some(here), "opened chest", 1.0);
        assert_eq!(banner.visible_in(Some(here)), Some("opened chest"));
        assert_eq!(banner.visible_in(Some(there)), None, "another room's player saw it");
        assert_eq!(banner.visible_in(None), None);
        banner.show_in(None, "SANDBOX RESET", 2.0);
        assert_eq!(banner.visible_in(Some(there)), Some("SANDBOX RESET"), "a banner with no room shows to every player");
        assert_eq!(banner.visible_in(Some(here)), Some("opened chest"), "the room's own first");
        assert_eq!(banner.shown().count(), 2);
        banner.tick(1.5);
        assert_eq!(banner.visible_in(Some(here)), Some("SANDBOX RESET"));
        assert!(banner.rooms.is_empty(), "a room's banner run down is forgotten");
        banner.tick(1.0);
        assert!(!banner.is_up());
    }
}
