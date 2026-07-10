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
    Commands, Component, Entity, Message, MessageReader, MessageWriter, Query, Res, With,
};

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use ambition_entity_catalog::{
    AttackDir, ClipBinding, EffectRef, HitVolume, MoveEvent, MoveEventKind, MoveSpec, MoveWindow,
    MovesetContract, VolumeShape, WindowTag,
};
use ambition_time::ProperTimeScale;

use super::components::{ActorFaction, BodyMelee, MeleeSwing};
use super::hitbox::{Hitbox, HitboxAnchor, HitboxHits};
use crate::{hit_side_from_actor_faction, AttackIntent, AttackSpec};
use ambition_characters::brain::action_set::{
    ActionRequest, MeleeActionSpec, RangedActionSpec, SpecialActionSpec,
};
use ambition_characters::brain::{ActorActionMessage, ActorControl};
use ambition_entity_catalog::placements::DamageKind;
use ambition_sfx::{SfxId, SfxMessage};
use ambition_time::WorldTime;

/// The canonical verb id a body's basic melee swing binds to in its moveset.
pub const ATTACK_VERB: &str = "attack";

/// [`HitVolume::vfx`] tags the move runtime knows (§7.2): the sweeping slash
/// arc and the grounded down-tilt's horizontal poke. Unknown tags draw the arc
/// (never a silent drop — a tagged volume asked for presentation).
pub const SLASH_ARC_VFX: &str = "slash_arc";
pub const SLASH_POKE_VFX: &str = "slash_poke";

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
        // The authored-melee adapter keeps the engine-default swing presentation
        // (byte-parity with the pre-CM5 path); per-move sfx/vfx is authored on
        // the prefab RON rows, not synthesized here.
        swing_sfx: None,
        swing_vfx: None,
    })
}

/// Params for the [`simple_melee`] engine prefab (A2 / R2.3) — a forward swing
/// as authored DATA. Every field defaults, so a roster prefab row omits what it
/// doesn't tune (`prefab: "simple_melee"` with empty params = a default jab).
/// `sword_slash` is literally this prefab + params, zero new code.
#[derive(Debug, Clone, serde::Deserialize)]
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
    /// CM5: the SFX cue this swing fires at its Active edge. `None` = the engine
    /// default (`SWING_SFX_CUE`), so an unauthored row is byte-parity; an
    /// authored row makes the move sound distinct (a heavy smash thuds, a jab
    /// snaps) with zero code.
    #[serde(default)]
    pub swing_sfx: Option<String>,
    /// CM5: an OPTIONAL cosmetic burst id (`ambition_vfx::move_vfx_kind`
    /// vocabulary) emitted at the Active edge on top of the slash arc — `None` =
    /// no extra burst (parity). Lets a launcher `"starburst"`, a smash
    /// `"shockwave"`. A typo is a startup validation error, never silent.
    #[serde(default)]
    pub swing_vfx: Option<String>,
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
            swing_sfx: None,
            swing_vfx: None,
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
        // Prefab swings are flat-knockback; percent growth is authored on
        // explicit RON volumes (CM1) — a prefab growth param can follow.
        kb_growth: 0.0,
        launch_dir: None,
        // A plain swing lands no on-hit technique; directional variants (a
        // down-air pogo) author `on_hit` per-move (R2.5 player-melee fold).
        on_hit: None,
        // A bladed swing (§7.1/§7.2): draws the slash arc from the spawned
        // volume, and — when the owner's sprite manifest authors a hit polygon
        // for this move's clip — swings THAT authored blade instead of this
        // synthetic rect (the rect is the fallback for unmanifested bodies).
        vfx: Some(SLASH_ARC_VFX.to_string()),
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
        events: {
            // The swing SFX at the Active edge (authored cue or the engine
            // default), plus — if the row authored one — a cosmetic burst at the
            // same edge (CM5 per-move presentation).
            let mut events = vec![MoveEvent {
                at_s: windup,
                kind: MoveEventKind::Sfx {
                    cue: p
                        .swing_sfx
                        .clone()
                        .unwrap_or_else(|| SWING_SFX_CUE.to_string()),
                },
            }];
            if let Some(effect) = &p.swing_vfx {
                events.push(MoveEvent {
                    at_s: windup,
                    kind: MoveEventKind::Vfx {
                        effect: effect.clone(),
                    },
                });
            }
            events
        },
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
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
        smash_charge_mult: 1.0,
    }
}

