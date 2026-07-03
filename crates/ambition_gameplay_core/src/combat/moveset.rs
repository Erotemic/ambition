//! Data-driven move playback — the runtime half of the Smash model.
//!
//! An actor plays a [`MoveSpec`](ambition_entity_catalog::MoveSpec) by
//! carrying a [`MovePlayback`] component; [`advance_move_playback`] is the
//! ONE system that turns the authored timeline into simulation:
//!
//! - **Proper time.** The playback clock advances by
//!   `WorldTime::entity_dt(ProperTimeScale)` (ADR 0011) — the owning actor's
//!   own clock. A dilated actor's windows, volumes, events, and picture all
//!   slow together because they are one timeline (`MovePlayback::phase` is
//!   what presentation samples the bound clip by).
//! - **Windows → hitbox entities.** Each `Active` window's volumes become
//!   `(Hitbox, HitboxHits)` entities (`FollowOwner`, facing-mirrored,
//!   entity-local offsets) on window entry and despawn on window exit —
//!   window-scoped by the move's own clock, so no wall-time lifetime can
//!   drift from a dilated owner. Damage resolution is the existing
//!   [`apply_hitbox_damage`](super::hitbox::apply_hitbox_damage) path:
//!   moves need NO parallel hit plumbing.
//! - **Events → messages.** Timed events emit [`MoveEventMessage`]s;
//!   consumers (audio bridge, techniques/effects) subscribe downstream.
//!
//! Re-binding a move onto a different actor is inserting the same
//! `MovePlayback` on a different entity — zero per-actor Rust. That is the
//! decomposability contract, pinned by the tests below.

use bevy::prelude::{
    Commands, Component, Entity, Message, MessageReader, MessageWriter, Query, Res, Without,
};

use ambition_engine_core as ae;
use ambition_entity_catalog::{MoveEventKind, MoveSpec, MovesetContract, VolumeShape, WindowTag};
use ambition_time::ProperTimeScale;

use super::components::ActorFaction;
use super::hitbox::{Hitbox, HitboxAnchor, HitboxHits};
use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_characters::brain::{ActorActionMessage, ActorControl};
use ambition_sfx::{SfxId, SfxMessage};
use ambition_time::WorldTime;

/// A timed move event fired by [`advance_move_playback`]. The move runtime
/// stays content-free: it names the event; downstream consumers (the audio
/// bridge, content techniques via the `Effect` vocabulary) resolve keys.
#[derive(Message, Debug, Clone)]
pub struct MoveEventMessage {
    pub owner: Entity,
    pub move_id: String,
    pub kind: MoveEventKind,
}

/// This actor is playing a move. Insert to start; the system removes it when
/// the timeline completes. Facing locks at move start (the Smash convention —
/// a swing doesn't re-aim mid-animation).
#[derive(bevy::prelude::Component, Debug, Clone)]
pub struct MovePlayback {
    pub spec: MoveSpec,
    /// `+1.0` faces right, `-1.0` left; mirrors every volume's x offset.
    pub facing: f32,
    /// Seconds of the OWNER'S proper time since move start.
    pub t: f32,
    /// Live hitbox entity per entered-but-not-exited Active window index.
    live_boxes: Vec<(usize, Entity)>,
    /// Which timed events already fired (parallel to `spec.events`).
    fired: Vec<bool>,
}

impl MovePlayback {
    pub fn new(spec: MoveSpec, facing: f32) -> Self {
        let fired = vec![false; spec.events.len()];
        Self {
            spec,
            facing,
            t: 0.0,
            live_boxes: Vec::new(),
            fired,
        }
    }

    /// Normalized move progress — what presentation samples the bound clip
    /// by (the clip is SLAVED to the move; it never runs its own clock).
    pub fn phase(&self) -> f32 {
        self.spec.phase_at(self.t)
    }

    pub fn finished(&self) -> bool {
        self.t >= self.spec.duration_s
    }
}

