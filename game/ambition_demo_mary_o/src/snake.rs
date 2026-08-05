//! Mary-O's Solid Snake — the Koopa-equivalent walker, authored as pure content.
//!
//! Replaces the earlier `ai_slop` "crony". A Solid Snake paces the flats; a
//! head-stomp makes it withdraw INTO ITS SHELL *in place* (the SAME body), sit as
//! a kickable shell, and — if left alone — peek and climb back out to a walker.
//! Kicked, the shell slides down the line of other snakes. Every stage plays the
//! matching row of the `solid_snake` sheet (walk / retreat / boxed_idle / peek /
//! emerge / death) through the engine's animation-override seam
//! ([`ActorAnimOverride`]).
//!
//! ## Why the shell needs no brain swap, and never "dies"
//!
//! A stomp does NOT hurt the snake — it CHANGES ITS STATE. The body stays alive
//! (full HP) the whole time, so it is never hidden by the engine's "a dead hostile
//! actor is invisible" render rule; a shelled snake is always on screen. "Shelled"
//! is composed from two real engine levers, re-held on the SAME body each frame it
//! is withdrawn:
//!   * **Frozen in place** — its [`BodyCombat::recoil_lock_timer`] is refreshed:
//!     the engine's "carried, can't steer" gate hard-zeros movement input, so the
//!     Wanderer brain can't walk it while gravity still settles it on the ground.
//!   * **Inert (harmless)** — its [`ActorConfig`] `body_contact_damage` tuning is
//!     turned off, so the shared body-contact pass deals no damage: a resting shell
//!     is safe to walk up to and kick.
//!
//! Emerging clears both and the snake is a live, threatening walker again — there
//! is no revive-from-death, because it never died. Sliding is driven by writing the
//! body's horizontal velocity each tick, exactly as the old shell prop did.
//!
//! ## Standing on a snake is ALWAYS safe
//!
//! Every player contact is classified once by the shared [`crate::stomp`] rule.
//! From the TOP it is a stomp, and a stomp pre-empts every phase — walking, boxed,
//! running, peeking, emerging — so the body under the player's feet is inert this
//! tick and the bounce carries them off it. Only a SIDE contact with a running
//! shell (past its post-kick grace) can hurt them.
//!
//! The phase logic is a pure function ([`step_snake_shell`]) with the ECS system a
//! thin shell over it, mirroring `flag.rs` — so the choreography is unit-tested
//! even though its LOOK is not visible headlessly.
//!
//! Every type it names comes through the `ambition_platformer2d` umbrella — the E9 oracle.

use bevy::prelude::*;

use ambition_platformer2d::actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d::actors::features::{
    ActorAnimOverride, ActorConfig, CenteredAabb, FeatureId,
};
use ambition_platformer2d::actors::features::{HitEvent, HitMode, HitSource, HitTarget};
use ambition_platformer2d::characters::actor::BodyCombat;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

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
        /// **Spent immediately by a wall bounce**, not only by the clock: a shell
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

