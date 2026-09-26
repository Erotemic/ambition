//! Mary-O's Solid Snake walker and shell state machine.
//!
//! A stomp changes the same live body into an inert shell rather than killing
//! or replacing it. While withdrawn, steering is suppressed and contact damage
//! is disabled; emerging restores walker behavior. A kicked shell drives its
//! horizontal velocity directly. Shared stomp classification makes top contact
//! safe in every phase, while side contact with a running shell can damage the
//! player. [`step_snake_shell`] contains the pure phase logic.

use bevy::prelude::*;

use ambition_platformer2d::characters::control::{
    claim_control_hold, release_control_hold, ControlHold, ControlHolds,
};
use ambition_platformer2d::combat::actor_tuning::{ActorConfig, ContactThreatWithdrawn};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d::sprite_sheet::character::{ActorAnimOverride, CharacterAnim};

use crate::stomp::{player_touch, PlayerTouch};

/// The catalog `display_name` a snake renders from, and the name every snake
/// spawn carries so its `solid_snake` sheet resolves.
pub const SNAKE_DISPLAY_NAME: &str = "Solid Snake";

/// The roster brain key the snake archetype is filed under, namespaced so it
/// never collides with a host provider's roster.
pub const SNAKE_BRAIN_KEY: &str = "mary_o_snake";

/// Upward speed Mary-O gets off a stomped snake — a lively hop, a touch under a
/// full jump so a stomp reads as a bounce, not a re-jump.
const BOUNCE_SPEED: f32 = 430.0;

/// How fast a kicked shell slides. Faster than a snake walks, so a kicked shell
/// reliably runs a line of them down instead of trailing behind.
const SHELL_SLIDE_SPEED: f32 = 300.0;

/// Damage a moving shell deals to another ENEMY it runs down — lethal, so a kicked
/// shell one-shots the trash mobs it mows through (snakes, AI Slop), like a Koopa
/// shell clearing a line. Routed through the shared hit pipeline, so the victim
/// dies with the same drops/score/reaction any other kill produces.
const SHELL_ENEMY_DAMAGE: i32 = 99;
/// Damage a moving shell deals to the PLAYER on a SIDE hit — an ordinary contact
/// tick (the shared i-frames keep it from re-hitting every frame it overlaps).
const SHELL_PLAYER_DAMAGE: i32 = 1;

/// Seconds a freshly kicked shell cannot hurt the PLAYER. You kick a shell from
/// the side, so the moment it starts moving it is still inside you — without this
/// the kick would hurt the kicker every single time. It still mows down ENEMIES
/// during the grace (kicking a shell into a crowd is instant), and long enough for
/// a shell that ricochets straight back off a nearby wall to be a real threat again.
const KICK_GRACE_S: f32 = 0.25;

/// Seconds the snake spends pulling into its shell (the `retreat` row) before it
/// is a settled, kickable shell.
const RETREAT_S: f32 = 0.35;
/// Seconds a shell sits boxed and kickable before it starts to peek back out. A
/// generous window so a kick almost always lands during it.
const BOXED_S: f32 = 4.0;
/// Seconds spent peeking (the `peek` row) — a wary look before committing.
const PEEK_S: f32 = 0.5;
/// Seconds spent climbing back out (the `emerge` row) before it walks again.
const EMERGE_S: f32 = 0.45;

/// A Solid Snake's shell lifecycle. `Walking` is the ordinary patroller; the rest
/// are the withdraw cycle, driven by [`step_snake_shell`]. Each timed stage's `f32`
/// is the time it has left.
///
/// It requires [`ContactThreatWithdrawn`], the read model `run_snake_shells`
/// re-derives from it, so every road that makes a snake (the tag pass, a
/// fixture, a rollback restore) gives the writer something to write.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[require(ContactThreatWithdrawn)]
pub enum SnakeShell {
    Walking,
    Retreating(f32),
    Boxed(f32),
    /// Sliding along the ground.
    Sliding {
        /// Travel direction: `-1.0` is leftward, `1.0` rightward.
        dir: f32,
        /// Seconds left of the post-kick grace, during which this shell cannot
        /// hurt the PLAYER (it is still inside the person who kicked it). Enemies
        /// it runs down are hit from the first tick.
        ///
        /// Spent immediately by a wall bounce, not only by the clock: a shell
        /// that turns around is coming back at the player, and that is a hit
        /// however recently they kicked it.
        grace: f32,
    },
    Peeking(f32),
    Emerging(f32),
}

