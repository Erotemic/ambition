//! A simulation message does not cross a session boundary.
//!
//! A simulation system that writes a message after the reader of that message
//! ran leaves the message on the bus for the next tick. When the session ends
//! there, the next tick is the first tick of another session, and that session
//! reads the message. Measured 2026-10-04 over `app_it`: at 10 of the 59 first
//! ticks of a later session, one `ClockScaleRequest` and one
//! `ActorActionMessage` of the session that ended were still on the bus. Both
//! are written every tick. A clock request of a session that ended in a
//! hitstop is `scale: 0.0`, and the next session then starts with a tick of no
//! time on this host only.
//!
//! ⭐ ONE OWNER, ONE LIST. The channels are those a domain declares to the
//! rollback census (`clear_message_on_rollback`): a channel that must not
//! cross a rewind must not cross a session. Each rollback registrar declares
//! its channels here also, so there is no second list to keep equal to the
//! first one.
//!
//! ⭐ EMPTIED AT ACTIVATION, AS THE OTHER SESSION STATE IS
//! ([`SessionScopeSet::Activate`]). That edge is before a provider builds the
//! world of the new session, so the messages the new session writes while it
//! is built are not taken. A reader does not drain its own bus when it refuses
//! for want of a session: that would be one rule in each reader.
//!
//! ⛔ A CHANNEL THAT PRESENTATION READS IS KEPT
//! ([`keep_message_across_session_activation`]). The host writes to such a
//! channel also: the sound of the menu row that started the session is on the
//! sound bus at the activation (measured, `participant_input`). A message of
//! the session that ended does no harm there, because no simulation reads it.

use std::any::TypeId;
use std::collections::BTreeSet;

use bevy::ecs::message::{Message, Messages};
use bevy::prelude::*;

use super::{SessionScopeActivated, SessionScopeSet};

/// The simulation message channels that are emptied when a session is
/// activated.
#[derive(Resource, Default)]
pub struct SessionMessageChannels {
    declared: BTreeSet<TypeId>,
    kept: BTreeSet<TypeId>,
    clear: Vec<(TypeId, fn(&mut World))>,
}

impl SessionMessageChannels {
    /// True when an activation empties the channel `T`.
    pub fn clears<T: Message>(&self) -> bool {
        let channel = TypeId::of::<T>();
        self.declared.contains(&channel) && !self.kept.contains(&channel)
    }
}

/// Declare that `T` carries simulation facts of one session.
///
/// A second declaration of the same channel changes nothing.
pub fn clear_message_at_session_activation<T: Message>(app: &mut App) {
    let mut channels = app
        .world_mut()
        .get_resource_or_init::<SessionMessageChannels>();
    if channels.declared.insert(TypeId::of::<T>()) {
        channels.clear.push((TypeId::of::<T>(), |world| {
            if let Some(mut messages) = world.get_resource_mut::<Messages<T>>() {
                messages.clear();
            }
        }));
    }
}

/// Declare that presentation reads `T`, so an activation does not empty it.
///
/// The order of this declaration and [`clear_message_at_session_activation`]
/// does not matter.
pub fn keep_message_across_session_activation<T: Message>(app: &mut App) {
    app.world_mut()
        .get_resource_or_init::<SessionMessageChannels>()
        .kept
        .insert(TypeId::of::<T>());
}

/// Empty every declared channel when a session is activated.
///
/// The clear is a queued command. It lands at the sync point between
/// [`SessionScopeSet::Activate`] and the providers, as the other activation
/// resets do.
pub(super) fn clear_session_message_channels(
    mut activated: MessageReader<SessionScopeActivated>,
    mut commands: Commands,
) {
    if activated.read().count() == 0 {
        return;
    }
    commands.queue(|world: &mut World| {
        let Some(channels) = world.get_resource::<SessionMessageChannels>() else {
            return;
        };
        let clears: Vec<fn(&mut World)> = channels
            .clear
            .iter()
            .filter(|(channel, _)| !channels.kept.contains(channel))
            .map(|(_, clear)| *clear)
            .collect();
        for clear in clears {
            clear(world);
        }
    });
}

/// Install the clear on the activation edge.
pub(super) fn install(app: &mut App) {
    app.init_resource::<SessionMessageChannels>().add_systems(
        Update,
        clear_session_message_channels.in_set(SessionScopeSet::Activate),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::{SessionScopeId, SessionScopePlugin};
    use bevy::ecs::schedule::common_conditions::run_once;

    #[derive(Message, Clone, Copy)]
    struct Request;

    #[derive(Resource, Default)]
    struct Served(usize);

    fn serve(mut requests: MessageReader<Request>, mut served: ResMut<Served>) {
        served.0 += requests.read().count();
    }

    fn ask(mut requests: MessageWriter<Request>) {
        requests.write(Request);
    }

    /// A host with the session seam and one channel. `declare` states what the
    /// host says about the channel.
    fn host(declare: fn(&mut App)) -> App {
        let mut app = App::new();
        app.add_plugins(SessionScopePlugin)
            .add_message::<Request>()
            .init_resource::<Served>();
        declare(&mut app);
        app
    }

    /// The request the old session wrote AFTER its reader ran: the reader does
    /// not see it in that update, and it is on the bus for the next one.
    fn leave_a_request_behind(app: &mut App) {
        app.add_systems(
            Update,
            (serve, ask.run_if(run_once))
                .chain()
                .after(SessionScopeSet::Activate),
        );
        app.update();
        assert_eq!(app.world().resource::<Served>().0, 0, "the reader ran after the writer");
        assert_eq!(app.world().resource::<Messages<Request>>().len(), 1);
    }

    fn activate(app: &mut App) {
        app.world_mut()
            .write_message(SessionScopeActivated(SessionScopeId(2)));
    }

    #[test]
    fn a_declared_channel_does_not_carry_a_request_into_the_next_session() {
        let served_after = |activated: bool| {
            let mut app = host(clear_message_at_session_activation::<Request>);
            leave_a_request_behind(&mut app);
            if activated {
                activate(&mut app);
            }
            app.update();
            app.world().resource::<Served>().0
        };
        assert_eq!(
            served_after(false),
            1,
            "with no activation the request is served on the next update: the \
             fixture does carry a message across two updates"
        );
        assert_eq!(
            served_after(true),
            0,
            "the next session served a request of the session that ended"
        );
    }

    #[test]
    fn a_channel_that_presentation_reads_is_kept() {
        // Declared in the two orders: the order does not matter.
        for declare in [
            (|app: &mut App| {
                clear_message_at_session_activation::<Request>(app);
                keep_message_across_session_activation::<Request>(app);
            }) as fn(&mut App),
            |app: &mut App| {
                keep_message_across_session_activation::<Request>(app);
                clear_message_at_session_activation::<Request>(app);
            },
        ] {
            let mut app = host(declare);
            assert!(!app.world().resource::<SessionMessageChannels>().clears::<Request>());
            leave_a_request_behind(&mut app);
            activate(&mut app);
            app.update();
            assert_eq!(
                app.world().resource::<Served>().0,
                1,
                "the activation took a message from a channel that presentation reads"
            );
        }
    }

    /// The new session writes its first messages while its world is built,
    /// after the activation edge in the same update, and its first tick reads
    /// them in a later update. The clear does not take them.
    #[test]
    fn a_message_the_new_session_writes_at_its_activation_is_not_taken() {
        let mut app = host(clear_message_at_session_activation::<Request>);
        activate(&mut app);
        leave_a_request_behind(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<Served>().0,
            1,
            "the clear took the message the new session wrote after its activation"
        );
    }
}
