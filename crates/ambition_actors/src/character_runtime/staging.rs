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

use bevy::prelude::Resource;

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
    /// The stable `CharacterDefinitionId` this seat wears.
    pub character: String,
    /// Who drives it. Lives HERE and not on the definition, because the same
    /// character must be playable by a human, a CPU, a replay, and an RL policy
    /// without four definitions.
    pub controller: ControllerBinding,
    /// Free-form team/slot label. The load projection does not interpret it; it
    /// exists so this type is usable as the real roster rather than a stub that
    /// gets replaced.
    pub team: Option<String>,
}

impl MatchParticipant {
    pub fn new(character: impl Into<String>) -> Self {
        Self {
            character: character.into(),
            controller: ControllerBinding::Human { device_slot: 0 },
            team: None,
        }
    }

    pub fn driven_by(mut self, controller: ControllerBinding) -> Self {
        self.controller = controller;
        self
    }

    pub fn on_team(mut self, team: impl Into<String>) -> Self {
        self.team = Some(team.into());
        self
    }
}

/// **Who drives a body.** (§4.7)
///
/// Not on the character definition. A definition describes physical limits,
/// vitals, moves, abilities, and hurt behaviour — a BODY — and the same body must
/// be drivable by any of these without becoming four characters. `default_brain`
/// on an identity is the shape this replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerBinding {
    Human {
        device_slot: u8,
    },
    /// A brain profile drives it. The profile is AI policy, and it is the only
    /// variant that carries one.
    Cpu {
        brain_profile: Option<String>,
    },
    /// A recorded control-frame stream drives it.
    Replay,
    /// An external policy drives it (the RL harness).
    Policy {
        policy_id: Option<String>,
    },
}

impl ControllerBinding {
    /// The AI brain profile, if this binding has one. Human, replay, and policy
    /// bindings deliberately do not: a replay that consulted a brain profile
    /// would stop being a replay.
    pub fn brain_profile(&self) -> Option<&str> {
        match self {
            Self::Cpu { brain_profile } => brain_profile.as_deref(),
            _ => None,
        }
    }
}

/// **Normalized exertion, the only thing locomotion intent may cross the seam
/// as.** (§4.7)
///
/// A brain says how hard to try; the BODY turns that into its own acceleration,
/// speed cap, and traction. `patrol_speed` / `chase_speed` / `aggro_radius` /
/// `attack_range` on `CharacterArchetypeSpec` are the standing inconsistency:
/// they are brain or encounter policy that knows absolute world speeds, so a
/// heavy and a light "chasing" move at the same authored number regardless of
/// what their bodies are.
///
/// A heavy at `0.9` and a light at `0.35` sometimes reaching the same absolute
/// speed is **not** wrong — effort is relative exertion, not a cross-character
/// ranking. Navigation that must reach a point by a deadline is a separate
/// concern and may legitimately use world-space constraints.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct NormalizedEffort(f32);

impl NormalizedEffort {
    pub const IDLE: Self = Self(0.0);
    pub const FULL: Self = Self(1.0);

    /// Clamped to `0.0..=1.0`. Out-of-range is a caller bug, but silently
    /// admitting `4.0` would let a brain outrun its own body's cap by naming a
    /// bigger number — which is exactly the world-space coupling this removes.
    pub fn new(effort: f32) -> Self {
        Self(if effort.is_finite() {
            effort.clamp(0.0, 1.0)
        } else {
            0.0
        })
    }

    pub fn get(self) -> f32 {
        self.0
    }

    /// Apply this exertion to the body's OWN maximum. The body owns the number;
    /// the brain owns only the fraction.
    pub fn applied_to(self, body_max: f32) -> f32 {
        body_max * self.0
    }
}

