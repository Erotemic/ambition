//! Per-body presentation POSE read-model for player-bodied entities (E4).
//!
//! `BodyPoseView` is the plain-data snapshot the renderer draws a
//! player-bodied entity from — the same role [`ActorAnimIndex`] plays for
//! id-keyed actor visuals, expressed as a COMPONENT because a player body's
//! sprite lives on the body entity itself (no id-keyed visual to join
//! through). Rebuilt once per tick, sim-side, LAST in the sim tail
//! (`FeatureViewSync`); presentation reads ONLY this and never queries the
//! live `Body*` clusters. Extraction is a pure function of sim state — no
//! caching across ticks — so a rollback resim rebuilds it for free.
//!
//! `ShieldRingsView` is the pooled-ring analogue for the bubble-shield
//! visual: EVERY body's raised shield (player and brain-driven alike)
//! materializes one row, so the render pool is a pure consumer.

use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
use ambition_sprite_sheet::character::CharacterAnim;
use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Resource, With};

/// Sim-resolved presentation pose for one player-bodied entity: everything
/// the renderer needs to place, size, animate, and flash the sprite. Plain
/// data (Copy, no `Entity`/`Handle` borrows) — snapshot-safe by construction.
#[derive(bevy::prelude::Component, Clone, Debug)]
pub struct BodyPoseView {
    pub pos: ambition_platformer2d_core::Vec2,
    pub vel: ambition_platformer2d_core::Vec2,
    /// Current collision AABB size (crouch/morph compaction included).
    pub size: ambition_platformer2d_core::Vec2,
    /// Standing (base) AABB size — the denominator of the crouch stance
    /// ratio and the body-profile sprite scale. Falls back to `size` for a
    /// body without a `BodyBaseSize`.
    pub base_size: ambition_platformer2d_core::Vec2,
    pub facing: f32,
    /// Aerial/gravity roll angle (radians) — the sprite rotation.
    pub roll_angle: f32,
    /// `size.y / base_size.y`, clamped (0.1, 1.0] — the trimmed-sheet
    /// stance compaction the animator applies.
    pub stance_ratio_y: f32,
    /// Gravity direction used for the facing flip (the global field read the
    /// player path has always used).
    pub gravity_dir: ambition_platformer2d_core::Vec2,
    /// The picked animation row for this tick (the player picker over the
    /// body's real clusters).
    pub anim: CharacterAnim,
    /// What this body's ACTIVE MOVE asks to be drawn as, when one is
    /// playing — see `ActorAnimFrame::clip`, which is the same request on the
    /// actor road. `None` means *draw the semantic pose*.
    pub clip: Option<crate::ClipRequest>,
    /// Seconds remaining on the damage flash (`BodyCombat::hit_flash`).
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
    /// HOW HARD the hit currently freezing this body was, `0..=1`, and `0.0`
    /// when no hitlag is running.
    ///
    /// Resolved by `ambition_platformer2d_core::hit_response::hit_strength_fraction`
    /// from the hitlag the hit already set — the same quantity camera shake
    /// reads. Presentation never re-derives weight from damage, knockback or a
    /// move name, and hit resolution is untouched by anything that consumes it.
    pub hit_strength: f32,
    /// This body CANNOT BE STRUCK right now — the presentation half of
    /// `ambition_combat::util::body_vulnerable`, resolved here so no renderer
    /// re-derives hit eligibility from a pose or a move name.
    ///
    /// Covers every body-generic grant at once because the damage rule does:
    /// dodge / spot dodge / air dodge, tech and getup, the ledge grab's earned
    /// intangibility, the timed untouchable a respawn hands out, and the
    /// i-frames a hit leaves behind.
    pub unhittable: bool,
    /// WHY the canonical damage gate is closed, preserved as semantic
    /// presentation vocabulary. Shared route policy decides which causes opt
    /// into generic cues; character-owned effects may independently consume
    /// their own gameplay state and therefore compose with them.
    pub defense_cues: crate::DefenseCueCauses,
    pub hp_current: i32,
    pub hp_max: i32,
    /// The body is in morph-ball mode (draws the procedural sphere instead
    /// of the character sheet).
    pub morph_ball: bool,
    /// The body is UNDER the stage and must not be drawn at all.
    ///
    /// ⛔⛔ A PRESENTATION FACT, DERIVED — not a second authority. The truth is
    /// `BodyMode::Submerged` on the simulation body; this is that fact carried
    /// to the renderer the same way `morph_ball` is, because presentation reads
    /// the view and never the sim's components.
    pub submerged: bool,
    /// Fireball charge tier while the fire button is held (`None` when not
    /// charging): 0 / 1 / 2+ pick the charge-indicator size/alpha.
    pub charge_tier: Option<u8>,
    /// This body's SMASH CHARGE, `None` when it is not charging.
    ///
    /// Normalized `0..=1`: it appears when the hold latches, rises to `1.0` at
    /// maximum, and goes back to `None` the instant the move releases — so
    /// latched / building / loaded / released are all readable from this one
    /// value. Resolved by simulation (`MovePlayback::smash_charge_fraction`).
    ///
    /// ⛔ presentation must not re-derive it from move names or Startup
    /// progress: a tapped smash and a fully held one share both.
    pub smash_charge: Option<f32>,
    /// The sprite quad this body's SHEET authored, when its geometry is
    /// sheet-authored (`SpritePosedBody`); `None` when the render must size the
    /// quad itself.
    ///
    /// Carried here rather than read as a component because the render layer
    /// does not depend on actor machinery — and because it is the same KIND of
    /// fact as `size` and `base_size` beside it: a sim-resolved geometry the
    /// renderer draws rather than re-derives.
    ///
    /// Its presence changes what the quad MEANS. Without it, `standing_render`
    /// is a guess about the art scaled by how far the collision box has drifted
    /// from its baseline — the ratio that drew Mary-O's tall form ~60% larger
    /// than the body it belonged to. With it, the box and the quad are two
    /// readings of ONE authored scale, so neither is derived from the other and
    /// there is no ratio to be wrong.
    pub authored_render: Option<ambition_platformer2d_core::Vec2>,
    /// Where to draw that quad, relative to the body centre — the companion
    /// to `authored_render` and gated on the same `SpritePosedBody`.
    pub authored_offset: Option<ambition_platformer2d_core::Vec2>,
}

