//! GNU-ton's conductor: the scholar's live move, performed by the fists.
//!
//! The boss brain decides WHAT he does (the scripted pattern in
//! `boss_profiles.ron`, every move a `Special` key). This module decides what
//! that looks like: it reads the scholar's [`BossAttackState`], latches the
//! facts a beat needs when it begins, asks [`super::choreography`] where each
//! fist is, and writes the result — pose, hit volume, telegraph ink, sound.
//!
//! Three more things the fight needs that no generic system knows:
//!
//! * **The giant's back is ground.** A one-way platform rides the gnu's back,
//!   so the way to reach the scholar is literally to stand on the shoulders of a
//!   giant. When he `buck`s, it throws you off.
//! * **The fists are his.** A fist is only hittable while it is stuck in the
//!   floor (or limp), and a blow to it lands on him.
//! * **Eureka.** After the apple rain, a golden apple finds his head; he tumbles
//!   off the gnu and sits dazed on the floor, fists limp, before climbing back.
//!
//! Ordering: after `RidersSyncedToMounts` in `AfterIntegrate`, so the saddle
//! pin has already put him on the gnu this tick and a fist pose written here is
//! the last word before combat reads it.

use ambition_platformer2d_core as ae;
use ae::Vec2;
use bevy::prelude::*;

use ambition_boss_encounter::{BossConfig, BossEncounter};
use ambition_characters::actor::{BodyCombat, BodyHealth, Invulnerability, Limb, LimbRig, LimbSlot};
use ambition_characters::brain::{BossAttackProfile, BossAttackState};
use ambition_combat::strike::{Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback, HitboxLifetime};
use ambition_mount::{MountSlot, RidingOn};
use ambition_platformer2d::sfx::{BodySfxWriter, SfxId, SfxMessage};
use ambition_platformer2d::vfx::{ParticleKind, VfxMessage};
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly;
use ambition_vfx::HitSide;

use super::choreography::{self as ch, Cue, Fist, Hall, Latch, Move, Pose, Stage};

/// The boss this conducts.
pub const GNU_TON_ID: &str = "gnu_ton_rider";

/// Where each fist rests, from the giant's centre, in its facing-right frame:
/// one beside the rump, one before the face.
const HOME: Vec2 = Vec2::new(300.0, -40.0);
/// A falling or swinging fist.
const FIST_DAMAGE: i32 = 2;
const FIST_KNOCKBACK: f32 = 1.4;
/// The hit volume is a little smaller than the drawn fist: a graze is not a hit.
const FIST_HIT_SCALE: f32 = 0.84;
/// A long move re-arms its hit this often, so it can land again once the
/// player's mercy frames are over.
const REARM: f32 = 0.9;
/// The buck: how hard the gnu hops, how long its back throws, and how hard.
const BUCK_HOP: f32 = 520.0;
const BUCK_WINDOW: f32 = 0.16;
const BUCK_THROW: Vec2 = Vec2::new(0.0, -980.0);
/// The stomp's shock: a floor wave each way.
const WAVE_SPEED: f32 = 540.0;
const WAVE_HALF: Vec2 = Vec2::new(26.0, 20.0);
const WAVE_DAMAGE: i32 = 2;
const WAVE_KNOCKBACK: f32 = 1.3;
/// The gnu's back, in its sprite's pixels about the body box centre, facing
/// right (the sheet's body box is 338×319 at 1.3 world units per pixel).
const BACK_SPAN_PX: (f32, f32) = (-160.0, 130.0);
const BACK_TOP_PX: f32 = -7.5;
const GNU_WORLD_PER_PIXEL: f32 = 1.3;
const BACK_THICKNESS: f32 = 14.0;

const INK: [f32; 4] = [1.0, 0.86, 0.45, 0.95];
const WARNING: [f32; 4] = [1.0, 0.42, 0.25, 1.0];
const GOLD: [f32; 4] = [1.0, 0.84, 0.2, 1.0];

const SFX_SNORT: &str = "boss.gnu_ton.snort";

/// Eureka, start to finish: the apple, the tumble, the daze, the climb. The
/// pattern's Rest after the apple rain is authored at least this long.
pub const EUREKA_LEN: f32 = ch::eureka::APPLE + ch::eureka::TUMBLE + ch::eureka::DAZED + 0.4;
const SFX_STOMP: &str = "boss.gnu_ton.stomp";

/// The part of a beat being performed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Part {
    mv: Move,
    striking: bool,
    t: f32,
    tel_dur: f32,
    beat_t: f32,
}

