//! Complete prepared-character body construction.
//!
//! This is construction authority, not a later projection repair: spawn roads and
//! match activation use the same grant primitive so a body is complete on the tick
//! it is created. The actor monolith may project/retract these facts later, but it
//! does not own their construction vocabulary.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::construction::EntityScope;

/// Character definition currently projected onto a body, including the catalog
/// generation that produced it so stale projections can be detected.
///
/// TODO(character-projection): record displaced moveset values as well as granted
/// values so removing an authored moveset can restore the body's prior one.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ProjectedCharacterKit {
    pub id: String,
    /// The cast this body's projected kit was derived from.
    pub generation: ambition_characters::prepared::CharacterCatalogGeneration,
    /// Facts granted by the projected character, recorded so they can be
    /// retracted even if the character changes or leaves the catalog.
    pub granted: GrantedBodyFacts,
}

/// What the projection put on a body, so it can take exactly that back.
///
/// `retract` DESTRUCTURES this struct, so adding a fact here is a COMPILE ERROR
/// until it is retracted too. That is the whole reason it is a struct rather than
/// three fields: the coupling is real, so it should be enforced rather than
/// remembered.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrantedBodyFacts {
    pub hurtboxes: bool,
    pub movement_tuning: bool,
    /// A sprite-authored body: the posed-body marker AND the standing geometry
    /// granted with it, carrying what that geometry displaced.
    pub posed_body: Option<DisplacedGeometry>,
}

/// The geometry a sprite-authored grant replaced, captured in the grant's own
/// batch. `None` = the body did not carry that component, so retraction
/// removes it rather than inventing a value.
///
/// The standing box is identity: a reset restores the collider from it and a
/// stance divides it. Removing only the marker left the outgoing character's
/// box on a body that no longer wears its art.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DisplacedGeometry {
    pub base_size: Option<ambition_platformer2d_core::Vec2>,
    pub render_size: Option<ambition_platformer2d_core::Vec2>,
    pub sprite_offset: Option<ambition_platformer2d_core::Vec2>,
}

impl GrantedBodyFacts {
    /// What projecting `prepared` onto a body WILL grant.
    ///
    /// `movement_tuning` is the CALLER's resolved answer, not
    /// `prepared.movement_tuning`: a seat in a match may be granted a body its
    /// character never authored, and a record that read the definition would
    /// then fail to retract exactly the fact that WAS granted. See
    /// [`grant_prepared_character_body`].
    fn of(
        prepared: &ambition_characters::prepared::PreparedCharacterDefinition,
        movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
    ) -> Self {
        Self {
            hurtboxes: prepared.hurtboxes.is_some(),
            movement_tuning: movement_tuning.is_some(),
            // Filled by the grant's capture edit, which reads the body.
            posed_body: posed_body_for(prepared).map(|_| DisplacedGeometry::default()),
        }
    }

    /// Take back exactly what was granted, and nothing else.
    ///
    /// Removing only what THIS system granted is what keeps it from fighting the
    /// worn path, which owns the movement-feel marker for a body whose feel came
    /// from the CATALOG — a case this system cannot see and must not overwrite.
    ///
    /// Retracting a sprite body puts back the geometry it displaced and stands
    /// the live collider in that box (under the body's current stance), feet
    /// planted against `gravity_dir`: no pose pass runs for the body once its
    /// marker is gone, so the retraction is the last writer of its shape. An
    /// incoming sprite character's grant, later in the same batch, re-captures
    /// the restored values and replaces them.
    pub fn retract(self, scope: &mut EntityScope, gravity_dir: ambition_platformer2d_core::Vec2) {
        // Exhaustive on purpose: a new fact does not compile until it is handled.
        let Self {
            hurtboxes,
            movement_tuning,
            posed_body,
        } = self;
        if hurtboxes {
            scope.remove::<ambition_combat::hurtbox_resolution::AuthoredHurtboxes>();
        }
        if movement_tuning {
            scope.remove::<ambition_platformer2d_core::AuthoredMovementTuning>();
        }
        if let Some(displaced) = posed_body {
            scope.remove::<ambition_sprite_sheet::character::SpritePosedBody>();
            let DisplacedGeometry {
                base_size,
                render_size,
                sprite_offset,
            } = displaced;
            match base_size {
                Some(base_size) => {
                    scope.insert(ambition_platformer2d_core::BodyBaseSize { base_size });
                    scope.queue_entity_edit(move |mut body| {
                        let stance = body
                            .get::<ambition_platformer2d_core::BodyModeState>()
                            .map_or(base_size, |mode| mode.body_mode.shape(base_size).size);
                        if let Some(mut kin) = body.get_mut::<ambition_platformer2d_core::BodyKinematics>() {
                            if kin.size != stance {
                                ambition_platformer2d_core::resize_feet_planted(&mut kin, stance, gravity_dir);
                            }
                        }
                    });
                }
                None => {
                    scope.remove::<ambition_platformer2d_core::BodyBaseSize>();
                }
            }
            match render_size {
                Some(size) => scope.insert(ambition_combat::components::ActorRenderSize(size)),
                None => scope.remove::<ambition_combat::components::ActorRenderSize>(),
            };
            match sprite_offset {
                Some(offset) => scope.insert(ambition_combat::components::ActorSpriteOffset(offset)),
                None => scope.remove::<ambition_combat::components::ActorSpriteOffset>(),
            };
        }
    }
}

