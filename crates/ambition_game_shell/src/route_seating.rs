//! The local seats a route offers, stated by the route's author.
//!
//! A route states its seats once, with [`RouteSeatingAppExt::declare_route_seating`].
//! The shell then projects the active route's statement into the input crate's
//! [`LocalSeatOffer`], and into [`SessionSeatingSource`] when the route decides
//! its channels before the session is sized. No route claims or releases the
//! offer itself, so a route that is left cannot leave its offer behind.

use std::collections::BTreeMap;

use bevy::prelude::{App, Res, ResMut, Resource};

use ambition_input::{
    InputAssignmentPolicy, LocalChannelPlan, LocalDeviceOrder, LocalSeatOffer,
    SessionSeatingSource,
};

use crate::{ShellRouteId, ShellRouter};

/// How many local seats a route offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeatCount {
    /// Exactly this many. `0` offers none but keeps the route's policy.
    Fixed(u8),
    /// One per local source that can claim a seat under the route's policy,
    /// at least one and at most `max`.
    OnePerSource { max: u8 },
}

/// What a route offers local players while it is the active route.
#[derive(Clone, Debug, PartialEq)]
pub struct RouteSeating {
    pub seats: SeatCount,
    pub policy: InputAssignmentPolicy,
    /// A device-to-channel plan the route decides before its session is sized,
    /// for a route with a fixed cast of local players.
    pub channels: Option<LocalChannelPlan>,
}

impl RouteSeating {
    pub fn new(seats: SeatCount, policy: InputAssignmentPolicy) -> Self {
        Self {
            seats,
            policy,
            channels: None,
        }
    }

    pub fn with_channels(mut self, channels: LocalChannelPlan) -> Self {
        self.channels = Some(channels);
        self
    }

    fn seats_offered(&self, devices: Option<&LocalDeviceOrder>) -> u8 {
        match self.seats {
            SeatCount::Fixed(seats) => seats,
            SeatCount::OnePerSource { max } => {
                let pads = devices.map_or(0, |devices| match self.policy {
                    // A session with no declared plan gives each connected
                    // pad a channel, counted from the first connected pad.
                    InputAssignmentPolicy::UnifiedPrimary => devices.connected().len(),
                    // A seat number names a pad slot. A pad above an empty
                    // slot keeps its number, so its seat stays on offer.
                    InputAssignmentPolicy::JoinToClaim
                    | InputAssignmentPolicy::ExplicitAssignment => devices.span(),
                });
                let sources = self.policy.sources_that_can_claim(pads);
                sources.clamp(1, usize::from(max.max(1))) as u8
            }
        }
    }
}

/// Every route's seating statement. Authored constants: written when plugins
/// are built, and read by [`project_route_seating`].
#[derive(Resource, Clone, Debug, Default)]
pub struct RouteSeatingCatalog {
    by_route: BTreeMap<ShellRouteId, RouteSeating>,
}

impl RouteSeatingCatalog {
    pub fn get(&self, route: &ShellRouteId) -> Option<&RouteSeating> {
        self.by_route.get(route)
    }

    /// Whether a declared route made the value `owner` names.
    fn owns(&self, owner: Option<&str>) -> bool {
        owner
            .and_then(|owner| owner.strip_prefix(ROUTE_OWNER_PREFIX))
            .is_some_and(|route| self.by_route.contains_key(&ShellRouteId::new(route)))
    }
}

/// Seat values this projection writes are owned by `route:<id>`, so a route id
/// can never be read as an experience's own claim.
const ROUTE_OWNER_PREFIX: &str = "route:";

/// The owner name the projection writes for `route`.
pub fn route_seating_owner(route: &ShellRouteId) -> String {
    format!("{ROUTE_OWNER_PREFIX}{}", route.as_str())
}

pub trait RouteSeatingAppExt {
    /// "While this route is active, it offers these local seats."
    fn declare_route_seating(
        &mut self,
        route: impl Into<ShellRouteId>,
        seating: RouteSeating,
    ) -> &mut Self;
}

