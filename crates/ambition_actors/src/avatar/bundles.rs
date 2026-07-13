//! Player ECS spawn bundles.

use ambition_engine_core as ae;
use ambition_engine_core::CenteredAabb;
use bevy::prelude::*;

use super::components::{PlayerBlinkCameraState, PlayerEntity, PlayerSafetyState, PrimaryPlayer};
use super::movement_components::BodyKinematics;
use crate::actor::AncillaryMovementBundle;
use crate::actor::{BodyAnimFacts, BodyMelee};
use crate::body_mode::BodyModeCapabilities;
use crate::control::{LocalPlayer, PlayerInputFrame, PlayerSlot};
use crate::features::{ActorFaction, ActorPose};
use ambition_characters::actor::{BodyCombat, BodyHealth, BodyWallet};
use ambition_characters::brain::{ActionSet, ActorControl, Brain};

/// All simulation components required on the player entity.
///
/// Use this bundle in `commands.spawn()` together with presentation-side
/// components (`Transform`, `PlayerVisual`) so the spawn call documents
/// what simulation state the player entity carries. The bundle does not
/// include `Transform` or `Sprite` — those are presentation concerns.
/// Identity tag bundle: every player entity carries exactly these
/// components. Useful as a building block in tests that want to spawn
/// an additional player without rebuilding the full simulation bundle.
#[derive(Bundle)]
pub struct PlayerIdentityBundle {
    pub marker: PlayerEntity,
    pub slot: PlayerSlot,
}

impl PlayerIdentityBundle {
    pub fn new(slot: PlayerSlot) -> Self {
        Self {
            marker: PlayerEntity,
            slot,
        }
    }
}

#[derive(Bundle)]
pub struct PlayerSimulationBundle {
    pub identity: PlayerIdentityBundle,
    pub primary: PrimaryPlayer,
    /// Runtime-side marker (`ambition_platformer_primitives`) tagging this as the
    /// body whose position drives live gravity resolution. The gravity runtime
    /// queries `With<PrimaryBody>` instead of the sandbox's player markers, so
    /// the gravity layer stays content-free.
    pub primary_body: ambition_platformer_primitives::body::PrimaryBody,
    pub local: LocalPlayer,
    pub health: BodyHealth,
    pub wallet: BodyWallet,
    pub combat: BodyCombat,
    /// Body-mode kit: the home player can crouch / morph / climb. A possessed
    /// actor uses ITS OWN capabilities (this is the home body's).
    pub body_mode_caps: BodyModeCapabilities,
    pub anim: BodyAnimFacts,
    pub blink_cam: PlayerBlinkCameraState,
    pub attack: BodyMelee,
    pub safety: PlayerSafetyState,
    pub input: PlayerInputFrame,
    pub faction: ActorFaction,
    pub name: Name,
    /// Universal-brain seam. The player entity carries a
    /// `Brain::Player(slot)`, an `ActionSet` (its full moveset), and
    /// an `ActorControl` that the brain-driver system fills each
    /// frame from `PlayerInputFrame`. Until Chunk 4d/e wires the
    /// authority to consume the frame, the brain and control
    /// component are *parallel* state — they're built but nothing
    /// reads them yet.
    pub brain: Brain,
    pub action_set: ActionSet,
    /// The player's melee as DATA (fable review R2.5 / I7): the controlled
    /// character's own swing, DERIVED into directional variants by
    /// `build_actor_moveset`, run through the SAME moveset runtime every actor
    /// uses. `MovesetMelee` makes the flat swing (`start_body_melee`) skip this
    /// body. Built from `action_set.melee`, so whatever character the player
    /// wears defines the melee — the non-player-centric / relativity principle:
    /// human, brain, or RL all attach to the same character behavior. Ranged
    /// stays on the player's charge system (`None` here), specials on the
    /// `Special` channel — `MovesetMelee` folds only the melee.
    pub moveset: crate::combat::moveset::ActorMoveset,
    pub moveset_melee: crate::combat::moveset::MovesetMelee,
    pub actor_control: ActorControl,
    /// Capability marker: this body uses the chargeable-projectile (Fireball)
    /// ability. Gates `emit_player_projectile_tick_messages` by CAPABILITY rather
    /// than `brain.is_player()`, so possession of this body keeps the charge
    /// mechanic. Pay-for-use: actors without it never enter the charge stream.
    pub charges_projectiles: ambition_characters::brain::ChargesProjectiles,
    /// Gameplay-space action origin / facing read model shared with
    /// non-player actors. Synced from `BodyKinematics`, not from any
    /// presentation `Transform`.
    pub actor_pose: ActorPose,
    // The authoritative movement-cluster components. `kinematics` is the
    // shared kinematic truth (its own component); the other 18 ancillary
    // clusters spawn through the shared `AncillaryMovementBundle` — the SAME
    // bundle every actor nests, so player and actor carry the identical real
    // component set. Every engine entry point reads / writes them through
    // `BodyClustersMut`. See `engine_core/body_clusters.rs` for the
    // per-cluster shape.
    pub kinematics: BodyKinematics,
    /// Explicit swappable movement policy. Every integrated body owns one;
    /// absence is never interpreted as the axis-swept default.
    pub motion_model: crate::features::MotionModel,
    /// The body's published combat footprint, ORIENTED to its gravity frame —
    /// the SAME single-source-of-truth component every actor publishes
    /// (fable review 2026-07-02 §A6: consumers used to rebuild the player
    /// hurtbox per-site, and the rebuilds had diverged). Written each tick by
    /// `integrate_home_body`.
    pub hurtbox: CenteredAabb,
    pub movement: AncillaryMovementBundle,
    /// Per-player projectile state — spawner cooldowns, charge timer,
    /// motion-input buffer, in-flight body list. Was previously a
    /// global `Res<PlayerProjectileState>`; per-actor migration so
    /// co-op / possession builds get one independent set per player.
    pub projectile: crate::projectile::PlayerProjectileState,
}

