//! Content-free entity markers shared by reusable mechanics and presentation.

use bevy::prelude::*;

/// Marks a body in the player population; zero or many may exist.
///
/// Generic simulation should use body capabilities/control authority instead.
/// Use [`PrimaryPlayer`] for home-avatar policy and [`ControlledSubject`] for the
/// body currently driven by primary local control.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerEntity;

/// Entity currently driven by the primary local control authority.
///
/// `None` is valid while no primary controlled subject is resolved.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlledSubject(pub Option<Entity>);

/// Ordered bodies to frame when no local authority drives a subject.
///
/// The session/match publishes this list; the camera does not infer which bodies
/// belong in the frame. Empty means no cast has been declared.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct FramedCast(pub Vec<Entity>);

/// Marks the exploration home body used by primary-player save/respawn policy.
///
/// Control may move elsewhere; use [`ControlledSubject`] for current control
/// authority. This is also distinct from [`crate::body::PrimaryBody`], which
/// selects live room-gravity resolution.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrimaryPlayer;

/// Query filter for a body carrying both [`PlayerEntity`] and [`PrimaryPlayer`].
pub type PrimaryPlayerOnly = (With<PlayerEntity>, With<PrimaryPlayer>);

/// Query filter for a body that collects a touched pickup: the player
/// population, or any body currently driven through possession.
///
/// ⭐ IT LIVES HERE, NOT IN THE ACTOR KERNEL, so a consumer BELOW the kernel can
/// name it. The filter is composed of nothing but [`PlayerEntity`] and
/// [`TemporaryControl`](crate::temporary_control::TemporaryControl), both of
/// which are already in this crate — while it sat in
/// `actor_monolith::features::ecs::pickups` it was the only reason the
/// world-item collect pass could not leave the kernel with it. Stating the rule
/// ONCE, here, is also what keeps it from being restated per system.
///
/// ⚠ `PlayerEntity` remains sufficient on its own: a player body whose brain is
/// temporarily absent still collects.
pub type TouchCollectorFilter = bevy::prelude::Or<(
    With<PlayerEntity>,
    With<crate::temporary_control::TemporaryControl>,
)>;

/// The VALUE half of [`TouchCollectorFilter`], for a pass that has already
/// fetched the two facts and cannot re-ask the query.
///
/// ⛔ THE TWO MUST AGREE, which is why they are adjacent: the filter decides who
/// a query RETURNS and this decides whether a returned body actually collects,
/// and a pass that mixes one rule with the other's population silently changes
/// who picks things up. `PlayerEntity` (here, `in_player_population`) is
/// sufficient on its own — a player body whose brain is temporarily absent still
/// collects.
pub fn body_collects_on_touch(
    in_player_population: bool,
    control: Option<&crate::temporary_control::TemporaryControl>,
) -> bool {
    in_player_population
        || matches!(
            control,
            Some(crate::temporary_control::TemporaryControl::Player { .. })
        )
}