/// One shock travelling along the floor.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Wave {
    hitbox: Entity,
    x: f32,
    dir: f32,
}

/// The conductor's memory, on the scholar.
#[derive(Component, Clone, Debug)]
pub struct GnuTonConductor {
    hall: Option<Hall>,
    part: Option<Part>,
    /// Last tick's time-remaining in the live part: a part that restarts
    /// (the same move twice in a row) shows up as this jumping back up.
    last_remaining: f32,
    latch: Latch,
    /// Each fist's own aim (a pair's follower keeps tracking after the lead).
    aims: [Vec2; 2],
    next_lead: Fist,
    clock: f32,
    ticks: u32,
    /// Seconds since the fists last had a beat: they ease home over it.
    resting: f32,
    last: [Option<Pose>; 2],
    hitboxes: [Option<Entity>; 2],
    armed_at: [f32; 2],
    waves: [Option<Wave>; 4],
    landing: Vec2,
}

impl GnuTonConductor {
    /// A conductor for fists standing at `fists` (left, right).
    ///
    /// ⛔ THERE IS NO `Default`, AND THAT IS THE FIX. The conductor eases each
    /// fist from `latch.from`, and a defaulted latch said the fists stood at the
    /// world origin. They were built in place, then the first idle beat dragged
    /// them to the room's top-left corner and eased them back over 0.6 s (Jon:
    /// "the hands start in the upper left corner, and then very quickly move
    /// into the correct location"). Where the fists are is a fact about the
    /// world, so the only constructor asks for it.
    pub fn new(fists: [Vec2; 2]) -> Self {
        Self {
            hall: None,
            part: None,
            last_remaining: 0.0,
            latch: Latch {
                aim: (fists[0] + fists[1]) * 0.5,
                lead: None,
                from: fists,
                curve_phase: 0.0,
            },
            aims: fists,
            next_lead: Fist::Left,
            clock: 0.0,
            ticks: 0,
            resting: 0.0,
            last: [None; 2],
            hitboxes: [None; 2],
            armed_at: [0.0; 2],
            waves: [None; 4],
            landing: Vec2::ZERO,
        }
    }

    /// The move being performed and whether it is striking, for tests and
    /// inspectors.
    pub fn performing(&self) -> Option<(Move, bool)> {
        self.part.map(|part| (part.mv, part.striking))
    }

    pub fn hall(&self) -> Option<Hall> {
        self.hall
    }
}

impl bevy::ecs::entity::MapEntities for GnuTonConductor {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        for hitbox in self.hitboxes.iter_mut().flatten() {
            *hitbox = mapper.get_mapped(*hitbox);
        }
        for wave in self.waves.iter_mut().flatten() {
            wave.hitbox = mapper.get_mapped(wave.hitbox);
        }
    }
}

/// The checksum projection: every value but the allocator-local handles.
impl ambition_platformer2d_core::snapshot::SnapshotCursor for GnuTonConductor {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        use ambition_platformer2d_core::snapshot::{put_bool, put_f32, put_u32, put_vec2};
        put_f32(out, self.clock);
        put_u32(out, self.ticks);
        put_f32(out, self.resting);
        match self.part {
            Some(part) => {
                put_u32(out, Move::ALL.iter().position(|mv| *mv == part.mv).unwrap_or(0) as u32 + 1);
                put_bool(out, part.striking);
                put_f32(out, part.t);
                put_f32(out, part.beat_t);
            }
            None => put_u32(out, 0),
        }
        put_bool(out, self.next_lead == Fist::Left);
        put_vec2(out, self.latch.aim);
        put_vec2(out, self.aims[0]);
        put_vec2(out, self.aims[1]);
        for wave in &self.waves {
            put_f32(out, wave.map_or(f32::NAN, |wave| wave.x));
        }
    }
}

pub fn is_gnu_ton(config: &BossConfig) -> bool {
    config.behavior.id == GNU_TON_ID
}

