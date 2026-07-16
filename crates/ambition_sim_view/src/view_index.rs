//! `FeatureViewIndex` resource and the per-frame rebuild pass.
//!
//! Presentation systems consult this read-model for sprite swaps, debug overlays,
//! and HUD readouts instead of re-scanning every feature family per visual.

use ambition_engine_core as ae;
use bevy::prelude::{Entity, Query, Res, ResMut, Resource, With, Without};

use crate::anim_index::ActorSpriteData;
use ambition_actors::features::HazardFeature;
use ambition_actors::features::{
    ActorConfig, ActorDisposition, ActorIdentity, ActorRenderSize, ActorSurfaceState, BodyMelee,
    BossDeathAnimation, BossPhase, BreakableFeature, CenteredAabb, ChestFeature, Collected,
    FeatureId, FeatureSimEntity, FeatureView, FeatureVisualKind, Opened, PickupFeature,
    SwitchFeature, SwitchOn,
};

/// Per-frame snapshot of every ECS-owned feature's `FeatureView`, keyed
/// by [`FeatureId`].
///
/// Rebuilt once per frame by [`rebuild_feature_view_index`] from the
/// pickup / chest / breakable / switch / actor / hazard / boss queries.
/// Presentation code (`sync_visuals`, `upgrade_actor_sprites`) used to
/// call into per-id helpers that
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

    /// Iterate every `(id, view)` row. Presentation passes that render "one
    /// thing per feature" (debug health bars, nameplates) walk the read-model
    /// instead of declaring sim-component queries.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &FeatureView)> {
        // AMBITION_REVIEW(determinism): hash-order iteration is safe here.
        // `SimView` is DERIVED state — rebuilt from the sim every tick, structurally
        // excluded from `SimSnapshot` and from the N0.4 state hash (netcode.md
        // §Excluded). Every consumer of this iterator is presentation. No sim state
        // reads it, so its order can never enter a trajectory.
        self.views.iter().map(|(id, (view, _))| (id.as_str(), view))
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
    actors: Query<
        (
            &FeatureId,
            &CenteredAabb,
            &ActorDisposition,
            Option<&ambition_characters::actor::BodyCombat>,
            Option<&ambition_characters::actor::BodyHealth>,
            Option<&BodyMelee>,
            Option<&ActorConfig>,
            Option<&ActorSurfaceState>,
            // Portal aerial-roll (same component the player uses) so actors
            // somersault + self-right through portals just like the player.
            Option<&ambition_actors::platformer_runtime::orientation::ActorRoll>,
        ),
        // Bosses carry the shared actor read-models (`ActorDisposition` etc., synced
        // by `sync_boss_actor_components`) but are their OWN feature family below.
        // Without this exclusion a boss matches here too and — because the actor
        // family is inserted before the boss family (first-wins priority) — it gets
        // classified as an invisible generic `Actor` (its `ActorStatus`/`ActorConfig`
        // are absent), shadowing the boss view → the boss renders as the generic
        // fallback sprite instead of its sheet. This is the boss-exclusion the
        // deleted `ActorRuntime` tag used to provide implicitly.
        Without<ambition_actors::features::BossConfig>,
    >,
    hazards: Query<(&FeatureId, &CenteredAabb, &HazardFeature)>,
    bosses: Query<(
        &FeatureId,
        ambition_actors::features::BossClusterRef,
        &ambition_characters::brain::BossAttackState,
        // Shared combat read-model, synced from the boss runtime by
        // `sync_boss_actor_components` (WorldPrep, before this rebuild).
        // Presentation reads alive / hit-flash from here instead of the
        // BossRuntime fields, the same component enemies/NPCs expose.
        &ambition_characters::actor::BodyCombat,
        Option<&ambition_characters::actor::BodyHealth>,
        Option<&BossDeathAnimation>,
        Option<&BossPhase>,
        // Gravity-upright roll — the SAME `ActorRoll` the player / enemies / NPCs
        // use, so a boss rights itself under flipped / sideways gravity instead of
        // staying screen-axis-aligned (it floats, but it should still flip).
        Option<&ambition_actors::platformer_runtime::orientation::ActorRoll>,
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
                breakable_state: None,
                chest_opened: false,
                fighting: false,
                switch_on: false,
                rotation_rad: 0.0,
                alive: true,
                hit_flash_secs: 0.0,
                hp_current: 0,
                hp_max: 0,
                training_dummy: false,
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
                breakable_state: None,
                chest_opened: opened.is_some(),
                fighting: false,
                switch_on: false,
                rotation_rad: 0.0,
                alive: true,
                hit_flash_secs: 0.0,
                hp_current: 0,
                hp_max: 0,
                training_dummy: false,
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
                flash: breakable.breakable.state == ambition_interaction::BreakableState::Cracking,
                breakable_state: Some(breakable.breakable.state),
                chest_opened: false,
                fighting: false,
                switch_on: false,
                rotation_rad: 0.0,
                alive: !breakable.broken(),
                hit_flash_secs: 0.0,
                hp_current: breakable.breakable.health.current,
                hp_max: breakable.breakable.health.max,
                training_dummy: false,
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
                breakable_state: None,
                chest_opened: false,
                fighting: false,
                switch_on: switch_on.0,
                rotation_rad: 0.0,
                alive: true,
                hit_flash_secs: 0.0,
                hp_current: 0,
                hp_max: 0,
                training_dummy: false,
            },
        );
    }
    for (id, aabb, disposition, combat, health, attack, config, surface, roll) in &actors {
        let roll_rad = roll.map_or(0.0, |r| r.angle);
        // ONE actor kind. "enemy vs NPC vs training-dummy" was never a render
        // *type* — it's the actor's STATE (fighting-or-not) plus its depiction
        // (sandbag/name fallback in the sprite-upgrade system). `fighting` is a
        // STATE flag stamped from the disposition signal (interim, until it moves
        // onto a `FightingAble` component): a provoked NPC enters the fighting
        // state and its placeholder shifts to the fighting tint with no type flip.
        let hostile = disposition.is_hostile();
        let alive = health.is_some_and(|h| h.alive());
        // Peaceful actors are always visible (they don't die); hostile actors are
        // visible while alive.
        let visible = !hostile || alive;
        let flash = combat.is_some_and(|c| c.hit_flash > 0.0)
            || (hostile && attack.is_some_and(|a| a.is_winding_up() || a.is_active()));
        // Sprite rotation. A *surface-walker* (PuppySlug) orients to the surface it
        // clings to (its `surface_normal` encodes floor/wall/ceiling + gravity
        // flips). EVERY OTHER actor rights to gravity via `roll_rad` — the SAME
        // path the player uses. The two must NOT be summed.
        let is_surface_walker = config.is_some_and(|c| c.tuning.surface_walker);
        let rotation_rad = if is_surface_walker {
            match surface {
                Some(s) => f32::atan2(-s.surface_normal.x, -s.surface_normal.y),
                None => roll_rad,
            }
        } else {
            roll_rad
        };
        // Render size is the RAW (un-oriented) body box. The sprite is oriented by
        // `rotation_rad`, so it must NOT also receive the surface-oriented
        // footprint (that double-counts the rotation and changes `view.size` when
        // the slug climbs a wall). Only a surface-walker on a wall swaps.
        let render_size = match surface {
            Some(s) if is_surface_walker && s.surface_normal.x.abs() > s.surface_normal.y.abs() => {
                let o = aabb.size();
                ae::Vec2::new(o.y, o.x)
            }
            _ => aabb.size(),
        };
        index.insert_if_absent(
            id.as_str(),
            FeatureView {
                pos: aabb.center,
                size: render_size,
                kind: FeatureVisualKind::Actor,
                visible,
                flash,
                breakable_state: None,
                chest_opened: false,
                fighting: hostile,
                switch_on: false,
                rotation_rad,
                // Liveness for presentation (nameplates, debug bars): dead if
                // EITHER cluster says so; an actor with neither cluster reads
                // alive (it has no pool to die from).
                alive: !(combat.is_some_and(|c| !c.alive) || health.is_some_and(|h| !h.alive())),
                hit_flash_secs: combat.map_or(0.0, |c| c.hit_flash),
                hp_current: health.map_or(0, |h| h.current()),
                hp_max: health.map_or(0, |h| h.max()),
                training_dummy: combat.is_some_and(|c| c.training_dummy),
            },
        );
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
                breakable_state: None,
                chest_opened: false,
                fighting: false,
                switch_on: false,
                rotation_rad: 0.0,
                alive: hazard.hazard.active(),
                hit_flash_secs: 0.0,
                hp_current: 0,
                hp_max: 0,
                training_dummy: false,
            },
        );
    }
    for (id, feature, attack_state, combat, health, death_anim, phase, roll) in &bosses {
        let boss = feature.as_boss_ref();
        // `alive` reads the shared `BodyCombat` mirror; pos / size
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
                kind: FeatureVisualKind::Actor,
                visible,
                // Hit-flash reads the shared combat mirror; telegraph /
                // active windows read `BossAttackState` (the move-derived
                // source of truth, already a component).
                flash: combat.hit_flash > 0.0
                    || attack_state.telegraph_profile.is_some()
                    || attack_state.active_profile.is_some(),
                breakable_state: None,
                chest_opened: false,
                // A boss in its encounter is definitionally a combatant.
                fighting: true,
                switch_on: false,
                rotation_rad: roll.map_or(0.0, |r| r.angle),
                alive: combat.alive && !phase.is_some_and(|p| p.is_defeated()),
                // A boss corpse must not read as a lit silhouette — death
                // rows are authored sprites (the old render-side rule).
                hit_flash_secs: if combat.alive { combat.hit_flash } else { 0.0 },
                hp_current: health.map_or(0, |h| h.current()),
                hp_max: health.map_or(0, |h| h.max()),
                training_dummy: false,
            },
        );
    }
    // Sweep entries for features that despawned this frame (those not
    // re-inserted under the current generation); surviving keys are reused.
    index.end_rebuild();
}

