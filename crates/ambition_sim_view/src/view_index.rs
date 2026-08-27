//! `FeatureViewIndex` resource and the per-frame rebuild pass.
//!
//! Presentation systems consult this read-model for sprite swaps, debug overlays,
//! and HUD readouts instead of re-scanning every feature family per visual.

use ambition_platformer2d_core as ae;
use bevy::prelude::{Entity, Query, Res, ResMut, Resource, With, Without};

use crate::anim_index::ActorSpriteData;
use ambition_combat::components::ActorDisposition;
use ambition_combat::components::ActorIdentity;
use ambition_combat::components::ActorRenderSize;
use ambition_combat::components::BodyMelee;
use ambition_combat::components::BossDeathAnimation;
use ambition_combat::components::BossPhase;
use ambition_combat::components::BreakableFeature;
use ambition_combat::components::ChestFeature;
use ambition_combat::components::Collected;
use ambition_combat::components::FeatureId;
use ambition_combat::components::Opened;
use ambition_combat::components::PickupFeature;
use ambition_encounter::switches::{SwitchFeature, SwitchOn};
use ambition_platformer2d_actor_monolith::features::ActorConfig;
use ambition_platformer2d_actor_monolith::features::HazardFeature;
use ambition_platformer2d_core::ActorSurfaceState;
use ambition_platformer2d_core::CenteredAabb;
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;

/// One feature's per-frame render snapshot — THE read-model row of [`FeatureViewIndex`].
#[derive(Clone, Copy, Debug)]
pub struct FeatureView {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub kind: FeatureVisualKind,
    pub visible: bool,
    pub flash: bool,
    /// For `FeatureVisualKind::Breakable`: the current authored breakable
    /// state, so presentation can select intact/cracked/broken art without
    /// querying live ECS feature components. `None` for every other kind.
    pub breakable_state: Option<ambition_interaction::BreakableState>,
    /// For `FeatureVisualKind::Chest`: true once the chest has been opened.
    /// Ignored for every other kind.
    pub chest_opened: bool,
    /// For `FeatureVisualKind::Actor`: true when the actor is in the FIGHTING
    /// state (a fact about the actor itself — NOT "hostile to the player";
    /// relativity principle). A STATE flag exactly like `flash`: a provoked NPC
    /// enters it, an at-rest enemy hasn't engaged yet. Stamped at the rebuild
    /// site from the disposition signal until the fighting-state machinery moves
    /// onto a `FightingAble` capability component. Ignored for non-actor kinds.
    pub fighting: bool,
    /// For `FeatureVisualKind::Switch`: true when the switch reads as
    /// "on" (encounter cleared / reset path armed). Renders green when
    /// true, red when false. Ignored for other kinds.
    pub switch_on: bool,
    /// Z-axis rotation to apply to the rendered sprite, in radians
    /// (Bevy frame; +π/2 is CCW). Non-zero for surface-walking
    /// archetypes that crawl on walls/ceilings; everyone else
    /// reports 0.0 and renders axis-aligned. Uses the engine → Bevy
    /// rotation mapping shared by actor rendering.
    pub rotation_rad: f32,
    /// Presentation (nameplates, debug bars) reads THIS, never the live clusters.
    pub alive: bool,
    /// Seconds remaining on the damage flash (actors + live bosses; `0.0`
    /// for everything else, including a boss corpse — death rows are
    /// authored sprites and must not read as a lit silhouette).
    pub hit_flash_secs: f32,
    /// Seconds left on this body's PARRY CATCH — a parry that actually caught a
    /// strike, not a parry window standing open.
    ///
    /// `0.0` almost always; positive for a short beat starting on the tick a
    /// perfect shield turned a strike away, whether the strike was a swing or a
    /// shot. Armed by `BodyShieldState::catch_parry` at the two seams that
    /// resolve a parry, so one fact covers both routes.
    ///
    /// ⛔ never `BodyShieldState::parrying()`, which answers whether the WINDOW
    /// is open and is therefore true of every raised guard for a few ticks —
    /// a cue driven off that one fires on every shield raise.
    pub parry_flash_secs: f32,
    /// Health facts for the kinds that carry a pool (actors, bosses,
    /// breakables); `0/0` elsewhere. Debug overlays read these by id.
    pub hp_current: i32,
    pub hp_max: i32,
    /// Actor rows only: the sandbag/training-dummy depiction flag the debug
    /// health overlay colors by.
    pub training_dummy: bool,
    /// HOW HARD the hit currently freezing this body was, `0..=1`, and `0.0`
    /// when no hitlag is running.
    ///
    /// Resolved by `ambition_platformer2d_core::hit_response::hit_strength_fraction`
    /// from the hitlag the hit already set — the same quantity camera shake
    /// reads. `0.0` for every feature that is not a body.
    pub hit_strength: f32,
    /// This body CANNOT BE STRUCK right now — the presentation half of
    /// `ambition_combat::util::body_vulnerable`, resolved at this one site so
    /// no renderer ever re-derives hit eligibility from a pose or a move name.
    ///
    /// Covers every body-generic grant at once because the damage rule does:
    /// dodge / spot dodge / air dodge, tech and getup, the ledge grab's earned
    /// intangibility, the timed untouchable a respawn hands out, and the
    /// i-frames a hit leaves behind. `false` for every feature that is not a
    /// body.
    pub unhittable: bool,
    /// WHY the canonical damage gate is closed, preserved as semantic
    /// presentation vocabulary. A route can opt individual causes into shared
    /// effects without the read-model growing `unhittable_beyond_*` fields.
    pub defense_cues: crate::DefenseCueCauses,
    /// `Some`  this body PUBLISHES where its sprite quad goes relative to
    /// `pos` (see `ActorSpriteOffset`), and that placement is authoritative:
    /// the renderer centres the quad and shifts it by this, instead of using
    /// the sheet's one static feet anchor. The two are alternatives, never
    /// summed — a per-pose placement already accounts for the feet.
    ///
    /// `None` (every other feature, and every actor that doesn't opt in)  the
    /// legacy placement, unchanged.
    pub sprite_offset: Option<ae::Vec2>,
}

