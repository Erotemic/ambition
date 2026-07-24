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
//! ## Why the shell needs no brain swap or contact-damage plumbing
//!
//! The withdraw is expressed as **death without despawn**: entering the shell sets
//! the body's HP to 0. A zero-HP body is a `body_is_corpse` — the engine's enemy
//! integration returns a NEUTRAL control frame for it (so it stops walking with no
//! brain change) and the shared body-contact-damage pass skips it (so a resting
//! shell is safe to walk up to and kick). Its `respawn` policy is NOT `InPlace`,
//! so the engine never auto-revives it; this module owns the revive, restoring HP
//! when it finishes emerging. Sliding is driven by writing the body's horizontal
//! velocity each tick, exactly as the old shell prop did.
//!
//! The phase logic is a pure function ([`step_snake_shell`]) with the ECS system a
//! thin shell over it, mirroring `flag.rs` — so the choreography is unit-tested
//! even though its LOOK is not visible headlessly.
//!
//! Every type it names comes through the `ambition` umbrella — the E9 oracle.

use bevy::prelude::*;

use ambition::actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition::actors::combat::components::ActorFaction;
use ambition::actors::features::{ActorAnimOverride, FeatureName};
use ambition::actors::features::{SpawnActorKind, SpawnActorRequest};
use ambition::characters::actor::BodyHealth;
use ambition::engine_core as ae;
use ambition::entity_catalog::placements::CharacterBrain;
use ambition::sprite_sheet::character::CharacterAnim;

use crate::{LEVEL_1_1_ROOM_ID, T};

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

/// Forgiving band (px) for "the shell overlaps a snake" — same slack the stomp uses.
const SHELL_HIT_BAND: f32 = 4.0;

