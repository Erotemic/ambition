//! A provider whose world cannot be built refuses, and the session's
//! preparation fails with the reasons; the game keeps running.
//!
//! A demo world is read off disk in a development build (`content_text!`), so
//! a bad save reaches a running game with no build in between. Mary-O's
//! source refuses through this road (`try_mary_o_session_world_entering`).

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::game_shell::{ShellCommand, ShellCommandRejection, ShellEvent, ShellRouteId};
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring, WorldRefusal};
use ambition_platformer2d::runtime::PreparedPlatformerSource;
use bevy::prelude::*;

const ROUTE: &str = "refusing_world_gameplay";
const REASON: &str = "probe.ldtk: the pipe `descnet` has no partner";

/// The developer detail of each preparation the shell reports as failed.
#[derive(Resource, Default)]
struct FailedPreparations(Vec<String>);

fn record_failed_preparations(mut events: MessageReader<ShellEvent>, mut seen: ResMut<FailedPreparations>) {
    for event in events.read() {
        if let ShellEvent::CommandRejected(ShellCommandRejection::LoadFailed { failures, .. }) = event {
            seen.0.extend(failures.iter().map(|failure| failure.developer_detail.clone()));
        }
    }
}

#[test]
fn a_world_that_cannot_be_built_fails_its_preparation_and_the_game_keeps_running() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    PlatformerExperienceAuthoring::new(
        "refusing_world",
        ROUTE,
        "Refusing world",
        "A provider whose world file does not load",
        "Prepare refusing world",
        AuthoredCatalogFragments::new("player_robot_v3", "refusing_world"),
    )
    .install(&mut app, || -> Result<PreparedPlatformerSource, WorldRefusal> {
        Err(WorldRefusal::new([REASON]))
    });
    app.init_resource::<FailedPreparations>().add_systems(Update, record_failed_preparations);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::GoTo(ShellRouteId::new(ROUTE)));
    for _ in 0..120 {
        app.update();
        if !app.world().resource::<FailedPreparations>().0.is_empty() {
            break;
        }
    }
    // The game keeps running after the refusal.
    for _ in 0..10 {
        app.update();
    }
    let failed = &app.world().resource::<FailedPreparations>().0;
    assert!(
        failed.iter().any(|detail| detail.contains(REASON)),
        "the refusal did not fail the preparation with its reason: {failed:?}"
    );
}
