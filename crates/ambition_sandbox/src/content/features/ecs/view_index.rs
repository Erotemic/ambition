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
    actors: Query<(
        &FeatureId,
        &FeatureAabb,
        &ActorRuntime,
        Option<&super::enemy_clusters::EnemyStatus>,
        Option<&ActorAttackState>,
        Option<&super::enemy_clusters::EnemyConfig>,
        Option<&ActorSurfaceState>,
        Option<&super::npc_clusters::NpcStatus>,
    )>,
    hazards: Query<(&FeatureId, &FeatureAabb, &HazardFeature)>,
    bosses: Query<(
        &FeatureId,
        &BossFeature,
        &crate::brain::BossAttackState,
        Option<&BossDeathAnimation>,
        Option<&BossPhase>,
    )>,
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
                rotation_rad: 0.0,
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
                rotation_rad: 0.0,
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
                flash: breakable.breakable.state == crate::interaction::BreakableState::Cracking,
                switch_on: false,
                rotation_rad: 0.0,
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
                rotation_rad: 0.0,
            },
        );
    }
    for (id, aabb, actor, status, attack, config, surface, npc_status) in &actors {
        let view = match actor {
            ActorRuntime::Npc => FeatureView {
                pos: aabb.center,
                size: aabb.size(),
                kind: FeatureVisualKind::Npc,
                visible: true,
                flash: npc_status.is_some_and(|s| s.hit_flash > 0.0),
                switch_on: false,
                rotation_rad: 0.0,
            },
            ActorRuntime::Enemy => {
                let alive = status.is_some_and(|s| s.alive);
                let flash = status.is_some_and(|s| s.hit_flash > 0.0)
                    || attack.is_some_and(|a| a.is_winding_up() || a.is_active());
                let kind = if config.is_some_and(|c| c.archetype.is_sandbag()) {
                    FeatureVisualKind::Sandbag
                } else {
                    FeatureVisualKind::Enemy
                };
                // Surface-walker (PuppySlug) sprite rotation from the
                // clung surface normal; flat actors render upright.
                let rotation_rad = surface
                    .map(|s| f32::atan2(-s.surface_normal.x, -s.surface_normal.y))
                    .unwrap_or(0.0);
                FeatureView {
                    pos: aabb.center,
                    size: aabb.size(),
                    kind,
                    visible: alive,
                    flash,
                    switch_on: false,
                    rotation_rad,
                }
            }
        };
        index.insert_if_absent(id.as_str(), view);
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
                rotation_rad: 0.0,
            },
        );
    }
    for (id, feature, attack_state, death_anim, phase) in &bosses {
        let boss = &feature.boss;
        let visible = boss.alive
            || death_anim.is_some_and(|d| d.remaining_s > 0.0)
            || phase.is_some_and(|p| p.is_active());
        index.insert_if_absent(
            id.as_str(),
            FeatureView {
                pos: boss.pos,
                size: boss.render_size(),
                kind: FeatureVisualKind::Boss,
                visible,
                // `flash` reads `BossAttackState` (the brain's
                // single source of truth) instead of the deleted
                // `attack_timer` / `attack_windup_timer` mirror
                // fields on `BossRuntime`.
                flash: boss.hit_flash > 0.0
                    || attack_state.telegraph_profile.is_some()
                    || attack_state.active_profile.is_some(),
                switch_on: false,
                rotation_rad: 0.0,
            },
        );
    }
}