impl PlayerSimulationBundle {
    /// Build the canonical local-primary player bundle from a
    /// `BodyClusterScratch` and initial `Health`. The result spawns
    /// with `PlayerSlot(0)`, `PrimaryPlayer`, and `LocalPlayer` — the
    /// single-player default.
    ///
    /// Future code that needs to spawn a second / guest / remote
    /// player should compose `PlayerIdentityBundle::new(PlayerSlot(n))`
    /// with the simulation components manually rather than calling
    /// this helper, since the second player should not inherit
    /// `PrimaryPlayer` and may not be `LocalPlayer`.
    pub fn from_scratch(
        scratch: ae::BodyClusterScratch,
        health: ambition_characters::actor::Health,
    ) -> Self {
        let action_set = default_player_action_set(scratch.abilities.abilities);
        let moveset = crate::combat::moveset::ActorMoveset(
            crate::combat::moveset::build_actor_moveset(None, action_set.melee.as_ref(), None)
                .unwrap_or_default(),
        );
        let initial_safe_pos = scratch.kinematics.pos;
        // `BodyKinematics` is the shared kinematic truth (its own component);
        // copy it out before the rest folds into the shared movement bundle.
        let kinematics = scratch.kinematics;
        let hurtbox = CenteredAabb::from_center_size(kinematics.pos, kinematics.size);
        Self {
            identity: PlayerIdentityBundle::new(PlayerSlot::PRIMARY),
            primary: PrimaryPlayer,
            primary_body: ambition_platformer_primitives::body::PrimaryBody,
            local: LocalPlayer,
            health: BodyHealth::new(health),
            wallet: BodyWallet::default(),
            combat: BodyCombat::default(),
            body_mode_caps: BodyModeCapabilities::full(),
            anim: BodyAnimFacts::default(),
            blink_cam: PlayerBlinkCameraState::default(),
            attack: BodyMelee::default(),
            safety: PlayerSafetyState::new(initial_safe_pos),
            input: PlayerInputFrame::default(),
            faction: ActorFaction::Player,
            name: Name::new("Player"),
            brain: Brain::Player(PlayerSlot::PRIMARY),
            // Player ActionSet derived from the player's AbilitySet.
            // Today nothing reads it for combat effects —
            // update_player still spawns hitboxes via the existing
            // pipeline. The set lights up when the ActionSet
            // effect-resolver flip lands (daytime). Possession of a
            // non-player body keeps that body's ActionSet — this
            // default fires only for actual player entities.
            action_set,
            moveset,
            moveset_melee: crate::combat::moveset::MovesetMelee,
            actor_control: ActorControl::default(),
            charges_projectiles: ambition_characters::brain::ChargesProjectiles,
            actor_pose: ActorPose::from_parts(
                kinematics.pos,
                kinematics.size * 0.5,
                kinematics.facing,
            ),
            kinematics,
            motion_model: crate::features::MotionModel::default(),
            hurtbox,
            movement: AncillaryMovementBundle::from_scratch(scratch),
            projectile: crate::projectile::PlayerProjectileState::default(),
        }
    }

