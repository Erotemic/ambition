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
    Commands, Component, Entity, Message, MessageReader, MessageWriter, Query, Res, With, Without,
};

use ambition_engine_core as ae;
use ambition_entity_catalog::{
    AttackDir, ClipBinding, EffectRef, HitVolume, MoveEvent, MoveEventKind, MoveSpec, MoveWindow,
    MovesetContract, VolumeShape, WindowTag,
};
use ambition_time::ProperTimeScale;

use super::components::{ActorFaction, BodyMelee, MeleeSwing};
use super::hitbox::{Hitbox, HitboxAnchor, HitboxHits};
use crate::combat::{AttackIntent, AttackSpec};
use ambition_characters::brain::action_set::{
    ActionRequest, MeleeActionSpec, RangedActionSpec, SpecialActionSpec,
};
use ambition_characters::brain::{ActorActionMessage, ActorControl};
use ambition_combat::DamageKind;
use ambition_sfx::{SfxId, SfxMessage};
use ambition_time::WorldTime;

/// The canonical verb id a body's basic melee swing binds to in its moveset.
pub const ATTACK_VERB: &str = "attack";

/// The SFX cue a plain swing fires. Names the engine's procedural `slash` cue
/// (`ambition_sfx::ids::PLAYER_SLASH` = `"player.slash"`) so the audio runtime
/// resolves it to the guaranteed procedural sound — the old bespoke melee path
/// used `SfxMessage::Slash`, and the moveset must stay audible. (The prior
/// `"melee_swing"` string matched no bank sample and no procedural cue, so it
/// silently no-op-ed — the "no attack SFX" bug.)
pub const SWING_SFX_CUE: &str = "player.slash";

/// The canonical verb id a body's ranged shot binds to in its moveset.
pub const RANGED_VERB: &str = "ranged";

/// Convert an authored [`MeleeActionSpec`] into a data-driven `"attack"`
/// [`MoveSpec`] — the melee subsumption (fable review §A1 / §3a). The swing's
/// windup/active/recover timeline becomes Startup / Active(one forward Rect hit
/// volume) / Recovery windows on the owner's proper-time clock, so a plain melee
/// runs through the SAME moveset runtime as the body's specials. Re-binding a
/// swing onto another body is now a data edit, and the swing composes with
/// dilation / pause for free.
///
/// The forward hit volume is a body-local rect sized from the spec's `reach_px`
/// (offset a bit past the body, half-extent covering the reach + a torso-height
/// band). Knockback is a sensible default; directional variants (up/down/air) and
/// pogo remain on the flat player path for now (bulk-review: player-melee fold).
pub fn attack_move_from_melee(spec: &MeleeActionSpec) -> MoveSpec {
    let (windup, active, recover, damage, reach) = spec.timeline();
    // The authored-melee path is now a thin adapter over the `simple_melee`
    // engine prefab (A2): the MeleeActionSpec timeline becomes prefab params.
    // Byte-identical — the clamps + volume shape live in the prefab core.
    simple_melee(&SimpleMeleeParams {
        windup_s: windup,
        active_s: active,
        recover_s: recover,
        damage,
        reach_px: reach,
        knockback: 120.0,
    })
}

/// Params for the [`simple_melee`] engine prefab (A2 / R2.3) — a forward swing
/// as authored DATA. Every field defaults, so a roster prefab row omits what it
/// doesn't tune (`prefab: "simple_melee"` with empty params = a default jab).
/// `sword_slash` is literally this prefab + params, zero new code.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct SimpleMeleeParams {
    #[serde(default = "smp_windup")]
    pub windup_s: f32,
    #[serde(default = "smp_active")]
    pub active_s: f32,
    #[serde(default = "smp_recover")]
    pub recover_s: f32,
    #[serde(default = "smp_damage")]
    pub damage: i32,
    #[serde(default = "smp_reach")]
    pub reach_px: f32,
    #[serde(default = "smp_knockback")]
    pub knockback: f32,
}

fn smp_windup() -> f32 {
    0.12
}
fn smp_active() -> f32 {
    0.10
}
fn smp_recover() -> f32 {
    0.18
}
fn smp_damage() -> i32 {
    1
}
fn smp_reach() -> f32 {
    36.0
}
fn smp_knockback() -> f32 {
    120.0
}

impl Default for SimpleMeleeParams {
    fn default() -> Self {
        Self {
            windup_s: smp_windup(),
            active_s: smp_active(),
            recover_s: smp_recover(),
            damage: smp_damage(),
            reach_px: smp_reach(),
            knockback: smp_knockback(),
        }
    }
}

/// The `simple_melee` prefab core: a forward Startup/Active(one Rect hit)/Recovery
/// swing on the owner's proper-time clock. Shared by the authored-melee adapter
/// ([`attack_move_from_melee`]) and the prefab registry.
pub fn simple_melee(p: &SimpleMeleeParams) -> MoveSpec {
    let windup = p.windup_s.max(0.0);
    let active = p.active_s.max(0.02);
    let recover = p.recover_s.max(0.0);
    let duration = windup + active + recover;
    // Forward rect: centered just past the body, extending to `reach`, with a
    // torso-height band. Authored body-local (x = side/forward); the runtime
    // mirrors it by facing and rotates it into the gravity frame at spawn.
    let half_x = (p.reach_px * 0.5).max(8.0);
    let volume = HitVolume {
        shape: VolumeShape::Rect {
            offset: (p.reach_px * 0.6, 0.0),
            half_extents: (half_x, 16.0),
        },
        damage: p.damage.max(1),
        knockback: p.knockback,
        // A plain swing lands no on-hit technique; directional variants (a
        // down-air pogo) author `on_hit` per-move (R2.5 player-melee fold).
        on_hit: None,
    };
    MoveSpec {
        id: ATTACK_VERB.to_string(),
        clip: ClipBinding {
            clip: "attack_side".to_string(),
            fallbacks: vec!["slash".to_string(), "idle".to_string()],
        },
        duration_s: duration,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: windup,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
            },
            MoveWindow {
                start_s: windup,
                end_s: windup + active,
                tag: WindowTag::Active,
                volumes: vec![volume],
                sustain_effect: None,
            },
            MoveWindow {
                start_s: windup + active,
                end_s: duration,
                tag: WindowTag::Recovery,
                volumes: vec![],
                sustain_effect: None,
            },
        ],
        events: vec![MoveEvent {
            at_s: windup,
            kind: MoveEventKind::Sfx {
                cue: SWING_SFX_CUE.to_string(),
            },
        }],
        gates: Default::default(),
        start_impulse: None,
    }
}

