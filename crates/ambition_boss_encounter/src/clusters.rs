//! Authoritative boss ECS components and `BossMut` / `BossRef` views.
//!
//! [`BossConfig`] owns identity and authored policy and is also the boss marker.
//! [`BossEncounter`] owns encounter-only state. Health, combat state, and
//! kinematics use the same shared body components as other actors. Mutable boss
//! queries stay disjoint from other actor archetypes through the marker.

use super::behavior::{canonical_boss_id_from, BossBehaviorProfile, BossBehaviorProfileExt};
use super::BossEncounterPhase;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::snapshot::{put_bool, put_f32, SnapshotCursor, SnapshotState};
use ambition_platformer2d_core::AabbExt;
use ambition_sprite_sheet::ActorSpriteMetrics;
use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

use ambition_platformer2d_shared_tangle::body::BodyKinematics;

/// Authored configuration + identity for a boss actor. Also serves as
/// the boss marker component (see module docs).
#[derive(Component, Clone, Debug)]
pub struct BossConfig {
    pub id: String,
    pub name: String,
    /// Authored spawn anchor; `reset` restores `kin.pos` to it.
    pub spawn: ae::Vec2,
    pub brain: ambition_entity_catalog::placements::BossBrain,
    pub behavior: BossBehaviorProfile,
}

/// Mutable encounter-only boss state. Health, liveness, and hit flash live on
/// the shared body components.
#[derive(Component, Clone, Debug)]
pub struct BossEncounter {
    /// Sprite-driven body metrics, resolved at construction from the baked
    /// sheet. `None` for a boss whose sheet publishes no `body_metrics` (then
    /// the authored `combat_size` is its body).
    pub sprite_metrics: Option<ActorSpriteMetrics>,
    /// The sprite render-basis size: the box the sheet's
    /// `render_size(basis)` scales the drawn quad from (the LDtk spawn seed).
    /// `kin.size` is the collision envelope (`combat_size`), which the shared
    /// movement seam sweeps, so the render basis is stored here. The render
    /// (`upgrade_boss_sprites` / `animate_bosses`) reads it via
    /// [`BossRef::render_size`]. Distinct from
    /// `sprite_metrics.sprite_render_size`, the derived world quad; this is
    /// the input the sheet spec scales.
    pub render_size: ae::Vec2,
    /// Entity-local phase state and intrinsic phase triggers. Together with
    /// `health`, this is fight authority; music, walls, HUD, and other encounter
    /// presentation remain encounter-owned.
    pub encounter: Option<super::ActorPhaseState>,
}

impl BossEncounter {
    /// The active encounter phase: the entity-local phase state's, `Dormant`
    /// until it is seeded. Read from the state itself; a stored copy lagged it
    /// by whatever ran between the phase machine and its readers.
    pub fn encounter_phase(&self) -> BossEncounterPhase {
        self.encounter
            .as_ref()
            .map_or(BossEncounterPhase::Dormant, |state| state.phase)
    }
}

/// Per-spawn boss tweaks: the data behind "spawn boss X with tweaks Z at
/// position Y".
///
/// Carried on the spawned boss entity and read at seed time by
/// `update_boss_encounters` (hp / size / phase triggers) and by
/// `sync_boss_encounter_entities` (the encounter opt-out). `Default` is no
/// tweaks (use the archetype profile).
#[derive(bevy::prelude::Component, Clone, Debug, Default)]
pub struct BossOverrides {
    /// Override max HP (also the starting HP). `None`  the profile's `max_hp`.
    pub max_hp: Option<i32>,
    /// Override the combat/contact box size. `None` uses the profile's
    /// `combat_size`.
    pub combat_size: Option<ae::Vec2>,
    /// Override the intrinsic phase triggers as data. `Some(vec![])` means the
    /// boss never phases up (it fights to death, for example a boss reused as
    /// a plain tough enemy); `None` uses the profile-derived triggers.
    pub phase_triggers: Option<Vec<super::PhaseTrigger>>,
    /// Spawn the boss without an encounter wrapper, as a plain tough enemy:
    /// no HUD, no lock-walls, no win/lose (`sync_boss_encounter_entities`
    /// skips it). The creature still fights and dies normally.
    pub no_encounter: bool,
}

/// Immutable borrow view over the boss clusters. Hosts the read-only
/// geometry/identity helpers.
pub struct BossRef<'a> {
    pub kin: &'a BodyKinematics,
    pub config: &'a BossConfig,
    pub status: &'a BossEncounter,
}

