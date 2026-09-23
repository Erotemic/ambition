//! The per-actor resource bank: the one mutable authority for every resource a
//! body holds.
//!
//! See `docs/planning/engine/composable-actor-resources.md`. A body carries an
//! [`ActorResources`] only when something that owns a resource declared one for
//! it — a match's Limit, a ruleset's Mana. A body with none carries no bank,
//! and that is a complete composition, not a degraded one.
//!
//! ⭐ THE BANK CARRIES THE LAYOUT IT IS READ THROUGH. Values are dense, one per
//! slot, in the order of an immutable [`ResourceLayout`] the component holds by
//! `Arc`. A rollback restores the component, so it restores the values AND the
//! layout together — a restored value can never be read through another
//! layout's slots.

use std::sync::Arc;

use ambition_resource_spec::{
    ResourceCost, ResourceDeclaration, ResourceId, ResourceStart,
};

/// An immutable, canonically ordered set of resource declarations.
///
/// Ordered by resource id (digest first), so the same declarations produce the
/// same layout — and the same [`ResourceLayoutId`] — whatever order they were
/// written in, on every peer.
#[derive(Debug, PartialEq)]
pub struct ResourceLayout {
    id: ResourceLayoutId,
    declarations: Vec<ResourceDeclaration>,
}

/// A content digest of a layout's declarations: which resources, in what
/// order, with what capacity and start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResourceLayoutId(pub u64);

/// Why a set of declarations is not a layout.
#[derive(Clone, Debug, PartialEq)]
pub enum ResourceLayoutError {
    /// The same resource was declared twice. Two slots for one resource would
    /// be two mutable answers to one question.
    Duplicate(ResourceId),
    /// A capacity that is negative or not finite.
    InvalidCapacity(ResourceId),
}

impl ResourceLayout {
    pub fn new(mut declarations: Vec<ResourceDeclaration>) -> Result<Self, ResourceLayoutError> {
        declarations.sort_by(|a, b| a.resource.cmp(&b.resource));
        for pair in declarations.windows(2) {
            if pair[0].resource == pair[1].resource {
                return Err(ResourceLayoutError::Duplicate(pair[0].resource.clone()));
            }
        }
        if let Some(bad) = declarations
            .iter()
            .find(|d| !d.capacity.is_finite() || d.capacity < 0.0)
        {
            return Err(ResourceLayoutError::InvalidCapacity(bad.resource.clone()));
        }
        let mut digest: u64 = 0xcbf2_9ce4_8422_2325;
        let mut fold = |word: u64| {
            digest ^= word;
            digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
        };
        for declaration in &declarations {
            fold(declaration.resource.digest());
            fold(u64::from(declaration.capacity.to_bits()));
            fold(match declaration.start {
                ResourceStart::Empty => 0,
                ResourceStart::Full => 1,
            });
        }
        Ok(Self {
            id: ResourceLayoutId(digest),
            declarations,
        })
    }

    pub fn id(&self) -> ResourceLayoutId {
        self.id
    }

    pub fn declarations(&self) -> &[ResourceDeclaration] {
        &self.declarations
    }

    /// The prepared handle for `resource` in this layout, or `None` when the
    /// layout does not hold it.
    ///
    /// A binary search over the layout's canonically ordered digests — never a
    /// name hash, and never a walk over the body's other state.
    pub fn slot(&self, resource: &ResourceId) -> Option<ResourceSlot> {
        self.declarations
            .binary_search_by(|d| d.resource.cmp(resource))
            .ok()
            .map(|index| ResourceSlot {
                layout: self.id,
                index: index as u16,
            })
    }
}

/// A prepared address of one resource in one layout.
///
/// It names its layout, so a handle prepared against one body's layout and
/// used on another's is refused rather than read at the wrong slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceSlot {
    layout: ResourceLayoutId,
    index: u16,
}

/// One resource's current value and capacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResourceLevel {
    pub current: f32,
    pub max: f32,
}

impl ResourceLevel {
    fn at_start(declaration: &ResourceDeclaration) -> Self {
        Self {
            current: match declaration.start {
                ResourceStart::Empty => 0.0,
                ResourceStart::Full => declaration.capacity,
            },
            max: declaration.capacity,
        }
    }

    /// Add `amount`, clamped to `[0, max]`.
    pub fn refill(&mut self, amount: f32) {
        self.current = (self.current + amount).clamp(0.0, self.max);
    }

