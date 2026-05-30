//! ADR 0010 — time-control authority as data.
//!
//! Gameplay code never mutates [`crate::SandboxSimState::time_scale`]
//! (or any future per-domain clock) directly. Instead it writes a
//! [`ClockScaleRequest`] message naming the [`ClockDomain`] it wants
//! to affect, the requested `scale`, and the [`ClockRequester`] doing
//! the asking. [`apply_clock_scale_requests`] consults the active
//! [`RegimePolicy`] and either grants, denies, rebinds, or broadcasts
//! the request.
//!
//! In the default [`Regime::Solo`] regime — single-player —
//! every requester is granted. The shape of the dispatch is the same
//! as it will be in CoopConsensual / Competitive / RLDeterministic
//! regimes; what changes is only the policy table.
//!
//! See ADR 0010 §Vocabulary, ADR 0011 §Two time-control operations.

use bevy::prelude::*;

use crate::player::components::{PlayerCombatState, PlayerSlot, PrimaryPlayer};
use crate::time::feel::SandboxFeelTuning;
use crate::SandboxDevState;
use crate::{ClockDomain, SandboxSimState};

/// ADR 0011 — per-entity proper-time scale.
///
/// An entity with `ProperTimeScale(2.0)` ticks at twice the world's
/// sim rate; an entity with `ProperTimeScale(0.5)` ticks at half.
/// Default (and the SP-everywhere assumption today) is `1.0`, in
/// which case [`crate::WorldTime::entity_dt`] returns sim_dt
/// unchanged.
///
/// This is the seam for two future mechanics:
///
/// - **MP bullet-time** (ADR 0011 §Two time-control operations) —
///   one player boosts their proper time without slowing other
///   players' clocks. `BoostEntityProperTime(p, factor)` sets this
///   component on the player entity.
/// - **Special relativity** (ADR 0011 §Galilean→SR ladder) — a
///   future room metric computes this from velocity via the Lorentz
///   factor `γ(v) = 1 / √(1 − v²/c²)`. The integrator already reads
///   per-entity proper-time scale, so adding SR is a data change.
///
/// Most entities never carry this component; `entity_dt` defaults
/// to `ProperTimeScale::ONE` (i.e., `sim_dt`) when it's missing so
/// the change is invisible until something opts in.
#[derive(Component, Copy, Clone, Debug, PartialEq)]
pub struct ProperTimeScale(pub f32);

impl Default for ProperTimeScale {
    fn default() -> Self {
        Self::ONE
    }
}

impl ProperTimeScale {
    /// The everywhere-default scale. Returned by lookups for entities
    /// that have no [`ProperTimeScale`] component.
    pub const ONE: ProperTimeScale = ProperTimeScale(1.0);

    /// Read the scalar value.
    pub fn value(self) -> f32 {
        self.0
    }

    /// Resolve an `Option<&ProperTimeScale>` lookup, defaulting to
    /// [`Self::ONE`] when missing. Convenience for animator + AI
    /// systems that query `Option<&ProperTimeScale>` so they don't
    /// require every entity to carry the component.
    pub fn or_default(opt: Option<&ProperTimeScale>) -> ProperTimeScale {
        opt.copied().unwrap_or(Self::ONE)
    }
}

/// Who is asking for a clock change. Encoded as data so a policy
/// table can grant/deny based on identity without hard-coding which
/// systems are allowed to touch which clocks.
///
/// ADR 0010 calls this the `requester` field of `ClockScaleRequest`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)] // `Scripted` + `Boss` are reserved per ADR 0010 (narrative authority).
pub enum ClockRequester {
    /// A player ability (today: bullet-time blink) or future player-
    /// triggered time mechanic.
    Player(PlayerSlot),
    /// Developer tools — keyboard slowmo toggle, inspector overrides.
    DevTool,
    /// Scripted cutscene / quest / encounter — narrative authority.
    Scripted,
    /// The engine itself — game-mode pause / suspended-gameplay zeroing.
    Engine,
    /// A boss (or other in-world entity) that has been granted time
    /// authority by a room-scoped policy override. ADR 0010 §Narrative
    /// authority: "the boss got root on the simulator."
    Boss,
}