/// Convert an authored [`RangedActionSpec`] into a data-driven `"ranged"`
/// [`MoveSpec`] — the ranged subsumption (fable review, option A). Ranged had NO
/// windup/recovery before (it fired instantly on a body-side cooldown); giving it a
/// move timeline is the expressivity win: a `Startup` draw/aim window, a single
/// [`MoveEventKind::Ranged`] fire event at release (which spawns the projectile,
/// sampling LIVE aim), and a `Recovery` settle window — all on the owner's
/// proper-time clock, so a dilated shooter's draw slows with it. The projectile IS
/// the damage (spawned by the event through the shared enemy-projectile consumer),
/// so the move carries NO hit volume. Windup/recovery are authored defaults per
/// spec kind (deferred-tuning); the body-side refire cooldown remains the hard rate
/// floor, the move duration an additional cadence gate.
pub fn fire_move_from_ranged(spec: &RangedActionSpec) -> MoveSpec {
    // Draw/settle timing per weapon kind — Arrow winds up slowest (per its doc),
    // Pistol snappiest. New authored defaults (ranged had none); tune later.
    let (windup, recover) = match spec {
        RangedActionSpec::Pistol { .. } => (0.08, 0.15),
        RangedActionSpec::Rock { .. } => (0.12, 0.18),
        RangedActionSpec::Bolt { .. } => (0.18, 0.20),
        RangedActionSpec::Arrow { .. } => (0.28, 0.22),
    };
    // Thin adapter over the `simple_ranged` engine prefab (A2). The projectile
    // still comes from the owner's live ActionSet.ranged at the fire event.
    simple_ranged(&SimpleRangedParams {
        windup_s: windup,
        recover_s: recover,
    })
}

/// Params for the [`simple_ranged`] engine prefab (A2). The move carries NO
/// projectile spec — the fire event samples the owner's live `ActionSet.ranged`
/// — so its only knobs are the draw/settle timings.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct SimpleRangedParams {
    #[serde(default = "srp_windup")]
    pub windup_s: f32,
    #[serde(default = "srp_recover")]
    pub recover_s: f32,
}

fn srp_windup() -> f32 {
    0.12
}
fn srp_recover() -> f32 {
    0.18
}

impl Default for SimpleRangedParams {
    fn default() -> Self {
        Self {
            windup_s: srp_windup(),
            recover_s: srp_recover(),
        }
    }
}

/// The `simple_ranged` prefab core: a Startup(draw)/Recovery(settle) timeline
/// whose single [`MoveEventKind::Ranged`] fire event spawns the owner's shot.
pub fn simple_ranged(p: &SimpleRangedParams) -> MoveSpec {
    let windup = p.windup_s.max(0.0);
    let recover = p.recover_s.max(0.0);
    let duration = windup + recover;
    MoveSpec {
        id: RANGED_VERB.to_string(),
        clip: ClipBinding {
            clip: "shoot".to_string(),
            fallbacks: vec!["attack_side".to_string(), "idle".to_string()],
        },
        duration_s: duration,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: windup,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
            },
            // No Active hit volume — the projectile spawned by the fire event is the
            // damage. The Recovery window just holds the post-shot settle.
            MoveWindow {
                start_s: windup,
                end_s: duration,
                tag: WindowTag::Recovery,
                volumes: vec![],
                sustain_effect: None,
            },
        ],
        events: vec![MoveEvent {
            at_s: windup,
            kind: MoveEventKind::Ranged,
        }],
        gates: Default::default(),
        start_impulse: None,
    }
}

/// Params for the [`simple_charge`] engine prefab (A2) — a hold-then-release
/// heavy hit the demos need (SMB1's crouch-charge, a wind-up smash). The
/// `charge_s` Startup window is the hold; the `active_s` Active window lands one
/// forward Rect hit sized from `reach_px`, then `recover_s` settle.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct SimpleChargeParams {
    #[serde(default = "scp_charge")]
    pub charge_s: f32,
    #[serde(default = "scp_active")]
    pub active_s: f32,
    #[serde(default = "scp_recover")]
    pub recover_s: f32,
    #[serde(default = "scp_damage")]
    pub damage: i32,
    #[serde(default = "scp_reach")]
    pub reach_px: f32,
    #[serde(default = "scp_knockback")]
    pub knockback: f32,
}

fn scp_charge() -> f32 {
    0.45
}
fn scp_active() -> f32 {
    0.12
}
fn scp_recover() -> f32 {
    0.30
}
fn scp_damage() -> i32 {
    3
}
fn scp_reach() -> f32 {
    44.0
}
fn scp_knockback() -> f32 {
    260.0
}

impl Default for SimpleChargeParams {
    fn default() -> Self {
        Self {
            charge_s: scp_charge(),
            active_s: scp_active(),
            recover_s: scp_recover(),
            damage: scp_damage(),
            reach_px: scp_reach(),
            knockback: scp_knockback(),
        }
    }
}

/// The `simple_charge` prefab core.
pub fn simple_charge(p: &SimpleChargeParams) -> MoveSpec {
    let charge = p.charge_s.max(0.0);
    let active = p.active_s.max(0.02);
    let recover = p.recover_s.max(0.0);
    let duration = charge + active + recover;
    let half_x = (p.reach_px * 0.5).max(8.0);
    let volume = HitVolume {
        shape: VolumeShape::Rect {
            offset: (p.reach_px * 0.6, 0.0),
            half_extents: (half_x, 18.0),
        },
        damage: p.damage.max(1),
        knockback: p.knockback,
        on_hit: None,
    };
    MoveSpec {
        id: "charge".to_string(),
        clip: ClipBinding {
            clip: "attack_side".to_string(),
            fallbacks: vec!["slash".to_string(), "idle".to_string()],
        },
        duration_s: duration,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: charge,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
            },
            MoveWindow {
                start_s: charge,
                end_s: charge + active,
                tag: WindowTag::Active,
                volumes: vec![volume],
                sustain_effect: None,
            },
            MoveWindow {
                start_s: charge + active,
                end_s: duration,
                tag: WindowTag::Recovery,
                volumes: vec![],
                sustain_effect: None,
            },
        ],
        events: vec![MoveEvent {
            at_s: charge,
            kind: MoveEventKind::Sfx {
                cue: SWING_SFX_CUE.to_string(),
            },
        }],
        gates: Default::default(),
        start_impulse: None,
    }
}

/// A prefab builder: hydrate an authored [`ParamValue`] into the prefab's own
/// params and expand it into a [`MoveSpec`]. `fn`-pointer shaped so the registry
/// stays a plain data table.
pub type MovePrefabBuilder = fn(&ambition_entity_catalog::ParamValue) -> Result<MoveSpec, String>;