impl Default for BodyPoseView {
    fn default() -> Self {
        Self {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::ONE,
            base_size: ambition_platformer2d_core::Vec2::ONE,
            facing: 1.0,
            // A body with no pose yet is playing no move.
            clip: None,
            roll_angle: 0.0,
            stance_ratio_y: 1.0,
            gravity_dir: ambition_platformer2d_core::Vec2::Y,
            anim: CharacterAnim::Idle,
            hit_flash_secs: 0.0,
            parry_flash_secs: 0.0,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: crate::DefenseCueCauses::NONE,
            hp_current: 0,
            hp_max: 0,
            morph_ball: false,
            submerged: false,
            charge_tier: None,
            smash_charge: None,
            authored_render: None,
            authored_offset: None,
        }
    }
}

/// Only `BodyKinematics` is REQUIRED: a partial body (a test fixture that
/// spawns `PlayerVisual` + kinematics alone) still gets its transform facts;
/// the anim pick needs the full movement/ability cluster set (the same set
/// `animate_player` demanded) and holds `Idle` when any piece is absent —
/// exactly the frames the old live-query path would have skipped.
#[allow(clippy::type_complexity)]
pub fn rebuild_body_pose_views(
    mut commands: Commands,
    gravity: Option<Res<ambition_platformer2d_shared_tangle::gravity::GravityField>>,
    // The reference the hitlag law scales from, so the published strength is a
    // fraction rather than a raw freeze presentation would have to interpret.
    feel: Option<Res<ambition_combat::feel::Platformer2dFeelTuningMonolith>>,
    mut bodies: Query<
        (
            (
                Entity,
                &ambition_platformer2d_core::BodyKinematics,
                Option<&ambition_platformer2d_core::BodyGroundState>,
                Option<&ambition_platformer2d_core::BodyMotionFacts>,
                Option<&ambition_platformer2d_core::BodyFlightState>,
                Option<&BodyCombat>,
                Option<&ambition_characters::actor::BodyAnimFacts>,
                Option<&ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState>,
                // This body's own resolved basis, so the locomotion metric is
                // measured along ITS run axis.  deliberately not the global
                // `GravityField` read below: that one drives the facing flip and
                // is a mirror of the PRIMARY body's frame.
                Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
            ),
            (
                Option<&ambition_platformer2d_core::BodyModeState>,
                Option<&ambition_platformer2d_core::BodyEnvironmentContact>,
                Option<&ambition_platformer2d_core::BodyAbilities>,
                Option<&ambition_platformer2d_core::BodyShieldState>,
                Option<&ambition_combat::BodyMelee>,
                Option<&ambition_platformer2d_core::BodyBaseSize>,
                Option<&BodyHealth>,
                Option<&ambition_platformer2d_shared_tangle::orientation::ActorRoll>,
                Option<&ambition_projectiles::PlayerProjectileState>,
            ),
            // ⛔ A THIRD GROUP BECAUSE BEVY'S QUERY TUPLE LIMIT IS FIFTEEN, not
            // sixteen — the group above reached sixteen and stopped being a
            // `QueryData` at all. The split is drawn where the SUBJECT changes:
            // everything below is the ART this body draws with, plus the row
            // this pass writes.
            (
                // Content-driven pose PIN — the SAME component the actor path
                // honours in `rebuild_actor_anim_index`. See the read below.
                Option<
                    &ambition_sprite_sheet::character::ActorAnimOverride,
                >,
                // The sheet-authored sprite quad, when this body's geometry is its art.
                //
                //  gated on `SpritePosedBody`, NOT on the render size alone.
                // `ActorRenderSize` is the SHARED sprite-quad component — several
                // spawn paths insert it from sheet metadata for bodies whose
                // collision box is still hand-authored. Reading it bare here
                // would tell the renderer "the sheet authored this geometry"
                // about every one of them, and hand it a quad that was never
                // derived from their boxes.
                Option<&ambition_combat::components::ActorRenderSize>,
                // The quad's PLACEMENT, published beside its size by the same
                // pass. Gated on `SpritePosedBody` below for the same reason the
                // size is: several spawn paths insert these from sheet metadata
                // for bodies whose collision box is still hand-authored.
                Option<&ambition_combat::components::ActorSpriteOffset>,
                bevy::prelude::Has<
                    ambition_sprite_sheet::character::SpritePosedBody,
                >,
                // Match/ruleset respawn protection is a semantic presentation
                // cause independent of the current Empowered implementation.
                bevy::prelude::Has<ambition_combat::stocks::RespawnGrace>,
                // The move this body is playing, so the drawn row can be the
                // one the move NAMES — the same request the actor path carries
                // on `ActorAnimFrame::clip` (sprite redirect P0).
                Option<&ambition_combat::moveset::MovePlayback>,
                Option<&mut BodyPoseView>,
            ),
        ),
        With<PlayerVisual>,
    >,
) {
    // The player path has always read the GLOBAL gravity field for its facing
    // flip (localized zone gravity is the actor path's read) — preserved.
    let gravity_dir = gravity
        .as_deref()
        .map_or(ambition_platformer2d_core::Vec2::Y, |g| g.dir);
    // No feel tuning means no hitlag law to measure against: every body reports
    // no strength rather than a number derived from a reference nobody set.
    let hitlag_reference = feel.as_deref().map_or(0.0, |feel| feel.hitlag_time);
    for (
        (
            entity,
            kinematics,
            ground,
            motion_facts,
            flight,
            combat,
            anim_facts,
            blink_cam,
            body_frame,
        ),
        (
            body_mode,
            env_contact,
            abilities,
            shield,
            active_attack,
            base_size,
            health,
            roll,
            projectile_state,
        ),
        (
            anim_override,
            authored_render,
            authored_offset,
            sheet_authored_body,
            respawn_grace,
            playback,
            pose,
        ),
    ) in &mut bodies
    {
        let base = base_size.map_or(kinematics.size, |b| b.base_size);
        let stance_ratio_y = base_size
            .map(|b| (kinematics.size.y / b.base_size.y.max(1.0)).clamp(0.1, 1.0))
            .unwrap_or(1.0);
        // The anim pick runs only over the FULL cluster set `animate_player`
        // used to require — a partial body keeps `Idle` (it never animated
        // before either) while its transform facts stay live.
        let anim = match (
            (ground, motion_facts, flight),
            (combat, anim_facts, blink_cam),
            (body_mode, env_contact, abilities, shield),
        ) {
            (
                (Some(ground), Some(motion_facts), Some(flight)),
                (Some(combat), Some(anim_facts), Some(blink_cam)),
                (Some(body_mode), Some(env_contact), Some(abilities), Some(shield)),
            ) => ambition_character_sprites::pick_player_anim(
                anim_facts,
                combat,
                blink_cam,
                active_attack.and_then(|a| a.swing.as_ref()),
                kinematics,
                ground,
                motion_facts,
                flight,
                body_mode,
                env_contact,
                abilities,
                shield,
                body_frame.map_or_else(
                    || {
                        ambition_platformer2d_core::AccelerationFrame::new(
                            ambition_platformer2d_core::DEFAULT_GRAVITY_DIR,
                        )
                    },
                    |f| f.basis(),
                ),
            ),
            _ => CharacterAnim::Idle,
        };
        // A content pose PIN wins over the picked pose — the SAME rule `rebuild_actor_anim_index`
        // applies to every brain-driven actor, and it belongs here for the identical reason: a pose
        // the disposition-agnostic picker cannot infer (a shelled enemy's withdraw, a body
        // mid-power-up) is stated by the content that owns the state machine.
        let anim = anim_override.map(|o| o.0).unwrap_or(anim);
        let charge = playback
            .as_deref()
            .and_then(ambition_combat::moveset::MovePlayback::smash_charge_fraction);
        // A move may draw its owner MIRRORED — the crude spin read. Applied to
        // the published pose and nowhere else: the body's own `facing` is
        // untouched, so every rule that reads which way it is looking is
        // unaffected. See `MoveSpec::sprite_spin_hz`.
        let drawn_facing = if playback
            .as_deref()
            .is_some_and(ambition_combat::moveset::MovePlayback::sprite_mirrored_now)
        {
            -kinematics.facing
        } else {
            kinematics.facing
        };
        let next = BodyPoseView {
            pos: kinematics.pos,
            vel: kinematics.vel,
            size: kinematics.size,
            base_size: base,
            facing: drawn_facing,
            roll_angle: roll.map_or(0.0, |r| r.angle),
            stance_ratio_y,
            gravity_dir,
            anim,
            //  a MOVE names its row; failing that, a fighter STATE does — the
            // same two-step the actor road takes, so the two never disagree.
            clip: playback
                .map(|playback| crate::ClipRequest {
                    clip: playback.spec.clip.clip.clone(),
                    fallbacks: playback.spec.clip.fallbacks.clone(),
                })
                .or_else(|| {
                    crate::ClipRequest::from_chain(ambition_character_sprites::body_state_clip(
                        motion_facts?,
                        ambition_character_sprites::FighterClipFacts {
                            held: anim_facts.is_some_and(|f| f.held),
                            holding: anim_facts.is_some_and(|f| f.holding),
                            guard_break: shield
                                .and_then(|s| s.break_phase())
                                .map(ambition_character_sprites::GuardBreakBeat::from_phase),
                            parrying: shield.is_some_and(|s| s.parrying()),
                            guard_stunned: shield.is_some_and(|s| s.stun_timer > 0.0),
                        },
                    )?)
                })
                // A HELD CHARGE outranks the move's own row, and only while it
                // is held: the whole point of the beat is that a fighter
                // winding up looks different from one swinging. It goes AHEAD
                // of the move's chain rather than replacing it, so a sheet that
                // authors no charge row draws exactly what it drew before.
                .map(|chain| match charge {
                    Some(_) => chain.ahead_of(crate::SMASH_CHARGE_CLIP),
                    None => chain,
                })
                .or_else(|| charge.map(|_| crate::ClipRequest::only(crate::SMASH_CHARGE_CLIP))),
            hit_flash_secs: combat.map_or(0.0, |c| c.hit_flash),
            parry_flash_secs: shield.map_or(0.0, |s| s.parry_caught_timer),
            hit_strength: ambition_platformer2d_core::hit_response::hit_strength_fraction(
                combat.map_or(0.0, |c| c.hitstop_timer),
                hitlag_reference,
            ),
            // THE DAMAGE RULE ITSELF, inverted — not a second reading of it. A
            // body missing one of these clusters cannot be protected by it, so
            // the default stands in.
            unhittable: !ambition_combat::util::body_vulnerable(
                health.map_or_else(ambition_characters::actor::Invulnerability::none, |h| {
                    h.health.invulnerable
                }),
                motion_facts.is_some_and(|m| m.evading()),
                &shield.copied().unwrap_or_default(),
                &combat.copied().unwrap_or_default(),
            ),
            defense_cues: crate::defense_cue_causes(
                health.map_or_else(ambition_characters::actor::Invulnerability::none, |h| {
                    h.health.invulnerable
                }),
                motion_facts,
                &shield.copied().unwrap_or_default(),
                &combat.copied().unwrap_or_default(),
                respawn_grace,
            ),
            hp_current: health.map_or(0, |h| h.current()),
            hp_max: health.map_or(0, |h| h.max()),
            morph_ball: body_mode
                .is_some_and(|m| m.body_mode == ambition_platformer2d_core::BodyMode::MorphBall),
            // The SAME predicate the actor road asks — see
            // `BodyMode::hides_the_body`. Two read-models, one sentence.
            submerged: body_mode.is_some_and(|m| m.body_mode.hides_the_body()),
            charge_tier: projectile_state
                .and_then(|s| s.charging.map(|hold| s.charge_tuning.tier_for_hold(hold))),
            smash_charge: charge,
            authored_render: sheet_authored_body
                .then(|| authored_render.map(|r| r.0))
                .flatten(),
            authored_offset: sheet_authored_body
                .then(|| authored_offset.map(|o| o.0))
                .flatten(),
        };
        match pose {
            Some(mut pose) => *pose = next,
            None => {
                commands.entity(entity).insert(next);
            }
        }
    }
}