/// What the policy lets a requester do with a domain.
///
/// `Grant` writes the request through. `Deny` drops it. `Rebind`
/// rewrites the request to a different domain (e.g., a CoopConsensual
/// regime might rebind a player's `SimClock` request onto their own
/// `PlayerClock`). `Broadcast` applies the request to every domain in
/// scope (e.g., CoopConsensual sharing a player's bullet-time across
/// all PlayerClocks).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)] // `Rebind` + `Broadcast` are reserved per ADR 0010 (CoopConsensual / Regimes).
pub enum Permission {
    Grant,
    Deny,
    Rebind(ClockDomain),
    Broadcast,
}

/// The active permission table. ADR 0010 §Regimes — adding a regime
/// is a data change, not a code change.
///
/// `Solo` is the SP default: permissive, every request granted.
/// `RLDeterministic` denies all clock-scale requests so training
/// runs and CI use a fixed timestep. `Cinematic` defers player
/// requests while scripted authority holds; useful during cutscenes.
///
/// Future: `CoopConsensual` and `Competitive` (ADR 0010 §Regimes).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)] // RLDeterministic + Cinematic regimes reserved per ADR 0010.
pub enum Regime {
    Solo,
    RLDeterministic,
    Cinematic,
}

/// Resource carrying the active [`Regime`]. The default is `Solo`
/// — SP is what falls out of the most permissive policy.
#[derive(Resource, Copy, Clone, Debug)]
pub struct RegimePolicy {
    pub regime: Regime,
}

impl Default for RegimePolicy {
    fn default() -> Self {
        Self {
            regime: Regime::Solo,
        }
    }
}

impl RegimePolicy {
    /// Look up the permission for a given `(requester, domain)`
    /// pair under the active regime. Pure function — no resource
    /// access — so the policy table is unit-testable.
    pub fn permission_for(self, requester: ClockRequester, _domain: ClockDomain) -> Permission {
        match self.regime {
            Regime::Solo => Permission::Grant,
            Regime::RLDeterministic => Permission::Deny,
            Regime::Cinematic => match requester {
                ClockRequester::Scripted | ClockRequester::Engine => Permission::Grant,
                ClockRequester::Player(_) | ClockRequester::DevTool | ClockRequester::Boss => {
                    Permission::Deny
                }
            },
        }
    }
}

/// A request to scale a named clock. Written by gameplay systems
/// that want to bend time (bullet-time, hitstop, cutscene pause,
/// boss freeze) instead of mutating [`SandboxSimState::time_scale`]
/// directly. Consumed by [`apply_clock_scale_requests`].
///
/// `reason` is a short static label for telemetry and debug overlays
/// — keep it grep-able ("bullet_blink", "hitstop", "cinematic_freeze").
#[derive(Message, Copy, Clone, Debug)]
pub struct ClockScaleRequest {
    pub domain: ClockDomain,
    pub scale: f32,
    pub requester: ClockRequester,
    /// Telemetry/debug label only — not read by `apply_clock_scale_requests`.
    /// Kept on the request so traces and the debug overlay can attribute a
    /// scale change to its source without an additional out-of-band lookup.
    #[allow(dead_code)]
    pub reason: &'static str,
}

/// Target scale per-domain — the value [`SandboxSimState::time_scale`]
/// is currently smoothing toward. Written by [`apply_clock_scale_requests`]
/// (the policy-aware sink of [`ClockScaleRequest`] messages) and read
/// by [`smooth_sim_clock_toward_target_system`] (the per-frame ramp).
///
/// Decoupling target from current keeps the message system orthogonal
/// to feel: a one-shot request flips the target; the smoother takes
/// the next N frames to slide there at feel-tuned rates. Both can
/// land in the same frame for snap behavior, or be split for
/// cinematic ramps.
#[derive(Resource, Clone, Copy, Debug)]
pub struct RequestedClockScale {
    /// SimClock target. Default 1.0 (real-time pace). PlayerClock
    /// grants currently collapse onto this field (SP only has one
    /// player); per-player clocks are added when MP regimes land.
    pub sim_clock: f32,
}