/// String-keyed prefab registry (A2 / R2.3): `key + params -> MoveSpec`, expanded
/// at roster install. The engine ships `simple_melee` / `simple_ranged` /
/// `simple_charge`; a content roster names a prefab + params to mint a move with
/// ZERO new code (`sword_slash = simple_melee` + sword params). Content may
/// register its own prefabs for richer shapes.
pub struct MovePrefabRegistry {
    builders: std::collections::BTreeMap<String, MovePrefabBuilder>,
}

impl MovePrefabRegistry {
    /// A registry pre-seeded with the engine-shipped prefabs.
    pub fn with_engine_prefabs() -> Self {
        let mut reg = Self {
            builders: std::collections::BTreeMap::new(),
        };
        reg.register("simple_melee", |p| {
            Ok(simple_melee(&p.hydrate().map_err(|e| e.to_string())?))
        });
        reg.register("simple_ranged", |p| {
            Ok(simple_ranged(&p.hydrate().map_err(|e| e.to_string())?))
        });
        reg.register("simple_charge", |p| {
            Ok(simple_charge(&p.hydrate().map_err(|e| e.to_string())?))
        });
        reg
    }

    /// Register (or override) a prefab builder under `key`.
    pub fn register(&mut self, key: impl Into<String>, builder: MovePrefabBuilder) {
        self.builders.insert(key.into(), builder);
    }

    /// Expand a prefab row into a move named `move_id`. Errors if the key is
    /// unknown (a roster typo) or the authored params don't hydrate.
    pub fn expand(
        &self,
        key: &str,
        params: &ambition_entity_catalog::ParamValue,
        move_id: &str,
    ) -> Result<MoveSpec, String> {
        let builder = self
            .builders
            .get(key)
            .ok_or_else(|| format!("unknown move prefab '{key}'"))?;
        let mut spec = builder(params)?;
        spec.id = move_id.to_string();
        Ok(spec)
    }

    /// True iff no prefab is registered.
    pub fn is_empty(&self) -> bool {
        self.builders.is_empty()
    }
}

impl Default for MovePrefabRegistry {
    fn default() -> Self {
        Self::with_engine_prefabs()
    }
}

/// Fold a body's authored melee (`ActionSet.melee`) and ranged (`ActionSet.ranged`)
/// into its moveset as the `"attack"` and `"ranged"` moves, merging with any
/// signature-move repertoire. The single seam every actor-spawn path calls so a
/// body's basic swing, its shot, and its specials live in ONE `MovesetContract`.
/// Returns `None` when the body has none of them.
/// A body-local direction to transform the base swing's reach into.
#[derive(Clone, Copy)]
enum Dir {
    Fwd,
    Up,
    Down,
    Back,
}

/// Derive a body's DIRECTIONAL melee variants from its base `"attack"` move by
/// transforming that move's hit volume: the forward reach rotates up / down
/// (its dimensions swap) or mirrors behind, so a character's ONE authored swing
/// yields up-/down-tilt + the four aerials + the pogo down-air, all scaled by
/// ITS own reach and body. Presentation clip and grounded gate change per
/// variant; timing/damage/knockback are inherited. Each entry is
/// `(verb id, MoveSpec)` — the verb the directional trigger resolves to.
fn directional_attack_variants(base: &MoveSpec) -> Vec<(String, MoveSpec)> {
    fn xf(dir: Dir, offset: (f32, f32), half: (f32, f32)) -> ((f32, f32), (f32, f32)) {
        let ((ox, oy), (hx, hy)) = (offset, half);
        match dir {
            Dir::Fwd => ((ox, oy), (hx, hy)),
            // +x reach → up (-y); box dimensions swap under the quarter turn.
            Dir::Up => ((0.0, -ox), (hy, hx)),
            Dir::Down => ((0.0, ox), (hy, hx)),
            Dir::Back => ((-ox, oy), (hx, hy)),
        }
    }
    let variant = |id: &str, clip: &str, grounded: bool, dir: Dir, pogo: bool| -> MoveSpec {
        let mut m = base.clone();
        m.id = id.to_string();
        m.clip.clip = clip.to_string();
        m.gates.grounded = Some(grounded);
        for w in &mut m.windows {
            if !matches!(w.tag, WindowTag::Active) {
                continue;
            }
            for v in &mut w.volumes {
                if let VolumeShape::Rect {
                    offset,
                    half_extents,
                } = v.shape
                {
                    let (o, h) = xf(dir, offset, half_extents);
                    v.shape = VolumeShape::Rect {
                        offset: o,
                        half_extents: h,
                    };
                }
                if pogo {
                    // The down-air's landing pogo — an engine on-hit technique
                    // (fable review AJ1). Fires off any `PogoTarget` (enemy or orb).
                    v.on_hit = Some(EffectRef::new(crate::combat::on_hit::POGO_BOUNCE_KEY));
                }
            }
        }
        m
    };
    vec![
        (
            "attack_up".to_string(),
            variant("attack_up", "attack_up", true, Dir::Up, false),
        ),
        (
            "attack_down".to_string(),
            variant("attack_down", "attack_down", true, Dir::Down, false),
        ),
        (
            "attack_air".to_string(),
            variant("attack_air", "attack_air", false, Dir::Fwd, false),
        ),
        (
            "attack_air_up".to_string(),
            variant("attack_air_up", "attack_up", false, Dir::Up, false),
        ),
        (
            "attack_air_back".to_string(),
            variant("attack_air_back", "attack_air", false, Dir::Back, false),
        ),
        (
            "attack_air_down".to_string(),
            variant("attack_air_down", "attack_air_down", false, Dir::Down, true),
        ),
    ]
}

pub fn build_actor_moveset(
    signature: Option<&MovesetContract>,
    melee: Option<&MeleeActionSpec>,
    ranged: Option<&RangedActionSpec>,
) -> Option<MovesetContract> {
    let mut contract = signature.cloned().unwrap_or_default();
    if let Some(melee) = melee {
        let attack = attack_move_from_melee(melee);
        contract
            .verbs
            .insert(ATTACK_VERB.to_string(), attack.id.clone());
        // Directional variants DERIVED from the base swing (fable review R2.5):
        // the character's ONE authored melee becomes up-/down-tilt + the four
        // aerials + the pogo down-air, scaled by ITS reach — not a hardcoded
        // per-character table. Every controlled body (human / brain / RL) resolves
        // these through the SAME directional trigger; a neutral, grounded attacker
        // (every enemy today, since brains don't aim) still resolves `"attack"`,
        // so the swing that lands is byte-identical.
        for (verb, mv) in directional_attack_variants(&attack) {
            contract.verbs.insert(verb, mv.id.clone());
            contract.moves.retain(|m| m.id != mv.id);
            contract.moves.push(mv);
        }
        // Replace any existing attack move (idempotent) then push.
        contract.moves.retain(|m| m.id != attack.id);
        contract.moves.push(attack);
    }
    if let Some(ranged) = ranged {
        let fire = fire_move_from_ranged(ranged);
        contract
            .verbs
            .insert(RANGED_VERB.to_string(), fire.id.clone());
        contract.moves.retain(|m| m.id != fire.id);
        contract.moves.push(fire);
    }
    if contract.moves.is_empty() {
        None
    } else {
        Some(contract)
    }
}