/// Materialized per-actor identity facts the renderer needs to BIND and SIZE an
/// actor sprite, keyed by [`FeatureId`] — the STATIC half of the actor
/// read-model (display name, sprite-override label, sandbag flag, explicit
/// render-quad size). It lets `upgrade_actor_sprites` resolve a sprite WITHOUT
/// borrowing gameplay_core's live actor clusters (`ActorSpriteData`): the sim
/// produces this snapshot, presentation consumes it — the read-model seam the D3
/// render→gameplay_core cut needs. These facts are static per actor, so the
/// rebuild re-clones only on a genuine change (otherwise it just refreshes the
/// mark-and-sweep generation — no per-`String` churn as the sim steps).
#[derive(Clone, Debug, PartialEq)]
pub struct ActorRenderView {
    pub name: String,
    pub sprite_override_name: Option<String>,
    pub is_sandbag: bool,
    pub render_size: Option<ae::Vec2>,
    /// Authored deep-dream participation seed (`ActorTuning.dream_seed`) —
    /// the surreal-overlay pass reads this identity fact by id instead of
    /// borrowing the live actor clusters (E4 slice 2).
    pub dream_seed: Option<f32>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct ActorRenderIndex {
    views: std::collections::HashMap<String, (ActorRenderView, u64)>,
    generation: u64,
}

impl ActorRenderIndex {
    pub fn get(&self, id: &str) -> Option<&ActorRenderView> {
        self.views.get(id).map(|(view, _)| view)
    }