/// A body's data-driven move repertoire — the Bevy-side carrier of a headless
/// [`MovesetContract`]. A body that exposes this contract triggers its moves
/// through [`trigger_moveset_moves`]: a control-frame verb edge inserts the
/// matching [`MovePlayback`]. This + [`dispatch_move_events`] are the production
/// seam the moveset system was missing (nothing ever created a `MovePlayback` in
/// the live game) — the first real consumer is the PCA's data-driven signature
/// move (fable review 2026-07-02 §A1, Path B: prove the moveset on a real actor
/// before folding the boss onto it). The boss fold reuses the SAME trigger +
/// dispatch — a boss is an actor whose repertoire happens to be large.
#[derive(Component, Debug, Clone)]
pub struct ActorMoveset(pub MovesetContract);

/// Advance every playing move by its owner's proper time; manage
/// window-scoped hitboxes; fire timed events; retire finished moves.
pub fn advance_move_playback(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    gravity: crate::physics::GravityCtx,
    mut events: MessageWriter<MoveEventMessage>,
    mut players: Query<(
        Entity,
        &mut MovePlayback,
        &ActorFaction,
        &ae::BodyKinematics,
        Option<&ProperTimeScale>,
    )>,
) {
    for (owner, mut playback, faction, kin, scale) in &mut players {
        // ADR 0011: entity dt collapses to sim dt when the actor carries no
        // ProperTimeScale — undilated actors are the identity case.
        let dt = world_time.entity_dt(scale.copied().unwrap_or_default());
        let t_prev = playback.t;
        playback.t = (t_prev + dt).min(playback.spec.duration_s);
        let t = playback.t;

        // Timed events crossing (t_prev, t] fire exactly once, in order.
        // Split-borrow locals keep the fired flags and the spec readable
        // side by side.
        let pb = &mut *playback;
        for (idx, ev) in pb.spec.events.iter().enumerate() {
            if !pb.fired[idx] && ev.at_s > t_prev && ev.at_s <= t {
                pb.fired[idx] = true;
                events.write(MoveEventMessage {
                    owner,
                    move_id: pb.spec.id.clone(),
                    kind: ev.kind.clone(),
                });
            }
        }

        // Active windows: spawn volumes on entry, despawn on exit. The box
        // lives exactly while the OWNER'S clock is inside the window, so
        // dilation stretches the box's world-time life automatically.
        for (w_idx, window) in pb.spec.windows.iter().enumerate() {
            if !matches!(window.tag, WindowTag::Active) || window.volumes.is_empty() {
                continue;
            }
            let inside = window.start_s <= t && t < window.end_s;
            let live_slot = pb.live_boxes.iter().position(|(idx, _)| *idx == w_idx);
            match (inside, live_slot) {
                (true, None) => {
                    // Authored volume offsets are BODY-LOCAL (side, down); rotate
                    // them through the owner's gravity frame at spawn — the same
                    // resolution `spawn_melee_strike` performs — so an authored
                    // above-the-head volume stays above the head under any
                    // gravity (fable review 2026-07-02 §B1: the unrotated form
                    // spawned it screen-up, into a sideways body's ceiling).
                    let frame_down = gravity.dir_at(kin.pos);
                    let body_frame = ae::AccelerationFrame::new(frame_down);
                    for volume in &window.volumes {
                        let (local, half_extent, shape) = match volume.shape {
                            VolumeShape::Rect {
                                offset,
                                half_extents,
                            } => (
                                ae::Vec2::new(offset.0 * pb.facing, offset.1),
                                ae::Vec2::new(half_extents.0, half_extents.1),
                                None,
                            ),
                            VolumeShape::Circle { offset, radius } => (
                                ae::Vec2::new(offset.0 * pb.facing, offset.1),
                                ae::Vec2::splat(radius),
                                Some(ae::VolumeShape::circle(radius)),
                            ),
                        };
                        let local_offset = body_frame.to_world(local);
                        // Axis-aligned extents rotate with the frame too (a
                        // circle's splat is rotation-invariant, so this is
                        // uniform).
                        let half_extent = body_frame.to_world_half(half_extent);
                        // NO HitboxLifetime on purpose: the window's exit
                        // edge (owner proper time) is the despawn authority,
                        // not a wall-clock countdown.
                        let hitbox = commands
                            .spawn((
                                Hitbox {
                                    owner,
                                    source: *faction,
                                    anchor: HitboxAnchor::FollowOwner { local_offset },
                                    half_extent,
                                    shape,
                                    facing: pb.facing,
                                    damage: volume.damage,
                                    knockback_strength: volume.knockback,
                                    knock_x: 0.0,
                                    frame_down,
                                },
                                HitboxHits::default(),
                            ))
                            .id();
                        pb.live_boxes.push((w_idx, hitbox));
                    }
                }
                (false, Some(_)) => {
                    pb.live_boxes.retain(|(idx, entity)| {
                        if *idx == w_idx {
                            commands.entity(*entity).despawn();
                            false
                        } else {
                            true
                        }
                    });
                }
                _ => {}
            }
        }

        if pb.finished() {
            for (_, entity) in pb.live_boxes.drain(..) {
                commands.entity(entity).despawn();
            }
            commands.entity(owner).remove::<MovePlayback>();
        }
    }
}