/// One raised bubble shield, resolved sim-side. The renderer positions one
/// pooled bubble sprite per row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShieldRingFact {
    pub pos: ambition_platformer2d_core::Vec2,
    pub size: ambition_platformer2d_core::Vec2,
    pub parrying: bool,
    /// `1.0` whole, `0.0` about to break — and `1.0` for a body whose guard is
    /// not a resource, so the bubble reads the same for every body that has one.
    pub integrity: f32,
    /// Seconds of SHIELDSTUN still owed — positive for exactly the beat after
    /// this guard absorbed a hit.
    ///
    /// Raw seconds, like `BodyPoseView::hit_flash_secs`: the timer is the
    /// resolved fact, and how long a hit should stay visible is a presentation
    /// constant. Publishing a normalized fraction instead would put a
    /// presentation decision in the simulation and need `ShieldTuning` here to
    /// make it.
    pub stun_secs: f32,
    /// This body's OWN toward-feet direction — not the global field's, and not
    /// screen `+Y`.
    ///
    /// The bubble is an ellipse (wider than it is tall) and the ellipse belongs
    /// to the BODY, so under rotated gravity it has to rotate with it. A
    /// presentation that assumed screen axes would draw a wall-walker's guard
    /// lying on its side.
    pub gravity_dir: ambition_platformer2d_core::Vec2,
}