    pub fn len(&self) -> usize {
        self.views.len()
    }

    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    fn begin_rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn end_rebuild(&mut self) {
        let gen = self.generation;
        self.views.retain(|_, (_, g)| *g == gen);
    }

    /// Refresh `id`'s snapshot for this generation. A surviving entry whose facts
    /// are UNCHANGED (the common case — actor identity is static) only bumps its
    /// generation, allocating nothing; a new or genuinely-changed entry clones
    /// once. The comparison is by `&str`/value so no candidate `String` is built
    /// on the unchanged path.
    #[allow(clippy::too_many_arguments)]
    fn upsert(
        &mut self,
        id: &str,
        name: &str,
        override_name: Option<&str>,
        is_sandbag: bool,
        render_size: Option<ae::Vec2>,
        dream_seed: Option<f32>,
    ) {
        let gen = self.generation;
        if let Some(slot) = self.views.get_mut(id) {
            let v = &slot.0;
            let unchanged = v.name == name
                && v.sprite_override_name.as_deref() == override_name
                && v.is_sandbag == is_sandbag
                && v.render_size == render_size
                && v.dream_seed == dream_seed;
            if unchanged {
                slot.1 = gen;
                return;
            }
            slot.0 = ActorRenderView {
                name: name.to_string(),
                sprite_override_name: override_name.map(str::to_string),
                is_sandbag,
                render_size,
                dream_seed,
            };
            slot.1 = gen;
            return;
        }
        self.views.insert(
            id.to_string(),
            (
                ActorRenderView {
                    name: name.to_string(),
                    sprite_override_name: override_name.map(str::to_string),
                    is_sandbag,
                    render_size,
                    dream_seed,
                },
                gen,
            ),
        );
    }
}

/// Rebuild [`ActorRenderIndex`] from the live actor clusters + the shared
/// [`ActorRenderSize`] component (joined on the same entity, so
/// the pass is O(actors), not a per-actor cross-scan). Runs in the sim's
/// `FeatureViewSync` set beside [`rebuild_feature_view_index`], so the snapshot
/// is ready before presentation reads it. Bosses have their OWN sprite path
/// (`upgrade_boss_sprites`) and props aren't actors, so neither appears here.
pub fn rebuild_actor_render_index(
    mut index: ResMut<ActorRenderIndex>,
    actors: Query<(ActorSpriteData, Option<&ActorRenderSize>)>,
) {
    index.begin_rebuild();
    for (a, render_size) in &actors {
        index.upsert(
            a.feature_id.as_str(),
            &a.config.name,
            a.config.sprite_override_npc_name.as_deref(),
            a.config.tuning.is_sandbag,
            render_size.map(|s| s.0),
            a.config.tuning.dream_seed,
        );
    }
    index.end_rebuild();
}

/// Materialized per-boss identity the renderer needs to resolve a boss's
/// spritesheet, keyed by [`FeatureId`]: its display name and behavior id (the
/// two feed the boss-sheet lookup + the GNU-ton split-layer detection). The boss
/// analogue of [`ActorRenderView`] — it lets `upgrade_boss_sprites` bind the
/// sheet WITHOUT borrowing the live boss clusters (`BossClusterRef`); the boss's
/// geometry/visibility already rides its `FeatureView` in [`FeatureViewIndex`].
/// Static per boss, so the rebuild re-clones only on a genuine change.
#[derive(Clone, Debug, PartialEq)]
pub struct BossRenderView {
    pub name: String,
    pub behavior_id: String,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct BossRenderIndex {
    views: std::collections::HashMap<String, (BossRenderView, u64)>,
    generation: u64,
}

impl BossRenderIndex {
    pub fn get(&self, id: &str) -> Option<&BossRenderView> {
        self.views.get(id).map(|(view, _)| view)
    }

