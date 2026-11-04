//! Fixed-width participant requests for party travel preferences.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PartyTravelMode {
    #[default]
    Independent,
    Leader,
    AnyMember,
    AllMembers,
}

impl PartyTravelMode {
    pub const ALL: [Self; 4] = [Self::Independent, Self::Leader, Self::AnyMember, Self::AllMembers];
    pub fn label(self) -> &'static str {
        match self {
            Self::Independent => "Independent", Self::Leader => "Leader",
            Self::AnyMember => "Any member", Self::AllMembers => "All members",
        }
    }
    pub fn next(self) -> Self { Self::ALL[(self as usize + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { Self::ALL[(self as usize + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Zero requests no change. Every preference has the same one-byte wire width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct PartyControlPreference(u8);

impl PartyControlPreference {
    pub fn new(mode: PartyTravelMode, return_to_leader: bool) -> Self {
        Self(1 + mode as u8 + u8::from(return_to_leader) * 4)
    }
    pub fn choice(self) -> Option<(PartyTravelMode, bool)> {
        (self.0 != 0).then(|| {
            let value = self.0 - 1;
            (PartyTravelMode::ALL[usize::from(value & 3)], value & 4 != 0)
        })
    }
}
impl From<PartyControlPreference> for u8 { fn from(value: PartyControlPreference) -> Self { value.0 } }
impl TryFrom<u8> for PartyControlPreference {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= 8 { Ok(Self(value)) } else { Err("unknown party preference") }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_preferences_round_trip_and_unknown_values_are_refused() {
        assert_eq!(PartyControlPreference::default().choice(), None);
        for mode in PartyTravelMode::ALL {
            for returns in [false, true] {
                let preference = PartyControlPreference::new(mode, returns);
                assert_eq!(preference.choice(), Some((mode, returns)));
                assert_eq!(PartyControlPreference::try_from(u8::from(preference)), Ok(preference));
            }
        }
        for unknown in 9..=255 { assert!(PartyControlPreference::try_from(unknown).is_err()); }
    }
}