/// Every body (player AND brain-driven actor) whose shield is currently
/// raised, in query order — the read-model behind the pooled bubble-shield
/// rings.
#[derive(Resource, Default, Clone, Debug)]
pub struct ShieldRingsView(pub Vec<ShieldRingFact>);

pub fn rebuild_shield_rings_view(
    mut view: ResMut<ShieldRingsView>,
    bodies: Query<(
        &ambition_platformer2d_core::BodyKinematics,
        &ambition_platformer2d_core::BodyShieldState,
        // Publish the presented pose because pooled shield rendering has no
        // owner entity left with which to resolve presentation offsets.
        Option<&crate::presented_pose::PresentedPose>,
        // The body's own shield tuning, so the ring can SHOW the resource
        // draining. A body with no motion model draws a whole ring.
        Option<&ambition_platformer2d_core::MotionModel>,
        // The body's OWN basis, so guard presentation is oriented to the
        // surface this body stands on rather than to the screen.
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    )>,
) {
    view.0.clear();
    view.0.extend(
        bodies
            .iter()
            .filter(|(_, shield, _, _, _)| shield.active)
            .map(|(kin, shield, presented, model, frame)| {
                let down = body_down(frame);
                ShieldRingFact {
                    pos: presented.map_or(kin.pos, |p| p.presented())
                        + down * (shield_half_height(kin.size, down) * shield.shield_tilt),
                    size: kin.size,
                    parrying: shield.parrying(),
                    integrity: model.map_or(1.0, |m| shield.integrity_fraction(m.shield_tuning())),
                    stun_secs: shield.stun_timer,
                    gravity_dir: down,
                }
            }),
    );
}

