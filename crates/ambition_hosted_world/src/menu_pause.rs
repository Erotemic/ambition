//! Pause votes use the host's connection and seat assignment.

use std::collections::BTreeMap;

use ambition_platformer2d_shared_tangle::lifecycle::{
    LiveSessionScope, LocalMenuPauseVotes, MenuPauseRoster, MenuPauseRosterPublished,
    NetworkMenuPause,
};
use ambition_platformer2d_shared_tangle::schedule::{
    GameMode, GameplaySimulationRoot, SimScheduleExt,
};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{GuestSeat, HostedRole, InputFromSeat, LocalSeat};

#[derive(Message, Clone, Copy, Serialize, Deserialize)]
struct GuestPauseVote {
    epoch: u64,
    sequence: u64,
    requested: bool,
}

#[derive(Message, Clone, Copy, Serialize, Deserialize)]
struct PauseAuthority {
    epoch: u64,
    paused: bool,
}

#[derive(Resource, Default)]
struct HostPause {
    roster: MenuPauseRoster,
    seats: BTreeMap<u8, Entity>,
    votes: BTreeMap<Entity, GuestPauseVote>,
    epoch: u64,
    paused: bool,
}

#[derive(Resource, Default)]
struct GuestPause {
    authority: Option<PauseAuthority>,
    sent: Option<GuestPauseVote>,
}

pub(crate) fn install(app: &mut App, role: HostedRole) {
    app.insert_resource(NetworkMenuPause)
        .add_client_message::<GuestPauseVote>(Channel::Ordered)
        .add_server_message::<PauseAuthority>(Channel::Ordered);
    let schedule = app.sim_schedule();
    match role {
        HostedRole::Host => {
            app.init_resource::<HostPause>()
                .add_systems(
                    PreUpdate,
                    update_host_pause
                        .after(ServerSystems::Receive)
                        .after(MenuPauseRosterPublished),
                )
                .configure_sets(schedule, GameplaySimulationRoot.run_if(host_running));
        }
        HostedRole::Guest => {
            app.init_resource::<GuestPause>()
                .add_systems(
                    PreUpdate,
                    take_pause_authority.after(ClientSystems::Receive),
                )
                .add_systems(PostUpdate, send_pause_vote.before(ClientSystems::Send))
                .add_systems(
                    OnExit(ClientState::Connected),
                    |mut pause: ResMut<GuestPause>| *pause = GuestPause::default(),
                )
                .configure_sets(
                    schedule,
                    GameplaySimulationRoot.run_if(|pause: Res<GuestPause>| {
                        !pause.authority.is_some_and(|state| state.paused)
                    }),
                );
        }
    }
}

fn host_running(pause: Res<HostPause>) -> bool {
    !pause.paused
}

fn update_host_pause(
    roster: Option<Res<MenuPauseRoster>>,
    local: Option<Res<LocalMenuPauseVotes>>,
    mode: Option<Res<State<GameMode>>>,
    assigned: Query<(Entity, &GuestSeat), With<AuthorizedClient>>,
    mut received: MessageReader<FromClient<GuestPauseVote>>,
    mut send: MessageWriter<ToClients<PauseAuthority>>,
    mut pause: ResMut<HostPause>,
) {
    let Some(roster) = roster.as_deref() else {
        return;
    };
    let seats = assigned
        .iter()
        .map(|(client, seat)| (seat.0.0, client))
        .collect();
    let changed = pause.roster != *roster || pause.seats != seats;
    if changed {
        pause.epoch += 1;
        pause.votes.clear();
        pause.roster = roster.clone();
        pause.seats = seats;
    }
    for received in received.read() {
        let vote = received.message;
        let Some(client) = received.client_id.entity() else {
            continue;
        };
        if vote.epoch == pause.epoch
            && pause.seats.values().any(|owner| *owner == client)
            && pause
                .votes
                .get(&client)
                .is_none_or(|previous| vote.sequence > previous.sequence)
        {
            pause.votes.insert(client, vote);
        }
    }
    let pausable = mode.is_none_or(|mode| {
        matches!(
            mode.get(),
            GameMode::Playing | GameMode::Dialogue | GameMode::Cutscene
        )
    });
    let paused = pausable
        && roster.session_pause(|slot| {
            if let Some(client) = pause.seats.get(&slot) {
                return pause.votes.get(client).is_some_and(|vote| vote.requested);
            }
            roster
                .scope
                .zip(local.as_deref())
                .is_some_and(|(scope, local)| local.requested(scope, slot))
        });
    if changed || paused != pause.paused {
        pause.paused = paused;
        for &client in pause.seats.values() {
            send.write(ToClients {
                targets: SendTargets::Single(client.into()),
                message: PauseAuthority {
                    epoch: pause.epoch,
                    paused,
                },
            });
        }
    }
}