/// What the pure step wants the ECS to reflect for one snake this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellEffects {
    /// The next phase.
    pub phase: SnakeShell,
    /// Pose to pin, or `None` to let the shared picker choose (Walking).
    pub anim: Option<CharacterAnim>,
    /// Whether the snake is a live, threatening WALKER this tick (`true`) or an
    /// inert SHELL (`false`) — frozen in place and harmless to the touch. The snake
    /// never dies: this toggles only its freeze lock + contact-damage threat, never
    /// its HP, so a shelled snake stays visible and comes back.
    pub alive: bool,
    /// Horizontal velocity to command, or `None` to leave the body's own physics
    /// alone (Walking). Shell phases hold still (`Some(0.0)`) or slide.
    pub vel_x: Option<f32>,
    /// A fresh withdraw just started (play the squash pop + thud).
    pub just_squashed: bool,
    /// A resting shell was just kicked into a slide (play the kick thud).
    pub just_kicked: bool,
}

/// The discrete things that can happen TO a snake this tick, read from the world
/// by the ECS wrapper and fed to the pure step.
///
/// `stomped` and `side_kick` are mutually exclusive by construction: both are
/// derived from the ONE [`PlayerTouch`] classification, so a contact is a stomp or
/// a side, never both and never neither-when-touching.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShellInputs {
    /// The player is on this body's head this tick ([`PlayerTouch::Top`]).
    pub stomped: bool,
    /// The player is touching a RESTING shell, from any side; `Some(dir)` is the
    /// way to launch it — `-1.0` left, `1.0` right.
    ///
    /// Set for a stomp as well as a side bump, because in the game this pays
    /// homage to both kick it. The direction is away from whichever side of the
    /// shell the player is more on, and a body exactly at its center kicks it
    /// RIGHT: some answer has to be picked, and picking by position keeps it a
    /// pure function of the two poses rather than of arrival order.
    pub kick_dir: Option<f32>,
    /// A sliding shell's velocity collapsed against a wall this tick.
    pub blocked: bool,
}