/// Take the pair into the fight: the conductor on the scholar, the fists made
/// his (posed by him, hittable, their deaths his to rule), the giant made
/// untouchable. Idempotent, so it re-runs harmlessly after a rollback.
pub fn adopt_gnu_ton(
    mut commands: Commands,
    scholars: Query<(Entity, &BossConfig, &RidingOn), Without<GnuTonConductor>>,
    giants: Query<(&LimbRig, &ae::BodyKinematics), With<MountSlot>>,
    fists: Query<(), (With<Limb>, Without<ae::PoseOwnedExternally>)>,
    fist_bodies: Query<&ae::BodyKinematics, With<Limb>>,
) {
    for (scholar, config, riding) in &scholars {
        if !is_gnu_ton(config) {
            continue;
        }
        // Not until the giant is here: the conductor starts from where its
        // fists stand, and a fist that is gone starts at its home.
        let Ok((rig, giant)) = giants.get(riding.mount) else {
            continue;
        };
        let home = homes(giant);
        let at = [LimbSlot::HAND_LEFT, LimbSlot::HAND_RIGHT].map(|slot| rig.get(slot).and_then(|fist| fist_bodies.get(fist).ok()));
        let fists_at = [0, 1].map(|i| at[i].map_or(home[i], |kin| kin.pos));
        commands.entity(scholar).insert(GnuTonConductor::new(fists_at));
        for fist in [LimbSlot::HAND_LEFT, LimbSlot::HAND_RIGHT].into_iter().filter_map(|slot| rig.get(slot)) {
            if fists.contains(fist) {
                commands.entity(fist).insert((
                    // His hands: a fist's blow is the boss's, so it never lands
                    // on him (the relational rule every resolver asks).
                    ambition_combat::components::ActorFaction::Boss,
                    ae::PoseOwnedExternally,
                    ambition_combat::components::ActiveCombatant,
                    ambition_combat::components::RulesetOwnsDeath,
                ));
            }
        }
    }
}

/// Read the hall off the room: the floor under the giant and the walls either
/// side of it.
pub fn measure_hall(world: &ae::World, from: Vec2) -> Option<Hall> {
    let solid = |block: &ae::Block| matches!(block.kind, ae::BlockKind::Solid);
    let probe = ae::Aabb::new(from, Vec2::splat(2.0));
    let floor = world.first_body_sweep(probe, Vec2::new(0.0, 4000.0), solid)?.block.aabb.min.y;
    let row = ae::Aabb::new(Vec2::new(from.x, floor - 24.0), Vec2::splat(2.0));
    let left = world
        .first_body_sweep(row, Vec2::new(-8000.0, 0.0), solid)
        .map_or(0.0, |hit| hit.block.aabb.max.x);
    let right = world
        .first_body_sweep(row, Vec2::new(8000.0, 0.0), solid)
        .map_or(world.size.x, |hit| hit.block.aabb.min.x);
    Some(Hall { floor, left, right })
}

/// The live part of the scholar's move: its `Special` key, whether it is
/// striking, and the time left in it.
fn live_part(attack: &BossAttackState) -> Option<(Move, bool, f32)> {
    let special = |profile: &Option<BossAttackProfile>| match profile {
        Some(BossAttackProfile::Special(key)) => Move::from_key(key),
        _ => None,
    };
    if let Some(mv) = special(&attack.active_profile) {
        return Some((mv, true, attack.active_remaining));
    }
    special(&attack.telegraph_profile).map(|mv| (mv, false, attack.telegraph_remaining))
}

/// The gnu's back, as the one-way ground the player stands on.
pub fn back_platform(giant: &ae::BodyKinematics) -> ae::Aabb {
    let facing = if giant.facing < 0.0 { -1.0 } else { 1.0 };
    let (a, b) = (
        giant.pos.x + facing * BACK_SPAN_PX.0 * GNU_WORLD_PER_PIXEL,
        giant.pos.x + facing * BACK_SPAN_PX.1 * GNU_WORLD_PER_PIXEL,
    );
    let top = giant.pos.y + BACK_TOP_PX * GNU_WORLD_PER_PIXEL;
    ae::aabb_from_min_size(Vec2::new(a.min(b), top), Vec2::new((a - b).abs(), BACK_THICKNESS))
}

fn homes(giant: &ae::BodyKinematics) -> [Vec2; 2] {
    let facing = if giant.facing < 0.0 { -1.0 } else { 1.0 };
    let a = giant.pos + Vec2::new(-HOME.x * facing, HOME.y);
    let b = giant.pos + Vec2::new(HOME.x * facing, HOME.y);
    if a.x <= b.x { [a, b] } else { [b, a] }
}

fn spark(vfx: &mut MessageWriter<VfxMessage>, pos: Vec2, color: [f32; 4], count: u32, speed: f32) {
    vfx.write(VfxMessage::Burst {
        pos,
        count,
        speed,
        color,
        kind: ParticleKind::Spark,
    });
}

fn play(sfx: &mut BodySfxWriter, body: Entity, id: &'static str, pos: Vec2) {
    sfx.write_for(
        body,
        SfxMessage::Play {
            id: SfxId::from_static(id),
            pos,
        },
    );
}