fn take_pause_authority(
    mut messages: MessageReader<PauseAuthority>,
    mut pause: ResMut<GuestPause>,
) {
    for &state in messages.read() {
        if pause
            .authority
            .is_none_or(|previous| previous.epoch <= state.epoch)
        {
            pause.authority = Some(state);
        }
    }
}

fn send_pause_vote(
    scope: LiveSessionScope,
    local: Option<Res<LocalMenuPauseVotes>>,
    seat: Option<Res<LocalSeat>>,
    source: Option<Res<InputFromSeat>>,
    mut pause: ResMut<GuestPause>,
    mut send: MessageWriter<GuestPauseVote>,
) {
    let (Some(state), Some(seat)) = (pause.authority, seat) else {
        return;
    };
    let channel = source.map_or(seat.0, |source| source.0);
    let requested = scope
        .get()
        .zip(local.as_deref())
        .is_some_and(|(scope, local)| local.requested(scope, channel.0));
    let previous = pause.sent.filter(|vote| vote.epoch == state.epoch);
    if previous.is_some_and(|vote| vote.requested == requested) {
        return;
    }
    let vote = GuestPauseVote {
        epoch: state.epoch,
        sequence: previous.map_or(0, |vote| vote.sequence + 1),
        requested,
    };
    send.write(vote);
    pause.sent = Some(vote);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::control::PlayerSlot;
    use ambition_platformer2d_shared_tangle::lifecycle::{
        SessionRoot, SessionScopeId, SimulationIsland,
    };
    use bevy_replicon::test_app::{ServerTestAppExt, TestClientEntity};

    #[derive(Resource, Default)]
    struct Steps(u32);

    #[test]
    fn resume_reaches_the_host_while_both_gameplay_schedules_are_held() {
        let mut host = App::new();
        let mut guest = App::new();
        for (app, role) in [
            (&mut host, HostedRole::Host),
            (&mut guest, HostedRole::Guest),
        ] {
            app.add_plugins((
                MinimalPlugins,
                bevy::state::app::StatesPlugin,
                RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
            ));
            app.set_sim_schedule(Update);
            install(app, role);
            app.init_resource::<LocalMenuPauseVotes>()
                .init_resource::<Steps>()
                .add_systems(
                    Update,
                    (|mut steps: ResMut<Steps>| steps.0 += 1).in_set(GameplaySimulationRoot),
                );
            app.finish();
            app.cleanup();
        }
        let scope = SessionScopeId(1);
        host.insert_resource(MenuPauseRoster {
            scope: Some(scope),
            members: vec![
                (0, Some(SimulationIsland::Session(scope))),
                (1, Some(SimulationIsland::Session(scope))),
            ],
        });
        let guest_scope = SessionScopeId(8);
        guest.world_mut().spawn(SessionRoot(guest_scope));
        guest
            .insert_resource(LocalSeat(PlayerSlot(1)))
            .insert_resource(InputFromSeat(PlayerSlot(0)));
        host.connect_client(&mut guest);
        let client = **guest.world().resource::<TestClientEntity>();
        host.world_mut()
            .entity_mut(client)
            .insert(GuestSeat(PlayerSlot(1)));
        let advance = |host: &mut App, guest: &mut App| {
            for _ in 0..6 {
                host.update();
                host.exchange_with_client(guest);
                guest.update();
            }
        };
        host.world_mut()
            .resource_mut::<LocalMenuPauseVotes>()
            .publish(Some(scope), [0]);
        advance(&mut host, &mut guest);
        let before = host.world().resource::<Steps>().0;
        guest
            .world_mut()
            .resource_mut::<LocalMenuPauseVotes>()
            .publish(Some(guest_scope), [0]);
        advance(&mut host, &mut guest);
        let frozen = [
            host.world().resource::<Steps>().0,
            guest.world().resource::<Steps>().0,
        ];
        assert!(frozen[0] > before);
        advance(&mut host, &mut guest);
        assert_eq!(
            [
                host.world().resource::<Steps>().0,
                guest.world().resource::<Steps>().0
            ],
            frozen
        );

        guest
            .world_mut()
            .resource_mut::<LocalMenuPauseVotes>()
            .publish(Some(guest_scope), []);
        advance(&mut host, &mut guest);
        assert!(host.world().resource::<Steps>().0 > frozen[0]);
        assert!(guest.world().resource::<Steps>().0 > frozen[1]);
    }

    #[test]
    fn only_the_current_guest_can_pause_and_release_the_host_tick() {
        let mut app = App::new();
        let scope = SessionScopeId(1);
        let island = SimulationIsland::Session(scope);
        app.insert_resource(MenuPauseRoster {
            scope: Some(scope),
            members: vec![(0, Some(island)), (1, Some(island))],
        })
        .init_resource::<LocalMenuPauseVotes>()
        .init_resource::<HostPause>()
        .init_resource::<Steps>()
        .add_message::<FromClient<GuestPauseVote>>()
        .add_message::<ToClients<PauseAuthority>>()
        .add_systems(PreUpdate, update_host_pause)
        .add_systems(
            Update,
            (|mut steps: ResMut<Steps>| steps.0 += 1).run_if(host_running),
        );
        let guest = app
            .world_mut()
            .spawn((AuthorizedClient, GuestSeat(PlayerSlot(1))))
            .id();
        let stranger = app.world_mut().spawn(GuestSeat(PlayerSlot(1))).id();
        app.world_mut()
            .resource_mut::<LocalMenuPauseVotes>()
            .publish(Some(scope), [0]);
        app.update();
        let epoch = app.world().resource::<HostPause>().epoch;
        let ballot = |client: Entity, epoch, sequence, requested| FromClient {
            client_id: client.into(),
            message: GuestPauseVote {
                epoch,
                sequence,
                requested,
            },
        };

        app.world_mut()
            .write_message(ballot(stranger, epoch, 0, true));
        app.update();
        assert_eq!(app.world().resource::<Steps>().0, 2);
        app.world_mut().write_message(ballot(guest, epoch, 1, true));
        app.update();
        assert_eq!(app.world().resource::<Steps>().0, 2);
        app.world_mut()
            .write_message(ballot(guest, epoch, 0, false));
        app.update();
        assert_eq!(app.world().resource::<Steps>().0, 2);
        app.world_mut()
            .write_message(ballot(guest, epoch, 2, false));
        app.update();
        assert_eq!(app.world().resource::<Steps>().0, 3);

        app.world_mut().despawn(guest);
        let replacement = app
            .world_mut()
            .spawn((AuthorizedClient, GuestSeat(PlayerSlot(1))))
            .id();
        app.world_mut()
            .write_message(ballot(replacement, epoch, 3, true));
        app.update();
        assert_eq!(app.world().resource::<Steps>().0, 4);
    }
}
