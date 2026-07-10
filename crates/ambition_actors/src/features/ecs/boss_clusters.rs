//! Authoritative ECS components for a boss actor + the `BossMut` /
//! `BossRef` views the per-tick systems mutate / read in place.
//!
//! Bosses follow the enemy / NPC cluster pattern: real ECS state split across
//! [`BossConfig`] (identity, spawn anchor, brain, behavior profile) and
//! [`BossEncounter`] (encounter phase, derived sprite metrics, entity-local
//! phase machine — HP/liveness live on the shared `BodyHealth`, damage-blink on
//! `BodyCombat::hit_flash`, like every body). The boss carries the shared
//! [`BodyKinematics`] component (pos / vel / size / facing) — the same component
//! the player and enemies/NPCs use. Since the archetype swap (AS4c) a boss IS an
//! aerial actor: its body integrates through the SHARED flight limb
//! (`integrate_boss_bodies` → `ActorMut::update`, direct-velocity), NOT a bespoke
//! float. A `&mut BodyKinematics` boss query is kept disjoint from player/enemy
//! ones with `With<BossConfig>` / `Without<BossConfig>` filters (boss / enemy /
//! player are mutually exclusive archetypes).
//!
//! [`BossConfig`] doubles as the *is-a-boss* marker component — every boss
//! entity carries exactly one, no other actor does — so boss / non-boss systems
//! filter on `With<BossConfig>` / `Without<BossConfig>`.

use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

use crate::boss_encounter::behavior::{
    canonical_boss_id_from, ActorSpriteMetrics, BossBehaviorProfile,
};
use crate::boss_encounter::BossEncounterPhase;
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;

pub use crate::platformer_runtime::body::BodyKinematics;

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

/// Mutable per-tick boss ENCOUNTER status: active phase, sprite-derived body
/// metrics, and the entity-local phase machine.
///
/// §A1 authority flip (fable review 2026-07-02): health / liveness / hit-flash
/// are NOT here anymore — a boss's HP authority is the same [`ambition_characters::actor::BodyHealth`]
/// every body carries (alive = `health.alive()`), and its damage-blink is
/// [`ambition_characters::actor::BodyCombat::hit_flash`]. What remains is genuinely
/// encounter-specific.
#[derive(Component, Clone, Debug)]
pub struct BossEncounter {
    /// Active encounter phase. Forwarded by `sync_boss_encounter_phase`
    /// from `BossEncounterRegistry`. `Dormant` until the encounter
    /// wakes up. The brain reads this via `BossPatternContext`.
    pub encounter_phase: BossEncounterPhase,
    /// Sprite-driven body metrics — populated by the
    /// `derive_boss_sprite_metrics` system after the SheetRegistry
    /// has loaded. `None` for bosses whose sprite has no `body_metrics`
    /// entry (the legacy `combat_size` path applies).
    pub sprite_metrics: Option<ActorSpriteMetrics>,
    /// The sprite RENDER-BASIS size — the collision box the sheet's
    /// `render_size(basis)` scales the drawn quad from (the LDtk spawn seed).
    /// Archetype swap AS4b: `kin.size` becomes the COLLISION envelope
    /// (`combat_size`) so the boss integrates through the shared movement seam
    /// (which sweeps `kin.size`), so the render basis can no longer BE `kin.size`.
    /// The render (`upgrade_boss_sprites` / `animate_bosses`) reads this via
    /// [`BossRef::render_size`], keeping the drawn sprite byte-identical across the
    /// flip. (Deliberately distinct from `sprite_metrics.sprite_render_size`, the
    /// derived world quad; this is the *input* the sheet spec scales.)
    pub render_size: ae::Vec2,
    /// Entity-local phase state + the trigger-driven phase mechanism: current
    /// phase, the `transition_lock` tell timer, and the intrinsic phase triggers
    /// as DATA. This + `health` ARE the source of truth for the fight (the old
    /// global registry live-map is gone); the encounter-only concerns (per-phase
    /// music, lock-walls, HUD, display) live on the data catalog / the optional
    /// encounter entity. `update_boss_encounters` seeds it once from the boss's
    /// `BossProfile` (or `BossOverrides`) and ticks it. Keeping the state ON the
    /// entity is what makes two of the same boss (a gauntlet) carry independent
    /// fights by construction rather than by a string-keyed side map. See
    /// `docs/planning/boss-entity-local-refactor.md`. `None` until seeded.
    pub encounter: Option<crate::boss_encounter::BossPhaseState>,
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
    /// The sprite RENDER-BASIS size (the drawn quad's collision scale input).
    /// Post-AS4b this is NO LONGER `kin.size` (which is now the collision envelope,
    /// `combat_size`) — it's the stored spawn-seed basis, so the drawn sprite is
    /// unchanged by the size flip. See [`BossEncounter::render_size`].
    pub fn render_size(&self) -> ae::Vec2 {
        self.status.render_size
    }