/// The whole shell choreography, as a pure function. Given the current phase,
/// the tick length, and what happened to the body, return the next phase and the
/// presentation/physics the ECS should apply. No world access, so it is exhaustively
/// unit-tested below.
pub fn step_snake_shell(phase: SnakeShell, dt: f32, inputs: ShellInputs) -> ShellEffects {
    use SnakeShell::*;
    // Defaults for the shelled (corpse) stages: pinned pose, held still, dead.
    let shelled = |phase, anim| ShellEffects {
        phase,
        anim: Some(anim),
        alive: false,
        vel_x: Some(0.0),
        just_squashed: false,
        just_kicked: false,
    };
    // A stomp pre-empts every phase. It is the one input that always means "be
    // in the shell", and it always bounces the stomper. That is what makes standing
    // over ANY snake safe: whatever the body under the player's feet was doing —
    // walking, running as a kicked shell, peeking back out — this tick it is inert,
    // and the bounce carries them off it.
    if inputs.stomped {
        let reseat = ShellEffects {
            just_squashed: true,
            ..shelled(Boxed(BOXED_S), CharacterAnim::ShellIdle)
        };
        // Landing on a RESTING shell kicks it, exactly as a side bump does. The classic never
        // does that — you land on a still shell and it shoots out from under you.
        //
        // Only a RESTING shell. A shell already running is stopped by a stomp
        // (that is the tech), and a walker, a mid-withdraw snake or one climbing
        // back out has no shell to kick yet.
        if let (Boxed(_), Some(dir)) = (phase, inputs.kick_dir) {
            return ShellEffects {
                just_squashed: true,
                just_kicked: true,
                vel_x: Some(dir * SHELL_SLIDE_SPEED),
                ..shelled(
                    Sliding {
                        dir,
                        grace: KICK_GRACE_S,
                    },
                    CharacterAnim::ShellIdle,
                )
            };
        }
        return match phase {
            // A live walker starts the withdraw; the retreat row plays first.
            Walking => ShellEffects {
                just_squashed: true,
                ..shelled(Retreating(RETREAT_S), CharacterAnim::Retreat)
            },
            // Mid-withdraw: let the retreat run out on its own clock.
            Retreating(t) => {
                let t = t - dt;
                if t <= 0.0 {
                    reseat
                } else {
                    ShellEffects {
                        just_squashed: true,
                        ..shelled(Retreating(t), CharacterAnim::Retreat)
                    }
                }
            }
            // Resting, running, peeking or climbing out: re-seat the shell. This is
            // the "stop the runaway" tech, and it also shoves a snake that was about
            // to emerge under the player's feet back into its shell.
            Boxed(_) | Sliding { .. } | Peeking(_) | Emerging(_) => reseat,
        };
    }
    match phase {
        Walking => {
            // The ordinary walker: let the shared picker choose walk/idle and
            // the brain drive movement.
            ShellEffects {
                phase: Walking,
                anim: None,
                alive: true,
                vel_x: None,
                just_squashed: false,
                just_kicked: false,
            }
        }
        Retreating(t) => {
            let t = t - dt;
            if t <= 0.0 {
                shelled(Boxed(BOXED_S), CharacterAnim::ShellIdle)
            } else {
                shelled(Retreating(t), CharacterAnim::Retreat)
            }
        }
        Boxed(t) => {
            if let Some(dir) = inputs.kick_dir {
                ShellEffects {
                    just_kicked: true,
                    vel_x: Some(dir * SHELL_SLIDE_SPEED),
                    ..shelled(
                        Sliding {
                            dir,
                            grace: KICK_GRACE_S,
                        },
                        CharacterAnim::ShellIdle,
                    )
                }
            } else {
                let t = t - dt;
                if t <= 0.0 {
                    shelled(Peeking(PEEK_S), CharacterAnim::Peek)
                } else {
                    shelled(Boxed(t), CharacterAnim::ShellIdle)
                }
            }
        }
        // A SIDE contact never stops a running shell: that is the shell running you
        // (or an enemy) down, which the ECS turns into a real hit through the shared
        // damage pipeline while the shell keeps going. (The stop-it-dead branch is
        // the stomp pre-emption above.)
        Sliding { dir, grace } => {
            // A wall ARMS it. The grace exists for one reason — the shell is
            // still inside the body that just kicked it, and a shell you cannot
            // get off is not a mechanic. A shell that has hit a wall and turned
            // around is not that shell any more: it is coming BACK at you, which
            // is the whole danger of kicking one down a corridor. So the bounce
            // spends the grace outright rather than letting it run out on a
            // timer that might still be ticking when the shell returns.
            let (dir, grace) = if inputs.blocked {
                (-dir, 0.0)
            } else {
                (dir, (grace - dt).max(0.0))
            };
            ShellEffects {
                vel_x: Some(dir * SHELL_SLIDE_SPEED),
                ..shelled(Sliding { dir, grace }, CharacterAnim::ShellIdle)
            }
        }
        Peeking(t) => {
            let t = t - dt;
            if t <= 0.0 {
                shelled(Emerging(EMERGE_S), CharacterAnim::Emerge)
            } else {
                shelled(Peeking(t), CharacterAnim::Peek)
            }
        }
        Emerging(t) => {
            let t = t - dt;
            if t <= 0.0 {
                // Back to a live walker: the pure step hands control back; the ECS
                // restores HP and drops the pose pin.
                ShellEffects {
                    phase: Walking,
                    anim: None,
                    alive: true,
                    vel_x: None,
                    just_squashed: false,
                    just_kicked: false,
                }
            } else {
                shelled(Emerging(t), CharacterAnim::Emerge)
            }
        }
    }
}

// Demo-owned hostile roster: ONE 1-HP `Wanderer` archetype. It walks forward and reverses at
// walls; `aggro_radius`/`attack_range` are ignored by that template. It carries no `melee`, so its
// only offense is the default-on body contact — which the shell state withdraws (via
// `ContactThreatWithdrawn`, never its authored tuning) while shelled, then restores when it
// walks again.

