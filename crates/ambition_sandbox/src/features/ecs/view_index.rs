//! `FeatureViewIndex` resource and the per-frame rebuild pass.
//!
//! Presentation systems consult this read-model for sprite swaps, debug overlays,
//! and HUD readouts instead of re-scanning every feature family per visual.

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
    /// `(view, generation)` per id. The generation lets the per-frame rebuild
    /// MARK-AND-SWEEP instead of clear()+reinsert: a surviving id keeps its
    /// existing key allocation, so a `String` is allocated only for a genuinely
    /// new feature id — not for every id every frame. This index rebuilds every
    /// frame and RL steps the sim millions of times, so avoid per-id churn.
    views: std::collections::HashMap<String, (FeatureView, u64)>,
    generation: u64,
}

impl FeatureViewIndex {
    pub fn get(&self, id: &str) -> Option<&FeatureView> {
        self.views.get(id).map(|(view, _)| view)
    }

    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    pub fn len(&self) -> usize {
        self.views.len()
    }

    /// Begin a rebuild pass: bump the generation so this frame's writes are
    /// distinguishable from last frame's (swept by [`Self::end_rebuild`]).
    fn begin_rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// End a rebuild pass: drop every entry not written this generation — the
    /// features that despawned. Surviving keys keep their allocations.
    fn end_rebuild(&mut self) {
        let gen = self.generation;
        self.views.retain(|_, (_, g)| *g == gen);
    }