/// Mutable borrow view over the boss clusters. Hosts the integration /
/// profile-mutation helpers.
pub struct BossMut<'a> {
    pub kin: &'a mut BodyKinematics,
    pub config: &'a mut BossConfig,
    pub status: &'a mut BossEncounter,
}

impl<'a> BossRef<'a> {
    /// The sprite render-basis size (the drawn quad's scale input). This is
    /// not `kin.size`, which is the collision envelope (`combat_size`); it is
    /// the stored spawn-seed basis. See [`BossEncounter::render_size`].
    pub fn render_size(&self) -> ae::Vec2 {
        self.status.render_size
    }

    /// The collision body: `kin.size`, which construction resolved once.
    /// `behavior.combat_size` is authoring input to that resolution, not a
    /// second answer.
    pub fn combat_size(&self) -> ae::Vec2 {
        self.kin.size
    }

    /// World offset from `kin.pos` to the body's bounding-AABB center.
    /// Non-zero for bosses whose sprite metadata reports an off-center
    /// body bbox; `ZERO` otherwise.
    ///
    /// Mirrored horizontally when the boss faces left: the sprite flips to face
    /// the player, so an off-center body's collision/contact envelope must flip
    /// with it (otherwise it lands on the wrong side). No-op for a centered body
    /// (`combat_offset.x == 0`).
    pub fn combat_offset(&self) -> ae::Vec2 {
        let raw = self
            .status
            .sprite_metrics
            .as_ref()
            .map(|m| m.combat_offset)
            .unwrap_or(ae::Vec2::ZERO);
        if self.kin.facing < 0.0 {
            ae::Vec2::new(-raw.x, raw.y)
        } else {
            raw
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(
            self.kin.pos + self.combat_offset(),
            self.combat_size() * 0.5,
        )
    }

    /// World-space anchor for a combat-banter speech bubble, from the
    /// profile's authored [`BarkAnchorSpec`] (multi-part bosses hang
    /// the bubble off-center; the default sits just above the body).
    pub fn bark_anchor(&self) -> ae::Vec2 {
        let spec = self.config.behavior.bark_anchor;
        let half_h = self.combat_size().y * 0.5;
        ae::Vec2::new(
            self.kin.pos.x + spec.dx_px,
            self.kin.pos.y + spec.dy_half_h * half_h + spec.dy_px,
        )
    }
}

impl<'a> BossMut<'a> {
    /// Reborrow as an immutable view to reach the read-only helpers.
    pub fn as_ref(&self) -> BossRef<'_> {
        BossRef {
            kin: self.kin,
            config: self.config,
            status: self.status,
        }
    }

    pub fn combat_size(&self) -> ae::Vec2 {
        self.as_ref().combat_size()
    }

    pub fn aabb(&self) -> ae::Aabb {
        self.as_ref().aabb()
    }

    pub fn bark_anchor(&self) -> ae::Vec2 {
        self.as_ref().bark_anchor()
    }

    pub fn render_size(&self) -> ae::Vec2 {
        self.as_ref().render_size()
    }

    pub fn apply_behavior_profile(&mut self, behavior: BossBehaviorProfile) {
        self.config.behavior = behavior;
    }

    // `reset_to_spawn` is in the room-reset system (its only caller): a boss
    // respawn is a discrete transit (ADR 0024) and needs the unified
    // actor-cluster view and MotionModel, which this view does not carry.

    // Boss body integration is on the shared movement seam:
    // `integrate_boss_bodies` → `ActorMut::update` → the flight limb in
    // direct-velocity mode. A boss is an aerial actor.
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct BossClusterQueryData {
    pub kin: &'static mut BodyKinematics,
    pub config: &'static mut BossConfig,
    pub status: &'static mut BossEncounter,
}

impl<'w, 's> BossClusterQueryDataItem<'w, 's> {
    pub fn as_boss_mut<'a>(&'a mut self) -> BossMut<'a>
    where
        'w: 'a,
        's: 'a,
    {
        BossMut {
            kin: &mut self.kin,
            config: &mut self.config,
            status: &mut self.status,
        }
    }

    /// Immutable view of the same components — for read-only helpers
    /// (`aabb`, `combat_size`, `from_ref`, …) on a mutable boss query.
    pub fn as_boss_ref<'a>(&'a self) -> BossRef<'a>
    where
        'w: 'a,
        's: 'a,
    {
        BossRef {
            kin: &self.kin,
            config: &self.config,
            status: &self.status,
        }
    }
}