/// **The whole shell choreography, as a pure function.** Given the current phase,
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
    // **A stomp pre-empts every phase.** It is the one input that always means "be
    // in the shell", and it always bounces the stomper. That is what makes standing
    // over ANY snake safe: whatever the body under the player's feet was doing —
    // walking, running as a kicked shell, peeking back out — this tick it is inert,
    // and the bounce carries them off it.
    if inputs.stomped {
        let reseat = ShellEffects {
            just_squashed: true,
            ..shelled(Boxed(BOXED_S), CharacterAnim::ShellIdle)
        };
        // **Landing on a RESTING shell kicks it**, exactly as a side bump does.
        // It used to re-seat in place, and that is what let a shell under a
        // ?-block trap her: she bounced off it, the block stopped her rise, she
        // fell back onto it, forever. The classic never does that — you land on
        // a still shell and it shoots out from under you.
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
            // **A wall ARMS it.** The grace exists for one reason — the shell is
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

/// Demo-owned hostile roster: ONE 1-HP `Wanderer` archetype. It walks forward and
/// reverses at walls; `aggro_radius`/`attack_range` are ignored by that template.
/// It carries no `melee`, so its only offense is the default-on body contact —
/// which the shell state turns off (via its `body_contact_damage` tuning) while
/// withdrawn, then back on when it walks again.
/// The Solid Snake archetype ROW (no outer braces), so it can register on its own
/// for a single-enemy test OR fold into the combined Mary-O roster fragment — one
/// fragment per provider, since assembly rejects a second from the same provider.
pub(crate) const SNAKE_ROSTER_ROWS: &str = r#"
    "mary_o_snake": (
        max_health: 1,
        run_speed: 46.0,
        patrol_effort: 1.0,
        chase_effort: 1.0,
        aggro_radius: 0.0,
        attack_range: 0.0,
        contact_strength: 0.5,
        damage_amount: 1,
        brain_template: Wanderer,
        move_style: Walk,
        respawn: OnRoomReenter,
    ),
"#;

/// The `solid_snake` sheet TARGET (also the catalog id) — the generated sheet the
/// enemy render resolves for a Solid Snake.
pub const SNAKE_SHEET_TARGET: &str = "solid_snake";

/// **World units per sheet pixel — the snake's ENTIRE authored geometry.**
///
/// A Solid Snake changes SHAPE, not just pose: sprawled it is a long low serpent,
/// withdrawn it is a small cardboard cube. One hand-authored rectangle cannot be
/// right for both, so the body's collision box, hurt box, sprite quad and quad
/// placement all come from the sheet's per-animation body rectangles, scaled by
/// this. Everything else about its size is READ OFF THE ART — including the fact
/// that stomping it shrinks its box, which is the whole point.
///
/// > `0.5` draws the 128px frame two tiles across, which puts the walker at
/// > about 1.8 tiles long and the kickable box at about three quarters of a
/// > tile: a snake you jump on, and a shell you kick down a line of them.
///
/// (The paragraph above is the ORIGINAL, kept because its intent is right.)
///
/// ⚠ **`0.5` did not deliver 1.8 tiles.** Jon, from play, 2026-08-05: *"the
/// scale of solid snake increased a lot for some reason in the ldtk transition,
/// they need to be a bit smaller in game."* It did not increase in the
/// migration — until the enemies got their sheets back that same day they drew
/// as 28px placeholder boxes, so this size had never actually been on screen.
///
/// ⛔ **and my first measurement of it was WRONG.** I colour-sampled "green" out
/// of a capture and got 213x57 px, called the snake 5.8 tiles long, and wrote
/// that number into this comment as fact. Mary-O's world is full of green warp
/// PIPES; the sample had swallowed one. Halving the constant on that basis
/// produced a snake visibly smaller than the crate beside it — which is what
/// LOOKING at the render showed immediately and the arithmetic never would have.
/// A colour filter is not an instrument for "how big is that thing" in a scene
/// containing other things of that colour.
///
/// ⚠ **and the SECOND measurement was no better.** A snake has two bodies — the
/// sprawled walker and the withdrawn box — and captures taken at different
/// moments catch different ones, so "how long is the snake on screen" answered
/// 213px, then 53px, then 230px across three runs at three different constants.
/// None of those comparisons was like-for-like. (It also means the tan crates
/// visible beside the snakes in every Mary-O capture are BOXED SNAKES, not
/// scenery and not bricks — I reported them as bricks drawing with a wooden
/// texture, and that was wrong.)
///
/// So `0.35` is chosen by ARITHMETIC, honestly labelled: it is a 30% reduction
/// on the value Jon asked to shrink, and nothing more. Turning this knob again
/// is a taste call best made by whoever is looking at the running game.
pub const SNAKE_WORLD_PER_PIXEL: f32 = 0.35;

/// How near an observer has to be for a snake to keep thinking.
///
/// The same distance the AI Slop uses. They share a level and a job — patrol
/// until somebody arrives — so a snake waking at a different range from a slop
/// standing beside it would be a difference with no reason behind it.
pub const SNAKE_WAKE_RADIUS: f32 = crate::ai_slop::AI_SLOP_WAKE_RADIUS;

/// **Ensure the `solid_snake` sheet is drawable**, keyed by BOTH its catalog id
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

/// Register the demo's hostile roster fragment. Shares the Mary-O provider id so
/// its brain key namespaces under this experience.
pub fn register_snake_roster(app: &mut App) {
    use ambition_platformer2d::actors::features::{CharacterRosterAppExt, CharacterRosterFragment};
    app.register_character_roster_fragment(
        CharacterRosterFragment::from_ron(
            crate::provider::MARY_O_EXPERIENCE,
            None::<String>,
            &format!("{{{SNAKE_ROSTER_ROWS}}}"),
        )
        .expect("Mary-O snake roster fragment should be valid"),
    );
}

// ⭐ `SNAKE_TILE_COLUMNS` is GONE. Where a snake patrols is authored.
//
// ⚠ its SIZE stays, and the difference is the rule: how big a snake is comes
// from its sheet; where it patrols comes from the level.

/// The half-extents a freshly staged snake spawns with.
///
/// Read from the SAME sheet rectangle the shell system will hold it to, so the
/// body never pops on its first tick. A sheet that publishes no body metrics
/// (`--no-assets`, a stripped test fixture) falls back to a plain tile-ish box:
/// the snake still walks and is still stompable, it just isn't art-shaped.
fn snake_half_size() -> ae::Vec2 {
    ambition_platformer2d::actors::character_sprites::posed_body_geometry(
        SNAKE_SHEET_TARGET,
        CharacterAnim::Idle,
        SNAKE_WORLD_PER_PIXEL,
    )
    .map_or(ae::Vec2::new(14.0, 16.0), |geometry| {
        geometry.collision * 0.5
    })
}

/// **Is this actor a snake?** The one question every snake system asks.
///
/// ⛔ **identity is the ARCHETYPE, not a name and not an id prefix.** This
/// answer has now been wrong twice, each time because it was reading something
/// that merely correlated with being a snake.
///
/// The first version matched `name.0 == SNAKE_DISPLAY_NAME`, and `FeatureName`'s
/// own doc says what that is: *"human-facing authored name for debug overlays /
/// inspectors."* Renaming the character in the catalog would have silently
/// stopped every stomp.
///
/// The second matched a `FeatureId` PREFIX — `mary_o_snake_<n>` — which was
/// stable, but only because this crate minted the id itself in a staging
/// closure. That closure was a SECOND construction path over the same authored
/// placements, and when the level moved into LDtk the engine's own
/// `authored_actor_requests` began building them too. Every snake existed
/// twice: one under the raw placement id with no shell and no stomp, one under
/// the prefixed id with both. A prefix is only identity while you control who
/// mints it, and by then nobody did.
///
/// `ActorConfig.brain` is the authored placement's `CharacterBrain` carried onto
/// the entity verbatim, whichever path built it. That is the archetype key
/// itself rather than a string that happens to contain it.
///
/// ⚠ **it is a read-model, not an authority.** `ActorConfig`'s doc calls it a
/// projection, and the reconcile paths in `autonomous_reconcile.rs` do write it
/// back — but only for actors carrying a `BrainBinding`, which the peaceful-NPC
/// and provocation paths attach and authored enemies never get. True today,
/// unprotected structurally, and worth knowing if a snake is ever provoked into
/// a rebuilt brain.
pub fn is_snake_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == SNAKE_BRAIN_KEY)
}