    /// Like [`from_scratch`](Self::from_scratch), but the player spawns *as* the
    /// catalog character `character_id`: its display name becomes the entity
    /// [`Name`], and its authored ActionSet **IS the kit** — wearing is a full
    /// re-parametrisation of the one control box (possession semantics: a
    /// goblin swipes, a pirate fires a pistol, a peaceful character does not
    /// secretly shoot the robot's fireballs). Slots the character leaves empty
    /// stay EMPTY. The player box is otherwise untouched — same
    /// `Brain::Player`, same markers, same collision. The chosen character's
    /// SPRITE is bound presentation-side by the reusable `ambition_render`
    /// binder, which reads the `WornCharacter` identity the spawn records — not
    /// here, and not app-locally.
    ///
    /// A row marked [`PlayableKitSource::HostCode`](ambition_characters::actor::character_catalog::PlayableKitSource::HostCode)
    /// (a protagonist whose combat is a runtime `AbilitySet` concern) keeps the
    /// code-built kit — the overlay rebuilds it from the body's own `AbilitySet`,
    /// so wearing that id yields a bundle equivalent to `from_scratch`. This is
    /// keyed on the ROW's declared kit source, not on "is this the content
    /// default": a standalone demo whose default character authors its own kit
    /// gets that authored kit.
    pub fn from_scratch_as_character(
        catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
        scratch: ae::BodyClusterScratch,
        health: ambition_characters::actor::Health,
        character_id: &str,
    ) -> Self {
        // The body's code-side capability set — the source of the protagonist's
        // kit, captured before `scratch` folds into the movement bundle so the
        // overlay can rebuild the code kit deterministically (host-code rows and
        // unknown ids fall back to it).
        let base_abilities = scratch.abilities.abilities;
        let mut bundle = Self::from_scratch(scratch, health);
        // The SAME overlay the runtime re-wear system applies (name + the resolved
        // kit), so spawn and runtime can never disagree on what a character is.
        let _ = crate::avatar::apply_worn_character_overlay(
            catalog,
            &mut bundle.name,
            &mut bundle.action_set,
            &mut bundle.moveset,
            character_id,
            base_abilities,
        );
        bundle
            .motion_model
            .apply_spec(crate::avatar::motion_model_spec_for_character_id(
                catalog,
                character_id,
            ));
        // The returned capability is synchronized on the spawned entity by
        // `apply_worn_character_gameplay` from its Added<WornCharacter> edge.
        // A Bundle cannot conditionally omit a component, so the canonical
        // derive system owns marker insertion/removal before player effects run.
        bundle
    }
}

