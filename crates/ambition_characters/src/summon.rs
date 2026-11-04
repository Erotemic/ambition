//! Requests and data for constructing a summoned character.

use crate::actor::ActorFaction;
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

/// An authored summon specification. The construction owner materializes it.
pub struct SummonSpec {
    pub id: String,
    pub pos: ae::Vec2,
    pub half_size: ae::Vec2,
    pub character_id: String,
    pub encounter_id: String,
    pub faction: ActorFaction,
    /// What the body is summoned FOR, which decides how it behaves once it
    /// exists. See [`SummonCalling`].
    pub calling: SummonCalling,
    /// The summoner boards what it just made, if the pair is legal.
    ///
    /// This is on the summon, not a follow-up, because only the executor's
    /// exclusive command holds both the new entity and its summoner. A later
    /// board would need a channel back and would find a mount that already
    /// simulated a tick without a rider.
    ///
    /// It is a request: `mount::board` refuses an illegal pair, and then the
    /// entity spawns with no rider. `None` means no ride (every boss minion).
    ///
    /// It carries the ride length so the lease and the board are one transaction.
    /// A refused board must not leave a lease on a body that rides nothing.
    pub ridden_by_summoner: Option<SummonedRide>,
    /// Health for this occurrence, overriding the character's authored vitals.
    ///
    /// The same creature can need different health in different games: a summon
    /// used as a recovery must survive a stray hit in a fighter whose moves deal
    /// 2 to 17. This is on the summon, not the character, so other games keep
    /// the authored value. `None` keeps the character's own vitals.
    pub health: Option<u32>,
    /// Whether this occurrence keeps the character's authored contact hazard.
    ///
    /// "Neutral" is not "harmless": `damage_lands` is true for `Foe | Neutral`,
    /// so an opponent can gimp a recovery. Without this flag, a neutral summon
    /// avoids contact damage only because it acquires no target. `true` keeps the
    /// character's own answer.
    pub keeps_contact_damage: bool,
    /// The statuses the body holds from birth (`ambition.body.hold_status`
    /// kinds), so the module that summoned it ticks it. Empty for none.
    pub holds: Vec<String>,
}

/// What a summoned body is FOR.
///
/// A summon names its calling instead of its aggression or a marker type: the
/// emitter states intent and the executor, which builds the body, decides what
/// that intent is made of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize)]
pub enum SummonCalling {
    /// Called into its summoner's fight (a boss's minion, a ride). It fights
    /// along faction lines and knows where its quarry is.
    #[default]
    Fight,
    /// A companion at its summoner's side (the puppy-slug gun's slug). It wanders
    /// rather than hunts, and is marked so its summoner can count it. It has no
    /// encounter to be stable in, so [`SummonSpec::id`] names its kind and the
    /// executor names the body under the identity it derives for it.
    Companion,
}

/// The summoner rides what it makes, for this long.
///
/// Seconds rather than ticks, because the lease it becomes counts on the sim
/// clock like every other gameplay countdown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SummonedRide {
    /// How long the ride lasts once the summoner is aboard.
    pub seconds: f32,
    /// A distance, not a flag: "it appears underneath me" and "it flies in from
    /// off-screen" are the same request with different numbers. A mount summoned
    /// at the rider's position satisfies this on its first tick. See
    /// `ambition_mount::MountReservedFor`.
    pub board_within: f32,
    /// How long the mount waits to be reached before giving up and telling the
    /// ruleset. See `ambition_mount::MountReservedFor::expires_in`.
    pub board_deadline_s: f32,
}

/// A summon request. The construction owner validates the live summoner.
#[derive(Message)]
pub struct SummonRequest {
    pub owner: Entity,
    pub spec: SummonSpec,
}