/// TRIGGER a body's data-driven move from its control-frame verb edges: a
/// `special_pressed` → the `"special"` verb, a `melee_pressed` → the `"attack"`
/// verb. A body already playing a move (`With<MovePlayback>`) is excluded, so a
/// move plays to completion before another starts — the move's own duration IS the
/// fire-rate gate (the same shape as the ranged refire floor). Facing locks at
/// trigger from the body's kinematics (the Smash convention — a committed swing
/// doesn't re-aim).
///
/// ONE trigger seam for every body (guardrail #1): the same system drives an
/// actor's melee, the PCA's signature move, and a folded boss's pattern. It is the
/// production insert the moveset runtime was missing.
pub fn trigger_moveset_moves(
    mut commands: Commands,
    bodies: Query<(Entity, &ActorMoveset, &ActorControl, &ae::BodyKinematics), Without<MovePlayback>>,
) {
    for (entity, moveset, control, kin) in &bodies {
        let frame = &control.0;
        let verb = if frame.special_pressed {
            "special"
        } else if frame.melee_pressed {
            "attack"
        } else {
            continue;
        };
        if let Some(spec) = moveset.0.move_for_verb(verb) {
            commands
                .entity(entity)
                .insert(MovePlayback::new(spec.clone(), kin.facing));
        }
    }
}