    /// Insert `view` for `id` only if no view has been recorded yet THIS
    /// rebuild.
    ///
    /// Preserves family priority (pickup → chest → breakable → switch →
    /// actor → hazard → boss): first matching family wins when ids collide.
    ///
    /// A same-generation entry is kept (first wins); a stale prior-frame entry
    /// is refreshed in place; only a genuinely new id allocates a `String`.
    fn insert_if_absent(&mut self, id: &str, view: FeatureView) {
        let gen = self.generation;
        if let Some(slot) = self.views.get_mut(id) {
            if slot.1 != gen {
                *slot = (view, gen);
            }
        } else {
            self.views.insert(id.to_string(), (view, gen));
        }
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
    pickups: Query<(&FeatureId, &CenteredAabb, Option<&Collected>), With<PickupFeature>>,
    chests: Query<(&FeatureId, &CenteredAabb, Option<&Opened>), With<ChestFeature>>,
    breakables: Query<(&FeatureId, &CenteredAabb, &BreakableFeature)>,
    switches: Query<(&FeatureId, &CenteredAabb, &SwitchOn), With<SwitchFeature>>,
    actors: Query<(
        &FeatureId,
        &CenteredAabb,
        &ActorRuntime,
        Option<&super::enemy_clusters::EnemyStatus>,
        Option<&ActorAttackState>,
        Option<&super::enemy_clusters::EnemyConfig>,
        Option<&ActorSurfaceState>,
        Option<&super::npc_clusters::NpcStatus>,
        // Portal aerial-roll (same component the player uses) so actors
        // somersault + self-right through portals just like the player.
        Option<&crate::platformer_runtime::orientation::ActorRoll>,
    )>,
    hazards: Query<(&FeatureId, &CenteredAabb, &HazardFeature)>,
    bosses: Query<(
        &FeatureId,
        super::boss_clusters::BossClusterRef,
        &crate::brain::BossAttackState,
        // Shared combat read-model, synced from the boss runtime by
        // `sync_boss_actor_components` (WorldPrep, before this rebuild).
        // Presentation reads alive / hit-flash from here instead of the
        // BossRuntime fields, the same component enemies/NPCs expose.
        &super::super::components::ActorCombatState,
        Option<&BossDeathAnimation>,
        Option<&BossPhase>,
        // Gravity-upright roll — the SAME `ActorRoll` the player / enemies / NPCs
        // use, so a boss rights itself under flipped / sideways gravity instead of
        // staying screen-axis-aligned (it floats, but it should still flip).
        Option<&crate::platformer_runtime::orientation::ActorRoll>,
    )>,
) {
    index.begin_rebuild();
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
    for (id, aabb, actor, status, attack, config, surface, npc_status, roll) in &actors {
        let roll_rad = roll.map_or(0.0, |r| r.angle);
        let view = match actor {
            ActorRuntime::Npc => FeatureView {
                pos: aabb.center,
                size: aabb.size(),
                kind: FeatureVisualKind::Npc,
                visible: true,
                flash: npc_status.is_some_and(|s| s.hit_flash > 0.0),
                switch_on: false,
                rotation_rad: roll_rad,
            },
            ActorRuntime::Enemy => {
                let alive = status.is_some_and(|s| s.alive);
                let flash = status.is_some_and(|s| s.hit_flash > 0.0)
                    || attack.is_some_and(|a| a.is_winding_up() || a.is_active());
                let kind = if config.is_some_and(|c| c.tuning.is_sandbag) {
                    FeatureVisualKind::TrainingDummy
                } else {
                    FeatureVisualKind::Enemy
                };
                // Sprite rotation. A *surface-walker* (PuppySlug) orients to the
                // surface it clings to (its `surface_normal` already encodes which
                // floor/wall/ceiling it's on, so it handles gravity flips too). EVERY
                // OTHER actor rights to gravity via `roll_rad` — the SAME path NPCs
                // and the player use, so a flipped-gravity sprite flips upright. Using
                // `surface_normal` here for a non-walker pinned it to `(0,-1)` and
                // left it gravity-blind, so a now-hostile NPC (NPC -> Enemy) stopped
                // tracking gravity (the orientation diverged on the hostility flip).
                // The two must NOT be summed — under a sideways gravity zone a
                // surface-walker's clung surface IS the gravity floor, so adding both
                // over-rotates.
                let is_surface_walker = config.is_some_and(|c| c.tuning.surface_walker);
                let rotation_rad = if is_surface_walker {
                    match surface {
                        Some(s) => f32::atan2(-s.surface_normal.x, -s.surface_normal.y),
                        None => roll_rad,
                    }
                } else {
                    roll_rad
                };
                // Render size is the RAW (un-oriented) body box. The sprite is
                // oriented by `rotation_rad`, so it must NOT also receive the
                // surface-oriented footprint — that double-counts the rotation and,
                // worse, changes `view.size` when the slug climbs a wall, tripping
                // the `BoundFeatureKind` re-bind which re-bakes the feet anchor off
                // the swapped (long) dimension and shoves the sprite off its box.
                // The ORIENTED footprint still lives in the `CenteredAabb` component
                // (read directly by the debug overlay + hurtbox), so un-swap here to
                // recover the raw dims. Only a surface-walker on a wall swaps.
                let render_size = match surface {
                    Some(s)
                        if is_surface_walker
                            && s.surface_normal.x.abs() > s.surface_normal.y.abs() =>
                    {
                        let o = aabb.size();
                        ae::Vec2::new(o.y, o.x)
                    }
                    _ => aabb.size(),
                };
                FeatureView {
                    pos: aabb.center,
                    size: render_size,
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
    for (id, feature, attack_state, combat, death_anim, phase, roll) in &bosses {
        let boss = feature.as_boss_ref();
        // `alive` reads the shared `ActorCombatState` mirror; pos / size
        // still come from `BossRuntime` until the boss body migrates to
        // `CenteredAabb` (ecs-cleanup-plan #9).
        let visible = combat.alive
            || death_anim.is_some_and(|d| d.remaining_s > 0.0)
            || phase.is_some_and(|p| p.is_active());
        index.insert_if_absent(
            id.as_str(),
            FeatureView {
                pos: boss.kin.pos,
                size: boss.render_size(),
                kind: FeatureVisualKind::Boss,
                visible,
                // Hit-flash reads the shared combat mirror; telegraph /
                // active windows read `BossAttackState` (the brain's
                // source of truth, already a component).
                flash: combat.hit_flash > 0.0
                    || attack_state.telegraph_profile.is_some()
                    || attack_state.active_profile.is_some(),
                switch_on: false,
                rotation_rad: roll.map_or(0.0, |r| r.angle),
            },
        );
    }
    // Sweep entries for features that despawned this frame (those not
    // re-inserted under the current generation); surviving keys are reused.
    index.end_rebuild();
}

#[cfg(test)]
mod view_index_tests {
    //! The FeatureViewIndex read-model. The load-bearing invariant is
    //! insert_if_absent's first-wins semantics: it preserves the old
    //! linear-scan family priority (pickup -> ... -> boss) so an id that
    //! collides across families renders as the first family, not whichever
    //! HashMap write happened to land last.
    use super::*;

    fn view(visible: bool) -> FeatureView {
        FeatureView {
            pos: ae::Vec2::ZERO,
            size: ae::Vec2::new(1.0, 1.0),
            kind: FeatureVisualKind::Switch,
            visible,
            flash: false,
            switch_on: false,
            rotation_rad: 0.0,
        }
    }

    #[test]
    fn empty_index_reports_empty_and_none() {
        let idx = FeatureViewIndex::default();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
        assert!(idx.get("anything").is_none());
    }

    #[test]
    fn insert_if_absent_keeps_the_first_write_for_a_colliding_id() {
        let mut idx = FeatureViewIndex::default();
        idx.insert_if_absent("dup", view(true)); // first family wins
        idx.insert_if_absent("dup", view(false)); // later family dropped
        idx.insert_if_absent("other", view(false));
        assert_eq!(idx.len(), 2);
        assert!(!idx.is_empty());
        assert!(
            idx.get("dup").unwrap().visible,
            "first write for an id wins on cross-family collision"
        );
        assert!(!idx.get("other").unwrap().visible);
        assert!(idx.get("missing").is_none());
    }

    #[test]
    fn rebuild_generations_refresh_survivors_and_sweep_the_despawned() {
        let mut idx = FeatureViewIndex::default();
        // Frame 1: two features present.
        idx.begin_rebuild();
        idx.insert_if_absent("a", view(true));
        idx.insert_if_absent("b", view(true));
        idx.end_rebuild();
        assert_eq!(idx.len(), 2);

        // Frame 2: "a" survives (re-inserted, refreshed in place), "b" despawned
        // (not re-inserted) — the sweep must drop it, exactly like the old
        // clear()+rebuild did.
        idx.begin_rebuild();
        idx.insert_if_absent("a", view(false));
        idx.end_rebuild();
        assert_eq!(idx.len(), 1, "the despawned 'b' is swept");
        assert!(idx.get("b").is_none(), "'b' is gone");
        assert_eq!(
            idx.get("a").map(|v| v.visible),
            Some(false),
            "'a' refreshed to this frame's view"
        );

        // First-wins still holds *within* a generation across rebuilds.
        idx.begin_rebuild();
        idx.insert_if_absent("a", view(true)); // first this frame wins
        idx.insert_if_absent("a", view(false)); // dropped
        idx.end_rebuild();
        assert_eq!(idx.get("a").map(|v| v.visible), Some(true));
    }
}