/// What a MATCH stages: one character per seat. Several seats may name the SAME
/// character (a mirror match), which the demand set collapses.
/// A `Resource` because a match's roster is SESSION state: it is what the seating
/// pass reads to turn participants into bodies (C4), and what the load projection
/// reads to demand their art. Both are per-session facts with one owner.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchParticipantRoster {
    pub participants: Vec<MatchParticipant>,
    /// **Whether a seated fighter may act on the tick it appears.**
    ///
    /// A ruleset that opens on a countdown wants `true`: the fighters are reset,
    /// placed and VISIBLE through "3, 2, 1" and none of them — human or CPU —
    /// may decide anything until it ends.
    ///
    /// It lives on the ROSTER because it is a fact about the match, and seating
    /// is the only place that can act on it without a window. The versus stage
    /// suspends control when its countdown begins, and a fighter that seats on
    /// the same tick could take a simulation step before that insert lands: one
    /// tick of a CPU deciding, or of a held direction, before the count starts
    /// (GPT 5.6, 2026-07-29). Applying it AT seating closes the window rather
    /// than narrowing it, which is the difference between a fix and a smaller bug.
    ///
    /// Taken off by whoever put the countdown up — for versus, the `Starting`
    /// arm reaching zero, which is the one place a round goes live.
    pub opens_suspended: bool,
    /// **Which frozen seat topology this roster was built from.**
    ///
    /// `None` means it was built from live device discovery because no session
    /// had decided its seating yet — the ordinary case, since a route is entered
    /// before its rollback session starts.
    ///
    /// The roster, the GGRS handle count and the per-seat latches must agree
    /// about how many people are playing, and freezing the topology made them
    /// agree by construction ONLY for consumers that read it. This roster is
    /// built first, so the stamp is what turns "they should match" into a
    /// question the code can ask (GPT 5.6, 2026-07-29).
    pub seat_topology: Option<u64>,
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
            ..Default::default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character_runtime::CharacterLoadDemand;

    /// **§4.8's one shared projection.** Two semantically different stagings that
    /// name the same cast must produce the same demand, or "the room worked and the
    /// match did not" becomes a real bug class again.
    #[test]
    fn match_roster_and_room_plan_project_to_the_same_demand() {
        let room = RoomStagingPlan {
            placement_characters: vec!["mary_o".into()],
            enemy_names: vec!["ai_slop".into()],
            staged_actor_names: vec!["solid_snake".into()],
        };
        // The same three characters, seated instead of placed — and one seat is a
        // CPU, which must not change what art is needed.
        let match_roster = MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("solid_snake").driven_by(ControllerBinding::Cpu {
                    brain_profile: Some("aggressive".into()),
                }),
                MatchParticipant::new("mary_o").on_team("blue"),
                MatchParticipant::new("ai_slop").driven_by(ControllerBinding::Replay),
            ],
            ..Default::default()
        };

        let mut from_room = CharacterLoadDemand::default();
        room.project_demand(&mut from_room);
        let mut from_match = CharacterLoadDemand::default();
        match_roster.project_demand(&mut from_match);

        let room_tokens: Vec<&str> = from_room.pending().collect();
        assert_eq!(
            room_tokens,
            from_match.pending().collect::<Vec<_>>(),
            "a room and a match naming the same cast must demand the same art"
        );
        // Deterministic and deduplicated, whatever order the sources listed them in.
        assert_eq!(room_tokens, vec!["ai_slop", "mary_o", "solid_snake"]);
    }

    /// A mirror match is one decode, not two.
    #[test]
    fn a_mirror_match_demands_one_decode_per_character() {
        let roster = MatchParticipantRoster::of(["mary_o", "mary_o", "mary_o"]);
        let mut demand = CharacterLoadDemand::default();
        roster.project_demand(&mut demand);
        assert_eq!(demand.pending().collect::<Vec<_>>(), vec!["mary_o"]);
    }

    /// §4.8: transformations, summons, assists, alternate forms, and a boss
    /// revealed mid-fight all arrive by demanding MORE tokens later. No new
    /// staging concept, and nothing has to know it was a late arrival.
    #[test]
    fn a_late_arrival_needs_no_new_staging_concept() {
        let mut demand = CharacterLoadDemand::default();
        MatchParticipantRoster::of(["mary_o"]).project_demand(&mut demand);
        // Mary-O grows mid-match; the grown form is its own character (§4.3).
        demand.request("mary_o_tall");
        assert_eq!(
            demand.pending().collect::<Vec<_>>(),
            vec!["mary_o", "mary_o_tall"]
        );
    }

    /// Only a CPU seat carries a brain profile. A replay that consulted one would
    /// stop being a replay.
    #[test]
    fn only_a_cpu_binding_carries_a_brain_profile() {
        assert_eq!(
            ControllerBinding::Cpu {
                brain_profile: Some("aggressive".into())
            }
            .brain_profile(),
            Some("aggressive")
        );
        for binding in [
            ControllerBinding::Human { device_slot: 1 },
            ControllerBinding::Replay,
            ControllerBinding::Policy {
                policy_id: Some("ppo_7".into()),
            },
        ] {
            assert_eq!(binding.brain_profile(), None, "{binding:?}");
        }
    }

    /// Effort is a fraction of the BODY's own maximum, so two bodies at the same
    /// exertion legitimately move at different speeds — and the same body cannot be
    /// made to exceed its cap by a brain naming a bigger number.
    #[test]
    fn effort_scales_the_bodys_own_maximum_and_cannot_exceed_it() {
        let chasing = NormalizedEffort::new(0.9);
        assert_eq!(chasing.applied_to(100.0), 90.0);
        assert_eq!(
            chasing.applied_to(400.0),
            360.0,
            "a faster body goes faster"
        );

        // A brain cannot outrun its body by asking for more than everything.
        assert_eq!(NormalizedEffort::new(4.0), NormalizedEffort::FULL);
        assert_eq!(NormalizedEffort::new(-1.0), NormalizedEffort::IDLE);
        assert_eq!(NormalizedEffort::new(f32::NAN), NormalizedEffort::IDLE);

        // A heavy trying hard and a light loafing may coincide in world speed.
        // That is not a bug: effort is relative exertion, not a ranking (§4.7).
        assert_eq!(
            NormalizedEffort::new(0.9).applied_to(100.0),
            NormalizedEffort::new(0.3).applied_to(300.0)
        );
    }
}