    /// Iterate every `(id, view)` boss identity row — the "which ids are
    /// bosses" oracle presentation passes join against `FeatureViewIndex`.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &BossRenderView)> {
        // AMBITION_REVIEW(determinism): hash-order iteration is safe here.
        // `SimView` is DERIVED state — rebuilt from the sim every tick, structurally
        // excluded from `SimSnapshot` and from the N0.4 state hash (netcode.md
        // §Excluded). Every consumer of this iterator is presentation. No sim state
        // reads it, so its order can never enter a trajectory.
        self.views.iter().map(|(id, (view, _))| (id.as_str(), view))
    }

    pub fn len(&self) -> usize {
        self.views.len()
    }

    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    fn begin_rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn end_rebuild(&mut self) {
        let gen = self.generation;
        self.views.retain(|_, (_, g)| *g == gen);
    }

    fn upsert(&mut self, id: &str, name: &str, behavior_id: &str) {
        let gen = self.generation;
        if let Some(slot) = self.views.get_mut(id) {
            if slot.0.name == name && slot.0.behavior_id == behavior_id {
                slot.1 = gen;
                return;
            }
            slot.0 = BossRenderView {
                name: name.to_string(),
                behavior_id: behavior_id.to_string(),
            };
            slot.1 = gen;
            return;
        }
        self.views.insert(
            id.to_string(),
            (
                BossRenderView {
                    name: name.to_string(),
                    behavior_id: behavior_id.to_string(),
                },
                gen,
            ),
        );
    }
}

/// Rebuild [`BossRenderIndex`] from the live boss clusters. Runs in the sim's
/// `FeatureViewSync` set beside the other read-model rebuilds; boss identity is
/// static, so the cost is a per-boss `&str` compare with no allocation once
/// materialized.
pub fn rebuild_boss_render_index(
    mut index: ResMut<BossRenderIndex>,
    bosses: Query<(&FeatureId, ambition_actors::features::BossClusterRef)>,
) {
    index.begin_rebuild();
    for (id, boss) in &bosses {
        index.upsert(
            id.as_str(),
            boss.config.name.as_str(),
            boss.config.behavior.id.as_str(),
        );
    }
    index.end_rebuild();
}

/// One labeled actor's nameplate facts for this frame, resolved sim-side
/// (E4 slices 5+16): the display label, the anchor geometry, and whether
/// this is the body the local player is DRIVING (the controlled subject's
/// own plate is suppressed). Door plates stay render-side (they are static
/// presentation entities); this index carries the ACTOR half.
#[derive(Clone, Debug, PartialEq)]
pub struct NameplateFact {
    pub label: String,
    pub center: ae::Vec2,
    pub size: ae::Vec2,
    pub controlled: bool,
}

/// Per-frame nameplate rows for every eligible (alive, visible) labeled
/// actor, keyed by [`FeatureId`]. Mark-and-sweep like the sibling indexes so
/// surviving ids re-use their `String` allocations.
#[derive(Resource, Default, Clone, Debug)]
pub struct NameplateIndex {
    rows: std::collections::HashMap<String, (NameplateFact, u64)>,
    generation: u64,
}

impl NameplateIndex {
    pub fn get(&self, id: &str) -> Option<&NameplateFact> {
        self.rows.get(id).map(|(fact, _)| fact)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &NameplateFact)> {
        // AMBITION_REVIEW(determinism): hash-order iteration is safe here.
        // `SimView` is DERIVED state — rebuilt from the sim every tick, structurally
        // excluded from `SimSnapshot` and from the N0.4 state hash (netcode.md
        // §Excluded). Every consumer of this iterator is presentation. No sim state
        // reads it, so its order can never enter a trajectory.
        self.rows.iter().map(|(id, (fact, _))| (id.as_str(), fact))
    }