/// The `solid_snake` sheet TARGET (also the catalog id) — the generated sheet the
/// enemy render resolves for a Solid Snake.
pub const SNAKE_SHEET_TARGET: &str = "solid_snake";

/// Ensure the `solid_snake` sheet is drawable, keyed by BOTH its catalog id
/// and its display name, so the enemy render's `npc_asset_for_name` finds it
/// instead of falling back to the generic goblin sheet.
///
/// The catalog defers a non-eager character's sheet to a room-staging barrier
/// that lives in the app host — a path a standalone demo, or a host that stages
/// this enemy through the content registry rather than a room `enemy_spawn`, does
/// not reliably drive (the same class of gap the player-avatar sheet hit). Rather
/// than depend on that, the demo OWNS its enemy sheet the same way Sanic owns its
/// ring prop: a per-frame insert-if-missing load through the target loader (which
/// bypasses the lean sandbox catalog), self-healing across a `GameAssets` rebuild
/// and a no-op headless / `--no-assets`.
pub fn register_solid_snake_sheet(
    game_assets: Option<ResMut<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>>,
    config: Option<Res<ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig>>,
    asset_server: Option<Res<AssetServer>>,
    layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
) {
    let (Some(mut game_assets), Some(config), Some(asset_server), Some(mut layouts)) =
        (game_assets, config, asset_server, layouts)
    else {
        return;
    };
    if config.no_assets || game_assets.characters.sheet(SNAKE_DISPLAY_NAME).is_some() {
        return;
    }
    // ⛔ THE ENGINE'S ROAD OWNS A DECLARED CHARACTER: this id is a registered
    // definition, so the room demand declares and realizes it at the room's
    // tier, and this fallback must not re-publish it at full resolution after
    // a tier convergence retires it (see `plane.rs` for the measurement). Only
    // a composition where nothing declared it still needs this road.
    if !matches!(
        game_assets.characters.sheet_state(SNAKE_SHEET_TARGET),
        ambition_platformer2d::sprite_sheet::character::CharacterSheetState::Unknown
    ) {
        return;
    }
    if let Some(asset) =
        ambition_platformer2d::actors::character_sprites::load_prop_sheet_for_target(
            &asset_server,
            &mut layouts,
            &config.sprite_folder,
            SNAKE_SHEET_TARGET,
            &ambition_platformer2d::sprite_sheet::character::SheetTuning::new(1.0, 0),
        )
    {
        // Double-keyed exactly like the eager loader: the render resolves an actor
        // by its display name, and other seams by the catalog id.
        game_assets
            .characters
            .publish_under(SNAKE_SHEET_TARGET, asset.clone());
        game_assets
            .characters
            .publish_under(SNAKE_DISPLAY_NAME, asset);
    }
}

// `SNAKE_TILE_COLUMNS` is GONE. Where a snake patrols is authored.
//
// its SIZE stays, and the difference is the rule: how big a snake is comes
// from its sheet; where it patrols comes from the level.

// ⛔⛤ `snake_half_size` WAS HERE, AND CONSTRUCTION MADE IT REDUNDANT —
// 2026-09-21. It resolved the sheet's idle rectangle so a tag pass could write
// it onto an already-built body, and its own doc claimed that kept the body
// from popping on its first tick. It did not: the engine had already built the
// body at the CATALOG's size, so writing this here produced the pop rather
// than preventing it. The character now declares
// `BodySource::SpriteAuthored`, and the seed resolves the same rectangle
// before the body exists. Deleting the second answer is the repair; keeping
// both in sync was never going to be.

/// Identify snakes by their authored `CharacterBrain` archetype key.
///
/// `ActorConfig.brain` is a read-model of authored identity, so callers should
/// not substitute display names or generated feature-id conventions.
pub fn is_snake_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == SNAKE_BRAIN_KEY)
}

