//! Game-defined types of damage.

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DamageTypeId(Box<str>);

impl DamageTypeId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The types of one damage amount. A hit can carry several types, including
/// a qualifier such as energy. A future rule can add resistance by type or
/// supply separate amounts for each type.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DamageTypes(Vec<DamageTypeId>);

impl DamageTypes {
    /// Make a set of IDs. An error returns the first invalid ID.
    pub fn from_names(names: Vec<String>) -> Result<Self, String> {
        let mut ids = Vec::with_capacity(names.len());
        for name in names {
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
            {
                return Err(name);
            }
            ids.push(DamageTypeId(name.into_boxed_str()));
        }
        ids.sort_unstable();
        ids.dedup();
        Ok(Self(ids))
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.binary_search_by(|id| id.as_str().cmp(name)).is_ok()
    }

    pub fn names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.0.iter().map(DamageTypeId::as_str)
    }
}