    pub fn contains(&self, id: &str) -> bool {
        self.rows.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn begin_rebuild(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn end_rebuild(&mut self) {
        let gen = self.generation;
        self.rows.retain(|_, (_, g)| *g == gen);
    }

    fn upsert(
        &mut self,
        id: &str,
        label: &str,
        center: ae::Vec2,
        size: ae::Vec2,
        controlled: bool,
    ) {
        let gen = self.generation;
        if let Some(slot) = self.rows.get_mut(id) {
            let f = &slot.0;
            if f.label == label
                && f.center == center
                && f.size == size
                && f.controlled == controlled
            {
                slot.1 = gen;
                return;
            }
            slot.0 = NameplateFact {
                label: label.to_string(),
                center,
                size,
                controlled,
            };
            slot.1 = gen;
            return;
        }
        self.rows.insert(
            id.to_string(),
            (
                NameplateFact {
                    label: label.to_string(),
                    center,
                    size,
                    controlled,
                },
                gen,
            ),
        );
    }
}

/// Rebuild [`NameplateIndex`] from every identity-bearing sim actor — the
/// same query + liveness rules render's nameplate pass used to declare,
/// moved sim-side. Chains AFTER [`rebuild_feature_view_index`] (it prefers
/// the view row's geometry/visibility over the raw AABB, exactly like the
/// old render read).
#[allow(clippy::type_complexity)]
pub fn rebuild_nameplate_index(
    mut index: ResMut<NameplateIndex>,
    controlled: Option<Res<ambition_platformer_primitives::markers::ControlledSubject>>,
    primary_player: Query<Entity, ambition_platformer_primitives::markers::PrimaryPlayerOnly>,
    views: Res<FeatureViewIndex>,
    actors: Query<
        (
            Entity,
            &FeatureId,
            &ActorIdentity,
            &CenteredAabb,
            Option<&ambition_characters::actor::BodyCombat>,
            Option<&ambition_characters::actor::BodyHealth>,
            Option<&BossPhase>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    // The camera/HUD/nameplates all follow the CONTROLLED SUBJECT — the body
    // carrying `Brain::Player(PRIMARY)` — with the primary-player fallback
    // for the startup frame before the subject resolver has run.
    let controlled_body = controlled
        .as_deref()
        .and_then(|subject| subject.0)
        .or_else(|| primary_player.single().ok());
    index.begin_rebuild();
    for (entity, feature_id, identity, aabb, combat, health, boss_phase) in &actors {
        // Dead actors carry no plate (defeated boss / drained pool).
        if boss_phase.is_some_and(|phase| phase.is_defeated())
            || combat.is_some_and(|combat| !combat.alive)
            || health.is_some_and(|health| !health.alive())
        {
            continue;
        }
        let (center, size, visible) = views
            .get(feature_id.as_str())
            .map(|view| (view.pos, view.size, view.visible))
            .unwrap_or_else(|| (aabb.center, aabb.size(), true));
        if !visible {
            continue;
        }
        index.upsert(
            feature_id.as_str(),
            identity.name(),
            center,
            size,
            Some(entity) == controlled_body,
        );
    }
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
            breakable_state: None,
            chest_opened: false,
            fighting: false,
            switch_on: false,
            rotation_rad: 0.0,
            alive: true,
            hit_flash_secs: 0.0,
            hp_current: 0,
            hp_max: 0,
            training_dummy: false,
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

    #[test]
    fn actor_render_index_snapshots_identity_sweeps_and_refreshes() {
        let mut idx = ActorRenderIndex::default();
        // Frame 1: two actors materialized.
        idx.begin_rebuild();
        idx.upsert(
            "a",
            "Goblin",
            None,
            false,
            Some(ae::Vec2::new(10.0, 20.0)),
            None,
        );
        idx.upsert("b", "Dummy", Some("sandbag_sheet"), true, None, None);
        idx.end_rebuild();
        assert_eq!(idx.len(), 2);
        let a = idx.get("a").expect("a present");
        assert_eq!(a.name, "Goblin");
        assert_eq!(a.render_size, Some(ae::Vec2::new(10.0, 20.0)));
        assert!(!a.is_sandbag);
        assert!(a.sprite_override_name.is_none());
        let b = idx.get("b").expect("b present");
        assert!(b.is_sandbag);
        assert_eq!(b.sprite_override_name.as_deref(), Some("sandbag_sheet"));
        assert!(b.render_size.is_none());

        // Frame 2: "a" survives UNCHANGED (refreshed in place); "b" despawns → swept.
        idx.begin_rebuild();
        idx.upsert(
            "a",
            "Goblin",
            None,
            false,
            Some(ae::Vec2::new(10.0, 20.0)),
            None,
        );
        idx.end_rebuild();
        assert_eq!(idx.len(), 1, "the despawned 'b' is swept");
        assert!(idx.get("b").is_none());
        assert_eq!(idx.get("a").map(|v| v.name.as_str()), Some("Goblin"));

        // Frame 3: "a"'s facts CHANGE (a hostile flip re-sizes it) → updated in place.
        idx.begin_rebuild();
        idx.upsert(
            "a",
            "Goblin",
            None,
            false,
            Some(ae::Vec2::new(30.0, 40.0)),
            None,
        );
        idx.end_rebuild();
        assert_eq!(
            idx.get("a").and_then(|v| v.render_size),
            Some(ae::Vec2::new(30.0, 40.0)),
            "changed facts are re-materialized, not stuck on the old snapshot"
        );
    }
}
