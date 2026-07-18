//! Boss-special rollback codecs, registered by the content crate that owns them.
//!
//! `docs/planning/engine/netcode.md` N3.1: *"each sim crate registers its components'
//! serialization."* These eleven Technique states are sim state — a `fired_this_strike`
//! latch that survives a rollback is a strike that fires twice — and no crate below
//! `ambition_content` can name them. `AmbitionRollbackApp` installs the real `bevy_ggrs` snapshot/checksum plugins and
//! records the exact schema identity owned by this content crate.
//!
//! That is the whole seam. It needed a resource, not a trait relocation: the codec
//! trait must live where `ambition_runtime` can implement it for `ambition_time` and
//! `ambition_engine_core` types, and those crates sit *below* every foundation crate
//! `ambition_content` names. Moving the trait down would trade one orphan-rule problem
//! for a worse one.

use ambition_runtime::rollback::{
    put_bool, put_f32, put_u32, put_vec2, AmbitionRollbackApp, Reader, SnapshotCursor,
    SnapshotState,
};
use bevy::prelude::*;

use super::{
    AppleRainSpawnState, EchoFanState, ExplodingGradientState, EyeBeamState, GradientCascadeState,
    MinimaTrapState, ModeCollapseState, OverfitVolleyState, OverflowState, SaddlePointState,
    SeismicStompState,
};

/// Add every boss-special state to the GGRS rollback contract. Called from
/// `BossSpecialContentPlugin::build`; registration is plugin-order independent.
pub(super) fn register(app: &mut App) {
    const OWNER: &str = "ambition_content::bosses::specials";
    app.rollback_component_canonical::<EchoFanState>(OWNER, "content.echo_fan_state")
        .rollback_component_canonical::<SeismicStompState>(OWNER, "content.seismic_stomp_state")
        .rollback_component_canonical::<ExplodingGradientState>(
            OWNER,
            "content.exploding_gradient_state",
        )
        .rollback_component_canonical::<OverflowState>(OWNER, "content.overflow_state")
        .rollback_component_canonical::<GradientCascadeState>(
            OWNER,
            "content.gradient_cascade_state",
        )
        .rollback_component_canonical::<MinimaTrapState>(OWNER, "content.minima_trap_state")
        .rollback_component_canonical::<AppleRainSpawnState>(
            OWNER,
            "content.apple_rain_spawn_state",
        )
        .rollback_component_canonical::<ModeCollapseState>(OWNER, "content.mode_collapse_state")
        .rollback_component_canonical::<EyeBeamState>(OWNER, "content.eye_beam_state")
        .rollback_component_canonical::<OverfitVolleyState>(OWNER, "content.overfit_volley_state")
        .rollback_component_cursor::<SaddlePointState>(OWNER, "content.saddle_point_state")
        .rollback_map_entities::<SaddlePointState>(OWNER, "map.content.saddle_point_state");
}

// A `fired_this_strike` latch that survives a rollback is a strike that fires twice;
// a `spawn_index` that survives one is a minion that is never born. Keep these
// content-owned codecs explicit instead of exporting the runtime crate's private
// convenience macro as a public API.
impl SnapshotState for EchoFanState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.fired_this_strike);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            fired_this_strike: r.bool()?,
        })
    }
}

impl SnapshotState for SeismicStompState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.fired_this_strike);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            fired_this_strike: r.bool()?,
        })
    }
}

impl SnapshotState for ExplodingGradientState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.fired_this_strike);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            fired_this_strike: r.bool()?,
        })
    }
}

impl SnapshotState for GradientCascadeState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.fired_this_strike);
        put_u32(out, self.spawn_index);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            fired_this_strike: r.bool()?,
            spawn_index: r.u32()?,
        })
    }
}

impl SnapshotState for MinimaTrapState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.fired_this_strike);
        put_u32(out, self.spawn_index);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            fired_this_strike: r.bool()?,
            spawn_index: r.u32()?,
        })
    }
}

impl SnapshotState for AppleRainSpawnState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.spawn_accum);
        put_u32(out, self.spawn_index);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            spawn_accum: r.f32()?,
            spawn_index: r.u32()?,
        })
    }
}

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
/// GGRS clones the exact component and remaps both handles after entity recreation.
/// The `SnapshotCursor` implementation below is now only the canonical checksum
/// projection: allocator-local handles are intentionally excluded, while the strike
/// clock and axis state participate in sync-test comparisons.
impl SnapshotCursor for SaddlePointState {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_bool(out, self.strike_active);
        put_bool(out, self.axis_horizontal);
        put_f32(out, self.axis_remaining_s);
    }
}