/// The body's half-extent ALONG its own gravity, which is what
/// `BodyShieldState::shield_tilt` is a fraction of.
///
/// ⛔ THE SAME MEASURE THE HIT TEST USES
/// (`ambition_combat::util::guard_covers_hit`). A bubble drawn against any
/// other half-height would lean by a different amount than the guard that
/// actually blocks, and the picture would be lying about where the shield is.
fn shield_half_height(
    size: ambition_platformer2d_core::Vec2,
    down: ambition_platformer2d_core::Vec2,
) -> f32 {
    ambition_platformer2d_core::AccelerationFrame::new(down)
        .to_world_half(size * 0.5)
        .dot(down)
        .abs()
}

/// This body's toward-feet direction, falling back to the engine default for a
/// body whose frame has not been resolved.
fn body_down(
    frame: Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
) -> ambition_platformer2d_core::Vec2 {
    frame.map_or(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR, |frame| {
        frame.down()
    })
}

/// One body whose guard has SHATTERED and is still paying for it.
///
/// Its own pooled view rather than a row on [`ShieldRingsView`], and the reason
/// is what a row there MEANS to the consumer that already exists:
/// `sync_bubble_shield_visual` assigns pooled bubble sprites by index over that
/// vector, so `len()` is "how many bubbles do I need". A broken guard is not
/// raised — `break_shield` drops `active` — so a row for one would either draw
/// a bubble on a shattered shield or force that loop to filter, at which point
/// the length stops answering the question it is asked. Two views, each meaning
/// one thing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuardBreakFact {
    pub pos: ambition_platformer2d_core::Vec2,
    pub size: ambition_platformer2d_core::Vec2,
    /// How far through the break this body is: `0.0` the instant the guard
    /// shatters, approaching `1.0` as it recovers
    /// (`BodyShieldState::break_phase`).
    pub phase: f32,
    /// This body's OWN toward-feet direction. Dizzy stars orbit the body's up,
    /// which is the opposite of this — never screen `-Y`.
    pub gravity_dir: ambition_platformer2d_core::Vec2,
}

/// Every body currently in a guard break, in query order.
#[derive(Resource, Default, Clone, Debug)]
pub struct GuardBreaksView(pub Vec<GuardBreakFact>);

pub fn rebuild_guard_breaks_view(
    mut view: ResMut<GuardBreaksView>,
    bodies: Query<(
        &ambition_platformer2d_core::BodyKinematics,
        &ambition_platformer2d_core::BodyShieldState,
        Option<&crate::presented_pose::PresentedPose>,
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    )>,
) {
    view.0.clear();
    view.0
        .extend(bodies.iter().filter_map(|(kin, shield, presented, frame)| {
            shield.break_phase().map(|phase| GuardBreakFact {
                pos: presented.map_or(kin.pos, |p| p.presented()),
                size: kin.size,
                phase,
                gravity_dir: body_down(frame),
            })
        }));
}

