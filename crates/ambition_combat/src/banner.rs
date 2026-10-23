//! Gameplay banner ticking and deferred-request application.

use super::*;

/// Run condition of [`tick_gameplay_banner`]: the banner shows or holds a
/// timer to run down.
pub fn the_banner_is_up(banner: Res<GameplayBanner>) -> bool {
    banner.timer > 0.0 || !banner.text.is_empty()
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
        banner.show(request.text.clone(), request.duration);
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
}