/// Per-frame snapshot of every ECS-owned feature's `FeatureView`, keyed
/// by [`FeatureId`].
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
    /// Build an index from explicit rows.
    ///
    /// The per-frame rebuild is the only writer in production, and its inserts
    /// are private so nothing can slip a row into a live read-model out of band.
    /// That also meant NOBODY outside this crate could build one — so a
    /// consumer's presentation pass, which takes this index as its whole input,
    /// had no way to be unit-tested at all. An engine another game is built on
    /// has to hand that game a fixture for the read-models it publishes.
    ///
    /// Constructs a NEW index rather than mutating an existing one, so it cannot
    /// be misused to edit the live one mid-frame.
    ///
    /// The parameter is `entries` rather than `rows` because the determinism lint matches
    /// std-hash bindings by NAME across the whole file, and `NameplateIndex` further down owns
    /// a `rows: HashMap`.
    pub fn from_rows(entries: impl IntoIterator<Item = (String, FeatureView)>) -> Self {
        let mut index = Self::default();
        for (id, view) in entries {
            index.views.insert(id, (view, index.generation));
        }
        index
    }

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
    // The reference the hitlag law scales from, so the published strength is a
    // fraction rather than a raw freeze presentation would have to interpret.
    feel: Option<Res<ambition_combat::feel::Platformer2dFeelTuningMonolith>>,
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
            // The two clusters the damage rule reads that this pass did not
            // already hold: the evade window and the guard.
            Option<&ae::BodyMotionFacts>,
            Option<&ambition_platformer2d_actor_monolith::actor::BodyShieldState>,
            // Portal aerial-roll (same component the player uses) so actors
            // somersault + self-right through portals just like the player.
            Option<
                &ambition_platformer2d_actor_monolith::platformer_runtime::orientation::ActorRoll,
            >,
            // Sheet-authored quad placement, for a body whose art does not sit
            // centred in its frame. Absent for every ordinary actor.
            Option<&ambition_platformer2d_actor_monolith::features::ActorSpriteOffset>,
            bevy::prelude::Has<ambition_combat::stocks::RespawnGrace>,
        ),
        // Bosses carry the shared actor read-models (`ActorDisposition` etc., synced by
        // `sync_boss_actor_components`) but are their OWN feature family below. Without this
        // exclusion a boss matches here too and — because the actor family is inserted before the
        // boss family (first-wins priority) — it gets classified as an invisible generic `Actor`
        // (its `ActorStatus`/`ActorConfig` are absent), shadowing the boss view → the boss renders
        // as the generic fallback sprite instead of its sheet.
        Without<ambition_boss_encounter::BossConfig>,
    >,
    hazards: Query<(&FeatureId, &CenteredAabb, &HazardFeature)>,
    bosses: Query<(
        &FeatureId,
        ambition_boss_encounter::BossClusterRef,
        &ambition_characters::brain::BossAttackState,
        // Presentation reads alive / hit-flash from here instead of the BossRuntime fields, the
        // same component enemies/NPCs expose.
        &ambition_characters::actor::BodyCombat,
        Option<&ambition_characters::actor::BodyHealth>,
        Option<&BossDeathAnimation>,
        Option<&BossPhase>,
        // Gravity-upright roll — the SAME `ActorRoll` the player / enemies / NPCs
        // use, so a boss rights itself under flipped / sideways gravity instead of
        // staying screen-axis-aligned (it floats, but it should still flip).
        Option<&ambition_platformer2d_actor_monolith::platformer_runtime::orientation::ActorRoll>,
    )>,
) {
    index.begin_rebuild();
    // No feel tuning means no hitlag law to measure against: every body reports
    // no strength rather than a number derived from a reference nobody set.
    let hitlag_reference = feel.as_deref().map_or(0.0, |feel| feel.hitlag_time);
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
                parry_flash_secs: 0.0,
                hp_current: 0,
                hp_max: 0,
                training_dummy: false,
                hit_strength: 0.0,
                unhittable: false,
                defense_cues: crate::DefenseCueCauses::NONE,
                sprite_offset: None,
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
                parry_flash_secs: 0.0,
                hp_current: 0,
                hp_max: 0,
                training_dummy: false,
                hit_strength: 0.0,
                unhittable: false,
                defense_cues: crate::DefenseCueCauses::NONE,
                sprite_offset: None,
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
                parry_flash_secs: 0.0,
                hp_current: breakable.breakable.health.current,
                hp_max: breakable.breakable.health.max,
                training_dummy: false,
                hit_strength: 0.0,
                unhittable: false,
                defense_cues: crate::DefenseCueCauses::NONE,
                sprite_offset: None,
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
                parry_flash_secs: 0.0,
                hp_current: 0,
                hp_max: 0,
                training_dummy: false,
                hit_strength: 0.0,
                unhittable: false,
                defense_cues: crate::DefenseCueCauses::NONE,
                sprite_offset: None,
            },
        );
    }
    for (
        id,
        aabb,
        disposition,
        combat,
        health,
        attack,
        config,
        surface,
        motion,
        shield,
        roll,
        sprite_offset,
        respawn_grace,
    ) in &actors
    {
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
                // Liveness for presentation (nameplates, debug bars), from the AUTHORITY. An actor
                // with no pool reads alive (it has nothing to die from).
                alive: !health.is_some_and(|h| !h.alive()),
                hit_flash_secs: combat.map_or(0.0, |c| c.hit_flash),
                parry_flash_secs: shield.map_or(0.0, |s| s.parry_caught_timer),
                hp_current: health.map_or(0, |h| h.current()),
                hp_max: health.map_or(0, |h| h.max()),
                training_dummy: combat.is_some_and(|c| c.training_dummy),
                hit_strength: ae::hit_response::hit_strength_fraction(
                    combat.map_or(0.0, |c| c.hitstop_timer),
                    hitlag_reference,
                ),
                // THE DAMAGE RULE ITSELF, inverted — not a second reading of
                // it. A body missing one of these clusters cannot be protected
                // by it, so the default stands in.
                unhittable: !ambition_combat::util::body_vulnerable(
                    health.map_or_else(ambition_characters::actor::Invulnerability::none, |h| {
                        h.health.invulnerable
                    }),
                    motion.is_some_and(|m| m.evading()),
                    &shield.copied().unwrap_or_default(),
                    &combat.copied().unwrap_or_default(),
                ),
                defense_cues: crate::defense_cue_causes(
                    health.map_or_else(ambition_characters::actor::Invulnerability::none, |h| {
                        h.health.invulnerable
                    }),
                    motion,
                    &shield.copied().unwrap_or_default(),
                    &combat.copied().unwrap_or_default(),
                    respawn_grace,
                ),
                sprite_offset: sprite_offset.map(|o| o.0),
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
                parry_flash_secs: 0.0,
                hp_current: 0,
                hp_max: 0,
                training_dummy: false,
                hit_strength: 0.0,
                unhittable: false,
                defense_cues: crate::DefenseCueCauses::NONE,
                sprite_offset: None,
            },
        );
    }
    for (id, feature, attack_state, combat, health, death_anim, phase, roll) in &bosses {
        let boss = feature.as_boss_ref();
        // pos / size still come from `BossRuntime` until the boss body migrates
        // to `CenteredAabb` (ecs-cleanup-plan #9).
        // §A1: a boss's liveness is its `BodyHealth`, never a mirror.
        let boss_alive = health.is_some_and(|h| h.alive());
        let visible = boss_alive
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
                alive: boss_alive && !phase.is_some_and(|p| p.is_defeated()),
                // A boss corpse must not read as a lit silhouette — death
                // rows are authored sprites (the old render-side rule).
                hit_flash_secs: if boss_alive { combat.hit_flash } else { 0.0 },
                // A boss carries no guard, so it never catches a parry.
                parry_flash_secs: 0.0,
                hp_current: health.map_or(0, |h| h.current()),
                hp_max: health.map_or(0, |h| h.max()),
                training_dummy: false,
                hit_strength: 0.0,
                unhittable: false,
                defense_cues: crate::DefenseCueCauses::NONE,
                sprite_offset: None,
            },
        );
    }
    // Sweep entries for features that despawned this frame (those not
    // re-inserted under the current generation); surviving keys are reused.
    index.end_rebuild();
}