/// The sprite-posed body a definition's authored `body` asks for, if any.
///
/// only `BodySource::SpriteAuthored` resolves here.
///
/// Returns `None` when the character authored no sheet: the posed body reads its
/// rectangles off a sheet manifest, so one without a target has nothing to pose
/// against. Preparation already RESOLVES that target, so a typo is named at load
/// rather than producing a body that silently never poses.
fn posed_body_for(
    prepared: &ambition_characters::prepared::PreparedCharacterDefinition,
) -> Option<(
    ambition_sprite_sheet::character::SpritePosedBody,
    ambition_sprite_sheet::character::sheets::PosedBodyGeometry,
)> {
    match prepared.body.as_ref()? {
        ambition_characters::actor::definition::BodySource::SpriteAuthored { world_per_pixel } => {
            let posed = ambition_sprite_sheet::character::SpritePosedBody::new(
                prepared.sheet.as_deref()?,
                *world_per_pixel,
            );
            // The marker and its standing geometry are granted together or not
            // at all: a sheet with no `Idle` rectangle has nothing to stand in,
            // and the pose pass could not project it either.
            let standing = ambition_sprite_sheet::character::sheets::posed_body_geometry(
                &posed.target,
                ambition_sprite_sheet::character::CharacterAnim::Idle,
                posed.world_per_pixel,
            )?;
            Some((posed, standing))
        }
        ambition_characters::actor::definition::BodySource::Explicit { .. } => None,
    }
}

/// Who writes this body's action set and moves — the one axis on which
/// projecting a definition differs between its two callers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KitOwnership {
    /// Put the definition's kit on the body. Construction always does; the
    /// re-template pass does for a body the persona derive cannot see.
    Grant,
    /// `apply_worn_character_gameplay` owns it.
    PersonaDerive,
    /// The CALLER already resolved this body's kit and inserted it — a match
    /// seat, whose repertoire is the character's overlaid with the match's own
    /// override and so cannot be read off the definition alone.
    ///
    /// everything else is granted, and BOTH applied-template records are stamped. That is
    /// the difference from [`Self::PersonaDerive`], which leaves the gameplay baseline to the
    /// derive because the derive is coming.
    CallerResolved,
}

/// Put a body's ranged execution on it as components: the charge capability
/// and its per-body state under `ChargedProjectile`, neither under
/// `MovesetVerb`, whose ranged press is an ordinary move.
///
/// Existing charge state is kept, so re-granting the same character does not
/// drop a charge in progress.
pub fn install_ranged_execution(
    scope: &mut EntityScope,
    execution: ambition_characters::brain::RangedExecution,
) {
    if execution.charges_projectiles() {
        scope.insert(ambition_characters::brain::ChargesProjectiles);
        scope.queue_component_upsert(
            ambition_projectiles::PlayerProjectileState::default,
            |_| {},
        );
    } else {
        scope.remove::<(
            ambition_characters::brain::ChargesProjectiles,
            ambition_projectiles::PlayerProjectileState,
        )>();
    }
}