/// A fist's hit volume, riding the fist.
fn fist_hitbox(owner: Entity, size: Vec2) -> impl Bundle {
    (
        Hitbox {
            strike_sfx: None,
            owner,
            source: HitSide::Boss,
            anchor: HitboxAnchor::FollowOwner { local_offset: Vec2::ZERO },
            half_extent: size * 0.5 * FIST_HIT_SCALE,
            shape: None,
            facing: 1.0,
            damage: FIST_DAMAGE,
            knockback: HitboxKnockback::FeelScale(FIST_KNOCKBACK),
            launch_dir: None,
            frame_down: Vec2::new(0.0, 1.0),
            reaction: None,
        },
        HitboxLifetime { remaining_s: REARM + 0.5 },
        HitboxHits::default(),
    )
}

/// Perform one tick of the scholar's live move with the fists (and, for the
/// gnu's own moves, the gnu).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn conduct_gnu_ton(
    mut commands: Commands,
    time: Res<ambition_time::WorldTime>,
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    mut scholars: Query<
        (
            Entity,
            &BossAttackState,
            &RidingOn,
            &mut GnuTonConductor,
            &mut ae::BodyKinematics,
            &mut ae::CenteredAabb,
            &mut BodyHealth,
            &mut BodyCombat,
            &mut BossEncounter,
            Option<&mut ae::SweepSample>,
        ),
        (With<BossConfig>, Without<MountSlot>, Without<Limb>),
    >,
    mut giants: Query<
        (&LimbRig, &mut ae::BodyKinematics, &mut ae::BodyFlightState, &mut BodyHealth),
        (With<MountSlot>, Without<Limb>, Without<BossConfig>),
    >,
    mut fists: Query<
        (
            &mut ae::BodyKinematics,
            &mut ae::CenteredAabb,
            &mut BodyHealth,
            &mut ae::ActorSurfaceState,
            &mut ae::BodyGroundState,
            Option<&mut ae::SweepSample>,
        ),
        (With<Limb>, Without<MountSlot>, Without<BossConfig>),
    >,
    players: Query<
        &ae::BodyKinematics,
        (PrimaryPlayerOnly, Without<Limb>, Without<MountSlot>, Without<BossConfig>),
    >,
    mut hitboxes: Query<&mut Hitbox>,
    mut vfx: MessageWriter<VfxMessage>,
    mut sfx: BodySfxWriter,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let player = players.iter().next().map(|kin| kin.pos);
    for (
        scholar,
        attack,
        riding,
        mut conductor,
        mut scholar_kin,
        mut scholar_aabb,
        mut scholar_health,
        mut scholar_combat,
        mut encounter,
        mut scholar_sweep,
    ) in &mut scholars
    {
        let Ok((rig, giant_kin, mut giant_flight, mut giant_health)) = giants.get_mut(riding.mount) else {
            continue;
        };
        // The gnu is scenery you can stand on, not a target.
        giant_health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
        let fist_entities = [rig.get(LimbSlot::HAND_LEFT), rig.get(LimbSlot::HAND_RIGHT)];
        // Measured every tick, not once: the pair can be carried into another
        // room (a possessed rider pilots the gnu through a door), and fists posed
        // against the last room's floor fall out of this one.
        let Some(hall) = measure_hall(&world.0, giant_kin.pos) else {
            continue;
        };
        conductor.hall = Some(hall);
        if !scholar_health.alive() {
            // Defeated: the fists fall where they are and stay there.
            for (index, entity) in fist_entities.iter().enumerate() {
                if let Some(hitbox) = conductor.hitboxes[index].take() {
                    commands.entity(hitbox).try_despawn();
                }
                let Some(Ok((mut kin, mut aabb, _, mut surface, _, mut sweep))) = entity.map(|e| fists.get_mut(e)) else {
                    continue;
                };
                surface.gravity_scale = 1.0;
                aabb.center = kin.pos;
                // His last pose for it: where it is, dropping straight down.
                let (pos, fall) = (kin.pos, kin.vel.y);
                ae::movement::constrain_body_pose(&mut kin, sweep.as_deref_mut(), pos, Vec2::new(0.0, fall));
            }
            continue;
        }
        conductor.clock += dt;
        conductor.ticks = conductor.ticks.wrapping_add(1);
        let clock = conductor.clock;
        let ticks = conductor.ticks;

        let fist_size = fist_entities
            .iter()
            .flatten()
            .find_map(|e| fists.get(*e).ok().map(|(kin, ..)| kin.size))
            .unwrap_or(Vec2::new(130.0, 100.0));
        let current: [Vec2; 2] = std::array::from_fn(|i| {
            fist_entities[i]
                .and_then(|e| fists.get(e).ok().map(|(kin, ..)| kin.pos))
                .unwrap_or(giant_kin.pos)
        });
        let stage = Stage {
            hall,
            homes: homes(&giant_kin),
            scholar: scholar_kin.pos,
            fist: fist_size,
        };
        let aim_now = player.unwrap_or(scholar_kin.pos);

        // ── Which part of which beat, and did it just begin? ──
        let live = live_part(attack);
        let previous = conductor.part;
        let in_eureka = previous.is_some_and(|p| p.mv == Move::Eureka);
        let rain_ended = previous.is_some_and(|p| p.mv == Move::AppleRain && p.striking)
            && live.is_none_or(|(mv, ..)| mv != Move::AppleRain);
        match live {
            // The rain is over and the golden apple is falling: EUREKA plays
            // out over the Rest the pattern leaves after the rain.
            None if rain_ended => {
                conductor.part = Some(Part {
                    mv: Move::Eureka,
                    striking: true,
                    t: 0.0,
                    tel_dur: 0.0,
                    beat_t: 0.0,
                });
                conductor.last_remaining = EUREKA_LEN;
                conductor.latch.from = current;
                let side = if aim_now.x < giant_kin.pos.x { -1.0 } else { 1.0 };
                let x = giant_kin.pos.x + side * (giant_kin.size.x * 0.5 + 50.0);
                let x = x.clamp(hall.left + 40.0, hall.right - 40.0);
                conductor.landing = Vec2::new(x, hall.floor - scholar_kin.size.y * 0.5);
            }
            None if in_eureka && conductor.last_remaining > dt => {
                let part = conductor.part.as_mut().expect("eureka");
                part.t += dt;
                part.beat_t += dt;
                conductor.last_remaining -= dt;
            }
            None => {
                if previous.is_some() {
                    conductor.latch.from = current;
                    conductor.resting = 0.0;
                    conductor.part = None;
                } else {
                    conductor.resting += dt;
                }
            }
            Some((mv, striking, remaining)) => {
                let same = previous.is_some_and(|p| p.mv == mv && p.striking == striking)
                    && remaining <= conductor.last_remaining + 1e-3;
                if same {
                    let part = conductor.part.as_mut().expect("same part");
                    part.t += dt;
                    part.beat_t += dt;
                } else {
                    let follows_telegraph = previous.is_some_and(|p| p.mv == mv && !p.striking && striking);
                    if follows_telegraph {
                        let p = previous.expect("a telegraph");
                        conductor.part = Some(Part {
                            mv,
                            striking,
                            t: 0.0,
                            tel_dur: p.t + dt,
                            beat_t: p.beat_t + dt,
                        });
                    } else {
                        // A new beat: latch what it needs.
                        let lead = conductor.next_lead;
                        conductor.next_lead = lead.other();
                        conductor.latch = Latch {
                            aim: aim_now,
                            lead: Some(lead),
                            from: current,
                            curve_phase: ch::fluxions::phase_for(&stage, aim_now.x, lead.side()),
                        };
                        conductor.aims = [aim_now; 2];
                        conductor.part = Some(Part {
                            mv,
                            striking,
                            t: 0.0,
                            tel_dur: 0.0,
                            beat_t: 0.0,
                        });
                    }
                    if striking {
                        // The strike's own onset.
                        match mv {
                            Move::Buck => {
                                // Through the launch gateway: the kernel spends it on
                                // the gnu's next step. Flinchless — a hop, not a hit.
                                giant_flight.stage_launch(Vec2::new(0.0, -BUCK_HOP), true);
                                play(&mut sfx, scholar, SFX_SNORT, giant_kin.pos);
                            }
                            Move::Stomp => {
                                let feet = Vec2::new(giant_kin.pos.x, hall.floor - WAVE_HALF.y);
                                for dir in [-1.0, 1.0] {
                                    let x = feet.x + dir * giant_kin.size.x * 0.5;
                                    let Some(slot) = conductor.waves.iter().position(Option::is_none) else {
                                        break;
                                    };
                                    let hitbox = ambition_combat::strike::spawn_damage_box(
                                        &mut commands,
                                        scholar,
                                        HitSide::Boss,
                                        Vec2::new(x, feet.y),
                                        ambition_combat::strike::DamageBox {
                                            half_extent: WAVE_HALF,
                                            shape: None,
                                            damage: WAVE_DAMAGE,
                                            knockback: WAVE_KNOCKBACK,
                                            lifetime_s: 6.0,
                                            name: Some("gnu_ton_shock"),
                                        },
                                    );
                                    conductor.waves[slot] = Some(Wave { hitbox, x, dir });
                                }
                                play(&mut sfx, scholar, SFX_STOMP, feet);
                                vfx.write(VfxMessage::Burst {
                                    pos: feet,
                                    count: 26,
                                    speed: 320.0,
                                    color: [0.55, 0.45, 0.35, 1.0],
                                    kind: ParticleKind::Dust,
                                });
                            }
                            _ => {}
                        }
                    }
                }
                conductor.last_remaining = remaining;
            }
        }

        // ── The fists ──
        let part = conductor.part;
        let lead = conductor.latch.lead.unwrap_or(Fist::Left);
        let cue = part.map(|p| Cue {
            mv: p.mv,
            striking: p.striking,
            t: p.t,
            dur: p.t + conductor.last_remaining,
            tel_dur: p.tel_dur,
            beat_t: p.beat_t,
            clock,
        });
        let mut poses = [Pose::idle(Vec2::ZERO); 2];
        for fist in Fist::BOTH {
            let i = fist.index();
            if let Some(cue) = cue.as_ref() {
                if ch::tracking(cue, lead, fist) {
                    let to = aim_now - conductor.aims[i];
                    let step = ch::demonstrate::TRACK_SPEED * dt;
                    conductor.aims[i] += if to.length() <= step { to } else { to.normalize() * step };
                }
            }
            let latch = Latch {
                aim: conductor.aims[i],
                ..conductor.latch
            };
            poses[i] = match cue.as_ref() {
                Some(cue) => ch::perform(&stage, &latch, fist, cue),
                None => {
                    let home = ch::idle(&stage, fist, clock).pos;
                    let ease = ch::ease(conductor.resting / 0.6);
                    Pose::idle(latch.from[i] + (home - latch.from[i]) * ease)
                }
            };
        }

        for fist in Fist::BOTH {
            let i = fist.index();
            let pose = poses[i];
            let last = conductor.last[i];
            let Some(entity) = fist_entities[i] else {
                continue;
            };
            let Ok((mut kin, mut aabb, mut health, mut surface, mut ground, mut sweep)) = fists.get_mut(entity) else {
                continue;
            };
            let vel = last.map_or(Vec2::ZERO, |last| (pose.pos - last.pos) / dt);
            ae::movement::constrain_body_pose(&mut kin, sweep.as_deref_mut(), pose.pos, vel);
            // Knuckles toward the gnu.
            kin.facing = -fist.side();
            surface.gravity_scale = 0.0;
            ground.invalidate();
            aabb.center = kin.pos;
            aabb.half_size = kin.size * 0.5;

            // A blow to his fist lands on him.
            let lost = health.max() - health.current();
            if lost > 0 {
                health.heal(lost);
                let refused = encounter.encounter.as_ref().is_some_and(|phase| phase.boss_invulnerable());
                if !refused {
                    scholar_combat.hit_flash = 0.18;
                    if scholar_health.damage(lost) {
                        if let Some(phase) = encounter.encounter.as_mut() {
                            let _ = phase.kill();
                        }
                    }
                }
            }
            // Only a stuck fist can be struck.
            health.health.invulnerable.set(Invulnerability::SCRIPTED, !pose.stuck);

            // Its hit volume.
            if pose.harmful {
                let stale = conductor.hitboxes[i].is_none() || clock - conductor.armed_at[i] >= REARM;
                if stale {
                    if let Some(old) = conductor.hitboxes[i].take() {
                        commands.entity(old).try_despawn();
                    }
                    conductor.hitboxes[i] = Some(commands.spawn(fist_hitbox(entity, kin.size)).id());
                    conductor.armed_at[i] = clock;
                }
            } else if let Some(old) = conductor.hitboxes[i].take() {
                commands.entity(old).try_despawn();
            }

            // Impact.
            let landed = pose.stuck && !last.is_some_and(|last| last.stuck);
            if landed {
                let at = Vec2::new(pose.pos.x, hall.floor);
                let heavy = pose.harmful;
                vfx.write(VfxMessage::Burst {
                    pos: at,
                    count: if heavy { 22 } else { 8 },
                    speed: if heavy { 300.0 } else { 140.0 },
                    color: [0.6, 0.5, 0.4, 1.0],
                    kind: ParticleKind::Dust,
                });
                if heavy {
                    play(&mut sfx, scholar, SFX_STOMP, at);
                }
            }
            // A stuck fist glints: here is your opening.
            if pose.stuck && ticks % 12 == 0 {
                spark(&mut vfx, pose.pos - Vec2::new(0.0, kin.size.y * 0.5), GOLD, 2, 60.0);
            }
            conductor.last[i] = Some(pose);
        }

        // ── Telegraph ink ──
        if let (Some(cue), true) = (cue.as_ref(), ticks % 3 == 0) {
            match cue.mv {
                Move::Demonstrate | Move::DemonstratePair => {
                    // A plumb line from each hovering fist to where it will land.
                    for fist in Fist::BOTH {
                        let pose = poses[fist.index()];
                        let aiming = cue.mv == Move::DemonstratePair || fist == lead;
                        if !aiming || pose.stuck || pose.harmful {
                            continue;
                        }
                        let top = pose.pos.y + fist_size.y * 0.5;
                        let locked = !ch::tracking(cue, lead, fist);
                        for k in 1..=6 {
                            let y = top + (hall.floor - top) * k as f32 / 6.0;
                            spark(&mut vfx, Vec2::new(pose.pos.x, y), if locked { WARNING } else { INK }, 1, 4.0);
                        }
                    }
                }
                Move::Pendulum | Move::Cradle => {
                    // The strings.
                    for fist in Fist::BOTH {
                        let swinging = cue.mv == Move::Cradle || fist == lead;
                        if !swinging {
                            continue;
                        }
                        let pose = poses[fist.index()];
                        let latch = Latch { aim: conductor.aims[fist.index()], ..conductor.latch };
                        let pivot = match cue.mv {
                            Move::Pendulum => ch::pendulum::pivot(&stage, &latch),
                            _ => Vec2::new(
                                hall.center_x() + fist.side() * fist_size.x * 0.5,
                                hall.floor - ch::cradle::PIVOT_HEIGHT,
                            ),
                        };
                        for k in 0..8 {
                            let p = pivot + (pose.pos - pivot) * (k as f32 / 8.0);
                            spark(&mut vfx, p, INK, 1, 3.0);
                        }
                    }
                }
                Move::Fluxions | Move::FluxionsPair if !cue.striking => {
                    // He writes the curve before he traces it.
                    let drawn = ch::fluxions::drawn(cue.t, cue.dur);
                    let mirrors: &[bool] = if cue.mv == Move::FluxionsPair { &[false, true] } else { &[false] };
                    for &mirror in mirrors {
                        let dots = (28.0 * drawn).ceil() as usize;
                        for k in 0..=dots {
                            let u = drawn * k as f32 / dots.max(1) as f32;
                            let p = ch::fluxions::point(&stage, &conductor.latch, u, mirror);
                            spark(&mut vfx, p + Vec2::new(0.0, fist_size.y * 0.5), INK, 1, 3.0);
                        }
                        if drawn < 1.0 {
                            let head = ch::fluxions::point(&stage, &conductor.latch, drawn, mirror);
                            spark(&mut vfx, head + Vec2::new(0.0, fist_size.y * 0.5), GOLD, 3, 20.0);
                        }
                    }
                }
                _ => {}
            }
        }
        // The cradle's clack.
        if let Some(cue) = cue.as_ref() {
            if cue.mv == Move::Cradle && cue.striking && ch::cradle::clacks((cue.t - dt).max(0.0), cue.t, cue.dur) {
                let at = (poses[0].pos + poses[1].pos) * 0.5;
                spark(&mut vfx, at, GOLD, 14, 260.0);
                play(&mut sfx, scholar, SFX_STOMP, at);
            }
        }

        // ── Eureka: the scholar himself leaves the saddle ──
        if let Some(cue) = cue.filter(|cue| cue.mv == Move::Eureka && cue.striking) {
            let saddle = scholar_kin.pos;
            let head = saddle - Vec2::new(0.0, scholar_kin.size.y * 0.5);
            match ch::eureka::part(cue.t, cue.dur) {
                ch::eureka::Part::AppleFalling(f) => {
                    let apple = head - Vec2::new(0.0, 320.0 * (1.0 - f * f));
                    spark(&mut vfx, apple, GOLD, 3, 6.0);
                    if cue.t + dt >= ch::eureka::APPLE {
                        spark(&mut vfx, head, GOLD, 30, 280.0);
                        play(&mut sfx, scholar, SFX_SNORT, head);
                        vfx.write(VfxMessage::SpeechBubble {
                            pos: head - Vec2::new(0.0, 30.0),
                            text: "Eureka!".to_string(),
                        });
                    }
                }
                ch::eureka::Part::Dazed => {
                    if ticks % 4 == 0 {
                        let top = conductor.landing - Vec2::new(0.0, scholar_kin.size.y * 0.5 + 12.0);
                        for k in 0..3 {
                            let a = clock * 5.0 + k as f32 * std::f32::consts::TAU / 3.0;
                            spark(&mut vfx, top + Vec2::new(a.cos() * 26.0, a.sin() * 8.0), GOLD, 1, 4.0);
                        }
                    }
                }
                _ => {}
            }
            let pos = ch::eureka::scholar(saddle, conductor.landing, cue.t, cue.dur);
            ae::movement::constrain_body_pose(&mut scholar_kin, scholar_sweep.as_deref_mut(), pos, Vec2::ZERO);
            scholar_aabb.center = pos;
        }

        // ── The stomp's shocks roll out along the floor ──
        for slot in 0..conductor.waves.len() {
            let Some(mut wave) = conductor.waves[slot] else {
                continue;
            };
            wave.x += wave.dir * WAVE_SPEED * dt;
            let gone = wave.x < hall.left + WAVE_HALF.x || wave.x > hall.right - WAVE_HALF.x;
            if gone || hitboxes.get(wave.hitbox).is_err() {
                commands.entity(wave.hitbox).try_despawn();
                conductor.waves[slot] = None;
                continue;
            }
            let center = Vec2::new(wave.x, hall.floor - WAVE_HALF.y);
            if let Ok(mut hitbox) = hitboxes.get_mut(wave.hitbox) {
                hitbox.anchor = HitboxAnchor::World { center };
            }
            if ticks % 2 == 0 {
                vfx.write(VfxMessage::Burst {
                    pos: center + Vec2::new(0.0, WAVE_HALF.y * 0.5),
                    count: 3,
                    speed: 120.0,
                    color: [0.62, 0.52, 0.4, 1.0],
                    kind: ParticleKind::Dust,
                });
            }
            conductor.waves[slot] = Some(wave);
        }
    }
}