/// Params for the [`simple_charge`] engine prefab (A2) — a hold-then-release
/// heavy hit the demos need (SMB1's crouch-charge, a wind-up smash). The
/// `charge_s` Startup window is the hold; the `active_s` Active window lands one
/// forward Rect hit sized from `reach_px`, then `recover_s` settle.
#[derive(Debug, Clone, serde::Deserialize)]
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
    /// CM3 smash-charge payoff: the multiplier a fully-charged release applies to
    /// damage + knockback (`1.0 → smash_charge_mult` by charge fraction). DEFAULT
    /// `1.0` = no scaling (parity); a smash roster authors e.g. `2.0`.
    #[serde(default = "scp_charge_mult")]
    pub smash_charge_mult: f32,
    /// CM5: the release SFX cue (`None` = engine default). See
    /// [`SimpleMeleeParams::swing_sfx`].
    #[serde(default)]
    pub swing_sfx: Option<String>,
    /// CM5: an optional cosmetic burst at the Active edge (`None` = parity). See
    /// [`SimpleMeleeParams::swing_vfx`].
    #[serde(default)]
    pub swing_vfx: Option<String>,
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
fn scp_charge_mult() -> f32 {
    1.0
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
            smash_charge_mult: scp_charge_mult(),
            swing_sfx: None,
            swing_vfx: None,
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
        kb_growth: 0.0,
        launch_dir: None,
        on_hit: None,
        // A charge is a bladed strike too — same slash + authored-blade rules.
        vfx: Some(SLASH_ARC_VFX.to_string()),
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
        events: {
            let mut events = vec![MoveEvent {
                at_s: charge,
                kind: MoveEventKind::Sfx {
                    cue: p
                        .swing_sfx
                        .clone()
                        .unwrap_or_else(|| SWING_SFX_CUE.to_string()),
                },
            }];
            if let Some(effect) = &p.swing_vfx {
                events.push(MoveEvent {
                    at_s: charge,
                    kind: MoveEventKind::Vfx {
                        effect: effect.clone(),
                    },
                });
            }
            events
        },
        gates: Default::default(),
        start_impulse: None,
        // CM3: the charge move's payoff — the authored release multiplier.
        smash_charge_mult: p.smash_charge_mult,
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
        // CM5: reject an unresolvable presentation id (a `Vfx`/`Sfx` typo) at
        // expansion time — the SAME startup-validation gate a bad prefab key or
        // param hits, so authored sound/vfx typos never survive to a silent
        // missing effect. The cosmetic-vfx vocabulary lives in `ambition_vfx`;
        // inject it (entity_catalog can't depend on the render-adjacent crate).
        let problems = spec.presentation_problems(|id| ambition_vfx::move_vfx_kind(id).is_some());
        if !problems.is_empty() {
            return Err(problems.join("; "));
        }
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
                    v.on_hit = Some(EffectRef::new(crate::on_hit::POGO_BOUNCE_KEY));
                }
                // The grounded down-tilt reads as a kneeling forward poke, not a
                // sweep (mirrors the bespoke path's `slash_kind`: Down → Poke);
                // every other direction keeps the base swing's arc.
                if matches!(dir, Dir::Down) && grounded && v.vfx.is_some() {
                    v.vfx = Some(SLASH_POKE_VFX.to_string());
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
    /// CM4: this move CONNECTED with a victim. Set by the hit-resolution side
    /// (`mark_move_playback_landed_hits` + the volume resolver) and read by the
    /// cancel conditions (`OnHit`/`OnWhiff`) — the combo-confirm fact.
    pub landed_hit: bool,
    /// Live hitbox entity per entered-but-not-exited Active window index.
    ///
    /// A CACHE, not the authority. Its authority is `(t, window)`: the box exists
    /// exactly while the owner's clock is inside the window, and
    /// [`retire_orphaned_strike_volumes`] enforces that against the world every
    /// frame. That matters because a rollback (`ambition_runtime::snapshot`) rebuilds
    /// this component from a blob, and a blob cannot carry an `Entity`.
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
    /// **Resume a move mid-flight**, for `ambition_runtime::snapshot`'s
    /// `SnapshotResolve`.
    ///
    /// The blob carries the CHOICE — which move, how far in, did it land — and the
    /// `MoveSpec` is resolved back out of the owner's authored `ActorMoveset`. The
    /// `live_boxes` cache comes back empty, which is exactly right: a blob cannot
    /// carry an `Entity` (N3.1 decision 2), and it does not have to.
    /// [`retire_orphaned_strike_volumes`] despawns the boxes the rewound tick left
    /// standing, and the window's own `(inside, not-live)` arm re-spawns whatever the
    /// restored clock says should exist. The box's existence is DERIVED from
    /// `(t, window)`, so restoring `t` restores the box.
    pub fn resumed(spec: MoveSpec, facing: f32, t: f32, landed_hit: bool) -> Self {
        let mut pb = Self::new_at(spec, facing, t);
        pb.landed_hit = landed_hit;
        pb
    }

    pub fn new_at(spec: MoveSpec, facing: f32, t0: f32) -> Self {
        let t0 = t0.clamp(0.0, spec.duration_s);
        let fired: Vec<bool> = spec.events.iter().map(|ev| ev.at_s <= t0).collect();
        Self {
            spec,
            facing,
            t: t0,
            landed_hit: false,
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

/// Which move window a spawned strike volume belongs to.
///
/// The volume's existence is DERIVED from `(owner's playback t, window)`. This marker
/// is what lets [`retire_orphaned_strike_volumes`] check that derivation against the
/// world without reading `MovePlayback`'s private cache — and without which a rollback
/// that rebuilds `MovePlayback` from a blob would strand every live box forever.
#[derive(bevy::prelude::Component, Clone, Copy, Debug)]
pub struct StrikeVolume {
    pub owner: Entity,
    pub window: usize,
}

/// **Despawn every strike volume whose owner's clock says it should not exist.**
///
/// Runs every frame, before [`advance_move_playback`], and is a no-op in the ordinary
/// case: the window's exit edge already despawned the box and dropped it from
/// `live_boxes`. It earns its keep when the two disagree, which happens when
/// `MovePlayback` is REPLACED rather than advanced:
///
/// - `ambition_runtime::snapshot::restore` rebuilds it from a blob (`MovePlayback::resumed`)
///   with an empty `live_boxes`. Without this system the boxes alive at the rewound-from
///   tick would leak, and `advance_move_playback` would spawn a second one beside each.
/// - Any future code that swaps a playback mid-move.
///
/// N3.1's rule, honoured: *"if restoring something requires a rebuild pass, the rebuild
/// must be the SAME system that maintains it per-frame (no restore-only code paths)."*
/// This is that system, and it runs whether or not anyone ever rolls back.
pub fn retire_orphaned_strike_volumes(
    mut commands: Commands,
    volumes: Query<(Entity, &StrikeVolume)>,
    owners: Query<&MovePlayback>,
) {
    // Sorted by entity-independent key: this despawns, and despawn order is not
    // observable, but the ITERATION must not depend on archetype layout for any
    // future side effect. `(owner index, window)` is stable within a tick.
    for (volume, mark) in &volumes {
        let alive = owners.get(mark.owner).is_ok_and(|pb| {
            pb.spec
                .windows
                .get(mark.window)
                .is_some_and(|w| w.start_s <= pb.t && pb.t < w.end_s)
        }) && owners
            .get(mark.owner)
            .is_ok_and(|pb| pb.live_boxes.iter().any(|(_, e)| *e == volume));
        if !alive {
            commands.entity(volume).despawn();
        }
    }
}

/// Advance every playing move by its owner's proper time; manage
/// window-scoped hitboxes; fire timed events; retire finished moves.
pub fn advance_move_playback(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    gravity: ambition_platformer_primitives::gravity::GravityCtx,
    mut events: MessageWriter<MoveEventMessage>,
    // §7.2: a vfx-tagged volume draws its slash FROM the spawned hitbox
    // geometry — one box drives damage AND presentation, so they can never
    // point different ways (the `spawn_melee_strike` invariant, restored onto
    // the moveset path).
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
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
        // §7.1: the owner's sprite catalog id (projected onto the combat-owned
        // `CombatTuning` at spawn; the home body carries none → the player
        // manifest root), resolving the authored per-animation blade polygon.
        Option<&super::components::CombatTuning>,
        &ae::BodyKinematics,
        Option<&ProperTimeScale>,
    )>,
) {
    for (owner, mut playback, faction, brain, config, kin, scale) in &mut players {
        let strike_faction = crate::targeting::effective_faction(*faction, brain);
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
                    // CM3: the smash-charge payoff. The scale interpolates
                    // `1.0 → smash_charge_mult` by the charge fraction reached at
                    // this release instant (`t`, the owner's clock), so a held
                    // smash lands harder than a tap. `1.0` (every non-charge move)
                    // leaves damage/knockback byte-identical — parity.
                    let charge_scale = pb.spec.charge_scale_at(t);
                    for volume in &window.volumes {
                        // §7.1: a vfx-tagged (bladed) volume prefers the owner's
                        // AUTHORED manifest hit polygon for this move's clip —
                        // the box you author and see in `debug-hitboxes` IS the
                        // gameplay damage box, restored onto the moveset path.
                        // Directional variants rebind `clip`, so `attack_up` /
                        // `attack_down` resolve their own rows the day they're
                        // authored. Resolved body-LOCAL (origin, facing +1,
                        // screen-down); the hitbox's own facing/frame_down
                        // mirror + rotate it at query time (`place_at`), the
                        // same math the bespoke path applied. `None` (no
                        // authored row / silent volume) falls back to the
                        // synthetic authored shape.
                        let manifest = volume.vfx.as_ref().and_then(|_| {
                            let clip = pb.spec.clip.clip.as_str();
                            let sprite_cid = config.and_then(|c| c.sprite_character_id.as_deref());
                            super::authored_volumes::authored_attack_volume(
                                sprite_cid,
                                clip,
                                ae::Vec2::ZERO,
                                kin.size,
                                1.0,
                                ae::Vec2::new(0.0, 1.0),
                            )
                        });
                        let (local, half_extent, shape) = match &manifest {
                            // The authored convex blade: body-local points; the
                            // hitbox anchors at the body and `place_at` mirrors
                            // + gravity-rotates the hull each query.
                            Some(ae::CombatVolume::Convex { points, bounds }) => (
                                ae::Vec2::ZERO,
                                bounds.half_size(),
                                Some(ae::VolumeShape::Convex {
                                    points: points.clone(),
                                }),
                            ),
                            // The authored bbox fallback: same spawn-time
                            // resolution as a synthetic Rect.
                            Some(vol) => {
                                let b = vol.bounds();
                                let c = b.center();
                                (ae::Vec2::new(c.x * pb.facing, c.y), b.half_size(), None)
                            }
                            None => match volume.shape {
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
                            },
                        };
                        let local_offset = body_frame.to_world(local);
                        // Axis-aligned extents rotate with the frame too (a
                        // circle's splat is rotation-invariant, so this is
                        // uniform).
                        let half_extent = body_frame.to_world_half(half_extent);
                        let hb = Hitbox {
                            owner,
                            source: hit_side_from_actor_faction(strike_faction),
                            anchor: HitboxAnchor::FollowOwner { local_offset },
                            half_extent,
                            shape,
                            facing: pb.facing,
                            // CM3: charge scaling folds onto the authored base —
                            // damage rounds, knockback scales linearly. Both are
                            // identity at `charge_scale == 1.0` (parity).
                            damage: ((volume.damage as f32) * charge_scale).round() as i32,
                            knockback_strength: volume.knockback * charge_scale,
                            // CM1: the smash-percent growth term rides the volume
                            // through to the victim-side scaling at overlap.
                            knockback_growth: volume.kb_growth,
                            // CM1: the authored launch direction rides the
                            // volume through to the victim-side resolver.
                            launch_dir: volume.launch_dir.map(|(x, y)| ae::Vec2::new(x, y)),
                            knock_x: 0.0,
                            frame_down,
                        };
                        // §7.2: the slash VFX rides the SAME resolved volume the
                        // damage does (the `spawn_melee_strike` invariant) —
                        // emitted once at the Active edge.
                        if let Some(tag) = &volume.vfx {
                            let kind = if tag == SLASH_POKE_VFX {
                                ambition_vfx::vfx::SlashKind::Poke
                            } else {
                                ambition_vfx::vfx::SlashKind::Arc
                            };
                            let b = hb.world_volume(kin.pos).bounds();
                            crate::util::emit_melee_slash(
                                &mut vfx,
                                b.center(),
                                b.half_size(),
                                kind,
                                b.center() - kin.pos,
                            );
                        }
                        // NO HitboxLifetime on purpose: the window's exit
                        // edge (owner proper time) is the despawn authority,
                        // not a wall-clock countdown.
                        let mut ec = commands.spawn((
                            hb,
                            HitboxHits::default(),
                            StrikeVolume {
                                owner,
                                window: w_idx,
                            },
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
pub fn attack_dir_from_axis(axis: ae::Vec2) -> AttackDir {
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
/// `"ranged"` verb. A body already playing a move refuses a new one — the move's
/// own duration IS the fire-rate gate — UNLESS the playing move authors a
/// `Cancelable` window covering this instant whose condition holds and whose
/// `into` names the request (CM4): then the live boxes tear down exactly as
/// natural completion does and the new move starts same-frame. `jump`/`dash`
/// entries END the move early on those edges — the normal locomotion path
/// (reading the SAME control frame this tick) performs the jump/dash itself;
/// no second dispatcher. An empty cancel timeline is byte-identical to the
/// pre-CM4 reject (the parity pin). Facing locks at trigger from the body's
/// kinematics (the Smash convention — a committed swing doesn't re-aim).
///
/// ONE trigger seam for every body (guardrail #1): the same system drives an
/// actor's melee, the PCA's signature move, a folded boss's pattern, and the
/// player's directional repertoire (R2.5). A body authoring only `"attack"`
/// resolves every direction to it — byte-identical to the pre-directional path.
pub fn trigger_moveset_moves(
    mut commands: Commands,
    gravity: ambition_platformer_primitives::gravity::GravityCtx,
    mut bodies: Query<(
        Entity,
        &ActorMoveset,
        &ActorControl,
        // Mutable so a move's authored `start_impulse` (self-motion) lands at
        // trigger — the move-start seam.
        &mut ae::BodyKinematics,
        // Grounded state selects tilt-vs-air variants. Absent on bare test
        // bodies → treated as grounded (immaterial: such bodies author only
        // the base `attack`, which every direction resolves to).
        Option<&ambition_engine_core::BodyGroundState>,
        // The playing move, if any — the CM4 cancel seam. `None` = the plain
        // trigger path.
        Option<&mut MovePlayback>,
    )>,
) {
    for (entity, moveset, control, mut kin, ground, playback) in &mut bodies {
        let frame = &control.0;
        let grounded = ground.map(|g| g.on_ground).unwrap_or(true);
        // Resolve the requested verb + the names the candidate answers to
        // (verb, class, resolved move id — the ONE cancel namespace).
        let (spec, verb_names): (_, &[&str]) = if frame.special_pressed {
            (moveset.0.move_for_verb("special"), &["special"])
        } else if frame.melee_pressed || frame.pogo_pressed {
            // A dedicated pogo press IS a down-air (the move carrying the pogo
            // on-hit technique); a plain melee press resolves by aim. When only
            // pogo is pressed, force Down so an aerial body reaches `attack_air_down`.
            let dir = if frame.pogo_pressed && !frame.melee_pressed {
                AttackDir::Down
            } else {
                attack_dir_from_axis(frame.attack_axis)
            };
            (
                moveset
                    .0
                    .move_for_directional_verb(ATTACK_VERB, dir, grounded),
                &[ATTACK_VERB, "any_attack"],
            )
        } else if frame.fire.is_some() {
            // A ranged intent (`frame.fire = Some(dir)`) starts the body's `"ranged"`
            // move; its fire event spawns the projectile, sampling live aim. The move
            // plays to completion before another starts (its duration is a cadence
            // gate; the body-side refire cooldown remains the hard rate floor).
            (moveset.0.move_for_verb(RANGED_VERB), &[RANGED_VERB])
        } else {
            (None, &[])
        };

        if let Some(mut pb) = playback {
            // CM4, locomotion escapes: a `jump`/`dash` edge inside a permitting
            // cancel window ENDS the move (early recovery-cancel); the verb
            // itself runs through the normal locomotion path this same tick.
            let loco = if frame.jump_pressed {
                Some("jump")
            } else if frame.dash_pressed {
                Some("dash")
            } else {
                None
            };
            if let Some(name) = loco {
                if pb.spec.cancel_permits(pb.t, pb.landed_hit, &[name]) {
                    for (_, e) in pb.live_boxes.drain(..) {
                        commands.entity(e).despawn();
                    }
                    commands.entity(entity).remove::<MovePlayback>();
                    continue;
                }
            }
            // CM4, move-into-move: the requested move starts same-frame iff a
            // cancel window covering `t` permits it under the hit-state
            // condition. Otherwise: today's reject, byte-identically.
            let Some(spec) = spec else { continue };
            let mut names: Vec<&str> = verb_names.to_vec();
            names.push(spec.id.as_str());
            if !pb.spec.cancel_permits(pb.t, pb.landed_hit, &names) {
                continue;
            }
            // Tear down exactly as natural completion does (the ONE teardown
            // path), then replace the playback — insert overwrites.
            for (_, e) in pb.live_boxes.drain(..) {
                commands.entity(e).despawn();
            }
            if let Some((ix, iy)) = spec.start_impulse {
                let local = ae::Vec2::new(ix * kin.facing, iy);
                let world_impulse =
                    ae::AccelerationFrame::new(gravity.dir_at(kin.pos)).to_world(local);
                kin.vel += world_impulse;
            }
            commands
                .entity(entity)
                .insert(MovePlayback::new(spec.clone(), kin.facing));
            continue;
        }

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

/// CM4: mark the attacker's playing move as CONNECTED when its strike resolves
/// onto a concrete victim. Pre-resolved victim events (`HitTarget::Actor` /
/// `HitTarget::Player`) are emitted only on physical overlap, so they ARE the
/// landing fact; `Volume` events are emitted every active tick regardless of
/// contact and are marked instead by the volume resolver
/// (`apply_feature_hit_events`) when a victim actually takes the hit. Reads the
/// shared `HitEvent` channel from its own reader position (the established
/// multi-consumer pattern on this channel).
pub fn mark_move_playback_landed_hits(
    mut events: MessageReader<crate::events::HitEvent>,
    mut playbacks: Query<&mut MovePlayback>,
) {
    for ev in events.read() {
        let Some(attacker) = ev.attacker else {
            continue;
        };
        if !matches!(
            ev.target,
            crate::events::HitTarget::Actor(_) | crate::events::HitTarget::Player(_)
        ) {
            continue;
        }
        if let Ok(mut pb) = playbacks.get_mut(attacker) {
            pb.landed_hit = true;
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
    mut vfx: MessageWriter<ambition_vfx::VfxMessage>,
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
            MoveEventKind::Vfx { effect } => {
                // CM5 per-move cosmetic burst: resolve the id against the
                // content-registered vocabulary and spawn it at the owner. A
                // typo can't reach here — `presentation_problems` rejects an
                // unresolvable id at startup — but stay robust if it somehow
                // does (skip, never panic on the RL-hot path).
                let Some(kind) = ambition_vfx::move_vfx_kind(effect) else {
                    continue;
                };
                let pos = positions
                    .get(ev.owner)
                    .map(|k| k.pos)
                    .unwrap_or(ae::Vec2::ZERO);
                vfx.write(ambition_vfx::VfxMessage::Explosion {
                    pos,
                    kind,
                    scale: 1.0,
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
mod tests;
