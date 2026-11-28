use std::collections::HashMap;
use std::hash::Hash;

use bevy::prelude::{Commands, Entity};

/// Keep one derived entity for each source key.
pub(crate) struct RetainedEntities<K>(HashMap<K, Entity>);

impl<K> Default for RetainedEntities<K> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<K: Eq + Hash> RetainedEntities<K> {
    /// Retire sources that are gone. Forget products removed by other systems.
    pub fn reconcile(&mut self, commands: &mut Commands, exists: impl Fn(&K) -> bool) {
        self.0.retain(|key, product| {
            let Ok(mut entity) = commands.get_entity(*product) else {
                return false;
            };
            if exists(key) {
                true
            } else {
                entity.try_despawn();
                false
            }
        });
    }

    pub fn get(&self, key: &K) -> Option<&Entity> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: K, entity: Entity) {
        self.0.insert(key, entity);
    }
}