impl Default for RequestedClockScale {
    fn default() -> Self {
        Self { sim_clock: 1.0 }
    }
}

/// Drain pending [`ClockScaleRequest`] messages, run them through
/// the active [`RegimePolicy`], and store the granted scales in
/// [`RequestedClockScale`].
///
/// This system DOES NOT mutate [`SandboxSimState::time_scale`]
/// directly — that's the smoother's job. The split exists so the
/// policy table sees every requester (auditability, telemetry) and
/// so multiple per-frame requesters can land in a sensible order
/// before the smoother converts the resulting target into the
/// current frame's time_scale.
///
/// Multiple requests in one frame: later requests win for the same
/// domain. SP wins on whatever the last grant says; deterministic
/// because the message order is the schedule order.
pub fn apply_clock_scale_requests(
    mut requests: MessageReader<ClockScaleRequest>,
    policy: Res<RegimePolicy>,
    mut target: ResMut<RequestedClockScale>,
) {
    for req in requests.read() {
        let permission = policy.permission_for(req.requester, req.domain);
        match permission {
            Permission::Grant => write_target(&mut target, req.domain, req.scale),
            Permission::Deny => continue,
            Permission::Rebind(other) => write_target(&mut target, other, req.scale),
            Permission::Broadcast => {
                // SP today has one player + one sim clock; broadcast
                // collapses to a SimClock write. CoopConsensual will
                // fan out to every PlayerClock here.
                write_target(&mut target, ClockDomain::SimClock, req.scale);
            }
        }
    }
}

fn write_target(target: &mut RequestedClockScale, domain: ClockDomain, scale: f32) {
    match domain {
        ClockDomain::SimClock => target.sim_clock = scale,
        // ADR 0011 §Two time-control operations — in SP the
        // "boost-player-proper-time" path collapses onto SimClock
        // (one observer, one frame). When MP lands, this arm
        // diverges into per-PlayerClock targets.
        ClockDomain::PlayerClock(_) => target.sim_clock = scale,
        ClockDomain::WallClock => { /* wall clock is never scaled */ }
    }
}

/// Read the primary player's state and the dev tools, decide what
/// SimClock scale should be in effect this frame, and fire one
/// [`ClockScaleRequest`].
///
/// Priority matches the historic [`crate::update_time_scale`]
/// ladder so behavior is preserved:
///
/// 1. hitstop active → 0.0   (Engine requester — the engine took
///    authority on the player's behalf)
/// 2. blink aiming → bullet_time_scale  (Player requester —
///    the "bullet_blink" verb)
/// 3. blink hold active → blink_hold_slow_scale  (Player —
///    "blink_hold_slow")
/// 4. dev_state.slowmo → debug_slowmo_scale  (DevTool —
///    inspector-driven)
/// 5. otherwise → 1.0  (Engine — restoring real-time pace)
///
/// ADR 0011 §"Two time-control operations" note: in SP, the
/// "slow sim" (Operation 1) and "boost player proper time"
/// (Operation 2) are observationally equivalent for one observer.
/// We implement Operation 1 here because it's the simpler write.
/// Step 3's per-entity `ProperTimeScale` component + `entity_dt`
/// accessor are the seam where future MP / RL regimes diverge.
pub fn emit_player_time_intent_system(
    primary: Query<(&crate::player::PlayerBlinkState, &PlayerCombatState), With<PrimaryPlayer>>,
    dev_state: Res<SandboxDevState>,
    feel: Res<SandboxFeelTuning>,
    mut writer: MessageWriter<ClockScaleRequest>,
) {
    let Ok((blink, combat)) = primary.single() else {
        return;
    };
    let (scale, requester, reason) = if combat.hitstop_timer > 0.0 {
        (0.0, ClockRequester::Engine, "hitstop")
    } else if blink.aiming {
        (
            feel.bullet_time_scale,
            ClockRequester::Player(PlayerSlot::PRIMARY),
            "bullet_blink",
        )
    } else if blink.hold_active {
        (
            feel.blink_hold_slow_scale,
            ClockRequester::Player(PlayerSlot::PRIMARY),
            "blink_hold_slow",
        )
    } else if dev_state.slowmo {
        (
            feel.debug_slowmo_scale,
            ClockRequester::DevTool,
            "dev_slowmo",
        )
    } else {
        (1.0, ClockRequester::Engine, "default")
    };
    writer.write(ClockScaleRequest {
        domain: ClockDomain::SimClock,
        scale,
        requester,
        reason,
    });
}