    /// Multi-part bosses (GNU-ton) expose a `combat_size` distinct from
    /// the sprite `size`; that's the size collision and volumes use.
    pub fn combat_size(&self) -> ae::Vec2 {
        self.config.behavior.combat_size.unwrap_or(self.kin.size)
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

    /// Full same-room revive of a boss: restore the authored spawn pose +
    /// liveness + HP and CLEAR the entity-local encounter so
    /// `update_boss_encounters` re-seeds fresh phase state (Dormant → wake) the
    /// next frame. The single definition of "revive a boss" — the room-reset
    /// loop routes through it (the actor mirror of `ActorMut::reset_to_spawn`),
    /// so adding a [`BossEncounter`] field can't desync the revive from the
    /// seed/save-skip paths.
    ///
    /// Clearing `encounter` is load-bearing: keep last attempt's `Death` phase
    /// and the death-resolution re-kills the boss the instant it "respawns" (the
    /// in-place replay regression, pinned by `boss_revives_after_a_room_reset`).
    /// The brain / attack-state / control are separate components the room-reset
    /// loop clears alongside this.
    pub fn reset_to_spawn(
        &mut self,
        health: &mut ambition_characters::actor::BodyHealth,
        combat: &mut ambition_characters::actor::BodyCombat,
    ) {
        self.kin.pos = self.config.spawn;
        self.kin.facing = 1.0;
        health.reset();
        combat.reset();
        combat.alive = true;
        self.status.encounter = None;
        self.status.encounter_phase = BossEncounterPhase::Dormant;
    }

    // Boss body integration lives on the SHARED movement seam now (archetype swap
    // AS4c): `integrate_boss_bodies` → `ActorMut::update` → the flight limb in
    // direct-velocity mode. A boss IS just an aerial actor — no bespoke float.
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
    /// The boss's HP authority — the SAME `BodyHealth` component every body
    /// carries (§A1). Spawned from here; never mirrored from boss state.
    pub health: ambition_characters::actor::BodyHealth,
}

impl BossClusterScratch {
    /// Build the boss clusters directly from spawn inputs (tests / non-ECS
    /// callers; see the struct docs).
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: ambition_entity_catalog::placements::BossBrain,
    ) -> Self {
        let name = name.into();
        // Behavior lookup prefers the brain's `PhaseScript:` id over the
        // LDtk display name, so a "System Boss" room whose brain is
        // `PhaseScript:clockwork_warden` still resolves to the
        // clockwork_warden / Gradient Sentinel profile.
        let canonical_id = canonical_boss_id_from(&name, &brain);
        let center = aabb.center();
        let behavior = BossBehaviorProfile::for_authored_boss(&canonical_id);
        // AS4b: the LDtk spawn box is the sprite RENDER-BASIS (`render_size`); the
        // COLLISION envelope is `combat_size` (the profile's, refined later by
        // `derive_boss_sprite_metrics`). `kin.size` carries the COLLISION size so the
        // shared movement seam sweeps the right box (AS4c); the render reads
        // `render_size` so the drawn sprite is unchanged.
        let render_basis = aabb.half_size() * 2.0;
        let collision_size = behavior.combat_size.unwrap_or(render_basis);
        Self {
            kin: BodyKinematics {
                pos: center,
                // Bosses float; the brain emits a fresh `desired_vel` each
                // tick (consumed by `integrate_body`), so `vel` is never
                // integrated and stays `ZERO`.
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
                encounter_phase: BossEncounterPhase::Dormant,
                sprite_metrics: None,
                encounter: None,
                render_size: render_basis,
            },
            health: ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(18),
            ),
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

/// Whether this boss PLACEMENT is recorded `Cleared` in the save.
///
/// R4 keys "cleared" by the boss's unique runtime/LDtk placement id
/// (`config.id`), NOT the archetype — so the same archetype reused at another
/// placement is not pre-marked defeated. The single definition of the
/// "cleared" predicate, called by both the room-load save-sync
/// (`sync_ecs_bosses_with_save`) and the per-tick encounter driver
/// (`update_boss_encounters`) so the skip-check can't drift between them.
pub fn boss_is_cleared(
    save: &ambition_persistence::save::SandboxSave,
    config: &BossConfig,
) -> bool {
    matches!(
        save.data().boss(&config.id),
        ambition_persistence::save_data::PersistedEncounterState::Cleared
    )
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Shared boss test fixtures. One definition of "a test `BossEncounter` /
    //! `BossConfig`" so the boss test modules build the same shape — adding a
    //! field updates them all at once instead of drifting per-module.
    use super::*;
    use crate::boss_encounter::{BossPhaseState, PhaseTrigger};

    /// A `(BossEncounter, BodyHealth)` pair at `hp` HP in `phase`, with
    /// entity-local `BossPhaseState` carrying `triggers` (empty ⇒ never phases
    /// up) already set to `phase`. HP lives on the shared `BodyHealth` (§A1).
    pub(crate) fn test_boss_status_with(
        hp: i32,
        phase: BossEncounterPhase,
        triggers: Vec<PhaseTrigger>,
    ) -> (BossEncounter, ambition_characters::actor::BodyHealth) {
        let mut encounter = BossPhaseState::new(triggers);
        encounter.phase = phase;
        let mut health = ambition_characters::actor::Health::new(hp);
        health.current = hp;
        (
            BossEncounter {
                encounter_phase: phase,
                sprite_metrics: None,
                encounter: Some(encounter),
                // Test fixtures don't render; a placeholder render basis is fine.
                render_size: ae::Vec2::splat(64.0),
            },
            ambition_characters::actor::BodyHealth::new(health),
        )
    }

    /// A `(BossEncounter, BodyHealth)` at `hp` HP in `phase` with no phase
    /// triggers (fights to death — the common single-phase fixture).
    pub(crate) fn test_boss_status(
        hp: i32,
        phase: BossEncounterPhase,
    ) -> (BossEncounter, ambition_characters::actor::BodyHealth) {
        test_boss_status_with(hp, phase, Vec::new())
    }

    /// A `BossConfig` whose brain `PhaseScript` and behavior profile both resolve
    /// to `script_id`'s authored profile (their real coupling), with the given
    /// placement `id` + display `name`.
    pub(crate) fn test_boss_config(
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
            behavior: BossBehaviorProfile::for_authored_boss(script_id),
        }
    }
}