/// One knockout, resolved sim-side, WITH THE PLACE IT HAPPENED.
///
/// ⭐ THE POSITION IS THE WHOLE REASON THIS VIEW EXISTS.
/// `FighterStockSpent` carries an `Entity`, and by the time any consumer can
/// look that entity is somewhere else or gone: `place_respawning_fighters`
/// reads the same message inside `CombatSet::Settle` and teleports the body
/// onto the respawn platform on the same tick, and an eliminated body is
/// despawned outright. A consumer that resolved the entity itself would draw
/// the knockout over the respawn platform — an effect that fires, looks
/// deliberate, and marks the wrong spot.
///
/// One body in INVOLUNTARY flight, resolved sim-side.
///
/// A row exists only while the body is launched — tumbling from a hit, or
/// still inside the hitstun that hit gave it. Presentation reads the row's
/// speed to decide how hard the launch reads; it never has to ask why a body
/// is moving fast, which is the question velocity alone answers wrongly for a
/// run, a fast fall or a recovery.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchedBodyFact {
    pub pos: ambition_platformer2d_core::Vec2,
    pub vel: ambition_platformer2d_core::Vec2,
    /// The body's current collision AABB — presentation offsets the trail
    /// behind the body by a fraction of it.
    pub size: ambition_platformer2d_core::Vec2,
    /// Seconds left in the HARD control lock at the FRONT of this launch:
    /// `BodyCombat::recoil_lock_timer`, the window in which the body has been
    /// thrown and has no authority at all over where it goes. `0.0` for a body
    /// still in involuntary flight that can answer for itself again.
    ///
    /// This is the ONE thing separating a body launched this instant from one
    /// that has been tumbling for a second: both are in this view, both are
    /// fast, and every other field reads the same for the two. Presentation
    /// gives the front of a launch its own beat off this and nothing else.
    ///
    /// A METEOR is the same window, longer — the Smash demo declares
    /// `meteor_lock_time: 0.30` against an ordinary `0.12` — so a spike reads
    /// as a longer, harder beat with no second fact.
    pub launch_beat_secs: f32,
    /// THIS BODY'S OWN launch threshold, in px/s — the speed at which the
    /// kernel calls a launch a tumble
    /// (`AxisSweptParams::abilities.tumble_speed`, the value
    /// `launch_into_tumble` gates on).
    ///
    /// ⭐ published so an onset can be read against the body it belongs to. It
    /// is authored PER BODY, so a heavyweight and a featherweight are in
    /// trouble at different speeds, and a cue fitted to one stage-wide
    /// percentile says "this one is fast" where the body's own threshold says
    /// "this one is in trouble" — which is the difference between a readable
    /// tell and a speedometer.
    ///
    /// `0.0` for a body whose policy has no tumble at all, which is every body
    /// outside a match: such a body never tumbles, so a threshold of zero is
    /// the honest answer rather than a sentinel.
    pub tumble_speed: f32,
}

/// Every body (player AND brain-driven fighter) currently in an involuntary
/// flight, in query order — the read-model behind the hard-launch trail.
///
/// Pooled rather than a per-entity component for the reason
/// [`ShieldRingsView`] is: the effect is world particles with no owner entity
/// to hang a view on, and a seated fighter's sprite is a SEPARATE entity from
/// its body, joined by feature id. A pooled row is the one shape both roads
/// share.
#[derive(Resource, Default, Clone, Debug)]
pub struct LaunchedBodiesView(pub Vec<LaunchedBodyFact>);

pub fn rebuild_launched_bodies_view(
    mut view: ResMut<LaunchedBodiesView>,
    bodies: Query<
        (
            &ambition_platformer2d_core::BodyKinematics,
            Option<&ambition_platformer2d_core::BodyMotionFacts>,
            Option<&BodyCombat>,
            // The presented position, so the plume leaves the body where the
            // sprite is drawn rather than at the tick position it is interpolating
            // away from.
            Option<&crate::presented_pose::PresentedPose>,
            // The body's own motion policy, which is where its launch threshold is
            // authored. Read for that ONE value; the maneuver state behind it is
            // model-private (ADR 0024) and stays that way.
            Option<&ambition_platformer2d_core::movement::MotionModel>,
        ),
        // ⛔⛔ A BODY OUT OF PLAY IS NOT IN A FLIGHT, AND THIS IS THE KO COMPOSITION
        // POLICY RATHER THAN AN AMPLITUDE TWEAK. The launch trail's own header says
        // its blast and plume are *"a LAYER over the hit spark and camera shake"*,
        // and the knockout beat then layers on top of that — three modules each
        // locally correct, and nothing anywhere saying *"the flight has RESOLVED;
        // the cues whose job was to predict danger are done"*. So an elimination
        // drew the trail that was still predicting the launch underneath the beat
        // that answered it.
        //
        // ⭐ THE SEAM WAS ALREADY HERE. This view is the sim's resolved "this motion
        // is INVOLUNTARY" fact, and a body the world has its hands off is not moving
        // involuntarily — it is not moving at all. Retiring the row is the whole
        // policy: the predictive cue stops at the instant the thing it predicted
        // happens, and the knockout owns the beat alone.
        bevy::prelude::Without<ambition_combat::death_rules::OutOfPlay>,
    >,
) {
    view.0.clear();
    view.0.extend(
        bodies
            .iter()
            .filter_map(|(kin, motion, combat, presented, model)| {
                // Two published sim facts, one resolved answer: the tumble is the
                // helpless half of a launch and the hitstun is the rest of it. A
                // consumer reading only the tumble would drop the row the instant a
                // launched body stopped tumbling, mid-flight.
                let launched = motion.is_some_and(|f| f.tumbling)
                    || combat.is_some_and(|c| c.hitstun_timer > 0.0);
                launched.then(|| LaunchedBodyFact {
                    pos: presented.map_or(kin.pos, |p| p.presented()),
                    vel: kin.vel,
                    size: kin.size,
                    launch_beat_secs: combat.map_or(0.0, |c| c.recoil_lock_timer),
                    tumble_speed: match model {
                        Some(ambition_platformer2d_core::MotionModel::AxisSwept(axis)) => {
                            axis.params.abilities.tumble_speed
                        }
                        // A policy with no floor game has no launch threshold.
                        _ => 0.0,
                    },
                })
            }),
    );
}