/// Smooth [`SandboxSimState::time_scale`] toward
/// [`RequestedClockScale::sim_clock`] at feel-tuned rates.
///
/// Replaces the imperative `crate::update_time_scale` helper. The
/// asymmetric ramp (`time_ramp_down_rate` when decelerating,
/// `time_ramp_up_rate` when accelerating) preserves the "snap into
/// bullet-time, breathe back to normal" feel the imperative version
/// shipped.
pub fn smooth_sim_clock_toward_target_system(
    target: Res<RequestedClockScale>,
    feel: Res<SandboxFeelTuning>,
    time: Res<Time>,
    mut sim_state: ResMut<SandboxSimState>,
) {
    let frame_dt = time.delta_secs();
    let rate = if target.sim_clock < sim_state.time_scale {
        feel.time_ramp_down_rate
    } else {
        feel.time_ramp_up_rate
    };
    sim_state.time_scale =
        crate::move_toward(sim_state.time_scale, target.sim_clock, rate * frame_dt);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::components::PlayerSlot;

    #[test]
    fn proper_time_scale_default_is_one() {
        let pts = ProperTimeScale::default();
        assert_eq!(pts, ProperTimeScale::ONE);
        assert_eq!(pts.value(), 1.0);
    }

    #[test]
    fn proper_time_scale_or_default_falls_back_to_one() {
        let some = ProperTimeScale(2.5);
        assert_eq!(ProperTimeScale::or_default(Some(&some)).value(), 2.5);
        assert_eq!(ProperTimeScale::or_default(None), ProperTimeScale::ONE);
    }

    #[test]
    fn solo_regime_grants_every_requester_every_domain() {
        let policy = RegimePolicy {
            regime: Regime::Solo,
        };
        for requester in [
            ClockRequester::Player(PlayerSlot::PRIMARY),
            ClockRequester::DevTool,
            ClockRequester::Scripted,
            ClockRequester::Engine,
            ClockRequester::Boss,
        ] {
            for domain in [
                ClockDomain::SimClock,
                ClockDomain::PlayerClock(PlayerSlot::PRIMARY),
                ClockDomain::WallClock,
            ] {
                assert_eq!(
                    policy.permission_for(requester, domain),
                    Permission::Grant,
                    "Solo must grant {:?} -> {:?}",
                    requester,
                    domain,
                );
            }
        }
    }

    #[test]
    fn rl_deterministic_denies_every_request() {
        let policy = RegimePolicy {
            regime: Regime::RLDeterministic,
        };
        assert_eq!(
            policy.permission_for(
                ClockRequester::Player(PlayerSlot::PRIMARY),
                ClockDomain::SimClock,
            ),
            Permission::Deny,
        );
        assert_eq!(
            policy.permission_for(ClockRequester::Scripted, ClockDomain::SimClock),
            Permission::Deny,
        );
    }

    #[test]
    fn cinematic_grants_scripted_denies_player() {
        let policy = RegimePolicy {
            regime: Regime::Cinematic,
        };
        assert_eq!(
            policy.permission_for(ClockRequester::Scripted, ClockDomain::SimClock),
            Permission::Grant,
        );
        assert_eq!(
            policy.permission_for(
                ClockRequester::Player(PlayerSlot::PRIMARY),
                ClockDomain::SimClock,
            ),
            Permission::Deny,
        );
        // Engine retains authority during cinematics so pause /
        // suspended-gameplay still works.
        assert_eq!(
            policy.permission_for(ClockRequester::Engine, ClockDomain::SimClock),
            Permission::Grant,
        );
    }

    /// End-to-end: build a minimal app, fire a request, run the
    /// system, observe the target change. This is the canonical
    /// regression check that the dispatch pipeline is wired.
    /// SandboxSimState::time_scale is touched by the SMOOTHER
    /// (smooth_sim_clock_toward_target_system), not by this system.
    #[test]
    fn solo_grant_writes_requested_clock_scale() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy::default())
            .insert_resource(RequestedClockScale::default())
            .add_systems(Update, apply_clock_scale_requests);

        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.125,
            requester: ClockRequester::Player(PlayerSlot::PRIMARY),
            reason: "bullet_blink_test",
        });

        app.update();

        let target = app.world().resource::<RequestedClockScale>();
        assert!((target.sim_clock - 0.125).abs() < 1e-6);
    }

    #[test]
    fn rl_regime_denies_blocks_the_scale_change() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy {
                regime: Regime::RLDeterministic,
            })
            .insert_resource(RequestedClockScale::default())
            .add_systems(Update, apply_clock_scale_requests);

        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.125,
            requester: ClockRequester::Player(PlayerSlot::PRIMARY),
            reason: "denied_test",
        });

        app.update();

        let target = app.world().resource::<RequestedClockScale>();
        assert!(
            (target.sim_clock - 1.0).abs() < 1e-6,
            "RL must keep sim-clock target at 1.0",
        );
    }

    /// Wall clock is by definition unscaled. A grant targeting it is
    /// a no-op so the host's real-time tick keeps advancing.
    #[test]
    fn wall_clock_grant_does_not_mutate_target() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy::default())
            .insert_resource(RequestedClockScale::default())
            .add_systems(Update, apply_clock_scale_requests);

        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::WallClock,
            scale: 0.25,
            requester: ClockRequester::DevTool,
            reason: "wall_noop_test",
        });

        app.update();

        let target = app.world().resource::<RequestedClockScale>();
        assert!((target.sim_clock - 1.0).abs() < 1e-6);
    }

    /// End-to-end: a SimClock grant + the smoother together actually
    /// move time_scale. After ~10 frames at default ramp rates the
    /// time_scale should be well below 1.0 (heading toward 0.125).
    #[test]
    fn smoother_ramps_sim_state_time_scale_toward_target() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy::default())
            .insert_resource(RequestedClockScale::default())
            .insert_resource(SandboxSimState::default())
            .insert_resource(SandboxFeelTuning::default())
            .insert_resource(Time::<()>::default())
            .add_systems(
                Update,
                (
                    apply_clock_scale_requests,
                    smooth_sim_clock_toward_target_system,
                )
                    .chain(),
            );

        // Pump a fixed 16ms tick into Bevy's Time so the smoother
        // sees a non-zero frame_dt.
        let frame = std::time::Duration::from_millis(16);
        for _ in 0..30 {
            app.world_mut().write_message(ClockScaleRequest {
                domain: ClockDomain::SimClock,
                scale: 0.125,
                requester: ClockRequester::Player(PlayerSlot::PRIMARY),
                reason: "ramp_test",
            });
            app.world_mut().resource_mut::<Time>().advance_by(frame);
            app.update();
        }

        let sim = app.world().resource::<SandboxSimState>();
        // SandboxFeelTuning::time_ramp_down_rate is 6.0 (units/s).
        // 30 frames * 16ms = 480ms => deltas of ~2.88 units, way
        // past the (1.0 -> 0.125) gap of 0.875.
        assert!(
            (sim.time_scale - 0.125).abs() < 1e-4,
            "expected sim time_scale ~= 0.125 after ramp; got {}",
            sim.time_scale,
        );
    }

    /// Static guardrail: gameplay-tick files must NOT read
    /// `Res<Time>` directly. Reading `Res<Time>` returns wall-clock
    /// dt and silently bypasses bullet-time / pause / hitstop. The
    /// canonical pattern (ADR 0010, see also
    /// `feedback_time_domains` memory) is to read `Res<WorldTime>`
    /// and pick a domain explicitly via `sim_dt()` (gameplay timer),
    /// `wall_dt()` (UI / audio bus), or `player_dt(slot)` (input
    /// buffer).
    ///
    /// This is encoded as a source-tree scan rather than a type-
    /// level constraint because Bevy's prelude leaks `Time` into
    /// every system signature and there is no language-level seam
    /// short of forking Bevy. The scan keeps the discipline visible
    /// and reviewable; the allowlist documents the legitimate
    /// wall-clock consumers so the next reviewer can audit by name.
    ///
    /// Tighten this allowlist over time as the player-clock /
    /// entity-clock plumbing lands (ADR 0011). Do NOT add gameplay
    /// systems to the allowlist — fix them.
    #[test]
    fn gameplay_systems_must_not_read_res_time_directly() {
        use std::path::{Path, PathBuf};

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let src = manifest_dir.join("src");

        // Files that are explicitly allowed to read `Res<Time>`.
        // Path is relative to `crates/ambition_sandbox/src/`.
        //
        // Add a one-line justification next to each entry — "why
        // does this need raw wall-clock dt and not WorldTime?"
        let allowlist: &[(&str, &str)] = &[
            // Producer side: `refresh_world_time` (now in `time::world_time`)
            // is THE system that writes WorldTime from Bevy's Time.
            ("time/world_time.rs", "refresh_world_time itself converts Time -> WorldTime"),
            // The time-control pipeline runs on real wall-clock to
            // smoothly ramp time_scale; ramping on its own output
            // would be circular.
            ("time/time_control.rs", "smoother / clock-scale dispatch is the controller, not a consumer"),
            // Cutscenes intentionally suspend bullet-time and run
            // on the wall clock so a paused cutscene still advances.
            ("presentation/cutscene.rs", "cutscene beats are wall-clock by design"),
            // VFX particles are presentation; the design decision
            // is wall-clock so juice survives bullet-time. Revisit
            // if you want VFX to slow alongside the sim.
            ("presentation/fx.rs", "VFX particles are wall-clock by design"),
            // Screen-effect shader modulation (CRT scanline jitter,
            // chromatic-aberration breathing, etc.) is a fullscreen
            // visual driven by wall-clock elapsed seconds so the
            // post-process still animates in bullet-time / hitstop.
            ("presentation/screen_effects.rs", "screen-effect shader modulation is wall-clock by design"),
            // Music director (track switching, fades) is wall-clock;
            // music should not slow in bullet-time.
            ("music/director.rs", "music timing is wall-clock by design"),
            // Camera smoothing is wall-clock so glide responsiveness
            // stays consistent. Switch to scaled if bullet-time camera
            // feel is desired.
            ("presentation/rendering/camera.rs", "camera smoothing is wall-clock by design"),
            // Physics debris is cosmetic; it spawns from sim events
            // but its falling animation is independent of sim time.
            ("world/physics.rs", "debris fall is cosmetic / wall-clock by design"),
            // Player-input timers + the player tick still compute
            // their own scaled dt via `sandbox_dt(hitstop,
            // time_scale, frame_dt)`. Migration target for ADR 0011
            // (PlayerClock); allowed for now.
            ("app/player_tick.rs", "player tick wraps its own sandbox_dt() — ADR 0011 follow-up"),
            ("app/sim_systems.rs", "input timers + attack advance still compute scaled dt manually — ADR 0011 follow-up"),
            ("app/input_systems.rs", "input buffer decay; ADR 0011 player-clock follow-up"),
            // Hot reload polls disk in wall-clock cadence.
            ("world/ldtk_world/hot_reload.rs", "filesystem watcher cadence is wall-clock"),
            // Mobile-touch menu bridge: UI bridging.
            ("host/mobile_input/menu_bridge.rs", "touch menu bridge is wall-clock UI"),
            // Trace recorder timestamps each frame on the wall clock.
            ("dev/trace/systems.rs", "trace timestamps are wall-clock by design"),
            // Falling-sand simulation manages its own step cadence
            // (chunk loader + particle stepping). Slowing it with
            // sim_dt would make sand pile up unphysically during
            // bullet-time; it stays wall-clock.
            ("falling_sand.rs", "falling-sand sim manages its own cadence"),
        ];

        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            let entries = match std::fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => return,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    out.push(path);
                }
            }
        }

        let mut files = Vec::new();
        walk(&src, &mut files);

        let mut violations: Vec<String> = Vec::new();
        for path in &files {
            let rel = path
                .strip_prefix(&src)
                .ok()
                .and_then(|p| p.to_str())
                .unwrap_or("<non-utf8>");
            // Allow this guardrail test file itself (it names `Res<Time>` literally).
            if rel == "time/time_control.rs" {
                continue;
            }
            if allowlist.iter().any(|(p, _)| *p == rel) {
                continue;
            }
            let contents = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            // Strip block-string literals before scanning so doc-
            // examples and rustdoc snippets don't trip the test.
            // Keep it dumb-simple: grep for `Res<Time>` (the exact
            // sig fragment) — comments mentioning it are fine because
            // they wouldn't compile as system params.
            for (lineno, line) in contents.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("*") {
                    continue;
                }
                if line.contains("Res<Time>") {
                    violations.push(format!(
                        "{rel}:{}: contains `Res<Time>` — use Res<WorldTime> + sim_dt()/wall_dt()/player_dt(). \
                         If this genuinely needs wall-clock dt, add an entry + justification to the \
                         `allowlist` in this test.",
                        lineno + 1,
                    ));
                }
            }
        }

        // Useful echo: which files ARE on the allowlist, so reviewers
        // can spot-check the justifications.
        if !violations.is_empty() {
            let allow_summary: Vec<String> = allowlist
                .iter()
                .map(|(p, why)| format!("  {p}: {why}"))
                .collect();
            panic!(
                "gameplay systems must read Res<WorldTime>, not Res<Time>. Violations:\n{}\n\nCurrent allowlist:\n{}",
                violations.join("\n"),
                allow_summary.join("\n"),
            );
        }
    }

    /// Integration check: with `WorldTime::scaled_dt` at 0.25, a
    /// timer driven by `sim_dt()` advances at exactly 0.25× the
    /// wall-clock dt. This pins the contract that bullet-time
    /// downstream consumers actually slow down — the exact bug the
    /// `Res<Time>` -> `Res<WorldTime>` refactor was meant to fix.
    #[test]
    fn world_time_sim_dt_respects_time_scale() {
        use crate::WorldTime;

        let mut wt = WorldTime::default();
        wt.raw_dt = 0.016;
        wt.scaled_dt = 0.016 * 0.25;
        assert!((wt.wall_dt() - 0.016).abs() < 1e-6);
        assert!((wt.sim_dt() - 0.004).abs() < 1e-6);

        // Pause behaviour: time_scale == 0 -> sim_dt == 0 even
        // though wall_dt keeps ticking.
        let mut paused = WorldTime::default();
        paused.raw_dt = 0.016;
        paused.scaled_dt = 0.0;
        assert_eq!(paused.sim_dt(), 0.0);
        assert!((paused.wall_dt() - 0.016).abs() < 1e-6);
    }
}
