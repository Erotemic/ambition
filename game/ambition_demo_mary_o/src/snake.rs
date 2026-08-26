//! Mary-O's Solid Snake walker and shell state machine.
//!
//! A stomp changes the same live body into an inert shell rather than killing
//! or replacing it. While withdrawn, steering is suppressed and contact damage
//! is disabled; emerging restores walker behavior. A kicked shell drives its
//! horizontal velocity directly. Shared stomp classification makes top contact
//! safe in every phase, while side contact with a running shell can damage the
//! player. [`step_snake_shell`] contains the pure phase logic.

use bevy::prelude::*;

use ambition_platformer2d::actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d::actors::features::{ActorConfig, CenteredAabb, FeatureId};
use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
use ambition_platformer2d::characters::actor::BodyCombat;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
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

/// The freeze-lock duration re-stamped onto a withdrawn snake each tick (the
/// engine's `recoil_lock_timer`, which hard-zeros movement input). Any value
/// comfortably above one frame works: it is refreshed every shelled tick and
/// cleared the instant the snake emerges, so it only has to outlast a tick's decay.
const SHELL_FREEZE_LOCK: f32 = 1.0;

/// A Solid Snake's shell lifecycle. `Walking` is the ordinary patroller; the rest
/// are the withdraw cycle, driven by [`step_snake_shell`]. Each timed stage's `f32`
/// is the time it has left.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
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

/// Demo-owned hostile roster: ONE 1-HP `Wanderer` archetype. It walks forward and reverses at
/// walls; `aggro_radius`/`attack_range` are ignored by that template. It carries no `melee`, so its
/// only offense is the default-on body contact — which the shell state turns off (via its
/// `body_contact_damage` tuning) while withdrawn, then back on when it walks again.

/// The `solid_snake` sheet TARGET (also the catalog id) — the generated sheet the
/// enemy render resolves for a Solid Snake.
pub const SNAKE_SHEET_TARGET: &str = "solid_snake";

/// World units per sheet pixel, derived from the authored snake body width and
/// the idle sheet collision width. Per-animation body rectangles then determine
/// collision, hurtbox, sprite size, and placement for each shape.
pub fn snake_world_per_pixel() -> f32 {
    // Stable fallback for headless/no-asset compositions.
    const NO_SHEET: f32 = 0.35;
    ambition_platformer2d::character_sprites::posed_body_geometry(
        SNAKE_SHEET_TARGET,
        CharacterAnim::Idle,
        1.0,
    )
    .map(|sheet| sheet.collision.x)
    .filter(|width| *width > 0.0)
    .map_or(NO_SHEET, |sheet_width| snake_body_width() / sheet_width)
}

/// WIDTH, not height, and that is the whole point of restating it. The snake is a 2.25:1
/// animal, so a scale that preserves its width fixes the only dimension a corridor cares about.
///
/// ```text
///            world_per_pixel   collision (world)   tiles tall
/// was 08-06       0.35            41 x 18            0.56
/// now 08-18       0.182           21.3 x 9.5         0.30
/// ```
///
/// that is the derivation working, not breaking — a snake occupies the
/// corridor width she does, and she got narrower. But nobody chose "a third of
/// a tile tall", and whether an enemy this flat still reads as an enemy is a
/// look-at-it call.  do not re-tune this constant to restore the old
/// number; the number to change, if any, is hers.
///
/// the reusable half: a value DERIVED from another character moves when
/// that character does, and no test says so — the ratchet beside this one
/// pins the quad/body RATIO, which is scale-invariant and therefore silent
/// about a halving.
///
/// AND IT IS DERIVED FROM MARY-O, not chosen. A snake
/// occupies the same corridor width she does — which is what a Koopa does beside
/// Mario, and the answer stays right when either sheet is regenerated.
pub fn snake_body_width() -> f32 {
    // The value this replaced, for a composition with no baked art — so a
    // headless fixture keeps the size it had rather than acquiring Mary-O's.
    const NO_SHEET: f32 = 41.0;
    crate::powerups::mary_o_body_width().unwrap_or(NO_SHEET)
}

/// How near an observer has to be for a snake to keep thinking.
///
/// The same distance the AI Slop uses. They share a level and a job — patrol
/// until somebody arrives — so a snake waking at a different range from a slop
/// standing beside it would be a difference with no reason behind it.
pub const SNAKE_WAKE_RADIUS: f32 = crate::ai_slop::AI_SLOP_WAKE_RADIUS;

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

/// shared by `install_mary_o_content` and the stomp fixtures, so a test
/// exercises the registration production uses. A fixture registering something
/// else would be measuring itself.
pub fn register_solid_snake_character(app: &mut App) {
    register_mary_o_enemy_character(app, SNAKE_SHEET_TARGET, SNAKE_DISPLAY_NAME, 46.0);
}

/// Register AI Slop as a CHARACTER. Same shape, one creature over: a plain
/// stomp-and-die walker whose only offense is the body it walks into you with.
pub fn register_ai_slop_character(app: &mut App) {
    register_mary_o_enemy_character(
        app,
        crate::ai_slop::AI_SLOP_SHEET_TARGET,
        crate::ai_slop::AI_SLOP_DISPLAY_NAME,
        42.0,
    );
}