/// Default player moveset derived from an `AbilitySet`:
/// - `melee = Some(Swipe)` iff `abilities.attack`
/// - `ranged = Some(Bolt)` unconditionally today (the existing
///   fireball / hadouken path is itself gated by `projectile`)
/// - `special = Some(Special("bubble_shield"))` iff `abilities.shield`
///
/// The resolver won't emit `ActionRequest`s for capabilities the
/// player doesn't have, so EFFECTS consumers can
/// read the ActionSet as the authoritative "what can this player
/// actually do right now" surface without re-checking AbilitySet.
pub(crate) fn default_player_action_set(abilities: ae::AbilitySet) -> ActionSet {
    use ambition_characters::brain::{
        MeleeActionSpec, MoveStyleSpec, RangedActionSpec, SpecialActionSpec, SwipeSpec,
    };
    ActionSet {
        melee: abilities
            .attack
            .then_some(MeleeActionSpec::Swipe(SwipeSpec {
                // Player swipe is faster than enemy Striker default
                // — the player's combat tempo runs ~2× snappier.
                windup_s: 0.12,
                active_s: 0.10,
                recover_s: 0.18,
                damage: 1,
                reach_px: 36.0,
            })),
        // The player's "ranged" today is the fireball / hadouken
        // path. There's no separate `projectile` ability in
        // AbilitySet — ranged is always available on the player.
        // If a future ability flag gates fireball, narrow this
        // slot the same way melee + special are.
        ranged: Some(RangedActionSpec::Bolt {
            speed: 600.0,
            damage: 1,
        }),
        move_style: MoveStyleSpec::Walk,
        // Special slot: bubble_shield, gated by the shield ability.
        // A possessed non-player body keeps that body's ActionSet so
        // this default fires only for actual player entities.
        special: abilities
            .shield
            .then_some(SpecialActionSpec::Special("bubble_shield".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::Health;
    use ambition_characters::brain::{MeleeActionSpec, RangedActionSpec};

    fn player_scratch() -> ae::BodyClusterScratch {
        crate::avatar::primary_player_scratch(ae::Vec2::ZERO, ae::AbilitySet::sandbox_all())
    }

    fn catalog() -> ambition_characters::actor::character_catalog::CharacterCatalog {
        ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(include_str!(
                "../../../../game/ambition_content/assets/data/character_catalog.ron"
            )),
        )
    }

    #[test]
    fn wearing_the_default_id_is_the_protagonist() {
        // Explicitly wearing the DEFAULT id keeps the protagonist name and the
        // full code-side player kit — the protagonist is the one row whose kit
        // is NOT its (peaceful) catalog action set. Production installs the
        // default at the content choke point; mirror that here.
        let catalog = catalog();
        let bundle = PlayerSimulationBundle::from_scratch_as_character(
            &catalog,
            player_scratch(),
            Health::new(20),
            "player",
        );
        assert_eq!(bundle.name.as_str(), "Player");
        assert!(bundle.brain.is_player());
        assert!(matches!(
            bundle.action_set.melee,
            Some(MeleeActionSpec::Swipe(_))
        ));
        assert!(matches!(
            bundle.action_set.ranged,
            Some(RangedActionSpec::Bolt { .. })
        ));
    }

    #[test]
    fn player_wears_pirate_admiral_identity_and_moveset() {
        // The player box stays (Brain::Player, PlayerEntity by type), but it now
        // reads as the Pirate Admiral: its name and its authored PISTOL — the
        // worn character's ActionSet IS the kit (no fallback to the player's
        // bolt). Pin the installed default so the protagonist branch is
        // deterministic regardless of test order.
        let catalog = catalog();
        let bundle = PlayerSimulationBundle::from_scratch_as_character(
            &catalog,
            player_scratch(),
            Health::new(20),
            "npc_pirate_admiral",
        );
        assert_eq!(bundle.name.as_str(), "Pirate Admiral");
        assert!(bundle.brain.is_player(), "still keyboard-controlled");
        assert!(
            matches!(
                bundle.action_set.ranged,
                Some(RangedActionSpec::Pistol { .. })
            ),
            "the pirate's pistol should override the player's default bolt",
        );
    }

    #[test]
    fn unknown_character_id_still_spawns_a_controllable_player() {
        // A stale / unknown id keeps the player fully playable: the KIT falls back
        // to the defined code kit (rebuilt from the body's abilities), and it is
        // still Brain::Player. The NAME becomes the id itself — a legible
        // diagnostic, never a stale prior name. The sprite falls back to the
        // colored rectangle presentation-side.
        let catalog = catalog();
        let bundle = PlayerSimulationBundle::from_scratch_as_character(
            &catalog,
            player_scratch(),
            Health::new(20),
            "not_a_real_character",
        );
        assert!(bundle.brain.is_player());
        assert_eq!(
            bundle.name.as_str(),
            "not_a_real_character",
            "an unknown id names the body after the id (deterministic diagnostic)"
        );
        assert!(matches!(
            bundle.action_set.melee,
            Some(MeleeActionSpec::Swipe(_))
        ));
    }
}