/// Vertical tolerance (px) for "feet on the snake's head".
const STOMP_BAND: f32 = 16.0;

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
/// are the withdraw cycle, driven by [`step_snake_shell`]. The `f32` is the
/// remaining time in that stage, or (for `Sliding`) the travel direction.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum SnakeShell {
    Walking,
    Retreating(f32),
    Boxed(f32),
    /// Sliding along the ground. `-1.0` is leftward, `1.0` rightward.
    Sliding(f32),
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
    /// Whether the body should read as ALIVE (tangible walker) this tick. Shell
    /// phases are `false` — a corpse that doesn't walk and can't be touched for
    /// damage — until it finishes emerging.
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShellInputs {
    /// A falling player's feet are on this body's head this tick.
    pub stomped: bool,
    /// The player is touching a boxed shell from a side; `Some(dir)` is the way to
    /// launch it (away from the player).
    pub side_kick: Option<f32>,
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
    match phase {
        Walking => {
            if inputs.stomped {
                ShellEffects {
                    just_squashed: true,
                    ..shelled(Retreating(RETREAT_S), CharacterAnim::Retreat)
                }
            } else {
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
            if let Some(dir) = inputs.side_kick {
                ShellEffects {
                    just_kicked: true,
                    vel_x: Some(dir * SHELL_SLIDE_SPEED),
                    ..shelled(Sliding(dir), CharacterAnim::ShellIdle)
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
        Sliding(dir) => {
            if inputs.stomped || inputs.side_kick.is_some() {
                // A stomp or a bump stops a running shell dead — it becomes a fresh
                // boxed shell you can re-kick.
                shelled(Boxed(BOXED_S), CharacterAnim::ShellIdle)
            } else {
                let dir = if inputs.blocked { -dir } else { dir };
                ShellEffects {
                    vel_x: Some(dir * SHELL_SLIDE_SPEED),
                    ..shelled(Sliding(dir), CharacterAnim::ShellIdle)
                }
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
/// which the shell's zero-HP corpse trick disables while withdrawn.
const SNAKE_ROSTER_RON: &str = r#"{
    "mary_o_snake": (
        max_health: 1,
        patrol_speed: 46.0,
        chase_speed: 46.0,
        aggro_radius: 0.0,
        attack_range: 0.0,
        contact_strength: 0.5,
        damage_amount: 1,
        brain_template: Wanderer,
        move_style: Walk,
        respawn: OnRoomReenter,
    ),
}"#;

/// The `solid_snake` sheet TARGET (also the catalog id) — the generated sheet the
/// enemy render resolves for a Solid Snake.
pub const SNAKE_SHEET_TARGET: &str = "solid_snake";

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
    game_assets: Option<ResMut<ambition::sprite_sheet::game_assets::GameAssets>>,
    config: Option<Res<ambition::sprite_sheet::game_assets::GameAssetConfig>>,
    asset_server: Option<Res<AssetServer>>,
    layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
) {
    let (Some(mut game_assets), Some(config), Some(asset_server), Some(mut layouts)) =
        (game_assets, config, asset_server, layouts)
    else {
        return;
    };
    if config.no_assets
        || game_assets
            .characters
            .npc_asset_for_name(SNAKE_DISPLAY_NAME)
            .is_some()
    {
        return;
    }
    if let Some(asset) = ambition::actors::character_sprites::load_prop_sheet_for_target(
        &asset_server,
        &mut layouts,
        &config.sprite_folder,
        SNAKE_SHEET_TARGET,
        &ambition::sprite_sheet::character::SheetTuning::new(1.0, 0),
    ) {
        // Double-keyed exactly like the eager loader: the render resolves an actor
        // by its display name, and other seams by the catalog id.
        game_assets
            .characters
            .npcs
            .insert(SNAKE_SHEET_TARGET.to_string(), asset.clone());
        game_assets
            .characters
            .npcs
            .insert(SNAKE_DISPLAY_NAME.to_string(), asset);
    }
}

/// Register the demo's hostile roster fragment. Shares the Mary-O provider id so
/// its brain key namespaces under this experience.
pub fn register_snake_roster(app: &mut App) {
    use ambition::actors::features::{CharacterRosterAppExt, CharacterRosterFragment};
    app.register_character_roster_fragment(
        CharacterRosterFragment::from_ron(
            crate::provider::MARY_O_EXPERIENCE,
            None::<String>,
            SNAKE_ROSTER_RON,
        )
        .expect("Mary-O snake roster fragment should be valid"),
    );
}

/// Tile x-columns (level grid) each snake paces near — the flats after the pit
/// rhythm, so the walker is a hazard on solid ground, not stranded over a pit.
const SNAKE_TILE_COLUMNS: &[f32] = &[9.0, 16.0, 27.0, 45.0, 63.0];

/// The snake spawn requests for level 1-1, dropped at the player's standing height
/// so gravity settles each onto the ground beneath its column.
fn snake_spawn_requests(player_spawn: ae::Vec2) -> Vec<SpawnActorRequest> {
    SNAKE_TILE_COLUMNS
        .iter()
        .enumerate()
        .map(|(i, col)| SpawnActorRequest {
            id: format!("mary_o_snake_{i}"),
            name: SNAKE_DISPLAY_NAME.to_string(),
            pos: ae::Vec2::new(col * T, player_spawn.y),
            half_size: ae::Vec2::new(14.0, 16.0),
            faction: ActorFaction::Enemy,
            grudge_against: None,
            kind: SpawnActorKind::Enemy {
                brain: CharacterBrain::Custom(SNAKE_BRAIN_KEY.to_string()),
            },
        })
        .collect()
}

/// Register the walkers as level 1-1's content staging: whenever the level's
/// contents are staged (initial load, every cyclic replay — the snakes
/// `respawn: OnRoomReenter` — and a snapshot restore), the walkers stage with them.
pub fn register_snake_content_staging(
    registry: &mut ambition::actors::features::RoomContentStagingRegistry,
) {
    registry
        .register(
            LEVEL_1_1_ROOM_ID,
            "ambition_demo_mary_o",
            "snake",
            "snake-staging.v1",
            |spec| snake_spawn_requests(spec.world.spawn),
        )
        .expect("snake staging registration is unique");
}

/// Tag freshly staged snakes with `SnakeShell::Walking`, so the shell system finds
/// its own. Matches `FeatureName` (the authored name), NOT `Name` (which the
/// spawner decorates) — matching `Name` is what silently broke the old shell tag.
pub fn tag_mary_o_snakes(
    mut commands: Commands,
    fresh: Query<(Entity, &FeatureName), Without<SnakeShell>>,
) {
    for (entity, name) in &fresh {
        if name.0 == SNAKE_DISPLAY_NAME {
            commands.entity(entity).try_insert(SnakeShell::Walking);
        }
    }
}

/// **The Solid Snake shell mechanic**, the thin ECS wrapper over [`step_snake_shell`].
///
/// Reads what happened to each snake this tick (stomp / kick / wall-block),
/// advances its phase, and reflects the result: bounce the stomper, pin the pose
/// via [`ActorAnimOverride`], hold the body dead-and-still (or slide it), and
/// revive it when it finishes emerging. A sliding shell also runs down other
/// snakes. Ordered BEFORE the shared contact-damage pass so a stomp neutralizes
/// the snake (to a corpse) before that pass could hurt the stomper.
#[allow(clippy::too_many_arguments)]
pub fn run_snake_shells(
    mut commands: Commands,
    world_time: Res<ambition::time::WorldTime>,
    mut vfx: MessageWriter<ambition::vfx::VfxMessage>,
    mut sfx: ambition::sfx::SfxWriter,
    mut players: Query<&mut ae::BodyKinematics, With<PrimaryPlayer>>,
    mut snakes: Query<
        (
            Entity,
            &mut ae::BodyKinematics,
            &mut BodyHealth,
            &mut SnakeShell,
        ),
        (Without<PrimaryPlayer>, Without<PlayerEntity>),
    >,
) {
    let dt = world_time.scaled_dt;
    // Read the player once; a missing player means no stomp/kick this tick.
    let player_box = players.single().ok().map(|p| (p.aabb(), p.pos, p.vel));

    // First pass: advance every snake's phase and apply presentation/physics.
    // Collect sliding-shell hitboxes to run down other snakes in a second pass.
    let mut sliding_hitboxes: Vec<(Entity, ae::Aabb)> = Vec::new();
    for (entity, mut kin, mut health, mut shell) in &mut snakes {
        let inputs = shell_inputs_for(*shell, &kin, player_box);
        let fx = step_snake_shell(*shell, dt, inputs);

        // The stomp bounce is applied to the PLAYER (only when a real stomp began).
        if fx.just_squashed {
            if let Ok(mut player) = players.single_mut() {
                ae::movement::set_jump_velocity(
                    &mut player.vel,
                    ae::DEFAULT_GRAVITY_DIR,
                    BOUNCE_SPEED,
                );
            }
            vfx.write(ambition::vfx::VfxMessage::Burst {
                pos: kin.pos,
                count: 12,
                speed: 130.0,
                color: [0.80, 0.68, 0.48, 1.0],
                kind: ambition::vfx::ParticleKind::Dust,
            });
            sfx.write(ambition::sfx::SfxMessage::Pogo { pos: kin.pos });
        }
        if fx.just_kicked {
            sfx.write(ambition::sfx::SfxMessage::Pogo { pos: kin.pos });
        }

        // Reflect aliveness on the body: withdrawing zeroes HP (→ intangible,
        // non-walking corpse); finishing emerging restores it to a live walker.
        if fx.alive {
            if !health.alive() {
                health.health.current = health.health.max;
            }
        } else if health.alive() {
            health.health.current = 0;
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

        if let SnakeShell::Sliding(_) = fx.phase {
            sliding_hitboxes.push((entity, kin.aabb()));
        }
        *shell = fx.phase;
    }

    // Second pass: a sliding shell runs down any OTHER snake it overlaps, popping
    // it into its own withdraw (a chain-reaction down the line).
    if sliding_hitboxes.is_empty() {
        return;
    }
    for (entity, mut kin, mut health, mut shell) in &mut snakes {
        // Only a live walker can be run down (a shell can't squash a shell).
        if !matches!(*shell, SnakeShell::Walking) || !health.alive() {
            continue;
        }
        let g = kin.aabb();
        let struck = sliding_hitboxes.iter().any(|(slider, s)| {
            *slider != entity
                && s.min.x < g.max.x + SHELL_HIT_BAND
                && s.max.x > g.min.x - SHELL_HIT_BAND
                && s.min.y < g.max.y
                && s.max.y > g.min.y
        });
        if !struck {
            continue;
        }
        vfx.write(ambition::vfx::VfxMessage::Burst {
            pos: kin.pos,
            count: 12,
            speed: 130.0,
            color: [0.80, 0.68, 0.48, 1.0],
            kind: ambition::vfx::ParticleKind::Dust,
        });
        sfx.write(ambition::sfx::SfxMessage::Pogo { pos: kin.pos });
        health.health.current = 0;
        kin.vel.x = 0.0;
        commands
            .entity(entity)
            .try_insert(ActorAnimOverride(CharacterAnim::Retreat));
        *shell = SnakeShell::Retreating(RETREAT_S);
    }
}

/// Read what happened to one snake this tick from the world.
fn shell_inputs_for(
    shell: SnakeShell,
    kin: &ae::BodyKinematics,
    player: Option<(ae::Aabb, ae::Vec2, ae::Vec2)>,
) -> ShellInputs {
    let g = kin.aabb();
    let Some((p, ppos, pvel)) = player else {
        // No player: a sliding shell can still notice a wall.
        return ShellInputs {
            blocked: matches!(shell, SnakeShell::Sliding(_))
                && kin.vel.x.abs() < SHELL_SLIDE_SPEED * 0.25,
            ..Default::default()
        };
    };
    let overlap_x = p.min.x < g.max.x && p.max.x > g.min.x;
    let overlap_y = p.min.y < g.max.y && p.max.y > g.min.y;
    let touching = overlap_x && overlap_y;
    // A falling player whose feet are on this body's head is stomping it.
    let feet = p.max.y;
    let on_head = feet >= g.min.y - STOMP_BAND && feet <= g.min.y + STOMP_BAND;
    let stomped = pvel.y > 0.0 && overlap_x && on_head;
    // A side touch on a BOXED shell (not a head stomp) kicks it away from the player.
    let side_kick = if matches!(shell, SnakeShell::Boxed(_)) && touching && !stomped {
        Some(if ppos.x <= kin.pos.x { 1.0 } else { -1.0 })
    } else {
        None
    };
    let blocked =
        matches!(shell, SnakeShell::Sliding(_)) && kin.vel.x.abs() < SHELL_SLIDE_SPEED * 0.25;
    ShellInputs {
        stomped,
        side_kick,
        blocked,
    }
}

#[cfg(test)]
mod tests;
