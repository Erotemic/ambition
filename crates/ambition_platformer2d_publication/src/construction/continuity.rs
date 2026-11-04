//! Mutable occurrence inputs consumed by one frozen room plan.

use ambition_platformer2d_items::pickup::minted_horizon::{
    MintedItemBaseline, MintedItemDescription,
};
use ambition_platformer2d_shared_tangle::{
    lifecycle::{OccurrenceDisposition, RoomOccurrenceOutlook},
    sim_id::SimId,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Default)]
pub(super) struct ContinuityDependencies {
    authored: BTreeSet<SimId>,
    minted: BTreeMap<SimId, Option<MintedItemDescription>>,
}

impl ContinuityDependencies {
    pub fn disposition(
        &mut self,
        outlook: &RoomOccurrenceOutlook,
        id: &SimId,
    ) -> OccurrenceDisposition {
        self.authored.insert(id.clone());
        outlook.disposition(id)
    }

    pub fn mint(&mut self, id: &SimId, description: Option<&MintedItemDescription>) {
        self.minted.insert(id.clone(), description.cloned());
    }

    pub fn matches(
        &self,
        prepared: &RoomOccurrenceOutlook,
        current: &RoomOccurrenceOutlook,
        minted: Option<&MintedItemBaseline>,
    ) -> bool {
        match (
            self.encoded(prepared, |id| self.minted.get(id).and_then(Option::as_ref)),
            self.encoded(current, |id| {
                minted.and_then(|minted| minted.description_of(id))
            }),
        ) {
            (Ok(prepared), Ok(current)) => prepared == current,
            _ => false,
        }
    }

    pub fn digest(
        &self,
        prepared: &RoomOccurrenceOutlook,
    ) -> Result<u64, ambition_platformer2d_core::snapshot::canonical::Error> {
        Ok(ambition_platformer2d_core::snapshot::checksum_bytes(
            &self.encoded(prepared, |id| self.minted.get(id).and_then(Option::as_ref))?,
        ))
    }

    fn encoded<'a>(
        &self,
        prepared: &RoomOccurrenceOutlook,
        mut description_of: impl FnMut(&SimId) -> Option<&'a MintedItemDescription>,
    ) -> Result<Vec<u8>, ambition_platformer2d_core::snapshot::canonical::Error> {
        use ambition_platformer2d_core::snapshot::{put_bool, put_str, put_u64};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.authored.len() as u64);
        for id in &self.authored {
            put_str(&mut bytes, id.as_str());
        }
        put_u64(&mut bytes, self.minted.len() as u64);
        for id in self.minted.keys() {
            let description = description_of(id);
            put_str(&mut bytes, id.as_str());
            put_bool(&mut bytes, description.is_some());
            if let Some(description) = description {
                put_str(&mut bytes, &description.origin.canonical_summary());
                put_str(&mut bytes, &description.held_item);
            }
        }
        let dispositions: Vec<_> = self
            .authored
            .iter()
            .map(|id| (id, prepared.disposition(id)))
            .collect();
        bytes.extend_from_slice(&ambition_platformer2d_core::snapshot::canonical::encode(
            &(dispositions, prepared.reinstatements()),
        )?);
        Ok(bytes)
    }
}