/// Consume [`MoveEventMessage`]s — the moveset runtime is content-free, it only
/// NAMES events; this resolves them:
/// - `Sfx { cue }` → play the cue at the owner's position.
/// - `Effect { key }` → BRIDGE to the existing content-technique seam by writing
///   the SAME `ActorActionMessage::Special { Special(key) }` the brain special path
///   emits, so every content `Technique` consumer fires unchanged. This is the
///   exact seam the boss's `Special(key)` profiles reuse once the boss folds onto
///   the moveset — a data-driven move fires a content technique with zero new
///   plumbing (fable review §A1, Path B).
pub fn dispatch_move_events(
    mut events: MessageReader<MoveEventMessage>,
    positions: Query<&ae::BodyKinematics>,
    mut sfx: MessageWriter<SfxMessage>,
    mut actions: MessageWriter<ActorActionMessage>,
) {
    for ev in events.read() {
        match &ev.kind {
            MoveEventKind::Sfx { cue } => {
                let pos = positions.get(ev.owner).map(|k| k.pos).unwrap_or(ae::Vec2::ZERO);
                sfx.write(SfxMessage::Play {
                    id: SfxId::new(cue),
                    pos,
                });
            }
            MoveEventKind::Effect { key } => {
                actions.write(ActorActionMessage {
                    actor: ev.owner,
                    request: ActionRequest::Special {
                        spec: SpecialActionSpec::Special(key.clone()),
                    },
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sfx::SfxMessage;
    use crate::combat::events::HitEvent;
    use crate::combat::hitbox::apply_hitbox_damage;
    use crate::world::physics::DebrisBurstMessage;
    use ambition_vfx::vfx::VfxMessage;
    use bevy::prelude::*;

    /// The seed move: SwipeSpec-as-data (0.28 windup / 0.08 active with one
    /// forward rect volume / recovery), one timed Sfx event on the swing.
    fn swat() -> MoveSpec {
        let doc = ambition_entity_catalog::EntityCatalogDoc::parse(
            r#"(
                schema_version: 1,
                entities: [(
                    id: "seed",
                    contracts: (moveset: Some((
                        verbs: {"attack": "swat"},
                        moves: [(
                            id: "swat",
                            clip: (clip: "slash", fallbacks: ["idle"]),
                            duration_s: 0.68,
                            windows: [
                                (start_s: 0.0, end_s: 0.28, tag: Startup, volumes: []),
                                (start_s: 0.28, end_s: 0.36, tag: Active, volumes: [
                                    (shape: Rect(offset: (28.0, 0.0), half_extents: (16.0, 12.0)),
                                     damage: 2, knockback: 40.0),
                                ]),
                                (start_s: 0.36, end_s: 0.68, tag: Recovery, volumes: []),
                            ],
                            events: [(at_s: 0.28, kind: Sfx(cue: "swing_light"))],
                        )],
                    ))),
                )],
            )"#,
        )
        .unwrap();
        assert!(doc.validate().is_empty());
        doc.entity("seed")
            .unwrap()
            .contracts
            .moveset
            .as_ref()
            .unwrap()
            .move_for_verb("attack")
            .unwrap()
            .clone()
    }

    /// The same seed move as a full repertoire, reachable by the `"special"` AND
    /// `"attack"` verbs — the shape a body carries in an `ActorMoveset`.
    fn swat_moveset() -> MovesetContract {
        MovesetContract {
            verbs: [
                ("special".to_string(), "swat".to_string()),
                ("attack".to_string(), "swat".to_string()),
            ]
            .into_iter()
            .collect(),
            moves: vec![swat()],
        }
    }

    #[derive(Resource, Default)]
    struct Captured {
        hits: Vec<HitEvent>,
        events: Vec<MoveEventMessage>,
    }

    fn capture(
        mut cap: ResMut<Captured>,
        mut hits: MessageReader<HitEvent>,
        mut evs: MessageReader<MoveEventMessage>,
    ) {
        cap.hits.extend(hits.read().cloned());
        cap.events.extend(evs.read().cloned());
    }

    /// Headless sim harness: move playback + the REAL hitbox damage path,
    /// fixed 16ms sim ticks, a vulnerable player standing in reach.
    fn app_with_victim() -> (App, Entity) {
        let mut app = App::new();
        app.add_message::<HitEvent>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_message::<MoveEventMessage>();
        app.init_resource::<Captured>();
        app.init_resource::<WorldTime>();
        app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
        app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
        app.add_systems(
            Update,
            (advance_move_playback, apply_hitbox_damage, capture).chain(),
        );
        let victim = app
            .world_mut()
            .spawn((
                crate::actor::PlayerEntity,
                ActorFaction::Player,
                crate::actor::BodyKinematics {
                    pos: ae::Vec2::new(128.0, 100.0),
                    size: ae::Vec2::new(28.0, 46.0),
                    facing: -1.0,
                    ..Default::default()
                },
                // The published combat footprint every body carries (§A6).
                ae::CenteredAabb::from_center_size(
                    ae::Vec2::new(128.0, 100.0),
                    ae::Vec2::new(28.0, 46.0),
                ),
                crate::actor::BodyOffense::default(),
                crate::actor::BodyDodgeState::default(),
                crate::actor::BodyShieldState::default(),
                ambition_characters::actor::BodyCombat::default(),
            ))
            .id();
        (app, victim)
    }

    fn spawn_attacker(app: &mut App, pos: ae::Vec2, body: ae::Vec2, spec: MoveSpec) -> Entity {
        app.world_mut()
            .spawn((
                crate::features::CenteredAabb::new(pos, body),
                // The playback system resolves the owner's gravity frame from
                // its authoritative kinematics, like every real actor carries.
                ae::BodyKinematics {
                    pos,
                    vel: ae::Vec2::ZERO,
                    size: body,
                    facing: 1.0,
                },
                ActorFaction::Enemy,
                MovePlayback::new(spec, 1.0),
            ))
            .id()
    }

    fn run_seconds(app: &mut App, seconds: f32) {
        let steps = (seconds / 0.016).ceil() as usize;
        for _ in 0..steps {
            app.update();
        }
    }

    /// W9 core: the authored timeline drives the REAL damage path. No hit
    /// during startup; the active window spawns the volume and the standing
    /// victim takes the authored damage; the window's exit despawns the box;
    /// move completion removes the component. The timed event fires once.
    #[test]
    fn data_driven_move_lands_a_hit_through_the_real_path() {
        let (mut app, _victim) = app_with_victim();
        let attacker = spawn_attacker(
            &mut app,
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(15.0, 24.0),
            swat(),
        );

        // Startup: nothing live, nothing hit, no event yet.
        run_seconds(&mut app, 0.20);
        {
            let cap = app.world().resource::<Captured>();
            assert!(cap.hits.is_empty(), "no hit during startup");
            assert!(cap.events.is_empty(), "no event during startup");
        }
        assert_eq!(count_hitboxes(&mut app), 0);

        // Cross into the active window: volume live, hit lands, event fired.
        run_seconds(&mut app, 0.12);
        assert_eq!(count_hitboxes(&mut app), 1, "active window volume is live");
        {
            let cap = app.world().resource::<Captured>();
            assert_eq!(cap.hits.len(), 1, "the swat landed exactly once");
            assert_eq!(cap.events.len(), 1, "swing event fired exactly once");
            assert!(matches!(
                &cap.events[0].kind,
                MoveEventKind::Sfx { cue } if cue == "swing_light"
            ));
        }

        // Past the window: box despawned. Past the move: component removed.
        run_seconds(&mut app, 0.1);
        assert_eq!(count_hitboxes(&mut app), 0, "window exit despawns the box");
        run_seconds(&mut app, 0.3);
        assert!(
            app.world().get::<MovePlayback>(attacker).is_none(),
            "finished move retires its playback"
        );
        let cap = app.world().resource::<Captured>();
        assert_eq!(cap.hits.len(), 1, "no double hit across the whole move");
    }

    /// W9 decomposability proof: the SAME MoveSpec value bound to a second,
    /// differently-shaped actor lands the same hit — re-binding is data.
    #[test]
    fn rebinding_the_same_move_to_another_actor_is_data_only() {
        let (mut app, _victim) = app_with_victim();
        // A "goblin": different body, different position, same move data.
        spawn_attacker(
            &mut app,
            ae::Vec2::new(156.0, 100.0), // attacks leftward…
            ae::Vec2::new(12.0, 18.0),
            swat(),
        );
        // …so flip its facing to reach the victim at x=128.
        let goblin = app
            .world_mut()
            .query_filtered::<Entity, With<MovePlayback>>()
            .iter(app.world())
            .next()
            .unwrap();
        app.world_mut()
            .get_mut::<MovePlayback>(goblin)
            .unwrap()
            .facing = -1.0;

        run_seconds(&mut app, 0.40);
        let cap = app.world().resource::<Captured>();
        assert_eq!(
            cap.hits.len(),
            1,
            "the goblin lands the player-authored move with zero Rust changes"
        );
    }

    /// W9 relativity proof: a 0.25x-dilated attacker's move — windows AND
    /// picture — runs at quarter speed. After 0.32s of world time the
    /// undilated attacker has already hit; the dilated one is still in
    /// startup with a proportionally smaller phase. Its hit arrives ~4x
    /// later, and the volume's world-time life stretches with it.
    #[test]
    fn dilated_owner_slows_windows_and_picture_together() {
        let (mut app, _victim) = app_with_victim();
        let dilated = spawn_attacker(
            &mut app,
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(15.0, 24.0),
            swat(),
        );
        app.world_mut()
            .entity_mut(dilated)
            .insert(ProperTimeScale(0.25));

        run_seconds(&mut app, 0.32);
        {
            let cap = app.world().resource::<Captured>();
            assert!(cap.hits.is_empty(), "dilated attacker is still winding up");
            let playback = app.world().get::<MovePlayback>(dilated).unwrap();
            // ~0.32s world → ~0.08s proper → phase ~0.12, picture in startup.
            assert!(
                playback.phase() < 0.28 / 0.68,
                "picture is slaved to the slow clock"
            );
        }

        // Four times the world time reaches the same proper-time window.
        run_seconds(&mut app, 1.0);
        let cap = app.world().resource::<Captured>();
        assert_eq!(cap.hits.len(), 1, "the dilated swat lands, just later");
    }

    fn count_hitboxes(app: &mut App) -> usize {
        app.world_mut().query::<&Hitbox>().iter(app.world()).count()
    }

    /// A two-window (two Active spans) move authored as data — a light poke into a
    /// heavier follow-up, the shape of the player-robot's "Theorem Chain".
    fn two_hit_combo() -> MoveSpec {
        let doc = ambition_entity_catalog::EntityCatalogDoc::parse(
            r#"(
                schema_version: 1,
                entities: [(
                    id: "combo",
                    contracts: (moveset: Some((
                        verbs: {"special": "chain"},
                        moves: [(
                            id: "chain",
                            clip: (clip: "slash", fallbacks: ["idle"]),
                            duration_s: 0.72,
                            windows: [
                                (start_s: 0.0, end_s: 0.14, tag: Startup, volumes: []),
                                (start_s: 0.14, end_s: 0.22, tag: Active, volumes: [
                                    (shape: Rect(offset: (28.0, 0.0), half_extents: (18.0, 14.0)),
                                     damage: 2, knockback: 90.0),
                                ]),
                                (start_s: 0.22, end_s: 0.36, tag: Recovery, volumes: []),
                                (start_s: 0.36, end_s: 0.46, tag: Active, volumes: [
                                    (shape: Rect(offset: (30.0, 0.0), half_extents: (20.0, 16.0)),
                                     damage: 3, knockback: 160.0),
                                ]),
                                (start_s: 0.46, end_s: 0.72, tag: Recovery, volumes: []),
                            ],
                        )],
                    ))),
                )],
            )"#,
        )
        .unwrap();
        assert!(doc.validate().is_empty(), "the two-hit combo is well-formed");
        doc.entity("combo")
            .unwrap()
            .contracts
            .moveset
            .as_ref()
            .unwrap()
            .move_for_verb("special")
            .unwrap()
            .clone()
    }

    /// Smash-like MULTI-HIT expressivity (fable review §A1): a single authored move
    /// with TWO Active windows lands TWO distinct hits on a standing victim — the
    /// first window's box despawns before the second spawns, and each carries its
    /// own `HitboxHits`, so the combo reads as two strikes, not one lingering box.
    /// Pins that the moveset runtime expresses combos, not just single swings.
    #[test]
    fn a_two_window_move_lands_two_distinct_hits() {
        let (mut app, _victim) = app_with_victim();
        let _attacker = spawn_attacker(
            &mut app,
            ae::Vec2::new(104.0, 100.0),
            ae::Vec2::new(15.0, 24.0),
            two_hit_combo(),
        );
        // Run the whole move. Two Active windows → two hits.
        run_seconds(&mut app, 0.75);
        let cap = app.world().resource::<Captured>();
        assert_eq!(
            cap.hits.len(),
            2,
            "the two-window combo lands exactly two distinct hits"
        );
        assert_eq!(cap.hits[0].damage, 2, "first window's authored damage");
        assert_eq!(cap.hits[1].damage, 3, "second window's authored damage");
    }

    /// Phase-0 keystone (fable review §A1, Path B): the PRODUCTION trigger — a body
    /// carrying an `ActorMoveset` whose control frame presses `special` starts the
    /// matching move (no test hand-inserts `MovePlayback`), and the move lands its
    /// authored hit through the real path. This is the insert the moveset runtime
    /// was missing; without it the whole system was dead in the shipping game.
    #[test]
    fn a_control_verb_edge_triggers_the_moveset_move_and_lands_it() {
        // Self-contained app: the full production chain registered ONCE
        // (trigger → advance → damage → capture) + a victim in reach.
        let mut app = App::new();
        app.add_message::<HitEvent>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<DebrisBurstMessage>();
        app.add_message::<MoveEventMessage>();
        app.init_resource::<Captured>();
        app.init_resource::<WorldTime>();
        app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
        app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
        app.add_systems(
            Update,
            (
                trigger_moveset_moves,
                advance_move_playback,
                apply_hitbox_damage,
                capture,
            )
                .chain(),
        );
        app.world_mut().spawn((
            crate::actor::PlayerEntity,
            ActorFaction::Player,
            crate::actor::BodyKinematics {
                pos: ae::Vec2::new(128.0, 100.0),
                size: ae::Vec2::new(28.0, 46.0),
                facing: -1.0,
                ..Default::default()
            },
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(128.0, 100.0),
                ae::Vec2::new(28.0, 46.0),
            ),
            crate::actor::BodyOffense::default(),
            crate::actor::BodyDodgeState::default(),
            crate::actor::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
        ));
        // A body that OWNS a repertoire and is pressing `special` this frame — but
        // is NOT hand-given a MovePlayback. The trigger must start the move.
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        frame.special_pressed = true;
        app.world_mut().spawn((
            crate::features::CenteredAabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(15.0, 24.0)),
            ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(15.0, 24.0),
                facing: 1.0,
            },
            ActorFaction::Enemy,
            ActorMoveset(swat_moveset()),
            ActorControl(frame),
        ));

        // Through one move: the verb edge started it, the active window landed the
        // authored hit exactly once (0.68s move; stop before it can re-trigger).
        run_seconds(&mut app, 0.5);
        let cap = app.world().resource::<Captured>();
        assert_eq!(
            cap.hits.len(),
            1,
            "the special verb edge triggered the move and it landed its hit"
        );
        assert_eq!(cap.events.len(), 1, "the move's timed Sfx event fired once");
    }

    /// Phase-0 keystone: the EFFECT dispatch — the moveset runtime only NAMES
    /// events; `dispatch_move_events` resolves an `Sfx{cue}` to a positioned
    /// `SfxMessage` and BRIDGES an `Effect{key}` to the SAME
    /// `ActorActionMessage::Special{Special(key)}` the brain special path emits, so
    /// a data-driven move fires a content technique with zero new plumbing (the
    /// exact seam the boss `Special(key)` profiles reuse).
    #[test]
    fn move_event_dispatch_bridges_sfx_to_sound_and_effect_to_special() {
        use ambition_characters::brain::ActorActionMessage;
        let mut app = App::new();
        app.add_message::<MoveEventMessage>();
        app.add_message::<SfxMessage>();
        app.add_message::<ActorActionMessage>();
        app.add_systems(Update, dispatch_move_events);
        let owner = app
            .world_mut()
            .spawn(ae::BodyKinematics {
                pos: ae::Vec2::new(42.0, 7.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(16.0, 24.0),
                facing: 1.0,
            })
            .id();
        app.world_mut()
            .resource_mut::<Messages<MoveEventMessage>>()
            .write(MoveEventMessage {
                owner,
                move_id: "sig".into(),
                kind: MoveEventKind::Sfx {
                    cue: "pca.signature".into(),
                },
            });
        app.world_mut()
            .resource_mut::<Messages<MoveEventMessage>>()
            .write(MoveEventMessage {
                owner,
                move_id: "sig".into(),
                kind: MoveEventKind::Effect {
                    key: "pca_glider".into(),
                },
            });
        app.update();

        let sfx: Vec<SfxMessage> = app
            .world_mut()
            .resource_mut::<Messages<SfxMessage>>()
            .drain()
            .collect();
        assert_eq!(sfx.len(), 1, "the Sfx event played one sound");
        assert!(
            matches!(sfx[0], SfxMessage::Play { pos, .. } if pos == ae::Vec2::new(42.0, 7.0)),
            "played at the owner's position"
        );
        let acts: Vec<ActorActionMessage> = app
            .world_mut()
            .resource_mut::<Messages<ActorActionMessage>>()
            .drain()
            .collect();
        assert_eq!(acts.len(), 1, "the Effect event bridged to one Special action");
        assert_eq!(acts[0].actor, owner);
        assert!(matches!(
            &acts[0].request,
            ActionRequest::Special { spec: SpecialActionSpec::Special(k) } if k == "pca_glider"
        ));
    }
}