/// Materialized per-actor identity facts the renderer needs to BIND and SIZE an actor sprite,
/// keyed by [`FeatureId`] — the STATIC half of the actor read-model (display name,
/// sprite-override label, sandbag flag, explicit render-quad size). These facts are static per
/// actor, so the rebuild re-clones only on a genuine change (otherwise it just refreshes the
/// mark-and-sweep generation — no per-`String` churn as the sim steps).
#[derive(Clone, Debug, PartialEq)]
pub struct ActorRenderView {
    pub name: String,
    /// Catalog identity used for art lookup. It is independent of the actor's
    /// display name.
    pub sprite_character_id: Option<String>,
    pub sprite_override_name: Option<String>,
    pub is_sandbag: bool,
    pub render_size: Option<ae::Vec2>,
    pub dream_seed: Option<f32>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct ActorRenderIndex {
    views: std::collections::HashMap<String, (ActorRenderView, u64)>,
    generation: u64,
}

impl ActorRenderIndex {
    /// Build an index from explicit rows — see
    /// [`FeatureViewIndex::from_rows`] for why a read-model owes its consumers
    /// a fixture constructor.
    pub fn from_rows(entries: impl IntoIterator<Item = (String, ActorRenderView)>) -> Self {
        let mut index = Self::default();
        for (id, view) in entries {
            index.views.insert(id, (view, index.generation));
        }
        index
    }

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
        sprite_character_id: Option<&str>,
        override_name: Option<&str>,
        is_sandbag: bool,
        render_size: Option<ae::Vec2>,
        dream_seed: Option<f32>,
    ) {
        let gen = self.generation;
        if let Some(slot) = self.views.get_mut(id) {
            let v = &slot.0;
            let unchanged = v.name == name
                && v.sprite_character_id.as_deref() == sprite_character_id
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
                sprite_character_id: sprite_character_id.map(str::to_string),
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
                    sprite_character_id: sprite_character_id.map(str::to_string),
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
            a.config.sprite_character_id.as_deref(),
            a.config.sprite_override_npc_name.as_deref(),
            a.combat.training_dummy,
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
    /// Build an index from explicit rows — see
    /// [`FeatureViewIndex::from_rows`] for why a read-model owes its consumers
    /// a fixture constructor.
    pub fn from_rows(entries: impl IntoIterator<Item = (String, BossRenderView)>) -> Self {
        let mut index = Self::default();
        for (id, view) in entries {
            index.views.insert(id, (view, index.generation));
        }
        index
    }

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
    bosses: Query<(&FeatureId, ambition_boss_encounter::BossClusterRef)>,
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
    /// Is a PARTICIPANT driving this body? ⛔ NOT "is the camera on it" — the
    /// two were conflated, and `label_driven_bodies` wants this one.
    pub driven: bool,
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

    fn upsert(&mut self, id: &str, label: &str, center: ae::Vec2, size: ae::Vec2, driven: bool) {
        let gen = self.generation;
        if let Some(slot) = self.rows.get_mut(id) {
            let f = &slot.0;
            if f.label == label && f.center == center && f.size == size && f.driven == driven {
                slot.1 = gen;
                return;
            }
            slot.0 = NameplateFact {
                label: label.to_string(),
                center,
                size,
                driven,
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
                    driven,
                },
                gen,
            ),
        );
    }
}

#[allow(clippy::type_complexity)]
pub fn rebuild_nameplate_index(
    mut index: ResMut<NameplateIndex>,
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
            // ⭐⭐ IS THIS BODY DRIVEN — asked of the body, not of the camera.
            bevy::prelude::Has<ambition_characters::control::DrivingParticipant>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    // ⛔⛔ A PLURAL POLICY WAS PROJECTED FROM ONE SINGULAR BODY. The room rule
    // `label_driven_bodies` is documented to apply uniformly to every body
    // SOMEBODY IS DRIVING, and this computed a single `controlled_body` from
    // `ControlledSubject` (the CAMERA's focus, with a `PrimaryPlayer` fallback)
    // and flagged each row by `Some(entity) == controlled_body`. In a couch match
    // that suppresses ONE driven fighter's plate and leaves the other's.
    //
    // ⭐ `DrivingParticipant` IS THE AUTHORITY AND ALREADY EXISTS —
    // `ControlledBodiesView` projects it correctly and says why in its own
    // comment: *"a couch-versus match has two driven bodies and neither is more
    // protected than the other"*.
    //
    // ⇒ THE BUG WAS A CONFLATION of two different facts: the ONE body
    // presentation focuses on, versus ANY body a participant drives. The plate
    // policy wants the second.

    index.begin_rebuild();
    for (_entity, feature_id, identity, aabb, _combat, health, boss_phase, driven) in &actors {
        // Dead actors carry no plate (defeated boss / drained pool).
        if boss_phase.is_some_and(|phase| phase.is_defeated())
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
        index.upsert(feature_id.as_str(), identity.name(), center, size, driven);
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
            parry_flash_secs: 0.0,
            hp_current: 0,
            hp_max: 0,
            training_dummy: false,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: crate::DefenseCueCauses::NONE,
            sprite_offset: None,
        }
    }