#[derive(QueryData)]
pub struct BossClusterRef {
    pub kin: &'static BodyKinematics,
    pub config: &'static BossConfig,
    pub status: &'static BossEncounter,
}

impl<'w, 's> BossClusterRefItem<'w, 's> {
    pub fn as_boss_ref(&self) -> BossRef<'_> {
        BossRef {
            kin: self.kin,
            config: self.config,
            status: self.status,
        }
    }
}

/// Owned aggregate for spawn construction / non-ECS callers (tests,
/// the gnu_ton_rider encounter setup). Mirrors the enemy/NPC scratch.
#[derive(Clone, Debug)]
pub struct BossClusterScratch {
    pub kin: BodyKinematics,
    pub config: BossConfig,
    pub status: BossEncounter,
    /// The boss's HP authority: the same `BodyHealth` component every body
    /// carries. Spawned from here; never mirrored from boss state.
    pub health: ambition_characters::actor::BodyHealth,
}

impl BossClusterScratch {
    /// Build the boss clusters directly from spawn inputs (tests / non-ECS
    /// callers; see the struct docs).
    pub fn new(
        boss_catalog: &super::BossCatalog,
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: ambition_entity_catalog::placements::BossBrain,
    ) -> Self {
        let name = name.into();
        // Behavior lookup prefers the brain's `PhaseScript:` id over the LDtk
        // display name, so a "System Boss" room whose brain is
        // `PhaseScript:clockwork_warden` resolves to the clockwork_warden
        // profile.
        let canonical_id = canonical_boss_id_from(&name, &brain);
        let center = aabb.center();
        let behavior = BossBehaviorProfile::for_authored_boss(boss_catalog, &canonical_id);
        // The LDtk spawn box is the sprite render basis (`render_size`).
        // `kin.size` holds the collision body, so the shared movement seam
        // sweeps the right box. See `resolve_sheet_body` for its source.
        let render_basis = aabb.half_size() * 2.0;
        let collision_size = behavior.combat_size.unwrap_or(render_basis);
        let mut boss = Self {
            kin: BodyKinematics {
                pos: center,
                // Bosses float: the brain emits a new `desired_vel` each tick,
                // so `vel` is never integrated and stays `ZERO`.
                vel: ae::Vec2::ZERO,
                size: collision_size,
                facing: 1.0,
            },
            config: BossConfig {
                id: id.into(),
                name,
                spawn: center,
                brain,
                behavior,
            },
            status: BossEncounter {
                sprite_metrics: None,
                encounter: None,
                render_size: render_basis,
            },
            health: ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(18),
            ),
        };
        boss.resolve_sheet_body(boss_catalog);
        boss
    }

    /// Size the body from the sheet it draws, when that sheet publishes
    /// `body_metrics`: the sprite metrics, and a collision body that bounds
    /// the drawn body parts. A sheet with no metrics leaves the authored
    /// `combat_size`. The sheet table is baked, so every build answers the
    /// same. Call it again after a change to `config.behavior`.
    pub fn resolve_sheet_body(&mut self, boss_catalog: &super::BossCatalog) {
        let Some((metrics, body)) = crate::ecs::boss_sprite_metrics_from_registry(
            boss_catalog,
            self.as_ref(),
            ambition_sprite_sheet::shared_baked_sheet_registry(),
        ) else {
            return;
        };
        self.status.sprite_metrics = Some(metrics);
        if let Some(body) = body {
            self.kin.size = body;
        }
    }

    pub fn as_mut(&mut self) -> BossMut<'_> {
        BossMut {
            kin: &mut self.kin,
            config: &mut self.config,
            status: &mut self.status,
        }
    }

    pub fn as_ref(&self) -> BossRef<'_> {
        BossRef {
            kin: &self.kin,
            config: &self.config,
            status: &self.status,
        }
    }

    /// The authoritative components as a spawnable Bundle (incl. the body's
    /// `BodyHealth` HP authority).
    pub fn into_components(
        self,
    ) -> (
        BodyKinematics,
        BossConfig,
        BossEncounter,
        ambition_characters::actor::BodyHealth,
    ) {
        (self.kin, self.config, self.status, self.health)
    }
}