/// Marker: this body's basic melee swing is a data-driven moveset `"attack"`
/// move, not the flat `BodyMelee` swing. The flat-melee phases
/// (`start_body_melee` / `advance_body_melee`'s swing logic) SKIP a body carrying
/// this marker — its swing is triggered by [`trigger_moveset_moves`] and run by
/// [`advance_move_playback`], and its `BodyMelee` read-model is projected from the
/// live [`MovePlayback`] by [`project_moveset_melee_to_body_melee`] so every
/// existing consumer (actor anim index, view/telegraph index, HUD) keeps working.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MovesetMelee;

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
        Self::new_at(spec, facing, 0.0)
    }

    /// Start a move with its clock pre-advanced to `t0` seconds (owner proper
    /// time). Used to SKIP a leading window: a boss strike commanded without a
    /// telegraph (possession, or a bare `Strike` step) starts at `t0 = telegraph
    /// window`, so its Active window is live immediately and the projected
    /// `active_elapsed` still folds in the telegraph offset (E53). Events with
    /// `at_s <= t0` are pre-marked fired so seeking past them doesn't retro-fire.
    pub fn new_at(spec: MoveSpec, facing: f32, t0: f32) -> Self {
        let t0 = t0.clamp(0.0, spec.duration_s);
        let fired: Vec<bool> = spec.events.iter().map(|ev| ev.at_s <= t0).collect();
        Self {
            spec,
            facing,
            t: t0,
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
        // The owner's brain, so a POSSESSED body's strike carries its EFFECTIVE
        // faction (a controlled body fights as `Player`): `effective_faction`'s
        // contract is that every hitbox stamp resolves through it, and this move
        // strike is one of them. `None`/non-player-brain ⇒ the authored faction
        // (identity for every ordinary actor + the player's own body).
        Option<&ambition_characters::brain::Brain>,
        &ae::BodyKinematics,
        Option<&ProperTimeScale>,
    )>,
) {
    for (owner, mut playback, faction, brain, kin, scale) in &mut players {
        let strike_faction = crate::combat::targeting::effective_faction(*faction, brain);
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

        // Sustained (held) effects: while `t` is inside a window carrying a
        // `sustain_effect`, emit its `Effect` EVERY frame — the consuming technique
        // times its own cadence off this per-frame "active this tick" signal. This
        // is how a move expresses a HELD special (a lingering beam, a continuous
        // rain), the shape the boss `apple_rain`-style specials need. Dilation
        // stretches the sustain the same way (fewer proper-time frames of it).
        for window in &pb.spec.windows {
            if let Some(effect) = &window.sustain_effect {
                if window.start_s <= t && t < window.end_s {
                    events.write(MoveEventMessage {
                        owner,
                        move_id: pb.spec.id.clone(),
                        kind: MoveEventKind::Effect(effect.clone()),
                    });
                }
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
                        let mut ec = commands.spawn((
                            Hitbox {
                                owner,
                                source: strike_faction,
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
                        ));
                        // Conditional on-hit technique (pogo, lifesteal, …): a
                        // volume authoring `on_hit` gets the sidecar the
                        // `dispatch_hitbox_on_hit` primitive reads (fable AJ1).
                        if let Some(effect) = &volume.on_hit {
                            ec.insert(super::on_hit::HitboxOnHit::new(effect.clone()));
                        }
                        let hitbox = ec.id();
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

/// Reduce a body-local attack aim axis to a discrete [`AttackDir`] for
/// directional move selection. The axis is body/gravity-local (+x = facing,
/// +y = gravity-down), so `y < 0` is "toward the head" (Up) under ANY gravity
/// — the same frame the move's volume offsets live in. A forward or neutral aim
/// both read `Neutral` (the plain jab); an aim opposite facing reads `Back`.
/// Vertical wins ties so a clear up/down aim beats slight horizontal drift.
pub(crate) fn attack_dir_from_axis(axis: ae::Vec2) -> AttackDir {
    const DEADZONE: f32 = 0.5;
    if axis.y.abs() >= axis.x.abs() && axis.y.abs() > DEADZONE {
        if axis.y < 0.0 {
            AttackDir::Up
        } else {
            AttackDir::Down
        }
    } else if axis.x < -DEADZONE {
        AttackDir::Back
    } else {
        AttackDir::Neutral
    }
}

/// TRIGGER a body's data-driven move from its control-frame verb edges: a
/// `special_pressed` → the `"special"` verb, a `melee_pressed` → the DIRECTIONAL
/// `"attack"` verb (resolved by aim + grounded state through the authored verb
/// chain — `attack_air_down` → `attack_down` → `attack`), a ranged intent → the
/// `"ranged"` verb. A body already playing a move (`With<MovePlayback>`) is
/// excluded, so a move plays to completion before another starts — the move's own
/// duration IS the fire-rate gate. Facing locks at trigger from the body's
/// kinematics (the Smash convention — a committed swing doesn't re-aim).
///
/// ONE trigger seam for every body (guardrail #1): the same system drives an
/// actor's melee, the PCA's signature move, a folded boss's pattern, and the
/// player's directional repertoire (R2.5). A body authoring only `"attack"`
/// resolves every direction to it — byte-identical to the pre-directional path.
pub fn trigger_moveset_moves(
    mut commands: Commands,
    gravity: crate::physics::GravityCtx,
    mut bodies: Query<
        (
            Entity,
            &ActorMoveset,
            &ActorControl,
            // Mutable so a move's authored `start_impulse` (self-motion) lands at
            // trigger — the move-start seam.
            &mut ae::BodyKinematics,
            // Grounded state selects tilt-vs-air variants. Absent on bare test
            // bodies → treated as grounded (immaterial: such bodies author only
            // the base `attack`, which every direction resolves to).
            Option<&crate::actor::BodyGroundState>,
        ),
        Without<MovePlayback>,
    >,
) {
    for (entity, moveset, control, mut kin, ground) in &mut bodies {
        let frame = &control.0;
        let grounded = ground.map(|g| g.on_ground).unwrap_or(true);
        let spec = if frame.special_pressed {
            moveset.0.move_for_verb("special")
        } else if frame.melee_pressed || frame.pogo_pressed {
            // A dedicated pogo press IS a down-air (the move carrying the pogo
            // on-hit technique); a plain melee press resolves by aim. When only
            // pogo is pressed, force Down so an aerial body reaches `attack_air_down`.
            let dir = if frame.pogo_pressed && !frame.melee_pressed {
                AttackDir::Down
            } else {
                attack_dir_from_axis(frame.attack_axis)
            };
            moveset
                .0
                .move_for_directional_verb(ATTACK_VERB, dir, grounded)
        } else if frame.fire.is_some() {
            // A ranged intent (`frame.fire = Some(dir)`) starts the body's `"ranged"`
            // move; its fire event spawns the projectile, sampling live aim. The move
            // plays to completion before another starts (its duration is a cadence
            // gate; the body-side refire cooldown remains the hard rate floor).
            moveset.0.move_for_verb(RANGED_VERB)
        } else {
            continue;
        };
        if let Some(spec) = spec {
            // Self-motion: a body-local impulse mirrored by facing and rotated
            // into the owner's gravity frame (a jab's lunge stays "forward"
            // under any gravity). Identity when the move authors none.
            if let Some((ix, iy)) = spec.start_impulse {
                let local = ae::Vec2::new(ix * kin.facing, iy);
                let world_impulse =
                    ae::AccelerationFrame::new(gravity.dir_at(kin.pos)).to_world(local);
                kin.vel += world_impulse;
            }
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
/// - `Ranged` → BRIDGE to the existing enemy-projectile seam by writing the SAME
///   `ActorActionMessage::Ranged` the flat `frame.fire` resolver emits, so the
///   mature `spawn_enemy_projectiles_from_brain_actions` consumer (body-side
///   fire-rate, recoil, muzzle, visual kind) fires the shot unchanged. The shot's
///   direction is SAMPLED LIVE from the owner's current `fire` intent at THIS event
///   frame (option A — a moveset shot still tracks a strafing target, unlike a
///   facing-locked `MovePlayback`); with no live intent it falls back to forward.
pub fn dispatch_move_events(
    mut events: MessageReader<MoveEventMessage>,
    positions: Query<&ae::BodyKinematics>,
    ranged_owners: Query<(
        &ambition_characters::brain::action_set::ActionSet,
        &ActorControl,
    )>,
    mut sfx: MessageWriter<SfxMessage>,
    mut actions: MessageWriter<ActorActionMessage>,
) {
    for ev in events.read() {
        match &ev.kind {
            MoveEventKind::Sfx { cue } => {
                let pos = positions
                    .get(ev.owner)
                    .map(|k| k.pos)
                    .unwrap_or(ae::Vec2::ZERO);
                sfx.write(SfxMessage::Play {
                    id: SfxId::new(cue),
                    pos,
                });
            }
            MoveEventKind::Effect(effect) => {
                // Bridge to the content-technique seam by the effect KEY, and
                // thread the opaque `effect.params` through the `Special`
                // channel (A1 / R2.2) so the keyed technique can hydrate its own
                // typed params. A paramless effect carries the empty default, so
                // every existing content-const technique stays byte-identical.
                actions.write(ActorActionMessage {
                    actor: ev.owner,
                    request: ActionRequest::Special {
                        spec: SpecialActionSpec::Special(effect.key.clone()),
                        params: effect.params.clone(),
                    },
                });
            }
            MoveEventKind::Ranged => {
                // The owner's ranged CAPABILITY + LIVE aim supply the concrete shot;
                // the move stays content-free.
                let Ok((actions_set, control)) = ranged_owners.get(ev.owner) else {
                    continue;
                };
                let Some(spec) = actions_set.ranged else {
                    continue; // owner has no ranged weapon — the move fires nothing
                };
                let kin = positions.get(ev.owner).ok();
                let origin = kin.map(|k| k.pos).unwrap_or(ae::Vec2::ZERO);
                // Sample the owner's live aim at the fire frame; fall back to forward
                // (controlled-body-local +x = the body's facing direction).
                let (dir, dir_policy) = match control.0.fire {
                    Some(req) => (req.dir, req.dir_policy),
                    None => (
                        ae::Vec2::new(1.0, 0.0),
                        ae::GameplayFramePolicy::ControlledBodyLocal,
                    ),
                };
                actions.write(ActorActionMessage {
                    actor: ev.owner,
                    request: ActionRequest::Ranged {
                        spec,
                        origin,
                        dir,
                        dir_policy,
                    },
                });
            }
        }
    }
}

/// Project a `MovesetMelee` body's live [`MovePlayback`] into its [`BodyMelee`]
/// read-model so every existing consumer — the actor anim index, the
/// view/telegraph index, the HUD, the melee integration tests — keeps working
/// unchanged after melee moved onto the moveset. The move's Active window(s) drive
/// a synthesized `MeleeSwing` whose phase (Startup/Active/Recovery) and elapsed
/// mirror the move; the real hitboxes/damage are owned by
/// [`advance_move_playback`], so this writes NO gameplay — it is purely the
/// read-model the flat `BodyMelee` swing used to publish. A body with no live move
/// has its projected swing cleared (its cooldown floors still tick in
/// `advance_body_melee`).
///
/// Runs AFTER `advance_move_playback` (so `t` is current) and after
/// `advance_body_melee` (which skips `MovesetMelee` bodies' swing logic), making
/// this the SOLE writer of a `MovesetMelee` body's swing.
pub fn project_moveset_melee_to_body_melee(
    mut bodies: Query<(Option<&MovePlayback>, &mut BodyMelee), With<MovesetMelee>>,
) {
    for (playback, mut melee) in &mut bodies {
        // Only a MELEE swing move projects a swing — the base `"attack"` AND its
        // directional variants (`attack_up` / `attack_air_down` / …). A body's
        // ranged shot (`"ranged"`) or a special as a moveset move is ALSO
        // `MovesetMelee`, and those are NOT swings — projecting one would publish a
        // phantom `BodyMelee.swing` the movement pipeline reads as "mid-attack",
        // freezing a firing/special-ing body. Match the melee verb family.
        match playback {
            Some(pb) if is_melee_swing_move(&pb.spec.id) => {
                melee.swing = Some(synth_swing_from_move(pb))
            }
            _ => melee.swing = None,
        }
    }
}

/// Whether a move id is a melee swing (the `"attack"` verb family) versus a
/// ranged shot or a content special. The directional derive names every swing
/// `attack` / `attack_<dir>`, so the melee family is exactly that prefix.
fn is_melee_swing_move(id: &str) -> bool {
    id == ATTACK_VERB || id.starts_with("attack_")
}

/// Build the read-model `MeleeSwing` for a live move: startup = first Active
/// window start, active = span from first Active start to last Active end
/// (covers multi-hit combos), recovery = remainder. Only the timing is
/// meaningful — every geometry/impulse field is inert (the real strike is the
/// moveset's own hitbox), so the derived phase answers is_active/is_winding_up
/// exactly as the flat swing did.
fn synth_swing_from_move(pb: &MovePlayback) -> MeleeSwing {
    let spec = &pb.spec;
    let actives: Vec<&MoveWindow> = spec
        .windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active))
        .collect();
    let (startup, active) = match (actives.first(), actives.last()) {
        (Some(first), Some(last)) => (first.start_s, (last.end_s - first.start_s).max(0.0)),
        // A move with no Active window (a pure sustain/telegraph) reads as all
        // windup until it ends — still "swinging" for the anim tint.
        _ => (spec.duration_s, 0.0),
    };
    let recovery = (spec.duration_s - startup - active).max(0.0);
    let attack_spec = AttackSpec {
        intent: AttackIntent::Forward,
        startup_seconds: startup,
        active_seconds: active,
        recovery_seconds: recovery,
        hitbox_offset: ae::Vec2::ZERO,
        hitbox_half_size: ae::Vec2::ZERO,
        self_impulse: ae::Vec2::ZERO,
        knockback: ae::Vec2::ZERO,
        damage_kind: DamageKind::Slash,
        can_pogo: false,
        damage_override: None,
    };
    let mut swing = MeleeSwing::new(attack_spec);
    swing.elapsed = pb.t;
    swing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::events::HitEvent;
    use crate::combat::hitbox::apply_hitbox_damage;
    use crate::world::physics::DebrisBurstMessage;
    use ambition_sfx::SfxMessage;
    use ambition_vfx::vfx::VfxMessage;
    use bevy::prelude::*;

    #[test]
    fn prefab_registry_expands_sword_slash_from_simple_melee_with_zero_new_code() {
        // A2 / R2.3: `sword_slash` is the `simple_melee` prefab + params, minted
        // by name at roster install — no bespoke builder.
        let reg = MovePrefabRegistry::with_engine_prefabs();
        assert!(!reg.is_empty());
        let params = ambition_entity_catalog::ParamValue::parse(
            "(windup_s: 0.2, active_s: 0.08, recover_s: 0.3, damage: 4, reach_px: 60.0)",
        )
        .unwrap();
        let sword = reg
            .expand("simple_melee", &params, "sword_slash")
            .expect("simple_melee expands");
        assert_eq!(
            sword.id, "sword_slash",
            "expand renames to the roster move id"
        );
        // The authored damage/reach flowed into the Active window's hit volume.
        let active = sword
            .windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("charge has an Active window");
        assert_eq!(active.volumes.len(), 1);
        assert_eq!(active.volumes[0].damage, 4);
        assert!((sword.duration_s - 0.58).abs() < 1e-5, "0.2+0.08+0.3");
    }

    #[test]
    fn prefab_registry_rejects_unknown_key_and_bad_params() {
        let reg = MovePrefabRegistry::with_engine_prefabs();
        let empty = ambition_entity_catalog::ParamValue::default();
        assert!(
            reg.expand("not_a_prefab", &empty, "x").is_err(),
            "typo'd key"
        );
        // Wrong type for a field fails at expand (install) time.
        let bad = ambition_entity_catalog::ParamValue::parse("(damage: \"lots\")").unwrap();
        assert!(reg.expand("simple_melee", &bad, "x").is_err(), "bad params");
        // Empty params hydrate to the prefab defaults (every field defaults).
        assert!(reg.expand("simple_charge", &empty, "smash").is_ok());
    }

    #[test]
    fn authored_melee_adapter_matches_the_simple_melee_prefab() {
        // The MeleeActionSpec path and the prefab produce the same move for the
        // same timeline — the adapter is byte-identical to the generalized core.
        use ambition_characters::brain::action_set::SwipeSpec;
        let spec = MeleeActionSpec::Swipe(SwipeSpec {
            windup_s: 0.15,
            active_s: 0.1,
            recover_s: 0.18,
            damage: 2,
            reach_px: 40.0,
        });
        let via_adapter = attack_move_from_melee(&spec);
        let via_prefab = simple_melee(&SimpleMeleeParams {
            windup_s: 0.15,
            active_s: 0.1,
            recover_s: 0.18,
            damage: 2,
            reach_px: 40.0,
            knockback: 120.0,
        });
        assert_eq!(via_adapter, via_prefab);
    }

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

    /// B1 (fable review §B1): a moveset volume's authored offset is BODY-LOCAL
    /// (side, down); the spawned `FollowOwner` hitbox must rotate it into the
    /// owner's gravity frame at spawn, so the SAME move lands its box in the same
    /// BODY-relative place under every gravity. Regression guard for the old
    /// screen-frame spawn: an unrotated offset put an above-the-head strike into
    /// the effective ceiling under sideways/inverted gravity, forking against the
    /// gravity-aware player melee path.
    #[test]
    fn moveset_hitboxes_spawn_in_the_owner_gravity_frame() {
        // Authored body-local rect: forward (side +28) AND above the head
        // (down −20), non-square half so a 90° rotation is observable.
        fn overhead_swat() -> MoveSpec {
            let doc = ambition_entity_catalog::EntityCatalogDoc::parse(
                r#"(
                    schema_version: 1,
                    entities: [(
                        id: "seed",
                        contracts: (moveset: Some((
                            verbs: {"attack": "overhead"},
                            moves: [(
                                id: "overhead",
                                clip: (clip: "slash", fallbacks: ["idle"]),
                                duration_s: 0.68,
                                windows: [
                                    (start_s: 0.0, end_s: 0.28, tag: Startup, volumes: []),
                                    (start_s: 0.28, end_s: 0.36, tag: Active, volumes: [
                                        (shape: Rect(offset: (28.0, -20.0), half_extents: (16.0, 12.0)),
                                         damage: 2, knockback: 40.0),
                                    ]),
                                    (start_s: 0.36, end_s: 0.68, tag: Recovery, volumes: []),
                                ],
                                events: [],
                            )],
                        ))),
                    )],
                )"#,
            )
            .unwrap();
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

        // Spawn under `gravity` (facing +1), run into the 0.28–0.36 active window,
        // and read the live `FollowOwner` hitbox's world-frame offset + half.
        fn spawn_and_read(gravity: ae::Vec2) -> (ae::Vec2, ae::Vec2) {
            let (mut app, _victim) = app_with_victim();
            app.insert_resource(crate::physics::GravityField { dir: gravity });
            spawn_attacker(
                &mut app,
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(15.0, 24.0),
                overhead_swat(),
            );
            run_seconds(&mut app, 0.31); // t ≈ 0.32, inside the active window
            let mut state = app.world_mut().query::<&Hitbox>();
            let hb = state
                .iter(app.world())
                .next()
                .expect("active window spawns the volume");
            match hb.anchor {
                HitboxAnchor::FollowOwner { local_offset } => (local_offset, hb.half_extent),
                _ => panic!("a moveset volume must anchor FollowOwner"),
            }
        }

        let authored_local = ae::Vec2::new(28.0, -20.0); // facing +1
        let authored_half = ae::Vec2::new(16.0, 12.0);
        for dir in [
            ae::Vec2::new(0.0, 1.0),  // down (baseline)
            ae::Vec2::new(1.0, 0.0),  // right
            ae::Vec2::new(0.0, -1.0), // up
            ae::Vec2::new(-1.0, 0.0), // left
        ] {
            let (world_offset, world_half) = spawn_and_read(dir);
            let frame = ae::AccelerationFrame::new(dir);
            // The stored WORLD offset, read back into the BODY frame, is invariant
            // across gravities — the symmetry property an unrotated spawn breaks.
            let recovered = frame.to_local(world_offset);
            assert!(
                (recovered - authored_local).length() < 1e-3,
                "dir {dir:?}: the body-local strike offset must be gravity-invariant; \
                 got {recovered:?}, want {authored_local:?}"
            );
            // The half-extent rotates too: (16,12) → (12,16) at 90°.
            let expected_half = frame.to_world_half(authored_half);
            assert!(
                (world_half - expected_half).length() < 1e-3,
                "dir {dir:?}: half-extent must rotate with gravity; got {world_half:?}, \
                 want {expected_half:?}"
            );
        }
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
        assert!(
            doc.validate().is_empty(),
            "the two-hit combo is well-formed"
        );
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

    /// A held "beam": a 0.30s window that SUSTAINS an `Effect` every active frame.
    fn beam_move() -> MoveSpec {
        let doc = ambition_entity_catalog::EntityCatalogDoc::parse(
            r#"(
                schema_version: 1,
                entities: [(
                    id: "caster",
                    contracts: (moveset: Some((
                        verbs: {"special": "beam"},
                        moves: [(
                            id: "beam",
                            clip: (clip: "special", fallbacks: ["idle"]),
                            duration_s: 0.40,
                            windows: [
                                (start_s: 0.0, end_s: 0.30, tag: Active, volumes: [],
                                 sustain_effect: Some((key: "beam_tick"))),
                                (start_s: 0.30, end_s: 0.40, tag: Recovery, volumes: []),
                            ],
                        )],
                    ))),
                )],
            )"#,
        )
        .unwrap();
        assert!(doc.validate().is_empty());
        doc.entity("caster")
            .unwrap()
            .contracts
            .moveset
            .as_ref()
            .unwrap()
            .move_for_verb("special")
            .unwrap()
            .clone()
    }

    /// The HELD-special primitive (fable review §A1, the shape the boss fold needs):
    /// a window carrying a `sustain_effect` emits its `Effect` EVERY frame it is
    /// active (not one-shot), and STOPS the frame the window ends — so a consuming
    /// technique gets the continuous "active this tick" signal the boss's
    /// apple_rain-style specials run on. Pins the per-frame sustain.
    #[test]
    fn a_sustained_effect_window_emits_its_effect_every_active_frame() {
        let (mut app, _victim) = app_with_victim();
        let _caster = spawn_attacker(
            &mut app,
            ae::Vec2::new(400.0, 100.0), // far from the victim — no hits, just the sustain
            ae::Vec2::new(15.0, 24.0),
            beam_move(),
        );
        // Run PAST the sustain window (0.30s) but within the move (0.40s).
        run_seconds(&mut app, 0.36);
        let cap = app.world().resource::<Captured>();
        let beam_ticks = cap
            .events
            .iter()
            .filter(
                |e| matches!(&e.kind, MoveEventKind::Effect(effect) if effect.key == "beam_tick"),
            )
            .count();
        // ~0.30s / 0.016 ≈ 18 active frames; robustly many, and it stopped (the
        // move is 0.40s but the sustain window ended at 0.30s → not every frame).
        assert!(
            (15..=19).contains(&beam_ticks),
            "the sustain fired once per active frame (~18), got {beam_ticks}"
        );
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
            crate::features::CenteredAabb::new(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(15.0, 24.0),
            ),
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

    /// A move authoring `start_impulse` lunges the body toward its facing at
    /// trigger — the self-motion the flat directional swings applied at
    /// `start_attack`, now move DATA the player-melee fold rides.
    #[test]
    fn a_move_start_impulse_lunges_the_body_toward_facing() {
        let mut app = App::new();
        app.add_message::<MoveEventMessage>();
        app.init_resource::<WorldTime>();
        app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
        app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
        app.add_systems(Update, trigger_moveset_moves);
        let mv = MoveSpec {
            id: ATTACK_VERB.into(),
            clip: ClipBinding {
                clip: "x".into(),
                fallbacks: vec![],
            },
            duration_s: 0.3,
            windows: vec![],
            events: vec![],
            gates: Default::default(),
            start_impulse: Some((150.0, 0.0)),
        };
        let mut verbs = std::collections::BTreeMap::new();
        verbs.insert(ATTACK_VERB.to_string(), ATTACK_VERB.to_string());
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        let body = app
            .world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos: ae::Vec2::ZERO,
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(28.0, 46.0),
                    facing: -1.0,
                },
                ActorFaction::Enemy,
                ActorMoveset(MovesetContract {
                    verbs,
                    moves: vec![mv],
                }),
                ActorControl(frame),
            ))
            .id();
        app.update();
        let vel = app.world().get::<ae::BodyKinematics>(body).unwrap().vel;
        // facing = -1 → forward is -x; default gravity → no rotation.
        assert!(
            (vel.x + 150.0).abs() < 1.0,
            "the move lunged the body toward its facing, vel={vel:?}"
        );
        assert!(
            vel.y.abs() < 1.0,
            "a horizontal lunge adds no vertical velocity, vel={vel:?}"
        );
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
                kind: MoveEventKind::Effect(EffectRef {
                    key: "pca_glider".into(),
                    // A1: authored params must SURVIVE the bridge so the keyed
                    // technique can hydrate them.
                    params: ambition_entity_catalog::ParamValue::parse("(rise: 320.0)")
                        .expect("param RON parses"),
                }),
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
        assert_eq!(
            acts.len(),
            1,
            "the Effect event bridged to one Special action"
        );
        assert_eq!(acts[0].actor, owner);
        let ActionRequest::Special { spec, params } = &acts[0].request else {
            panic!("the Effect event bridged to a Special action");
        };
        assert!(matches!(spec, SpecialActionSpec::Special(k) if k == "pca_glider"));
        // The authored params rode through the bridge and hydrate on the far
        // side (the first real consumer — a G3 limb technique / demo move —
        // reads them exactly this way).
        #[derive(serde::Deserialize)]
        struct GliderParams {
            rise: f32,
        }
        let hydrated: GliderParams = params.hydrate().expect("params hydrate");
        assert_eq!(
            hydrated.rise, 320.0,
            "authored params survived the dispatch"
        );
    }

    /// Ranged subsumption (option A): a `MoveEventKind::Ranged` fire event BRIDGES to
    /// the SAME `ActorActionMessage::Ranged` the flat `frame.fire` resolver emits —
    /// carrying the owner's authored `ActionSet.ranged` spec and SAMPLING its LIVE
    /// aim at the event frame — so the existing enemy-projectile consumer fires the
    /// shot unchanged and a moveset shot still tracks a strafing target.
    #[test]
    fn move_event_dispatch_bridges_ranged_to_a_live_aimed_shot() {
        use ambition_characters::actor::control::ActorFireRequest;
        use ambition_characters::brain::action_set::{ActionSet, RangedActionSpec};
        use ambition_characters::brain::{ActorActionMessage, ActorControl};
        let mut app = App::new();
        app.add_message::<MoveEventMessage>();
        app.add_message::<SfxMessage>();
        app.add_message::<ActorActionMessage>();
        app.add_systems(Update, dispatch_move_events);

        let mut control = ActorControl::default();
        // Live aim this frame: a world-space up-right shot toward a strafing target.
        control.0.fire = Some(ActorFireRequest::world_space(
            ae::Vec2::new(0.6, -0.8),
            240.0,
        ));
        let owner = app
            .world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos: ae::Vec2::new(100.0, 50.0),
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(16.0, 24.0),
                    facing: 1.0,
                },
                ActionSet {
                    ranged: Some(RangedActionSpec::Bolt {
                        speed: 240.0,
                        damage: 3,
                    }),
                    ..Default::default()
                },
                control,
            ))
            .id();
        app.world_mut()
            .resource_mut::<Messages<MoveEventMessage>>()
            .write(MoveEventMessage {
                owner,
                move_id: "fire".into(),
                kind: MoveEventKind::Ranged,
            });
        app.update();

        let acts: Vec<ActorActionMessage> = app
            .world_mut()
            .resource_mut::<Messages<ActorActionMessage>>()
            .drain()
            .collect();
        assert_eq!(
            acts.len(),
            1,
            "the Ranged event bridged to one Ranged action"
        );
        match &acts[0].request {
            ActionRequest::Ranged {
                spec, origin, dir, ..
            } => {
                assert!(matches!(spec, RangedActionSpec::Bolt { damage: 3, .. }));
                assert_eq!(*origin, ae::Vec2::new(100.0, 50.0), "origin = owner pos");
                assert_eq!(*dir, ae::Vec2::new(0.6, -0.8), "dir SAMPLED from live aim");
            }
            other => panic!("expected ActionRequest::Ranged, got {other:?}"),
        }
    }

    /// Ranged subsumption slice 2: `build_actor_moveset` folds `ActionSet.ranged`
    /// into a `"ranged"`-verb fire move (Startup → fire event → Recovery, no hit
    /// volume), and `trigger_moveset_moves` starts it on a `frame.fire` intent — the
    /// same trigger seam melee/specials use.
    #[test]
    fn a_fire_intent_triggers_the_ranged_move() {
        use ambition_characters::actor::control::ActorFireRequest;
        use ambition_characters::brain::action_set::RangedActionSpec;
        use ambition_characters::brain::ActorControl;

        let contract = build_actor_moveset(
            None,
            None,
            Some(&RangedActionSpec::Bolt {
                speed: 240.0,
                damage: 3,
            }),
        )
        .expect("a ranged weapon → a moveset with a fire move");
        let fire = contract
            .move_for_verb(RANGED_VERB)
            .expect("the ranged verb maps to the fire move");
        assert_eq!(fire.id, RANGED_VERB);
        assert!(
            fire.windows.iter().all(|w| w.volumes.is_empty()),
            "a shot carries no melee hit volume — the projectile is the damage"
        );
        assert_eq!(
            fire.events
                .iter()
                .filter(|e| e.kind == MoveEventKind::Ranged)
                .count(),
            1,
            "exactly one fire event"
        );

        let mut app = App::new();
        app.add_systems(Update, trigger_moveset_moves);
        let mut control = ActorControl::default();
        control.0.fire = Some(ActorFireRequest::world_space(
            ae::Vec2::new(1.0, 0.0),
            240.0,
        ));
        let body = app
            .world_mut()
            .spawn((
                ActorMoveset(contract),
                control,
                ae::BodyKinematics {
                    pos: ae::Vec2::ZERO,
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(16.0, 24.0),
                    facing: 1.0,
                },
            ))
            .id();
        app.update();
        let pb = app
            .world()
            .get::<MovePlayback>(body)
            .expect("the fire intent started the ranged move");
        assert_eq!(pb.spec.id, RANGED_VERB);
    }

    /// Regression (ranged-fold): a body that is BOTH `MovesetMelee` and playing its
    /// `"ranged"` (or any non-`"attack"`) move must NOT get a phantom `BodyMelee.swing`
    /// — otherwise the movement pipeline reads it as "mid-attack" and freezes the
    /// firing body in place (this froze the PCA's chase in `actor_phase_split`). Only
    /// the `"attack"` move projects a swing.
    #[test]
    fn a_ranged_move_does_not_project_a_phantom_melee_swing() {
        use ambition_characters::brain::action_set::{
            MeleeActionSpec, RangedActionSpec, SwipeSpec,
        };
        // Same body carries both a melee AND a ranged move (both verbs).
        let contract = build_actor_moveset(
            None,
            Some(&MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            Some(&RangedActionSpec::Rock {
                speed: 300.0,
                damage: 1,
            }),
        )
        .expect("melee + ranged → a moveset");
        let fire = contract.move_for_verb(RANGED_VERB).unwrap().clone();
        let attack = contract.move_for_verb(ATTACK_VERB).unwrap().clone();

        let mut app = App::new();
        app.add_systems(Update, project_moveset_melee_to_body_melee);

        // Playing the RANGED move → no swing (the body isn't attacking).
        let firing = app
            .world_mut()
            .spawn((
                MovesetMelee,
                BodyMelee::default(),
                MovePlayback::new(fire, 1.0),
            ))
            .id();
        // Playing the ATTACK move → a swing (the read-model the flat swing published).
        let swinging = app
            .world_mut()
            .spawn((
                MovesetMelee,
                BodyMelee::default(),
                MovePlayback::new(attack, 1.0),
            ))
            .id();
        app.update();
        assert!(
            app.world()
                .get::<BodyMelee>(firing)
                .unwrap()
                .swing
                .is_none(),
            "a firing body must not read as mid-swing"
        );
        assert!(
            app.world()
                .get::<BodyMelee>(swinging)
                .unwrap()
                .swing
                .is_some(),
            "the attack move still projects its swing read-model"
        );
    }
}