    /// THE PUBLICATION SEAM for a parry, and the bug it is named for.
    ///
    /// A caught parry is a full negation: no hit event, no landed-hit fact, no
    /// cost to the guard. So the published `parry_flash_secs` is the ONLY thing
    /// a spectator's cue can read, and if this row failed to carry it the cue
    /// would be silently dead with every other test still green.
    ///
    /// The negative half is the whole point: a guard merely inside its parry
    /// WINDOW must publish nothing. That window is open on every raised shield
    /// for a few ticks, so a row that reported it would clang and flash on
    /// every shield raise.
    #[test]
    fn the_published_parry_beat_is_the_catch_and_never_the_open_window() {
        use ambition_platformer2d_actor_monolith::actor::BodyShieldState;

        let published = |shield: BodyShieldState| {
            let mut app = bevy::prelude::App::new();
            app.init_resource::<FeatureViewIndex>();
            app.world_mut().spawn((
                FeatureId("seat_0".to_string()),
                CenteredAabb::from_center_size(ae::Vec2::ZERO, ae::Vec2::new(30.0, 48.0)),
                ActorDisposition::Hostile,
                shield,
            ));
            app.add_systems(bevy::prelude::Update, rebuild_feature_view_index);
            app.update();
            app.world()
                .resource::<FeatureViewIndex>()
                .get("seat_0")
                .expect("the actor family published a row")
                .parry_flash_secs
        };

        // A guard that CAUGHT a strike.
        let caught = BodyShieldState {
            active: true,
            parry_caught_timer: 0.18,
            ..Default::default()
        };
        assert_eq!(published(caught), 0.18, "the catch reaches the row");

        // A guard whose parry window is merely OPEN — every raised shield, for
        // its first few ticks.
        let window_open = BodyShieldState {
            active: true,
            parry_window_timer: 0.10,
            ..Default::default()
        };
        assert_eq!(
            published(window_open),
            0.0,
            "an open window is not a parry: a cue on this fires on every raise"
        );

        // And an ordinary raised guard, well past its window.
        let holding = BodyShieldState {
            active: true,
            ..Default::default()
        };
        assert_eq!(published(holding), 0.0);
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
            None,
            false,
            Some(ae::Vec2::new(10.0, 20.0)),
            None,
        );
        idx.upsert("b", "Dummy", None, Some("sandbag_sheet"), true, None, None);
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

#[cfg(test)]
mod driven_nameplate_tests {
    use super::*;