    /// Remove `amount`, floored at zero.
    pub fn drain(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    pub fn fraction(self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }
}

/// Every resource one body holds, dense, read through the layout it carries.
#[derive(bevy_ecs::component::Component, Clone, Debug, PartialEq)]
pub struct ActorResources {
    layout: Arc<ResourceLayout>,
    levels: Vec<ResourceLevel>,
}

impl ActorResources {
    /// A bank at the layout's declared start.
    pub fn new(layout: Arc<ResourceLayout>) -> Self {
        let levels = layout
            .declarations
            .iter()
            .map(ResourceLevel::at_start)
            .collect();
        Self { layout, levels }
    }

    /// Build a bank from declarations, or `None` when there are none — a body
    /// that declares no resource carries no bank.
    pub fn declared(
        declarations: &[ResourceDeclaration],
    ) -> Result<Option<Self>, ResourceLayoutError> {
        if declarations.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self::new(Arc::new(ResourceLayout::new(
            declarations.to_vec(),
        )?))))
    }

    pub fn layout(&self) -> &ResourceLayout {
        &self.layout
    }

    pub fn slot(&self, resource: &ResourceId) -> Option<ResourceSlot> {
        self.layout.slot(resource)
    }

    /// The level at `slot`, or `None` for a handle prepared against another
    /// layout.
    pub fn level(&self, slot: ResourceSlot) -> Option<ResourceLevel> {
        (slot.layout == self.layout.id)
            .then(|| self.levels.get(slot.index as usize).copied())
            .flatten()
    }

    pub fn level_mut(&mut self, slot: ResourceSlot) -> Option<&mut ResourceLevel> {
        if slot.layout != self.layout.id {
            return None;
        }
        self.levels.get_mut(slot.index as usize)
    }

    /// The level of `resource`, or `None` when this body does not hold it.
    pub fn level_of(&self, resource: &ResourceId) -> Option<ResourceLevel> {
        self.slot(resource).and_then(|slot| self.level(slot))
    }

    pub fn level_of_mut(&mut self, resource: &ResourceId) -> Option<&mut ResourceLevel> {
        let slot = self.slot(resource)?;
        self.level_mut(slot)
    }

    /// Every term affordable. A term naming a resource this body does not hold
    /// is unaffordable: absence is never free.
    pub fn can_pay(&self, costs: &[ResourceCost]) -> bool {
        costs.iter().all(|cost| {
            cost.amount >= 0.0
                && self
                    .level_of(&cost.resource)
                    .is_some_and(|level| level.current + 1e-6 >= cost.amount)
        })
    }

    /// Pay every term, or none: returns `false` and changes nothing when any
    /// term is unaffordable.
    pub fn pay(&mut self, costs: &[ResourceCost]) -> bool {
        if !self.can_pay(costs) {
            return false;
        }
        for cost in costs {
            if let Some(level) = self.level_of_mut(&cost.resource) {
                level.drain(cost.amount);
            }
        }
        true
    }

    /// Every level back to its declared start — the same baseline the bank was
    /// built from, so a reset cannot disagree with a spawn.
    pub fn reset_to_start(&mut self) {
        for (level, declaration) in self.levels.iter_mut().zip(&self.layout.declarations) {
            *level = ResourceLevel::at_start(declaration);
        }
    }

    /// `(resource digest, current, max)` per slot, in layout order — what a
    /// checksum folds.
    pub fn checksum_terms(&self) -> impl Iterator<Item = (u64, f32, f32)> + '_ {
        self.layout
            .declarations
            .iter()
            .zip(&self.levels)
            .map(|(d, level)| (d.resource.digest(), level.current, level.max))
    }
}

