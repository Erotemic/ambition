//! `FeatureViewIndex` resource and the per-frame rebuild pass.
//!
//! The index is the read-model that presentation systems consult to
//! drive sprite swaps, debug overlays, and HUD readouts without
//! re-running the per-FeatureVisual × seven-family linear scan that
//! the old `ecs_feature_view` performed each frame.

use super::*;

/// Per-frame snapshot of every ECS-owned feature's `FeatureView`, keyed
/// by [`FeatureId`].
///
/// Rebuilt once per frame by [`rebuild_feature_view_index`] from the
/// pickup / chest / breakable / switch / actor / hazard / boss queries.
/// Presentation code (`sync_visuals`, `upgrade_enemy_sprites`,
/// `upgrade_npc_sprites`) used to call into per-id helpers that
/// re-scanned every one of those queries on every visual every frame —
/// quadratic in the number of features. With the index, each scan is
/// O(features) once per frame and per-id lookup is O(1).
#[derive(Resource, Default, Clone, Debug)]
pub struct FeatureViewIndex {
    views: std::collections::HashMap<String, FeatureView>,
}

impl FeatureViewIndex {
    pub fn get(&self, id: &str) -> Option<&FeatureView> {
        self.views.get(id)
    }

    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    pub fn len(&self) -> usize {
        self.views.len()
    }

    fn clear(&mut self) {
        self.views.clear();
    }

    /// Insert `view` for `id` only if no view has been recorded yet this
    /// rebuild.
    ///
    /// Preserves the priority order of the old `ecs_feature_view`
    /// linear scan (pickup → chest → breakable → switch → actor →
    /// hazard → boss): the first matching family wins, later writes are
    /// dropped. This matters because authored ids occasionally collide
    /// between families (e.g. `encounter_chest_{encounter_id}` plus an
    /// LDtk-authored Switch id with the same string would have rendered
    /// as the chest under the linear scan; a plain HashMap `insert`
    /// would silently flip them to whichever family runs last).
    fn insert_if_absent(&mut self, id: &str, view: FeatureView) {
        self.views.entry(id.to_string()).or_insert(view);
    }
}

/// Rebuild [`FeatureViewIndex`] from the current ECS feature state.
///
/// One linear pass per feature family per frame, populating the cache
/// presentation systems then read by id. Replaces the
/// per-FeatureVisual × seven-family linear scan the old
/// `ecs_feature_view` performed.
pub fn rebuild_feature_view_index(
    mut index: ResMut<FeatureViewIndex>,
    pickups: Query<(&FeatureId, &FeatureAabb, Option<&Collected>), With<PickupFeature>>,
    chests: Query<(&FeatureId, &FeatureAabb, Option<&Opened>), With<ChestFeature>>,
    breakables: Query<(&FeatureId, &FeatureAabb, &BreakableFeature)>,
    switches: Query<(&FeatureId, &FeatureAabb, &SwitchOn), With<SwitchFeature>>,
    actors: Query<(&FeatureId, &ActorRuntime)>,
    hazards: Query<(&FeatureId, &FeatureAabb, &HazardFeature)>,
    bosses: Query<(&FeatureId, &BossFeature)>,
) {
    index.clear();
    for (id, aabb, collected) in &pickups {
        index.insert_if_absent(
            id.as_str(),
            FeatureView {
                pos: aabb.center,
                size: aabb.size(),
                kind: FeatureVisualKind::Pickup,
                visible: collected.is_none(),
                flash: false,
                switch_on: false,
            },
        );
    }
    for (id, aabb, opened) in &chests {
        index.insert_if_absent(
            id.as_str(),
            FeatureView {
                pos: aabb.center,
                size: aabb.size(),
                kind: FeatureVisualKind::Chest,
                visible: true,
                flash: opened.is_some(),
                switch_on: false,
            },
        );
    }
    for (id, aabb, breakable) in &breakables {
        index.insert_if_absent(
            id.as_str(),
            FeatureView {
                pos: aabb.center,
                size: aabb.size(),
                kind: FeatureVisualKind::Breakable,
                visible: !breakable.broken(),
                flash: breakable.breakable.state == ae::BreakableState::Cracking,
                switch_on: false,
            },
        );
    }
    for (id, aabb, switch_on) in &switches {
        index.insert_if_absent(
            id.as_str(),
            FeatureView {
                pos: aabb.center,
                size: aabb.size(),
                kind: FeatureVisualKind::Switch,
                visible: true,
                flash: false,
                switch_on: switch_on.0,
            },
        );
    }
    for (id, actor) in &actors {
        index.insert_if_absent(id.as_str(), actor.feature_view());
    }
    for (id, aabb, hazard) in &hazards {
        index.insert_if_absent(
            id.as_str(),
            FeatureView {
                pos: hazard.hazard.pos,
                size: aabb.size(),
                kind: FeatureVisualKind::Hazard,
                visible: hazard.hazard.active(),
                flash: false,
                switch_on: false,
            },
        );
    }
    for (id, feature) in &bosses {
        let boss = &feature.boss;
        index.insert_if_absent(
            id.as_str(),
            FeatureView {
                pos: boss.pos,
                size: boss.render_size(),
                kind: FeatureVisualKind::Boss,
                visible: boss.alive,
                flash: boss.hit_flash > 0.0
                    || boss.attack_windup_timer > 0.0
                    || boss.attack_timer > 0.0,
                switch_on: false,
            },
        );
    }
}