    /// EVERY DRIVEN BODY IS DRIVEN — not just the one the camera is on.
    ///
    /// ⛔⛔ THE POLICY IS PLURAL AND THE PRODUCER WAS SINGULAR.
    /// `label_driven_bodies` applies uniformly to every body somebody is
    /// driving, and this flag was computed as `Some(entity) == controlled_body`
    /// against `ControlledSubject` — the CAMERA's focus, with a `PrimaryPlayer`
    /// fallback. In a couch match that suppresses one driven fighter's plate and
    /// leaves the other's, which is the room policy applied to half the room.
    ///
    /// ⛔ NO PRIMARY DISTINCTION APPEARS IN THIS TEST, deliberately: the moment
    /// one does, the old conflation is back.
    #[test]
    fn two_driven_bodies_are_both_driven_and_an_undriven_one_is_not() {
        use ambition_characters::control::DrivingParticipant;
        let mut app = bevy::prelude::App::new();
        app.init_resource::<NameplateIndex>();
        app.init_resource::<FeatureViewIndex>();
        app.add_systems(bevy::prelude::Update, rebuild_nameplate_index);

        let body = |app: &mut bevy::prelude::App, id: &str, driver: Option<u8>| {
            let mut e = app.world_mut().spawn((
                FeatureSimEntity,
                FeatureId(id.to_string()),
                ActorIdentity::new(id.to_string(), id.to_string()),
                CenteredAabb::new(
                    ambition_platformer2d_core::Vec2::ZERO,
                    ambition_platformer2d_core::Vec2::new(16.0, 32.0),
                ),
            ));
            if let Some(slot) = driver {
                e.insert(DrivingParticipant(
                    ambition_characters::control::PlayerSlot(slot),
                ));
            }
        };
        // TWO drivers, and a body nobody drives.
        body(&mut app, "a", Some(0));
        body(&mut app, "b", Some(1));
        body(&mut app, "c", None);
        app.update();

        let index = app.world().resource::<NameplateIndex>();
        let driven_of = |id: &str| index.rows.get(id).map(|(fact, _)| fact.driven);
        assert_eq!(
            (driven_of("a"), driven_of("b"), driven_of("c")),
            (Some(true), Some(true), Some(false)),
            "a driven body was not reported as driven — the flag is still asking \
             which ONE body the camera is on rather than which bodies have a \
             participant driving them"
        );
    }
}