/// The gnu's back is ground: a one-way ledge riding the giant, which throws
/// whoever stands on it while he bucks.
pub fn gnu_back_is_ground(
    scholars: Query<(&GnuTonConductor, &RidingOn, &BodyHealth)>,
    giants: Query<&ae::BodyKinematics, With<MountSlot>>,
    mut overlay: ResMut<ambition_platformer2d::world::FeatureEcsWorldOverlay>,
) {
    for (conductor, riding, health) in &scholars {
        let Ok(giant) = giants.get(riding.mount) else {
            continue;
        };
        let back = back_platform(giant);
        overlay.blocks.push(ae::Block {
            id: ae::GeoId::anon(),
            name: "gnu_back".to_string(),
            aabb: back,
            kind: ae::BlockKind::OneWay,
            velocity: giant.vel,
            art_color: None,
        });
        let bucking = health.alive()
            && conductor
                .part
                .is_some_and(|part| part.mv == Move::Buck && part.striking && part.t < BUCK_WINDOW);
        if bucking {
            let throw = ae::aabb_from_min_size(back.min - Vec2::new(0.0, 24.0), Vec2::new(back.max.x - back.min.x, 30.0));
            overlay.blocks.push(ae::Block::rebound("gnu_back_buck", throw.min, throw.max - throw.min, BUCK_THROW));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_back_platform_mirrors_with_the_gnu() {
        let mut kin = ae::BodyKinematics::default();
        kin.pos = Vec2::new(900.0, 1040.0);
        kin.facing = 1.0;
        let right = back_platform(&kin);
        kin.facing = -1.0;
        let left = back_platform(&kin);
        assert!((right.min.x - 900.0 + 208.0).abs() < 0.5, "{right:?}");
        assert!((left.max.x - 900.0 - 208.0).abs() < 0.5, "{left:?}");
        assert_eq!(right.min.y, left.min.y);
    }

    #[test]
    fn the_fists_rest_on_either_side() {
        let mut kin = ae::BodyKinematics::default();
        kin.pos = Vec2::new(900.0, 1040.0);
        let [l, r] = homes(&kin);
        assert!(l.x < kin.pos.x && r.x > kin.pos.x);
    }
}