/// Tag freshly staged snakes with the RUNTIME state a snake has: its shell
/// phase and its dormancy policy.
///
/// ⛔⛤ **AND NOT ITS GEOMETRY, SINCE 2026-09-21.** This pass used to write
/// `CenteredAabb::half_size` and insert `SpritePosedBody` here, in
/// `AfterIntegrate`, on a body the engine had already built at a different
/// size. `character_body.rs` names that seam directly — *"body geometry was
/// still declared through a second seam, which is the problem
/// `register_character` exists to delete"* — and it was measurable: a snake
/// spent its first tick with the sheet's collision box (21.3x9.5) beside the
/// catalog's render size (118.2x118.2), and presentation binding inside that
/// one-tick window latched a quad five times too big that nothing afterwards
/// invalidated.
///
/// The rule is unchanged and now has one owner: how big a snake is comes from
/// its sheet, via `BodySource::SpriteAuthored` on its character definition,
/// resolved by construction. Where it patrols still comes from the level.
pub fn tag_mary_o_snakes(
    mut commands: Commands,
    fresh: Query<(Entity, &ActorConfig), Without<SnakeShell>>,
) {
    for (entity, config) in &fresh {
        if is_snake_brain(&config.brain) {
            commands.entity(entity).try_insert(SnakeShell::Walking);
        }
    }
}

// ⛔⛔ `reset_snakes_on_room_reset` WAS HERE, AND CANONICAL RECONSTRUCTION MADE
// IT REDUNDANT. It rewrote `SnakeShell`, the recoil lock and the contact-damage
// toggle on every snake when a replay was admitted, because the replay used to
// mutate survivors in place and a shelled snake survived it. A replay is a room
// REBUILD now: snakes are room-scoped authored actors, so the rebuild retires
// them and re-spawns them walking, with a fresh recoil lock and a fresh tuning.
//
// ⭐ MEASURED, NOT ASSUMED — and the first measurement was worthless. Gutting
// this listener left all 41 Mary-O tests green, which is evidence that NOTHING
// COVERED IT rather than evidence of redundancy; a deletion argued from silence
// is argued from nothing. `replay_rebuilds_the_snakes.rs` was written as the
// cover first, and the listener was gutted again with it in place. Still green.
// That is the measurement this deletion rests on.

