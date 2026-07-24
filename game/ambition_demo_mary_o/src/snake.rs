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
//! The phase logic is a pure function ([`step_snake_shell`]) with the ECS system a
//! thin shell over it, mirroring `flag.rs` — so the choreography is unit-tested
//! even though its LOOK is not visible headlessly.
//!
//! Every type it names comes through the `ambition` umbrella — the E9 oracle.

use bevy::prelude::*;

use ambition::actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition::actors::combat::components::ActorFaction;
use ambition::actors::features::{ActorAnimOverride, ActorConfig, FeatureId, FeatureName};
use ambition::actors::features::{HitEvent, HitMode, HitSource, HitTarget};
use ambition::actors::features::{SpawnActorKind, SpawnActorRequest};
use ambition::characters::actor::BodyCombat;
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

/// Damage a moving shell deals to another ENEMY it runs down — lethal, so a kicked
/// shell one-shots the trash mobs it mows through (snakes, AI Slop), like a Koopa
/// shell clearing a line. Routed through the shared hit pipeline, so the victim
/// dies with the same drops/score/reaction any other kill produces.
const SHELL_ENEMY_DAMAGE: i32 = 99;
/// Damage a moving shell deals to the PLAYER on a SIDE hit — an ordinary contact
/// tick (the shared i-frames keep it from re-hitting every frame it overlaps).
const SHELL_PLAYER_DAMAGE: i32 = 1;

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

/// The freeze-lock duration re-stamped onto a withdrawn snake each tick (the
/// engine's `recoil_lock_timer`, which hard-zeros movement input). Any value
/// comfortably above one frame works: it is refreshed every shelled tick and
/// cleared the instant the snake emerges, so it only has to outlast a tick's decay.
const SHELL_FREEZE_LOCK: f32 = 1.0;

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
            if inputs.stomped {
                // A stomp from ABOVE stops a running shell dead — it becomes a fresh
                // boxed shell you can re-kick — and bounces the stomper, exactly like
                // stomping a walker. A SIDE hit does NOT stop it: that is the shell
                // running you (or an enemy) down, which the ECS turns into a real hit
                // through the shared damage pipeline while the shell keeps going.
                ShellEffects {
                    just_squashed: true,
                    ..shelled(Boxed(BOXED_S), CharacterAnim::ShellIdle)
                }
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
/// which the shell state turns off (via its `body_contact_damage` tuning) while
/// withdrawn, then back on when it walks again.
/// The Solid Snake archetype ROW (no outer braces), so it can register on its own
/// for a single-enemy test OR fold into the combined Mary-O roster fragment — one
/// fragment per provider, since assembly rejects a second from the same provider.
pub(crate) const SNAKE_ROSTER_ROWS: &str = r#"
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
"#;

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
            &format!("{{{SNAKE_ROSTER_ROWS}}}"),
        )
        .expect("Mary-O snake roster fragment should be valid"),
    );
}

/// Tile x-columns (level grid) each snake paces near — the flats after the pit
/// rhythm, so the walker is a hazard on solid ground, not stranded over a pit.
/// Ground runs are now [0,20) [22,42) [45,60) [65,104) (pits [20,22) [42,45)
/// [60,65)) after the level was lengthened for the vault; these columns sit clear
/// of the pits and the surface exit pipe at column 45.
const SNAKE_TILE_COLUMNS: &[f32] = &[9.0, 16.0, 27.0, 52.0, 68.0];

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
/// via [`ActorAnimOverride`], freeze the (still-alive) body and turn its contact
/// threat off — or slide it — and let it walk and threaten again when it finishes
/// emerging.
///
/// **A moving shell is a kinetic hazard.** Each tick it is sliding, it broadcasts a
/// lethal hit over its own AABB through the SHARED damage pipeline ([`HitEvent`]),
/// so it kills every ENEMY it runs down (other snakes, AI Slop, anything) with the
/// same death/drops/reaction any other kill produces — no bespoke chain. The player
/// is DIRECTIONAL, exactly like Mario: a stomp from ABOVE stops the shell and bounces
/// you (handled by [`step_snake_shell`], safe); a SIDE hit hurts you (a real hit
/// emitted here). The enemy broadcast never touches the player — the shared drain's
/// actor query excludes the player — so the two are cleanly separate.
///
/// Ordered BEFORE the shared contact-damage pass so a stomp makes the snake inert
/// that frame, before that pass could hurt the stomper.
#[allow(clippy::too_many_arguments)]
pub fn run_snake_shells(
    mut commands: Commands,
    world_time: Res<ambition::time::WorldTime>,
    mut vfx: MessageWriter<ambition::vfx::VfxMessage>,
    mut sfx: ambition::sfx::SfxWriter,
    mut hits: MessageWriter<HitEvent>,
    mut players: Query<(Entity, &mut ae::BodyKinematics), With<PrimaryPlayer>>,
    mut snakes: Query<
        (
            Entity,
            &FeatureId,
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
    // Collect each sliding shell's (AABB, own-id, whether it is side-hitting the
    // player) so the shared-pipeline hits below can be emitted after the borrow.
    let mut sliding: Vec<(ae::Aabb, String, bool)> = Vec::new();
    for (entity, feature_id, mut kin, mut combat, mut config, mut shell) in &mut snakes {
        let inputs = shell_inputs_for(*shell, &kin, player_box);
        let fx = step_snake_shell(*shell, dt, inputs);

        // The stomp bounce is applied to the PLAYER (a fresh stomp on a walker OR a
        // stomp from above onto a moving shell — both stop the threat and bounce).
        if fx.just_squashed {
            if let Ok((_, mut player)) = players.single_mut() {
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

        if let SnakeShell::Sliding(_) = fx.phase {
            let g = kin.aabb();
            // A stomp from above turned the shell to Boxed (so it is not here); a
            // sliding shell that still overlaps the player is doing so from the SIDE.
            let side_hit = player_box.is_some_and(|(p, _, _)| {
                p.min.x < g.max.x && p.max.x > g.min.x && p.min.y < g.max.y && p.max.y > g.min.y
            });
            sliding.push((g, feature_id.as_str().to_string(), side_hit));
        }
        *shell = fx.phase;
    }

    // Second pass: every sliding shell deals its damage through the ONE shared hit
    // pipeline. The enemy broadcast (Volume) reaches every actor in the volume EXCEPT
    // the player (the drain's actor query is `Without<PlayerEntity>`) and EXCEPT the
    // shell itself (ignored by both disposition prefixes); the side-hit player event
    // is the only thing that can hurt the player.
    for (aabb, self_id, side_hit) in sliding {
        hits.write(HitEvent {
            strike_sfx: None,
            volume: aabb.into(),
            damage: SHELL_ENEMY_DAMAGE,
            source: HitSource::EnemyChargeCrash,
            attacker: None,
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
                    attacker: None,
                    target: HitTarget::Player(player_entity),
                    mode: HitMode::Knockback,
                    knockback: None,
                    ignored_targets: Vec::new(),
                });
            }
        }
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