#[cfg(test)]
mod pose_view_tests {
    use super::*;

    #[test]
    fn shield_rings_view_defaults_empty() {
        let view = ShieldRingsView::default();
        assert!(view.0.is_empty());
    }

    #[test]
    fn body_pose_view_default_is_inert() {
        let pose = BodyPoseView::default();
        assert_eq!(pose.stance_ratio_y, 1.0);
        assert_eq!(pose.hit_flash_secs, 0.0);
        assert!(pose.charge_tier.is_none());
        assert!(!pose.morph_ball);
    }

    /// ⭐⭐ A SPINNING MOVE FLIPS WHAT IS DRAWN AND NOT WHICH WAY THE BODY FACES.
    ///
    /// The crude spin read Jon asked for on Pointed's Up-B (*"fake the spin by
    /// repeatedly flipping the sprite horizontally"*), and the reason it is safe
    /// to fake: the mirror lives on the published POSE. The body's own `facing`
    /// is the fact hitboxes, launch directions and the brain all read, and a
    /// presentation trick that moved it would turn a drawing choice into a
    /// gameplay one six times a second.
    ///
    /// ⛔ SO THE ASSERTION IS A PAIR OF READINGS OF THE SAME FRAME — the pose
    /// disagrees with the body, and the body is unchanged.
    #[test]
    fn a_spinning_move_mirrors_the_drawn_pose_and_leaves_the_body_facing_alone() {
        use ambition_combat::moveset::MovePlayback;

        let mut app = bevy::prelude::App::new();
        app.add_systems(bevy::prelude::Update, rebuild_body_pose_views);

        let kin = ambition_platformer2d_core::BodyKinematics {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
            facing: 1.0,
        };

        let spec = ambition_entity_catalog::MoveSpec {
            display_name: None,
            id: "spin".to_string(),
            clip: ambition_entity_catalog::ClipBinding {
                clip: "spin".to_string(),
                fallbacks: vec![],
            },
            duration_s: 1.0,
            windows: vec![],
            events: vec![],
            gates: Default::default(),
            start_impulse: None,
            smash_charge_mult: 1.0,
            smash_charge: None,
            charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: Some(10.0),
            equips: None,
        };

        // Parked inside a mirrored half-period.
        let mut playback = MovePlayback::new(spec, 1.0);
        playback.t = 0.07;
        assert!(
            playback.sprite_mirrored_now(),
            "the fixture is not on a mirrored frame, so what follows measures \
             nothing"
        );

        let body = app.world_mut().spawn((PlayerVisual, kin, playback)).id();
        app.update();

        assert_eq!(
            app.world().get::<BodyPoseView>(body).map(|p| p.facing),
            Some(-1.0),
            "the drawn pose was not mirrored, so the spin is invisible"
        );
        assert_eq!(
            app.world()
                .get::<ambition_platformer2d_core::BodyKinematics>(body)
                .map(|k| k.facing),
            Some(1.0),
            "the presentation mirror reached the BODY's facing — every hitbox \
             and launch direction this fighter has now flips six times a second"
        );
    }

    /// A sheet-authored quad is reported only by a body that HAS one.
    ///
    /// `ActorRenderSize` alone cannot support that claim: it is the shared sprite-quad
    /// component, and several spawn paths set it from sheet metadata for bodies whose collision
    /// box is still hand-authored. Reading it bare would silently retarget the render for every
    /// such body, which is a change nothing would report.
    ///
    /// The negative case is the whole test — the positive one only proves the
    /// field is wired at all.
    #[test]
    fn only_a_sheet_authored_body_reports_an_authored_quad() {
        use ambition_combat::components::ActorRenderSize;
        use ambition_sprite_sheet::character::SpritePosedBody;

        let mut app = bevy::prelude::App::new();
        app.add_systems(bevy::prelude::Update, rebuild_body_pose_views);

        let kin = ambition_platformer2d_core::BodyKinematics {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
            facing: 1.0,
        };
        let quad = ambition_platformer2d_core::Vec2::new(61.0, 73.0);

        // A quad, but the body's box is its own business.
        let hand_authored = app
            .world_mut()
            .spawn((PlayerVisual, kin, ActorRenderSize(quad)))
            .id();
        // A quad that came from the same scale as the box.
        let sheet_authored = app
            .world_mut()
            .spawn((
                PlayerVisual,
                kin,
                ActorRenderSize(quad),
                SpritePosedBody::new("robot", 2.0),
            ))
            .id();

        app.update();

        assert_eq!(
            app.world()
                .get::<BodyPoseView>(hand_authored)
                .and_then(|p| p.authored_render),
            None,
            "a render size without a sheet-authored body is not a claim about \
             that body's geometry, and must not be reported as one"
        );
        assert_eq!(
            app.world()
                .get::<BodyPoseView>(sheet_authored)
                .and_then(|p| p.authored_render),
            Some(quad),
            "and a body whose geometry IS its art reports the quad that came \
             from the same scale as its box"
        );
    }

