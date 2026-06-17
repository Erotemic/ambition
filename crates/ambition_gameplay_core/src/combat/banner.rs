//! Gameplay banner ticking and deferred-request application.

use super::*;

/// Tick the gameplay banner resource once per frame.
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
