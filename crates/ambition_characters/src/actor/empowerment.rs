//! The traits an empowerment can grant a body.
//!
//! Two things grant them: a timed or held grant on the body (a pickup), and
//! the character the body wears (a super form). Both speak this one set, so a
//! form is content: its catalog row states `empowered: [Untouchable,
//! HarmsOnContact]` and no game system has to name the form.

/// The traits an empowerment can grant, as a set.
///
/// A set rather than an enum because they are INDEPENDENT: being unhittable and
/// hurting what you touch are different claims, either is useful alone, and a
/// game that wants both should say both rather than name a third thing.
///
/// Content writes it as a list of [`EmpowermentTrait`] names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "Vec<EmpowermentTrait>", into = "Vec<EmpowermentTrait>")]
pub struct Empowerment(u32);

/// One trait of an [`Empowerment`], as content names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EmpowermentTrait {
    /// See [`Empowerment::UNTOUCHABLE`].
    Untouchable,
    /// See [`Empowerment::HARMS_ON_CONTACT`].
    HarmsOnContact,
}

impl Empowerment {
    /// Nothing can hurt this body. Delegates to the
    /// [`Invulnerability::EMPOWERED`](crate::actor::Invulnerability::EMPOWERED)
    /// reason, so it coexists with every other reason rather than replacing them.
    pub const UNTOUCHABLE: Self = Self(1 << 0);
    /// This body's own footprint damages what it overlaps — a star-powered
    /// runner flattening what it touches.
    pub const HARMS_ON_CONTACT: Self = Self(1 << 1);

    /// Nothing granted.
    pub const fn none() -> Self {
        Self(0)
    }

    /// Both of these, and whatever else is added later.
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Is this trait granted?
    pub fn holds(self, trait_: Self) -> bool {
        self.0 & trait_.0 != 0
    }

    /// The raw set, for a checksum. Not for logic — ask [`Self::holds`].
    pub fn bits(self) -> u32 {
        self.0
    }

    const TRAITS: [(EmpowermentTrait, Self); 2] = [
        (EmpowermentTrait::Untouchable, Self::UNTOUCHABLE),
        (EmpowermentTrait::HarmsOnContact, Self::HARMS_ON_CONTACT),
    ];
}

impl From<Vec<EmpowermentTrait>> for Empowerment {
    fn from(traits: Vec<EmpowermentTrait>) -> Self {
        traits.into_iter().fold(Self::none(), |set, named| {
            let (_, bit) = Self::TRAITS
                .into_iter()
                .find(|(name, _)| *name == named)
                .expect("every trait name has a bit");
            set.with(bit)
        })
    }
}

impl From<Empowerment> for Vec<EmpowermentTrait> {
    fn from(set: Empowerment) -> Self {
        Empowerment::TRAITS
            .into_iter()
            .filter(|(_, bit)| set.holds(*bit))
            .map(|(name, _)| name)
            .collect()
    }
}