/// Both of Mary-O's enemies, which differ only in their art and their pace: one
/// hit point, a forward walk that reverses at walls, contact damage as their
/// whole offense, and a policy that notices nobody.
fn register_mary_o_enemy_character(app: &mut App, id: &str, display: &str, run_speed: f32) {
    use ambition_platformer2d::actors::character_runtime::{
        CharacterDefinition, CharacterDefinitionAppExt,
    };
    use ambition_platformer2d::characters::actor::{CharacterLocomotion, ContactDamage};
    use ambition_platformer2d::characters::brain::{
        BrainProfile, CharacterBrainTemplate, MoveStyleSpec,
    };

    let mut definition = CharacterDefinition::new(id, display, crate::provider::MARY_O_EXPERIENCE)
        .with_sheet(id)
        .with_locomotion(CharacterLocomotion {
            run_speed,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_contact_damage(ContactDamage {
            strength: 0.5,
            amount: 1,
        })
        .with_autonomous_profile(BrainProfile {
            template: CharacterBrainTemplate::Wanderer,
            aggro_radius: 0.0,
            attack_range: 0.0,
            // the deleted row's own pace. A snake PACES at full speed —
            // it is walking its line, not patrolling — and `BrainProfile`'s
            // default is the ordinary half-speed amble. Without this the
            // fragment's deletion would have halved every snake in the demo.
            patrol_effort: 1.0,
            ..Default::default()
        });
    definition.vitals.max_health = Some(1);
    app.register_character(definition);
}

// `SNAKE_TILE_COLUMNS` is GONE. Where a snake patrols is authored.
//
// its SIZE stays, and the difference is the rule: how big a snake is comes
// from its sheet; where it patrols comes from the level.

/// The half-extents a freshly staged snake spawns with.
///
/// Read from the SAME sheet rectangle the shell system will hold it to, so the
/// body never pops on its first tick. A sheet that publishes no body metrics
/// (`--no-assets`, a stripped test fixture) falls back to a plain tile-ish box:
/// the snake still walks and is still stompable, it just isn't art-shaped.
fn snake_half_size() -> ae::Vec2 {
    ambition_platformer2d::character_sprites::posed_body_geometry(
        SNAKE_SHEET_TARGET,
        CharacterAnim::Idle,
        snake_world_per_pixel(),
    )
    .map_or(ae::Vec2::new(14.0, 16.0), |geometry| {
        geometry.collision * 0.5
    })
}

/// Identify snakes by their authored `CharacterBrain` archetype key.
///
/// `ActorConfig.brain` is a read-model of authored identity, so callers should
/// not substitute display names or generated feature-id conventions.
pub fn is_snake_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == SNAKE_BRAIN_KEY)
}

/// Tag freshly staged snakes with `SnakeShell::Walking` (so the shell system
/// finds its own) and with `SpritePosedBody` (so its body geometry comes from
/// the sheet, per pose — the box a stomp shrinks).
///
/// and give it the sheet's box on the spot, which is not redundant with `SpritePosedBody`.
/// The authored placement's rectangle is what the engine spawns a body at, and a snake's LDtk
/// rect is one tile-ish while its sheet body is more than twice as wide.
///
/// the rule is unchanged: how big a snake is comes from its sheet, where
/// it patrols comes from the level. This is only the sheet answering sooner.
pub fn tag_mary_o_snakes(
    mut commands: Commands,
    mut fresh: Query<(Entity, &ActorConfig, &mut CenteredAabb), Without<SnakeShell>>,
) {
    for (entity, config, mut body) in &mut fresh {
        if is_snake_brain(&config.brain) {
            body.half_size = snake_half_size();
            commands.entity(entity).try_insert((
                SnakeShell::Walking,
                ambition_platformer2d::sprite_sheet::character::SpritePosedBody::new(
                    SNAKE_SHEET_TARGET,
                    snake_world_per_pixel(),
                ),
                // a kicked shell is unaffected, which is why this is safe.
                // Dormancy sleeps the BRAIN and clears the control frame;
                // `run_snake_shells` propels a slide by writing the body's
                // horizontal velocity directly, on a different channel. A shell
                // you kick still leaves the screen, as it should.
                ambition_platformer2d::actors::features::ecs::dormancy::DormancyPolicy::
                    AwakeNearObservers { radius: SNAKE_WAKE_RADIUS },
            ));
        }
    }
}

/// Restore demo-owned snake-shell state on room reset. Engine reset restores
/// body state, but `SnakeShell` and the shell mechanic's runtime contact-damage
/// toggle are owned here. Listen to `ResetRoomFeaturesEvent` so same-room retries
/// reset them as well as room reloads.
pub fn reset_snakes_on_room_reset(
    mut resets: MessageReader<ambition_platformer2d::combat::events::ResetRoomFeaturesEvent>,
    mut snakes: Query<(&mut SnakeShell, &mut BodyCombat, &mut ActorConfig)>,
) {
    if resets.read().count() == 0 {
        return;
    }
    for (mut shell, mut combat, mut config) in &mut snakes {
        *shell = SnakeShell::Walking;
        combat.recoil_lock_timer = 0.0;
        config.tuning.body_contact_damage = true;
    }
}

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
            &mut BodyCombat,
            &mut ActorConfig,
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
    for (entity, feature_id, health, mut kin, mut combat, mut config, mut shell) in &mut snakes {
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
        if fx.alive {
            combat.recoil_lock_timer = 0.0;
            config.tuning.body_contact_damage = true;
        } else {
            // Frozen in place (movement input hard-zeroed) and harmless to touch.
            combat.recoil_lock_timer = SHELL_FREEZE_LOCK;
            config.tuning.body_contact_damage = false;
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
