//! **The boss specials' snapshot codecs, registered by the crate that owns them.**
//!
//! `docs/planning/engine/netcode.md` N3.1: *"each sim crate registers its components'
//! serialization."* These eleven Technique states are sim state — a `fired_this_strike`
//! latch that survives a rollback is a strike that fires twice — and no crate below
//! `ambition_content` can name them. `SnapshotRegistry` is a resource, installed early
//! by `SnapshotRegistryPlugin`, so this crate reaches in and adds what it owns.
//!
//! That is the whole seam. It needed a resource, not a trait relocation: the codec
//! trait must live where `ambition_runtime` can implement it for `ambition_time` and
//! `ambition_engine_core` types, and those crates sit *below* every foundation crate
//! `ambition_content` names. Moving the trait down would trade one orphan-rule problem
//! for a worse one.

use ambition_runtime::snapshot::{
    put_bool, put_f32, put_u32, put_vec2, Reader, SnapshotCursor, SnapshotRegistry, SnapshotState,
};
use ambition_runtime::{snapshot_pod, snapshot_unit_enum};
use bevy::prelude::*;

use super::{
    AppleRainSpawnState, EchoFanState, ExplodingGradientState, EyeBeamState, GradientCascadeState,
    MinimaTrapState, ModeCollapseState, OverfitVolleyState, OverflowState, SaddlePointState,
    SeismicStompState,
};

/// Add every boss-special state to the registry. Called from
/// `BossSpecialContentPlugin::build`, after `SnapshotRegistryPlugin` has installed it.
pub(super) fn register(registry: &mut SnapshotRegistry) {
    registry.register_component::<EchoFanState>("echo_fan_state");
    registry.register_component::<SeismicStompState>("seismic_stomp_state");
    registry.register_component::<ExplodingGradientState>("exploding_gradient_state");
    registry.register_component::<OverflowState>("overflow_state");
    registry.register_component::<GradientCascadeState>("gradient_cascade_state");
    registry.register_component::<MinimaTrapState>("minima_trap_state");
    registry.register_component::<AppleRainSpawnState>("apple_rain_spawn_state");
    registry.register_component::<ModeCollapseState>("mode_collapse_state");
    registry.register_component::<EyeBeamState>("eye_beam_state");
    registry.register_component::<OverfitVolleyState>("overfit_volley_state");
    // A CURSOR: `SaddlePointState` holds two `Option<Entity>` hitbox handles. See below.
    registry.register_cursor::<SaddlePointState>("saddle_point_state");
}

// A `fired_this_strike` latch that survives a rollback is a strike that fires twice;
// a `spawn_index` that survives one is a minion that is never born.
snapshot_pod!(EchoFanState {
    fired_this_strike: bool,
});
snapshot_pod!(SeismicStompState {
    fired_this_strike: bool,
});
snapshot_pod!(ExplodingGradientState {
    fired_this_strike: bool,
});
snapshot_pod!(GradientCascadeState {
    fired_this_strike: bool,
    spawn_index: u32,
});
snapshot_pod!(MinimaTrapState {
    fired_this_strike: bool,
    spawn_index: u32,
});
snapshot_pod!(AppleRainSpawnState {
    spawn_accum: f32,
    spawn_index: u32,
});

/// `Option<f32>` / `Option<Vec2>` locks: the target a strike committed to. Rewinding
/// into a committed strike must rewind to the same commitment, or the beam bends.
impl SnapshotState for OverflowState {
    fn encode(&self, out: &mut Vec<u8>) {
        match self.locked_x {
            None => put_bool(out, false),
            Some(x) => {
                put_bool(out, true);
                put_f32(out, x);
            }
        }
        put_bool(out, self.fired_this_strike);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let locked_x = if r.bool()? { Some(r.f32()?) } else { None };
        Some(OverflowState {
            locked_x,
            fired_this_strike: r.bool()?,
        })
    }
}

macro_rules! locked_target_state {
    ($ty:ty) => {
        impl SnapshotState for $ty {
            fn encode(&self, out: &mut Vec<u8>) {
                match self.locked_target {
                    None => put_bool(out, false),
                    Some(p) => {
                        put_bool(out, true);
                        put_vec2(out, p);
                    }
                }
                put_bool(out, self.fired_this_strike);
            }
            fn decode(r: &mut Reader<'_>) -> Option<Self> {
                let locked_target = if r.bool()? { Some(r.vec2()?) } else { None };
                Some(Self {
                    locked_target,
                    fired_this_strike: r.bool()?,
                })
            }
        }
    };
}
locked_target_state!(ModeCollapseState);
locked_target_state!(EyeBeamState);

/// The volley's sampled aim points, in the order it took them. A `Vec`, so its order
/// IS its meaning.
impl SnapshotState for OverfitVolleyState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.samples.len() as u32);
        for s in &self.samples {
            put_vec2(out, *s);
        }
        put_f32(out, self.sample_accum);
        put_bool(out, self.fired_this_strike);
        put_bool(out, self.had_seed_sample);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let n = r.u32()?;
        let samples = (0..n).map(|_| r.vec2()).collect::<Option<Vec<_>>>()?;
        Some(OverfitVolleyState {
            samples,
            sample_accum: r.f32()?,
            fired_this_strike: r.bool()?,
            had_seed_sample: r.bool()?,
        })
    }
}

/// **`SaddlePointState` holds two `Option<Entity>` hitbox handles**, which N3.1
/// decision (2) forbids in sim state — the fourth such reference in the tree, after
/// `ActorTarget`, the mount cluster, and `MovePlayback.live_boxes`.
///
/// So it rides a `SnapshotCursor`: the axis clock and the strike latch rewind; the two
/// entity handles are left exactly where they are. That is sound only because the
/// component is spawned with the boss and never removed, so `restore` always has one to
/// apply the cursor to.
///
/// It is not *correct*, though: rewinding `axis_remaining_s` past an axis flip leaves
/// the hitbox for the wrong axis alive. The fix is the one `MovePlayback` already got —
/// make the hitbox's EXISTENCE derived from `(strike_active, axis_horizontal,
/// axis_remaining_s)` and maintained by a per-frame system, so there is nothing to hold
/// a handle to. Recorded here rather than in a desync report.
impl SnapshotCursor for SaddlePointState {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_bool(out, self.strike_active);
        put_bool(out, self.axis_horizontal);
        put_f32(out, self.axis_remaining_s);
    }
    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        self.strike_active = r.bool()?;
        self.axis_horizontal = r.bool()?;
        self.axis_remaining_s = r.f32()?;
        Some(())
    }
}

// Keeps the `snapshot_unit_enum` import honest if a special later grows an enum.
#[allow(unused_imports)]
use snapshot_unit_enum as _snapshot_unit_enum;
