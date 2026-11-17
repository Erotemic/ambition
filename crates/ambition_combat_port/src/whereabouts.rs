//! Where the things a scope's records name are (runtime authoring audit P5
//! step 3).
//!
//! A record keeps a reference between ticks: a swallow's coconut, a guard's
//! charge. What it names may have walked into another room, been put away
//! with its room, been eaten, or never come back. A reference that names
//! something absent from this tick's observation is not enough to tell those
//! apart, and they call for different decisions: wait for a thing that is
//! only away, forget a thing that is gone.

use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The observation port marker for the whereabouts of what a scope's records
/// name.
///
/// Port card:
///
/// * **Operation** — for each identity a reference field of the scope's
///   records holds (in any record, in any option or sequence), where the
///   world says it is now. Read as `this.observe.whereabouts.of[id]`.
/// * **Owner** — `ambition_platformer2d_runtime::whereabouts`, from the
///   owners of each fact: the live bodies and items, the occurrence ledger
///   (`AuthoredOccurrences`) and the save's death records.
/// * **Scope** — the invocation's scope: its records, and its live room.
/// * **Read model** — one of [`Standing`]'s words for each identity, as of
///   the start of the phase. An identity no record of the scope holds is not
///   listed.
pub struct WhereaboutsPort;

/// Where each identity the scope's records name is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Whereabouts {
    pub of: Vec<(String, String)>,
}

/// The words of [`Whereabouts::of`]. Each is a different decision for the
/// reader; nothing here is a status for its own sake.
pub enum Standing {}

impl Standing {
    /// Live, in the scope's own room.
    pub const HERE: &'static str = "here";
    /// Live, in another room.
    pub const AWAY: &'static str = "away";
    /// Live, in a body's hands: it goes where its carrier goes.
    pub const CARRIED: &'static str = "carried";
    /// Not live now, and the world will build it again: an occurrence put
    /// down in a room that is not loaded, or an authored one whose room is
    /// not loaded. The same identity comes back with its room.
    pub const DORMANT: &'static str = "dormant";
    /// The world remembers it gone for good: a consumed key, an opened chest,
    /// a recorded death.
    pub const ENDED: &'static str = "ended";
    /// Not live, and nothing will build it again: a runtime mint with no
    /// ledger row (a spent projectile, an add that died), or an identity the
    /// world never had.
    pub const GONE: &'static str = "gone";
}

impl Port for WhereaboutsPort {
    const KEY: PortKey = PortKey::new("ambition.world.whereabouts", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = Whereabouts;

    fn encode(v: &Whereabouts, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Whereabouts, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

ambition_extension_sdk::port_record! {
    Whereabouts {
        /// Identity → `here` (live, this room), `away` (live, another room), `carried` (in a body's hands), `dormant` (built again with its room), `ended` (remembered gone: eaten, opened, killed) or `gone` (nothing builds it again).
        of: Vec<(String, String)> as ambition_extension_sdk::structure::TextMap<String> = Vec::new(),
    }
}

ambition_extension_sdk::script_port!(WhereaboutsPort);