/// Price a set of costs against an optional bank: a free price is affordable to
/// every body, and a positive one only to a body whose bank can pay it.
pub fn can_pay(resources: Option<&ActorResources>, costs: &[ResourceCost]) -> bool {
    costs.is_empty() || resources.is_some_and(|bank| bank.can_pay(costs))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FUEL: ResourceId = ResourceId::from_static("fuel");
    const CATALYST: ResourceId = ResourceId::from_static("catalyst");
    const LIMIT: ResourceId = ResourceId::from_static("limit");

    fn bank() -> ActorResources {
        ActorResources::declared(&[
            ResourceDeclaration::new(FUEL, 100.0, ResourceStart::Full),
            ResourceDeclaration::new(CATALYST, 10.0, ResourceStart::Full),
        ])
        .expect("valid")
        .expect("declared")
    }

    #[test]
    fn no_declarations_is_no_bank() {
        assert_eq!(ActorResources::declared(&[]), Ok(None));
        assert!(can_pay(None, &[]));
        assert!(!can_pay(None, &[ResourceCost::new(FUEL, 1.0)]));
    }

    #[test]
    fn a_resource_the_body_does_not_hold_is_never_affordable() {
        let bank = bank();
        assert!(!bank.can_pay(&[ResourceCost::new(LIMIT, 0.5)]));
        assert!(bank.can_pay(&[ResourceCost::new(FUEL, 100.0)]));
    }

    #[test]
    fn a_multi_term_price_is_paid_whole_or_not_at_all() {
        let mut bank = bank();
        let price = [ResourceCost::new(FUEL, 30.0), ResourceCost::new(CATALYST, 11.0)];
        assert!(!bank.pay(&price));
        assert_eq!(bank.level_of(&FUEL).expect("held").current, 100.0);
        let price = [ResourceCost::new(FUEL, 30.0), ResourceCost::new(CATALYST, 4.0)];
        assert!(bank.pay(&price));
        assert_eq!(bank.level_of(&FUEL).expect("held").current, 70.0);
        assert_eq!(bank.level_of(&CATALYST).expect("held").current, 6.0);
    }

    #[test]
    fn declaration_order_does_not_change_the_layout() {
        let a = ResourceLayout::new(vec![
            ResourceDeclaration::new(FUEL, 1.0, ResourceStart::Full),
            ResourceDeclaration::new(LIMIT, 2.0, ResourceStart::Empty),
        ])
        .expect("valid");
        let b = ResourceLayout::new(vec![
            ResourceDeclaration::new(LIMIT, 2.0, ResourceStart::Empty),
            ResourceDeclaration::new(FUEL, 1.0, ResourceStart::Full),
        ])
        .expect("valid");
        assert_eq!(a.id(), b.id());
        assert_eq!(a.slot(&LIMIT), b.slot(&LIMIT));
    }

    #[test]
    fn a_handle_from_another_layout_reads_nothing() {
        let bank = bank();
        let other = ActorResources::declared(&[ResourceDeclaration::new(
            FUEL,
            5.0,
            ResourceStart::Empty,
        )])
        .expect("valid")
        .expect("declared");
        let foreign = other.slot(&FUEL).expect("held");
        assert_eq!(bank.level(foreign), None);
    }

    #[test]
    fn duplicate_and_invalid_declarations_are_refused() {
        assert_eq!(
            ResourceLayout::new(vec![
                ResourceDeclaration::new(FUEL, 1.0, ResourceStart::Full),
                ResourceDeclaration::new(FUEL, 2.0, ResourceStart::Full),
            ]),
            Err(ResourceLayoutError::Duplicate(FUEL))
        );
        assert_eq!(
            ResourceLayout::new(vec![ResourceDeclaration::new(
                LIMIT,
                f32::NAN,
                ResourceStart::Empty
            )]),
            Err(ResourceLayoutError::InvalidCapacity(LIMIT))
        );
    }

    #[test]
    fn a_snapshot_restores_the_values_and_the_layout_they_are_read_through() {
        use crate::snapshot::{Reader, SnapshotState};
        let mut bank = bank();
        bank.level_of_mut(&FUEL).expect("held").drain(12.5);
        let mut bytes = Vec::new();
        bank.encode(&mut bytes);
        let restored = ActorResources::decode(&mut Reader::new(&bytes)).expect("decodes");
        assert_eq!(restored, bank);
        assert_eq!(restored.layout().id(), bank.layout().id());

        let mut overfull = bank.clone();
        overfull.levels[0].current = overfull.levels[0].max + 1.0;
        let mut bytes = Vec::new();
        overfull.encode(&mut bytes);
        assert_eq!(ActorResources::decode(&mut Reader::new(&bytes)), None);
    }

    #[test]
    fn a_reset_returns_to_the_declared_start() {
        let mut bank = ActorResources::declared(&[
            ResourceDeclaration::new(LIMIT, 60.0, ResourceStart::Empty),
            ResourceDeclaration::new(FUEL, 100.0, ResourceStart::Full),
        ])
        .expect("valid")
        .expect("declared");
        bank.level_of_mut(&LIMIT).expect("held").refill(40.0);
        bank.level_of_mut(&FUEL).expect("held").drain(40.0);
        bank.reset_to_start();
        assert_eq!(bank.level_of(&LIMIT).expect("held").current, 0.0);
        assert_eq!(bank.level_of(&FUEL).expect("held").current, 100.0);
    }
}