/// Whether this boss placement is recorded `Cleared` in the save.
///
/// "Cleared" is keyed by the boss's unique placement id (`config.id`), not
/// the archetype, so the same archetype at another placement is not marked
/// defeated. This is the predicate for the ECS road (everything holding a
/// `BossConfig`: `update_boss_encounters` and the cut-rope victory NPC in
/// `victory.rs`). It delegates to [`placement_is_cleared`], which
/// construction asks before a `BossConfig` exists, so they agree.
///
/// It is not the only reading of the fact. The authored-condition road
/// (`conditions::cleared`, Yarn's `boss.cleared(...)`) reads the state
/// directly because it must explain why a false answer is false (`Untouched`
/// and `Failed` give different `WhyNot` text). Both read one authority:
/// `save.data().boss(id)`.
pub fn boss_is_cleared(
    save: &ambition_persistence::save::AmbitionGameSave,
    config: &BossConfig,
) -> bool {
    placement_is_cleared(save.data(), &config.id)
}

/// The same predicate before a `BossConfig` exists, for code that decides how
/// to build a placement (a construction commit, a programmatic spawn) and has
/// only its id.
pub fn placement_is_cleared(
    save: &ambition_persistence::save_data::AmbitionGameSaveData,
    placement_id: &str,
) -> bool {
    matches!(
        save.boss(placement_id),
        ambition_persistence::save_data::PersistedEncounterState::Cleared
    )
}

/// Which boss placements the save records Cleared, as a system parameter, so
/// a spawner that must not name the save can still build a cleared placement
/// defeated.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ClearedBossPlacements<'w> {
    save: bevy::prelude::Res<'w, ambition_persistence::save::AmbitionGameSave>,
}

impl ClearedBossPlacements<'_> {
    pub fn is_cleared(&self, placement_id: &str) -> bool {
        placement_is_cleared(self.save.data(), placement_id)
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod test_support {
    //! Shared boss test fixtures: one definition of a test `BossEncounter` /
    //! `BossConfig`, so a new field updates every test module.
    use super::super::{ActorPhaseState, PhaseTrigger};
    use super::*;

    /// Install the empty durable save that [`ClearedBossPlacements`] reads, so
    /// a fixture with a spawner builds under the same authority as
    /// production, without naming the persistence crate.
    pub fn install_empty_save(app: &mut bevy::prelude::App) {
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    }

    /// A `(BossEncounter, BodyHealth)` pair at `hp` HP in `phase`, with
    /// entity-local `ActorPhaseState` carrying `triggers` (empty means never
    /// phases up) already set to `phase`. HP is on the shared `BodyHealth`.
    pub fn test_boss_status_with(
        hp: i32,
        phase: BossEncounterPhase,
        triggers: Vec<PhaseTrigger>,
    ) -> (BossEncounter, ambition_characters::actor::BodyHealth) {
        let mut encounter = ActorPhaseState::new(triggers);
        encounter.phase = phase;
        let mut health = ambition_characters::actor::Health::new(hp);
        health.current = hp;
        (
            BossEncounter {
                sprite_metrics: None,
                encounter: Some(encounter),
                // Test fixtures don't render; a placeholder render basis is fine.
                render_size: ae::Vec2::splat(64.0),
            },
            ambition_characters::actor::BodyHealth::new(health),
        )
    }

    /// A `(BossEncounter, BodyHealth)` at `hp` HP in `phase` with no phase
    /// triggers (fights to death; the common single-phase fixture).
    pub fn test_boss_status(
        hp: i32,
        phase: BossEncounterPhase,
    ) -> (BossEncounter, ambition_characters::actor::BodyHealth) {
        test_boss_status_with(hp, phase, Vec::new())
    }

    /// A `BossConfig` whose brain `PhaseScript` and behavior profile both resolve
    /// to `script_id`'s authored profile (their real coupling), with the given
    /// placement `id` + display `name`.
    pub fn test_boss_config(
        id: impl Into<String>,
        name: impl Into<String>,
        script_id: &str,
    ) -> BossConfig {
        BossConfig {
            id: id.into(),
            name: name.into(),
            spawn: ae::Vec2::ZERO,
            brain: ambition_entity_catalog::placements::BossBrain::PhaseScript {
                script_id: script_id.to_string(),
            },
            behavior: BossBehaviorProfile::for_authored_boss(
                super::super::test_boss_catalog(),
                script_id,
            ),
        }
    }
}

/// The boss's entity-local phase state.
///
/// A cursor, because the rest of `BossEncounter` is sprite metrics derived
/// from the sheet registry, and `ActorPhaseState.triggers` is authored data.
impl SnapshotCursor for BossEncounter {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        match &self.encounter {
            None => put_bool(out, false),
            Some(e) => {
                put_bool(out, true);
                e.phase.encode(out);
                put_f32(out, e.phase_elapsed);
                put_f32(out, e.transition_lock);
                e.start_phase.encode(out);
            }
        }
    }
}