    /// The FRONT of a launch is published, and it is the only thing that tells
    /// a body thrown this instant from one that has been tumbling for a second.
    ///
    /// Both bodies here are launched, both are travelling at the same speed,
    /// and every other field of their rows is identical — which is exactly the
    /// state presentation could not read before this field existed. The
    /// zero-beat row is the whole test: drop the field's source and the two
    /// rows become indistinguishable again, silently.
    #[test]
    fn the_front_of_a_launch_is_published_apart_from_the_tumble() {
        use ambition_characters::actor::BodyCombat;
        use ambition_platformer2d_core::BodyKinematics;
        use ambition_platformer2d_core::{BodyMotionFacts, Vec2};

        let mut app = bevy::prelude::App::new();
        app.init_resource::<LaunchedBodiesView>();
        app.add_systems(bevy::prelude::Update, rebuild_launched_bodies_view);

        let kin = |x: f32| BodyKinematics {
            pos: Vec2::new(x, 0.0),
            vel: Vec2::new(900.0, 0.0),
            size: Vec2::new(30.0, 48.0),
            facing: 1.0,
        };
        let tumbling = BodyMotionFacts {
            tumbling: true,
            ..Default::default()
        };

        // Thrown THIS INSTANT: still inside the hard control lock.
        app.world_mut().spawn((
            kin(0.0),
            tumbling,
            BodyCombat {
                hitstun_timer: 0.4,
                recoil_lock_timer: 0.09,
                ..Default::default()
            },
        ));
        // Still flying, still helpless, but it can steer again.
        app.world_mut().spawn((
            kin(1.0),
            tumbling,
            BodyCombat {
                hitstun_timer: 0.4,
                recoil_lock_timer: 0.0,
                ..Default::default()
            },
        ));
        // Under its own power at the same speed: not a launch at all.
        app.world_mut().spawn(kin(2.0));

        app.update();

        let view = app.world().resource::<LaunchedBodiesView>();
        assert_eq!(
            view.0.len(),
            2,
            "only the launched bodies are rows: {view:?}"
        );
        let beat_at = |x: f32| {
            view.0
                .iter()
                .find(|row| row.pos.x == x)
                .unwrap_or_else(|| panic!("no row at {x}: {view:?}"))
                .launch_beat_secs
        };
        assert_eq!(beat_at(0.0), 0.09, "the lock the sim wrote, unaltered");
        assert_eq!(
            beat_at(1.0),
            0.0,
            "a sustained tumble is NOT a launch beat, and reporting one here \
             would make every trailing body flash forever"
        );
    }

    /// A content pose pin reaches the PLAYER's view, not only an actor's.
    ///
    /// `ActorAnimOverride` is how content states a pose the locomotion picker cannot infer — a
    /// shell withdrawing, a body mid-transformation.
    ///
    /// The pin is the whole assertion: an unpinned body here picks `Idle` (the
    /// partial-cluster fallback), so a view that answers `Idle` under a pin is
    /// exactly the silent-drop failure, and one that answers `Idle` without a
    /// pin proves the test is reading the field the pin would have changed.
    #[test]
    fn a_content_pose_pin_reaches_the_players_pose_view() {
        use ambition_sprite_sheet::character::ActorAnimOverride;

        let mut app = bevy::prelude::App::new();
        app.add_systems(bevy::prelude::Update, rebuild_body_pose_views);

        let kin = ambition_platformer2d_core::BodyKinematics {
            pos: ambition_platformer2d_core::Vec2::ZERO,
            vel: ambition_platformer2d_core::Vec2::ZERO,
            size: ambition_platformer2d_core::Vec2::new(30.0, 48.0),
            facing: 1.0,
        };
        let unpinned = app.world_mut().spawn((PlayerVisual, kin)).id();
        let pinned = app
            .world_mut()
            .spawn((PlayerVisual, kin, ActorAnimOverride(CharacterAnim::Grow)))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<BodyPoseView>(unpinned).map(|p| p.anim),
            Some(CharacterAnim::Idle),
            "an unpinned body picks its pose; `Idle` is the partial-cluster read"
        );
        assert_eq!(
            app.world().get::<BodyPoseView>(pinned).map(|p| p.anim),
            Some(CharacterAnim::Grow),
            "and a pinned one shows what the content pinned"
        );
    }
}
