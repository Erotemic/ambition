//! Group selection and agreement for native room admission.

use std::collections::{BTreeMap, BTreeSet};
use ambition_characters::control::PlayerSlot;
use ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId;
use bevy::prelude::Component;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PartyTravelMode {
    #[default]
    Independent,
    Leader,
    AnyMember,
    AllMembers,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PartyMembers {
    #[default]
    Session,
    Only(BTreeSet<PlayerSlot>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyTravelRules {
    pub mode: PartyTravelMode,
    pub leader: PlayerSlot,
    pub members: PartyMembers,
}

impl Default for PartyTravelRules {
    fn default() -> Self {
        Self { mode: PartyTravelMode::Independent, leader: PlayerSlot::PRIMARY, members: PartyMembers::Session }
    }
}

/// The source zone and destination route of an agreement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyExit {
    pub zone: String,
    pub target_room: String,
    pub target_zone: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyVote {
    pub body: LiveBodyId,
    pub exit: PartyExit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartyAdmission {
    Independent,
    Denied,
    Waiting,
    Together(Vec<PlayerSlot>),
}

/// Session rules and agreements share one owner. Configuration clears agreements.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct PartyTravel {
    rules: PartyTravelRules,
    votes: BTreeMap<PlayerSlot, PartyVote>,
}

impl PartyTravel {
    pub fn rules(&self) -> &PartyTravelRules { &self.rules }

    pub fn configure(&mut self, rules: PartyTravelRules) -> Result<(), &'static str> {
        if let PartyMembers::Only(members) = &rules.members {
            if rules.mode == PartyTravelMode::Leader && !members.contains(&rules.leader) {
                return Err("the party leader must be a member");
            }
        }
        if self.rules != rules {
            self.rules = rules;
            self.votes.clear();
        }
        Ok(())
    }

    pub fn members(&self, admitted: &crate::participants::SessionParticipants) -> Vec<PlayerSlot> {
        admitted.iter().filter(|slot| match &self.rules.members {
            PartyMembers::Session => true,
            PartyMembers::Only(members) => members.contains(slot),
        }).collect()
    }

    pub fn retain_votes(&mut self, valid: impl Fn(PlayerSlot, &PartyVote) -> bool) {
        self.votes.retain(|slot, vote| valid(*slot, vote));
    }

    pub fn vote(&self, slot: PlayerSlot) -> Option<&PartyVote> { self.votes.get(&slot) }

    pub fn has_votes(&self) -> bool { !self.votes.is_empty() }

    pub fn clear_votes(&mut self) { self.votes.clear(); }

    pub fn consider(&mut self, slot: PlayerSlot, vote: PartyVote, members: &[PlayerSlot]) -> PartyAdmission {
        if self.rules.mode == PartyTravelMode::Independent || !members.contains(&slot) {
            return PartyAdmission::Independent;
        }
        match self.rules.mode {
            PartyTravelMode::Independent => PartyAdmission::Independent,
            PartyTravelMode::Leader if slot != self.rules.leader => PartyAdmission::Denied,
            PartyTravelMode::Leader | PartyTravelMode::AnyMember => PartyAdmission::Together(members.to_vec()),
            PartyTravelMode::AllMembers => {
                if self.votes.get(&slot) != Some(&vote) { self.votes.insert(slot, vote); }
                let selected = &self.votes[&slot];
                if members.iter().all(|member| self.votes.get(member).is_some_and(|vote|
                    vote.body.room == selected.body.room && vote.exit == selected.exit)) {
                    PartyAdmission::Together(members.to_vec())
                } else {
                    PartyAdmission::Waiting
                }
            }
        }
    }

    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_u8, put_u64, put_str};
        let mut bytes = Vec::new();
        put_u8(&mut bytes, match self.rules.mode {
            PartyTravelMode::Independent => 0, PartyTravelMode::Leader => 1,
            PartyTravelMode::AnyMember => 2, PartyTravelMode::AllMembers => 3,
        });
        put_u8(&mut bytes, self.rules.leader.0);
        match &self.rules.members {
            PartyMembers::Session => put_u8(&mut bytes, 0),
            PartyMembers::Only(members) => {
                put_u8(&mut bytes, 1); put_u64(&mut bytes, members.len() as u64);
                for member in members { put_u8(&mut bytes, member.0); }
            }
        }
        put_u64(&mut bytes, self.votes.len() as u64);
        for (slot, vote) in &self.votes {
            put_u8(&mut bytes, slot.0);
            vote.body.write_peer_stable_into(&mut bytes);
            put_str(&mut bytes, &vote.exit.zone);
            put_str(&mut bytes, &vote.exit.target_room);
            put_str(&mut bytes, &vote.exit.target_zone);
        }
        checksum_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    #[test]
    fn a_policy_change_clears_the_previous_group_agreement() {
        let mut party = PartyTravel::default();
        let rules = PartyTravelRules { mode: PartyTravelMode::AllMembers, ..Default::default() };
        party.configure(rules.clone()).unwrap();
        let members = [PlayerSlot(0), PlayerSlot(1)];
        let vote = PartyVote {
            body: LiveBodyId::new(SimId::player_slot(0), Some(LiveRoomInstance::ACTIVATION)),
            exit: PartyExit { zone: "exit".into(), target_room: "next".into(), target_zone: "entry".into() },
        };
        let empty = party.checksum();
        assert_eq!(party.consider(members[0], vote, &members), PartyAdmission::Waiting);
        assert_ne!(party.checksum(), empty);
        party.configure(rules.clone()).unwrap();
        assert!(party.has_votes());
        party.configure(PartyTravelRules { mode: PartyTravelMode::AnyMember, ..rules }).unwrap();
        assert!(!party.has_votes());
    }

    #[test]
    fn membership_can_select_a_group_without_claiming_other_participants() {
        let mut admitted = crate::participants::SessionParticipants::default();
        for slot in [PlayerSlot(0), PlayerSlot(1), PlayerSlot(2)] { admitted.admit(slot); }
        let mut party = PartyTravel::default();
        party.configure(PartyTravelRules { mode: PartyTravelMode::Leader, leader: PlayerSlot(2),
            members: PartyMembers::Only([PlayerSlot(1), PlayerSlot(2)].into_iter().collect()) }).unwrap();
        assert_eq!(party.members(&admitted), vec![PlayerSlot(1), PlayerSlot(2)]);
        let before = party.clone();
        assert!(party.configure(PartyTravelRules { leader: PlayerSlot(0), ..party.rules().clone() }).is_err());
        assert_eq!(party, before);
    }
}
