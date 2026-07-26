//! **Three ways to stage a cast, one projection.** (§4.8)
//!
//! A room, a match, and a direct startup are semantically different things and
//! keep their own schemas — a room places NPCs at coordinates, a match seats
//! participants on teams, a startup spec names who you begin as. They are NOT
//! variants of one object, and deliberately so: the tempting move is a rich
//! universal `StagedCast` that every subsystem reads, and it would immediately
//! accumulate everyone's fields.
//!
//! What they genuinely share is one thing:
//!
//! ```text
//! RoomStagingPlan          ─┐
//! MatchParticipantRoster   ─┼─→ CharacterLoadDemand { tokens… }
//! DirectStartupSpec        ─┘
//! ```
//!
//! Because the projection is the only shared surface, transformations, summons,
//! assists, alternate forms, and a boss revealed mid-fight all arrive the same
//! way — by demanding more tokens later — with no new staging concept.

use super::CharacterLoadDemand;

/// Anything that knows which characters it needs art for.
///
/// The whole contract. An implementor does not learn what materialization is,
/// what an asset profile is, or when the reveal barrier opens.
pub trait StagesCharacters {
    /// Every character token this staging needs.
    fn character_tokens(&self) -> Vec<String>;

    /// Submit those tokens as demand.
    fn project_demand(&self, demand: &mut CharacterLoadDemand) {
        demand.request_all(self.character_tokens());
    }
}

/// What a ROOM stages: placement NPCs, authored enemies, staged actors.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoomStagingPlan {
    pub placement_characters: Vec<String>,
    pub enemy_names: Vec<String>,
    pub staged_actor_names: Vec<String>,
}

impl StagesCharacters for RoomStagingPlan {
    fn character_tokens(&self) -> Vec<String> {
        self.placement_characters
            .iter()
            .chain(&self.enemy_names)
            .chain(&self.staged_actor_names)
            .cloned()
            .collect()
    }
}

/// One seat in a match. Control assignment lives HERE, not on the character
/// definition (§4.7): a definition describes a body, and who drives it is a
/// session binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchParticipant {
    pub character: String,
    /// Free-form team/slot label. The load projection does not interpret it; it
    /// exists so this type is usable as the real roster rather than a stub that
    /// gets replaced.
    pub team: Option<String>,
}

impl MatchParticipant {
    pub fn new(character: impl Into<String>) -> Self {
        Self {
            character: character.into(),
            team: None,
        }
    }
}

/// What a MATCH stages: one character per seat. Several seats may name the SAME
/// character (a mirror match), which the demand set collapses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchParticipantRoster {
    pub participants: Vec<MatchParticipant>,
}

impl MatchParticipantRoster {
    pub fn of<I, S>(characters: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            participants: characters
                .into_iter()
                .map(|c| MatchParticipant::new(c))
                .collect(),
        }
    }
}

impl StagesCharacters for MatchParticipantRoster {
    fn character_tokens(&self) -> Vec<String> {
        self.participants
            .iter()
            .map(|p| p.character.clone())
            .collect()
    }
}

/// What DIRECT STARTUP stages: whoever the session begins as, plus anything the
/// opening scene shows before a room transition has ever run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirectStartupSpec {
    pub starting_characters: Vec<String>,
}

impl DirectStartupSpec {
    pub fn of<I, S>(characters: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            starting_characters: characters.into_iter().map(Into::into).collect(),
        }
    }
}

impl StagesCharacters for DirectStartupSpec {
    fn character_tokens(&self) -> Vec<String> {
        self.starting_characters.clone()
    }
}