/// Tag freshly staged snakes with `SnakeShell::Walking` (so the shell system
/// finds its own) and with `SpritePosedBody` (so its body geometry comes from
/// the sheet, per pose — the box a stomp shrinks).
///
/// ⚠ **and give it the sheet's box on the spot**, which is not redundant with
/// `SpritePosedBody`. The authored placement's rectangle is what the engine
/// spawns a body at, and a snake's LDtk rect is one tile-ish while its sheet
/// body is more than twice as wide. `SpritePosedBody` corrects that — measured,
/// on the THIRD frame — so without this the level opens with two frames of
/// half-width snake. The staging path this replaced never showed it, because it
/// spawned the request at [`snake_half_size`] directly.
///
/// ⭐ **the rule is unchanged**: how big a snake is comes from its sheet, where
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
                ambition_platformer2d::actors::character_sprites::SpritePosedBody::new(
                    SNAKE_SHEET_TARGET,
                    SNAKE_WORLD_PER_PIXEL,
                ),
                // ⛔ **the snake had no dormancy policy and the slop did.** Jon
                // named the slop — *"ai slop will just walk off the edge of the
                // level before she even gets to that part of the level"* — the
                // engine seam was built, the slop was wired, and the OTHER
                // patrolling enemy in the same level was left thinking for the
                // whole course. A family is not migrated when one member is.
                //
                // ⚠ **a kicked shell is unaffected, which is why this is safe.**
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

/// **A reset snake is a WALKER again.**
///
/// The engine's reset restores what the ENGINE owns — position, HP, disposition,
/// the pose pin — but a shell is composed from state the engine cannot know
/// about: this demo's own `SnakeShell` phase, and the `body_contact_damage`
/// tuning the shell machine flips to make a boxed snake safe to walk up to.
/// Neither is touched by `reset_to_spawn`, so a replay brought back snakes still
/// boxed, and one that was mid-kick came back still sliding — running down a
/// level in which nobody had stomped anything.
///
/// It hangs off `ResetRoomFeaturesEvent`, the engine's ONE "put this room back"
/// signal, rather than `RoomLoaded`: that also covers a same-room retry, which a
/// load-time hook would miss. Every field the shell machine writes is written
/// back, so this cannot drift into resetting only some of them:
/// - the phase → `Walking`,
/// - the freeze lock → cleared (it can walk),
/// - contact damage → armed (it is a threat again).
///
/// AMBITION_REVIEW: `body_contact_damage` living on `ActorConfig.tuning` is why
/// this has to exist at all. That struct is documented as the archetype's
/// authored projection, "never mutated at runtime" — which is why the engine's
/// reset does not restore it — and this demo mutates it every frame as a lever.
/// A per-body runtime switch, distinct from authored tuning, would let the
/// engine reset it generically for every game.
pub fn reset_snakes_on_room_reset(
    mut resets: MessageReader<ambition_platformer2d::actors::features::ResetRoomFeaturesEvent>,
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

/// **The Solid Snake shell mechanic**, the thin ECS wrapper over [`step_snake_shell`].
///
/// Reads what happened to each snake this tick (stomp / kick / wall-block),
/// advances its phase, and reflects the result: bounce the stomper, pin the pose
/// via [`ActorAnimOverride`], freeze the (still-alive) body and turn its contact
/// threat off — or slide it — and let it walk and threaten again when it finishes
/// emerging.
///
/// **A moving shell is a kinetic hazard.** Each tick it is sliding, it broadcasts a
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
/// **A snake that is actually DEAD leaves the machine entirely.** A shell can kill
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
            source: HitSource::EnemyChargeCrash,
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
                    source: HitSource::EnemyBody,
                    attacker: Some(shell_entity),
                    target: HitTarget::Player(player_entity),
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