/// The Solid Snake shell mechanic, the thin ECS wrapper over [`step_snake_shell`].
///
/// Reads what happened to each snake this tick (stomp / kick / wall-block),
/// advances its phase, and reflects the result: bounce the stomper, pin the pose
/// via [`ActorAnimOverride`], freeze the (still-alive) body and turn its contact
/// threat off — or slide it — and let it walk and threaten again when it finishes
/// emerging.
///
/// A moving shell is a kinetic hazard. Each tick it is sliding, it broadcasts a
/// lethal hit over its own AABB through the SHARED damage pipeline ([`HitEvent`]),
/// so it kills every ENEMY it runs down (other snakes, AI Slop, anything) with the
/// same death/drops/reaction any other kill produces — no bespoke chain. The enemy
/// broadcast never touches the player (the shared drain's actor query excludes the
/// player), so the player half is cleanly separate and DIRECTIONAL, exactly like
/// Mario: [`PlayerTouch::Top`] is a stomp — it stops the shell and bounces you,
/// always safe — and only [`PlayerTouch::Side`] on an ARMED shell (past its
/// [`KICK_GRACE_S`] window) is a real hit.
///
/// Ordered BEFORE the shared contact-damage pass so a stomp makes the snake inert
/// that frame, before that pass could hurt the stomper.
///
/// A snake that is actually DEAD leaves the machine entirely. A shell can kill
/// other snakes now, and the engine HIDES a dead hostile actor — so a corpse that
/// kept stepping would slide on invisibly, dealing hits nothing on screen explains.
#[allow(clippy::too_many_arguments)]
pub fn run_snake_shells(
    mut commands: Commands,
    world_time: Res<ambition_platformer2d::time::WorldTime>,
    mut vfx: MessageWriter<ambition_platformer2d::vfx::VfxMessage>,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut hits: MessageWriter<HitEvent>,
    mut players: Query<(Entity, &mut ae::BodyKinematics), With<PrimaryPlayer>>,
    mut snakes: Query<
        (
            Entity,
            &FeatureId,
            &ambition_platformer2d::characters::actor::BodyHealth,
            &mut ae::BodyKinematics,
            Option<&mut ControlHolds>,
            &mut ContactThreatWithdrawn,
            &mut SnakeShell,
        ),
        (Without<PrimaryPlayer>, Without<PlayerEntity>),
    >,
) {
    let dt = world_time.scaled_dt;
    // Read the player once (entity + body): a missing player means no stomp/hit.
    let player_read = players
        .single()
        .ok()
        .map(|(e, p)| (e, p.aabb(), p.pos, p.vel));
    let player_box = player_read.map(|(_, aabb, pos, vel)| (aabb, pos, vel));

    // First pass: advance every snake's phase and apply presentation/physics.
    // Collect each sliding shell's (entity, AABB, own-id, whether it is
    // side-hitting the player) so the shared-pipeline hits below retain causal
    // attribution after the mutable query borrow ends.
    let mut sliding: Vec<(Entity, ae::Aabb, String, bool)> = Vec::new();
    for (entity, feature_id, health, mut kin, mut holds, mut withdrawn, mut shell) in &mut snakes {
        if !health.alive() {
            // A corpse is out of the mechanic: it stops sliding, advances no phase,
            // and deals no hits. Its shell state is left as-is so a respawned body
            // (`OnRoomReenter`) that reuses it is not mistaken for a fresh walker.
            kin.vel.x = 0.0;
            continue;
        }
        let touch = player_box.and_then(|(p, _, pvel)| player_touch(kin.aabb(), p, pvel));
        let inputs = shell_inputs_for(*shell, &kin, player_box.map(|(_, pos, _)| pos), touch);
        let fx = step_snake_shell(*shell, dt, inputs);

        // The stomp bounce is applied to the PLAYER (a fresh stomp on a walker OR a
        // stomp from above onto a moving shell — both stop the threat and bounce).
        if fx.just_squashed {
            let mut stomper = None;
            if let Ok((player_entity, mut player)) = players.single_mut() {
                ae::movement::set_jump_velocity(
                    &mut player.vel,
                    ae::DEFAULT_GRAVITY_DIR,
                    BOUNCE_SPEED,
                );
                stomper = Some(player_entity);
            }
            vfx.write(ambition_platformer2d::vfx::VfxMessage::Burst {
                pos: kin.pos,
                count: 12,
                speed: 130.0,
                color: [0.80, 0.68, 0.48, 1.0],
                kind: ambition_platformer2d::vfx::ParticleKind::Dust,
            });
            // H2: a stomp is the STOMPER's verb — the same rule the engine's pogo
            // uses, where the bouncing owner owns the cue. It is the player who
            // bounced off this shell, so the thud is the player's.
            // I3: with nobody to credit, it falls back to the COURSE, not to the
            // session — a Mary-O sound stays Mary-O's even when the stomper is
            // gone by the time the effect resolves.
            match stomper {
                Some(stomper) => sfx.write_for(
                    stomper,
                    ambition_platformer2d::sfx::SfxMessage::Pogo { pos: kin.pos },
                ),
                None => sfx.write_from(
                    crate::provider::MARY_O_EXPERIENCE,
                    ambition_platformer2d::sfx::SfxMessage::Pogo { pos: kin.pos },
                ),
            }
        }
        if fx.just_kicked {
            let kicker = players.single().ok().map(|(entity, _)| entity);
            match kicker {
                Some(kicker) => sfx.write_for(
                    kicker,
                    ambition_platformer2d::sfx::SfxMessage::Pogo { pos: kin.pos },
                ),
                None => sfx.write_from(
                    crate::provider::MARY_O_EXPERIENCE,
                    ambition_platformer2d::sfx::SfxMessage::Pogo { pos: kin.pos },
                ),
            }
        }

        // A stomp changes the snake's STATE, it does not hurt it: the body stays
        // alive (never hidden as a dead hostile actor) while these two levers make
        // it a shell. A walker clears both — back to a moving, touchable threat.
        // The threat is a read of the shell phase, so it is written every tick
        // and the authored tuning is never touched.
        withdrawn.0 = !fx.alive;
        // Frozen in place while shelled: the shell cycle is a scripted beat that
        // drives this body, so it HOLDS control (`ControlHold::Sequence`) from
        // the withdraw to the emerge, as a transition, rather than re-stamping
        // the hit reaction's recoil lock every tick. That lock is the engine's
        // answer to "was this body struck", and writing it here both made a
        // shell read as hitstun and zeroed a real hit's lock on every walking
        // tick.
        let was_walking = matches!(*shell, SnakeShell::Walking);
        if was_walking && !fx.alive {
            claim_control_hold(&mut commands, entity, ControlHold::Sequence);
        } else if !was_walking && fx.alive {
            release_control_hold(&mut commands, entity, holds.as_deref_mut(), ControlHold::Sequence);
        }

        // Command horizontal velocity for the shell stages; leave a walker's own
        // physics alone.
        if let Some(vx) = fx.vel_x {
            kin.vel.x = vx;
        }

        // Pin (or release) the pose through the shared override seam.
        match fx.anim {
            Some(anim) => {
                commands.entity(entity).try_insert(ActorAnimOverride(anim));
            }
            None => {
                commands.entity(entity).remove::<ActorAnimOverride>();
            }
        }

        if let SnakeShell::Sliding { grace, .. } = fx.phase {
            // The player is hurt only by a SIDE contact with an ARMED shell. From
            // the top it is a stomp (the pre-emption above already turned this shell
            // Boxed, so it is not even here), and during the post-kick grace the
            // shell is still inside whoever kicked it.
            let side_hit = grace <= 0.0 && touch == Some(PlayerTouch::Side);
            sliding.push((
                entity,
                kin.aabb(),
                feature_id.as_str().to_string(),
                side_hit,
            ));
        }
        *shell = fx.phase;
    }

    // Second pass: every sliding shell deals its damage through the ONE shared hit
    // pipeline. The enemy broadcast (Volume) reaches every actor in the volume EXCEPT
    // the player (the drain's actor query is `Without<PlayerEntity>`) and EXCEPT the
    // shell itself (ignored by both disposition prefixes); the side-hit player event
    // is the only thing that can hurt the player.
    for (shell_entity, aabb, self_id, side_hit) in sliding {
        hits.write(HitEvent {
            strike_sfx: None,
            volume: aabb.into(),
            damage: SHELL_ENEMY_DAMAGE,
            source: HitSource::Contact,
            attacker: Some(shell_entity),
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: vec![format!("enemy:{self_id}"), format!("npc:{self_id}")],
                    attacker_move_instance: None,
        });
        if side_hit {
            if let Some((player_entity, ..)) = player_read {
                hits.write(HitEvent {
                    strike_sfx: None,
                    volume: aabb.into(),
                    damage: SHELL_PLAYER_DAMAGE,
                    source: HitSource::Contact,
                    attacker: Some(shell_entity),
                    target: HitTarget::Body(player_entity),
                    mode: HitMode::Knockback,
                    knockback: None,
                    ignored_targets: Vec::new(),
                                    attacker_move_instance: None,
                });
            }
        }
    }
}

/// Read what happened to one snake this tick from the world, given the already
/// classified [`PlayerTouch`]. Both player-driven inputs come from that ONE
/// classification, so "stomp" and "kick" can never disagree about a contact.
fn shell_inputs_for(
    shell: SnakeShell,
    kin: &ae::BodyKinematics,
    player_pos: Option<ae::Vec2>,
    touch: Option<PlayerTouch>,
) -> ShellInputs {
    // ANY touch on a RESTING shell kicks it away from the player — a stomp from
    // above as much as a bump from the side. Ties (dead centre) go right.
    let kick_dir = match (shell, touch, player_pos) {
        (SnakeShell::Boxed(_), Some(PlayerTouch::Top | PlayerTouch::Side), Some(ppos)) => {
            Some(if ppos.x <= kin.pos.x { 1.0 } else { -1.0 })
        }
        _ => None,
    };
    ShellInputs {
        stomped: touch == Some(PlayerTouch::Top),
        kick_dir,
        // A sliding shell whose speed collapsed hit something.
        blocked: matches!(shell, SnakeShell::Sliding { .. })
            && kin.vel.x.abs() < SHELL_SLIDE_SPEED * 0.25,
    }
}

#[cfg(test)]
mod tests;