/// Put every fact a prepared character owns onto a body, as ONE batch.
///
/// the ONE place a prepared definition becomes a body, and that is the
/// point of extracting it. Two callers:
///
/// * CONSTRUCTION, so a normal character actor is COMPLETE on the frame it
/// is built. This is the fix and it is the architecture the rule asked for (§3): *"There should be no next-tick persona grant required for correctness."* A body built this way carries the memo already, so the re-template pass below sees it as current and never touches it.
/// * RE-TEMPLATING — a cast hot reload, a deliberate runtime re-wear — which is what the actor runtime re-template pass is for once ordinary spawning stops depending on it.
///
/// the memo goes in the same batch as the grants, deliberately. Splitting them is the shape
/// investigation kept circling: a save taken between the two restores a world claiming to be
/// projected and missing what the projection grants. One batch, one archetype move, one tick.
pub fn grant_prepared_character_body(
    scope: &mut EntityScope,
    prepared: &ambition_characters::prepared::PreparedCharacterDefinition,
    generation: ambition_characters::prepared::CharacterCatalogGeneration,
    kit: KitOwnership,
    // THE BODY THIS ENTITY PLAYS WITH, already resolved by the caller.
    //
    // NOT read off `prepared` here, and that is the whole point. A seat
    // in a match answers to the MATCH's body as well as its character's, and
    // that weighing belongs in one place (`MatchRules::body_over`) rather than
    // in the materializer, which would then be a second authority on it — the
    // shape the kit already paid for once (`KitOwnership::CallerResolved`).
    // A caller with no match to answer to passes `prepared.movement_tuning`.
    movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
    // What the body's hand holds as this batch lands, so a granted kit is written
    // already folded with it. Only the `Grant` arm writes a kit to fold.
    hand: ambition_characters::repertoire::Hand<'_>,
) {
    {
        // ⭐⭐ EVERY GRANTED CHARACTER BODY PUBLISHES ITS POSE READ MODEL. Which
        // row it draws, which clip, which frame — `BodyPoseView` is a pure
        // function of sim state, declared rollback-DERIVED, and it costs no
        // renderer. Without this marker the read model was gated on
        // `PlayerVisual`, which only the exploration player's avatar ever
        // receives, so no match fighter had one and a headless diagnostic could
        // not say what the engine intended to DRAW.
        scope.insert(ambition_platformer2d_shared_tangle::lifecycle::PosedBody);
        // Construction writes the gameplay baseline unless persona derivation will
        // do so. A newly constructed body displaced nothing, so its baseline has
        // an empty `displaced` set even when the caller resolved its kit.
        if kit != KitOwnership::PersonaDerive {
            scope.insert(ambition_body_seed::PersonaBaseline {
                    id: prepared.id.as_str().to_string(),
                    generation,
                    displaced: Default::default(),
                });
        }
        scope.insert(ProjectedCharacterKit {
            id: prepared.id.as_str().to_string(),
            generation,
            granted: GrantedBodyFacts::of(prepared, movement_tuning),
        });
        // Only bodies without `WornCharacter` use this grant path; worn personas
        // are projected by `apply_worn_character_gameplay`.
        // TODO(compat-remove): give tuning-identified staged bodies a real worn
        // identity, then remove this secondary kit writer.
        if kit == KitOwnership::Grant {
            // That made two writers for one question, on two paths, and they answered
            // it differently: this one wrote what the definition AUTHORED, while the
            // worn path resolved authored-vs-catalog first. A character that authored
            // no action set therefore fought as the worn player and stood empty-handed
            // as player two. Phase A made the answer identical; giving
            // seated bodies `IdentityKit` and `BodyAbilities` makes the WRITER
            // identical, which is the half that stops it happening again.
            let projected_moveset = prepared.kit.projectable_moveset().cloned();
            if let Some(action_set) = prepared.kit.action_set().cloned() {
                // THE BASELINE AND THE LIVE PAIR, FROM ONE RESOLUTION: the live
                // `ActionSet` + `ActorMoveset` are the repertoire fold of this
                // identity and the hand, so a body granted while holding
                // something is not written empty-handed and folded again later.
                let identity = ambition_characters::brain::action_set::IdentityKit::of(
                    action_set,
                    projected_moveset.clone().unwrap_or_default(),
                );
                let live =
                    ambition_characters::repertoire::effective_repertoire(&identity, None, hand);
                // The routing markers are NOT set here. They are derived from
                // the live `ActorMoveset` by `reconcile_moveset_routing_markers`.
                scope.insert((
                    identity,
                    live.action_set,
                    ambition_combat::moveset::ActorMoveset(live.moveset),
                ));
            } else if let Some(moveset) = projected_moveset {
                scope.insert(ambition_combat::moveset::ActorMoveset(moveset));
            }
        }
        // HOW THIS BODY FIRES goes with the kit it fires. The persona derive
        // installs it on the bodies it owns; a road that owns its own kit gets it
        // here, from the CHARACTER even when a match supplied the action set,
        // because a borrowed set does not change how the borrower fires.
        if kit != KitOwnership::PersonaDerive {
            install_ranged_execution(scope, prepared.ranged_execution);
        }
        // The rest is what the persona derive does not own on ANY path: the
        // authored silhouette, the movement feel, and the motion model — body
        // facts rather than kit facts, each with a matching retraction above.
        if let Some(hurtboxes) = prepared.hurtboxes.clone() {
            scope.insert((
                ambition_combat::hurtbox_resolution::AuthoredHurtboxes(hurtboxes),
                ambition_combat::hurtbox_resolution::ResolvedHurtboxes::default(),
                ambition_combat::components::DamageableVolumes::default(),
            ));
        }
        // THE AUTHORED BODY, which had no consumer at all.
        //
        // `CharacterDefinition.body` has existed since §4.11 and nothing read it:
        // a provider could author `SpriteAuthored { world_per_pixel }` and receive
        // a body of some other size entirely.
        //
        // `SpritePosedBody` is the live authority for a sprite-shaped body — it
        // carries exactly this number, and `sync_sprite_posed_bodies` derives the
        // collision box, the sprite quad and its offset from the art every tick.
        // Until now it was inserted from ONE place in the repository: a bespoke
        // app-side system in the Mary-O snake matching on a display name. So body
        // geometry was still declared through a second seam, which is the problem
        // `register_character` exists to delete.
        //
        // The STANDING geometry goes with it, in the same batch: the body's
        // identity box (`BodyBaseSize`, what a reset restores and a stance
        // divides by) and the quad it is drawn with, all from the sheet's `Idle`
        // pose. The pose pass then projects only the pose the body is SHOWING.
        if let Some((posed, standing)) = posed_body_for(prepared) {
            // What this grant displaces, read in the batch before it is
            // replaced, onto the record `retract` reads.
            scope.queue_entity_edit(|mut body| {
                let displaced = DisplacedGeometry {
                    base_size: body
                        .get::<ambition_platformer2d_core::BodyBaseSize>()
                        .map(|base| base.base_size),
                    render_size: body
                        .get::<ambition_combat::components::ActorRenderSize>()
                        .map(|size| size.0),
                    sprite_offset: body
                        .get::<ambition_combat::components::ActorSpriteOffset>()
                        .map(|offset| offset.0),
                };
                if let Some(mut kit) = body.get_mut::<ProjectedCharacterKit>() {
                    kit.granted.posed_body = Some(displaced);
                }
            });
            scope.insert((
                ambition_platformer2d_core::BodyBaseSize {
                    base_size: standing.collision,
                },
                ambition_combat::components::ActorRenderSize(standing.render),
                ambition_combat::components::ActorSpriteOffset(standing.sprite_offset),
            ));
            scope.insert(posed);
        }
        // The MOTION MODEL, on the same path and for the X9 reason.
        //
        // The worn-player path resolves this in `apply_worn_character_kit`;
        // wiring only that one is exactly the mistake the action set made — a
        // seated fighter would move by its catalog row while a worn player moved
        // by its definition, for the same character.
        //
        // Applied through `switch_motion_model` rather than by inserting a fresh
        // `MotionModel`: a cross-model change must preserve every shared body
        // fact and initialize only the DESTINATION solver's private state
        // (ADR 0024). Replacing the component wholesale would reset a momentum
        // rider to Airborne mid-stride.
        // The movement FEEL, on the same path and for the same reason as the
        // motion model above.
        //
        // two earlier shapes were both wrong, and the reasons are worth
        // keeping. Removing on `None` HERE made this fight the worn path: for a
        // character with CATALOG tuning and no authored tuning, that path
        // inserts the marker and this removed it on the same tick. Passing the
        // catalog in to resolve both the same way then violated the workspace
        // policy against an OPTIONAL character catalog — and requiring it
        // broke three fixtures that deliberately run character demand with NO
        // catalog, which is a state another test exists to name.
        if let Some(tuning) = movement_tuning {
            scope.insert(ambition_platformer2d_core::AuthoredMovementTuning(tuning));
        }
        {
            let spec = prepared.motion_model;
            scope.queue_component_mut(
                move |model: &mut ambition_platformer2d_core::movement::MotionModel| {
                    ambition_platformer2d_core::switch_motion_model(model, spec);
                },
            );
        }
    }
}