impl RouteSeatingAppExt for App {
    fn declare_route_seating(
        &mut self,
        route: impl Into<ShellRouteId>,
        seating: RouteSeating,
    ) -> &mut Self {
        self.init_resource::<LocalSeatOffer>();
        self.init_resource::<SessionSeatingSource>();
        self.world_mut()
            .get_resource_or_insert_with(RouteSeatingCatalog::default)
            .by_route
            .insert(route.into(), seating);
        self
    }
}

/// Project the active route's seating into the input crate's seat resources.
///
/// Each projected value is owned by [`route_seating_owner`], so the projection
/// replaces only a value that a declared route made. An offer that no route
/// made (a test, a tool) stands until a declaring route becomes active.
pub fn project_route_seating(
    router: Res<ShellRouter>,
    catalog: Option<Res<RouteSeatingCatalog>>,
    devices: Option<Res<LocalDeviceOrder>>,
    offer: Option<ResMut<LocalSeatOffer>>,
    seating: Option<ResMut<SessionSeatingSource>>,
) {
    let Some(catalog) = catalog else {
        return;
    };
    let active = router
        .active
        .as_ref()
        .and_then(|active| Some((&active.route_id, catalog.get(&active.route_id)?)));

    if let Some(mut offer) = offer {
        match active {
            Some((route, declared)) => {
                let next = LocalSeatOffer::offered(
                    route_seating_owner(route),
                    declared.seats_offered(devices.as_deref()),
                    declared.policy,
                );
                if *offer != next {
                    *offer = next;
                }
            }
            None if catalog.owns(offer.owner()) => *offer = LocalSeatOffer::default(),
            None => {}
        }
    }

    if let Some(mut seating) = seating {
        match active.and_then(|(route, declared)| Some((route, declared.channels.as_ref()?))) {
            Some((route, plan)) => {
                let owner = route_seating_owner(route);
                if !seating.is_owned_by(&owner) || seating.channel_plan() != Some(plan) {
                    *seating = SessionSeatingSource::decided(owner, plan.clone());
                }
            }
            None => {
                if let Some(owner) = seating.owner().map(str::to_owned) {
                    if catalog.owns(Some(&owner)) {
                        seating.release(&owner);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{Entity, Update};

    use ambition_input::LocalInputSource;

    use super::*;
    use crate::{ActiveShellExperience, ShellActivationId, ShellExperienceId};

    const LOBBY: &str = "lobby";
    const PLAZA: &str = "plaza";
    const ELSEWHERE: &str = "elsewhere";

    fn plaza_channels() -> LocalChannelPlan {
        LocalChannelPlan::from_sources([LocalInputSource::Keyboard, LocalInputSource::FIRST_PAD])
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<ShellRouter>()
            .declare_route_seating(
                LOBBY,
                RouteSeating::new(
                    SeatCount::OnePerSource { max: 4 },
                    InputAssignmentPolicy::JoinToClaim,
                ),
            )
            .declare_route_seating(
                PLAZA,
                RouteSeating::new(SeatCount::Fixed(2), InputAssignmentPolicy::JoinToClaim)
                    .with_channels(plaza_channels()),
            )
            .add_systems(Update, project_route_seating);
        app
    }

    fn enter(app: &mut App, route: Option<&str>) {
        app.world_mut().resource_mut::<ShellRouter>().active =
            route.map(|route| ActiveShellExperience {
                activation_id: ShellActivationId(1),
                route_id: ShellRouteId::new(route),
                experience_id: ShellExperienceId::new(route),
                parameters: Default::default(),
                load_authorization: None,
                prepared_session: None,
            });
        app.update();
    }

    fn offer(app: &App) -> LocalSeatOffer {
        app.world().resource::<LocalSeatOffer>().clone()
    }

    fn seating(app: &App) -> SessionSeatingSource {
        app.world().resource::<SessionSeatingSource>().clone()
    }

    /// While a declared route is active, the offer and the channel plan are the
    /// route's statement, in the route's name.
    #[test]
    fn the_active_route_states_its_seats_and_its_channels() {
        let mut app = app();
        let owner = route_seating_owner(&ShellRouteId::new(PLAZA));
        // An equal value in another name: the projection still takes it over,
        // or the other owner could withdraw the plaza's seats.
        app.insert_resource(LocalSeatOffer::offered(
            "someone else",
            2,
            InputAssignmentPolicy::JoinToClaim,
        ));
        enter(&mut app, Some(PLAZA));
        assert_eq!(
            offer(&app),
            LocalSeatOffer::offered(owner.clone(), 2, InputAssignmentPolicy::JoinToClaim),
        );
        assert_eq!(seating(&app), SessionSeatingSource::decided(owner, plaza_channels()));
    }

    /// Leaving a declared route withdraws what it stated. A value another owner
    /// wrote stands, also when its numbers are the route's own.
    #[test]
    fn leaving_withdraws_only_what_a_route_stated() {
        let mut app = app();
        enter(&mut app, Some(PLAZA));
        enter(&mut app, Some(ELSEWHERE));
        assert_eq!(offer(&app), LocalSeatOffer::default());
        assert_eq!(seating(&app).channel_plan(), None);

        let theirs = LocalSeatOffer::offered("someone else", 2, InputAssignmentPolicy::JoinToClaim);
        let their_plan = SessionSeatingSource::decided("someone else", plaza_channels());
        app.insert_resource(theirs.clone());
        app.insert_resource(their_plan.clone());
        enter(&mut app, None);
        assert_eq!(offer(&app), theirs);
        assert_eq!(seating(&app), their_plan);
    }

    /// Moving between two declared routes replaces one statement with the
    /// other, and a route with no channel plan gives the other's plan back.
    #[test]
    fn a_route_with_no_channels_gives_the_previous_routes_plan_back() {
        let mut app = app();
        enter(&mut app, Some(PLAZA));
        enter(&mut app, Some(LOBBY));
        assert_eq!(
            offer(&app).owner(),
            Some(route_seating_owner(&ShellRouteId::new(LOBBY)).as_str())
        );
        assert_eq!(seating(&app).channel_plan(), None);
    }

    /// One seat per source that can join: the keyboard and each pad under
    /// `JoinToClaim`, at least one, at most the route's ceiling.
    #[test]
    fn one_seat_per_source_counts_the_keyboard_and_each_pad_up_to_the_ceiling() {
        let mut app = app();
        let mut world = bevy::prelude::World::new();
        let pads: Vec<Entity> = (0..5).map(|_| world.spawn_empty().id()).collect();
        let mut seats = Vec::new();
        for connected in [0, 1, 2, 5] {
            app.insert_resource(LocalDeviceOrder::from_devices(pads[..connected].to_vec()));
            enter(&mut app, Some(LOBBY));
            seats.push(offer(&app).seats());
        }
        assert_eq!(seats, [1, 2, 3, 4]);

        // With no device order the keyboard alone still gets a seat.
        app.world_mut().remove_resource::<LocalDeviceOrder>();
        enter(&mut app, Some(LOBBY));
        assert_eq!(offer(&app).seats(), 1);

        // Under `UnifiedPrimary` the keyboard shares the first pad's seat.
        let unified = RouteSeating::new(
            SeatCount::OnePerSource { max: 4 },
            InputAssignmentPolicy::UnifiedPrimary,
        );
        let counts: Vec<u8> = [0, 1, 2]
            .map(|connected| {
                unified.seats_offered(Some(&LocalDeviceOrder::from_devices(
                    pads[..connected].to_vec(),
                )))
            })
            .to_vec();
        assert_eq!(counts, [1, 1, 2]);
    }
}
