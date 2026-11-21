//! Transient routing policy for typed native observers. Domains retain their facts.

use crate::{AdmittedExtensions, ExtensionInvocations};
use ambition_extension_sdk::{Phase, Port};
use bevy::prelude::Entity;
use std::{collections::BTreeSet, marker::PhantomData};

/// The admitted selectors of one native fact port at this delivery cut.
pub struct ObserverDeliveryPlan<P: Port> {
    bound: BTreeSet<String>,
    port: PhantomData<P>,
}

impl<P: Port> ObserverDeliveryPlan<P> {
    pub fn new(admitted: &AdmittedExtensions) -> Self {
        Self {
            bound: admitted
                .0
                .entries
                .iter()
                .filter(|entry| entry.descriptor.trigger.port == P::KEY)
                .map(|entry| entry.descriptor.trigger.selector.to_string())
                .collect(),
            port: PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bound.is_empty()
    }

    /// Check admission before a domain retains a native fact.
    pub fn observes(&self, room: bool, body: Option<&[String]>, room_selectors: &[String]) -> bool {
        let selectors = if room {
            room_selectors
        } else {
            body.unwrap_or_default()
        };
        selectors.iter().any(|key| self.bound.contains(key))
    }

    /// A room uses its declared observers. A body uses its active selectors.
    pub fn selectors<'a>(
        &self,
        room: bool,
        body: Option<&'a [String]>,
        room_selectors: &'a [String],
    ) -> Vec<&'a String> {
        let selectors = if room {
            room_selectors
        } else {
            body.unwrap_or_default()
        };
        let mut keys: Vec<_> = selectors
            .iter()
            .filter(|key| self.bound.contains(*key))
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    pub fn deliver(
        &self,
        invocations: &mut ExtensionInvocations,
        phase: &Phase,
        scope: Entity,
        selectors: Vec<&String>,
        batch: P::Value,
    ) where
        P::Value: Clone,
    {
        for selector in selectors {
            invocations.trigger::<P>(phase, selector.clone(), scope, None, false, batch.clone());
        }
    }
}

/// Order independent observers by the domain's semantic scope key.
/// Entity allocation order is not a tie breaker.
pub fn ordered_observer_scopes<K: Ord>(
    scopes: impl IntoIterator<Item = (K, Entity)>,
) -> Vec<Entity> {
    let mut scopes: Vec<_> = scopes.into_iter().collect();
    scopes.sort_by(|a, b| a.0.cmp(&b.0));
    scopes.into_iter().map(|(_, entity)| entity).collect()
}
