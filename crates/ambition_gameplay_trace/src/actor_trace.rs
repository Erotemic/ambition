//! Non-player-centric body trace: a rolling timeline of EVERY simulated
//! body's kinematic state (player, boss, enemy, NPC — no privileged
//! observer) plus a per-body out-of-bounds classifier and a dump-on-OOB
//! writer.
//!
//! This is the relativity-respecting sibling of [`GameplayTraceBuffer`].
//! That recorder captures the rich, input-driven PLAYER feel timeline
//! (jumps, dashes, blink, ability charges) — legitimately player-specific
//! because only the player has inputs. THIS recorder captures the
//! universal BODY timeline so any character that leaves the world
//! envelope (the mockingbird flying out of its arena, an enemy tunnelling
//! a wall) is caught exactly the same way the player would be.
//!
//! The schema is deliberately lean — `pos/vel/size/aabb/facing` + an OOB
//! tag per body — so a multi-body ring buffer stays cheap. Each frame is a
//! snapshot of ALL tracked bodies on one shared timeline, so a dump shows
//! what every character (and the world) was doing around the anomaly.

use crate::{timestamp_label, CollisionTraceShape, TraceAabb, TracePoint};
use bevy::prelude::Resource;
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{fs, io};

/// One body's kinematic snapshot for a single frame. `oob` carries the
/// out-of-bounds reason (short label) this frame, if any.
#[derive(Serialize, Clone, Debug)]
pub struct BodyTraceSnapshot {
    /// Stable per-actor id (the boss / enemy / NPC authored id, or
    /// `"player"`). The trace keys arming + offender attribution on this,
    /// never on a privileged "the player" handle.
    pub actor_id: String,
    pub name: String,
    /// Faction-ish bucket: `player` | `boss` | `enemy` | `npc` | `body`.
    pub kind: String,
    pub pos: TracePoint,
    pub vel: TracePoint,
    pub size: TracePoint,
    pub aabb: TraceAabb,
    pub facing: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oob: Option<String>,
}

impl BodyTraceSnapshot {
    pub fn is_oob(&self) -> bool {
        self.oob.is_some()
    }
}

/// One frame: every tracked body plus the shared world / time context.
#[derive(Serialize, Clone, Debug)]
pub struct ActorTraceFrame {
    pub seq: u64,
    pub tick: u64,
    pub real_dt: f32,
    pub sim_dt: f32,
    pub time_scale: f32,
    pub game_mode: String,
    pub active_area: String,
    pub world_size: TracePoint,
    pub world_spawn: TracePoint,
    pub bodies: Vec<BodyTraceSnapshot>,
    /// The augmented world's solid blocks this frame (static geometry +
    /// feature/overlay solids). Captured so a dump is self-contained for
    /// geometry analysis: cross-referenced with a body's pre-anomaly
    /// trajectory it shows exactly which wall/floor a body was jammed into
    /// before it left bounds. The same set every frame for a static room, so
    /// the markdown only renders the latest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub solids: Vec<CollisionTraceShape>,
}

impl ActorTraceFrame {
    pub fn oob_bodies(&self) -> impl Iterator<Item = &BodyTraceSnapshot> {
        self.bodies.iter().filter(|b| b.is_oob())
    }
}

/// Why an actor-trace dump fired.
#[derive(Serialize, Clone, Debug)]
pub enum ActorDumpReason {
    Manual,
    OobAuto {
        actor_id: String,
        name: String,
        kind: String,
        reason: String,
    },
}

impl ActorDumpReason {
    pub fn label(&self) -> String {
        match self {
            ActorDumpReason::Manual => "Manual".into(),
            ActorDumpReason::OobAuto {
                actor_id,
                name,
                kind,
                reason,
            } => format!("OOB auto: {name} [{kind} #{actor_id}] — {reason}"),
        }
    }

    /// The offending actor id, if this is an OOB dump.
    pub fn actor_id(&self) -> Option<&str> {
        match self {
            ActorDumpReason::OobAuto { actor_id, .. } => Some(actor_id),
            ActorDumpReason::Manual => None,
        }
    }
}

/// Rolling ring buffer of [`ActorTraceFrame`]s. Mirrors
/// [`GameplayTraceBuffer`]'s lifecycle but keyed per body: each body arms
/// / disarms its own OOB auto-dump independently, so one stuck boss can't
/// suppress catching a different character that goes OOB later.
/// Minimum frames the buffer must hold before an OOB auto-dump fires. This
/// (a) skips spawn-settling transients — a body authored with its feet in
/// the floor reads `inside solid` for a tick or two before the first
/// collision resolve lifts it out — and (b) guarantees every dump carries
/// pre-anomaly lead-up frames instead of a useless 1-frame snapshot. ~0.5s
/// at 60fps; the real bugs this catches happen long after spawn, by which
/// point the ring is full anyway.
pub const DEFAULT_MIN_CONTEXT_FRAMES: usize = 30;

#[derive(Resource, Debug)]
pub struct ActorTraceBuffer {
    pub capacity_frames: usize,
    pub frames: VecDeque<ActorTraceFrame>,
    pub sequence: u64,
    pub tick: u64,
    pub dump_request: Option<ActorDumpReason>,
    /// Bodies (by id) currently OOB that have already auto-dumped — so a
    /// body that stays OOB for 60 frames produces one dump, not 60.
    /// Re-armed (removed) the frame the body returns in bounds.
    pub oob_disarmed: HashSet<String>,
    /// Auto-dumps are suppressed until the buffer holds at least this many
    /// frames (see [`DEFAULT_MIN_CONTEXT_FRAMES`]).
    pub min_context_frames: usize,
    pub last_dump_path: Option<String>,
    pub last_dump_status: Option<String>,
}

impl Default for ActorTraceBuffer {
    fn default() -> Self {
        Self::with_capacity(crate::DEFAULT_FRAME_CAPACITY)
    }
}

impl ActorTraceBuffer {
    pub fn with_capacity(frames: usize) -> Self {
        Self {
            capacity_frames: frames.max(1),
            frames: VecDeque::with_capacity(frames.max(1)),
            sequence: 0,
            tick: 0,
            dump_request: None,
            oob_disarmed: HashSet::new(),
            min_context_frames: DEFAULT_MIN_CONTEXT_FRAMES,
            last_dump_path: None,
            last_dump_status: None,
        }
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// First request wins, so a pending `Manual` dump can't be clobbered
    /// by a later auto-dump (matches [`GameplayTraceBuffer::request_dump`]).
    pub fn request_dump(&mut self, reason: ActorDumpReason) {
        if self.dump_request.is_none() {
            self.dump_request = Some(reason);
        }
    }

    /// Push a frame: update per-body OOB arming, request a dump for the
    /// first newly-OOB body, and evict the oldest frame if at capacity.
    pub fn record(&mut self, frame: ActorTraceFrame) {
        // Re-arm any previously-dumped body that is no longer OOB this
        // frame (absent bodies count as "no longer OOB").
        let oob_now: HashSet<&str> = frame
            .oob_bodies()
            .map(|b| b.actor_id.as_str())
            .collect();
        self.oob_disarmed.retain(|id| oob_now.contains(id.as_str()));

        // The first still-armed OOB body requests the dump — but only once
        // the buffer has warmed up enough to carry pre-anomaly context (and
        // so spawn-settling transients don't dump a useless 1-frame trace).
        if self.frames.len() >= self.min_context_frames {
            for b in frame.oob_bodies() {
                if !self.oob_disarmed.contains(&b.actor_id) {
                    self.oob_disarmed.insert(b.actor_id.clone());
                    self.request_dump(ActorDumpReason::OobAuto {
                        actor_id: b.actor_id.clone(),
                        name: b.name.clone(),
                        kind: b.kind.clone(),
                        reason: b.oob.clone().unwrap_or_default(),
                    });
                }
            }
        }

        if self.frames.len() == self.capacity_frames {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
        self.sequence = self.sequence.saturating_add(1);
        self.tick = self.tick.saturating_add(1);
    }

    pub fn frames(&self) -> impl Iterator<Item = &ActorTraceFrame> {
        self.frames.iter()
    }
}

#[derive(Serialize, Debug)]
struct ActorDumpPayload<'a> {
    schema_version: u32,
    timestamp_label: String,
    dump_reason: String,
    capacity_frames: usize,
    frame_count: usize,
    sequence: u64,
    current_tick: u64,
    frames: &'a [ActorTraceFrame],
}

/// Serialize the buffer to a timestamped JSON + Markdown pair under `dir`.
/// Returns the JSON path. Filenames use the `ambition_actor_trace_` stem so
/// they sit alongside (and never collide with) the player feel-trace dumps.
pub fn write_actor_dump(
    buffer: &ActorTraceBuffer,
    reason: &ActorDumpReason,
    dir: &Path,
) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let label = timestamp_label(SystemTime::now());
    let stem = format!("ambition_actor_trace_{label}");
    let json_path = dir.join(format!("{stem}.json"));
    let md_path = dir.join(format!("{stem}.md"));

    let frames: Vec<ActorTraceFrame> = buffer.frames.iter().cloned().collect();
    let payload = ActorDumpPayload {
        schema_version: 1,
        timestamp_label: label,
        dump_reason: reason.label(),
        capacity_frames: buffer.capacity_frames,
        frame_count: frames.len(),
        sequence: buffer.sequence,
        current_tick: buffer.tick,
        frames: &frames,
    };
    let json_body = serde_json::to_string_pretty(&payload)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(&json_path, json_body)?;
    fs::write(&md_path, render_actor_markdown(&payload, reason))?;
    Ok(json_path)
}

fn render_actor_markdown(payload: &ActorDumpPayload<'_>, reason: &ActorDumpReason) -> String {
    let mut out = String::new();
    out.push_str("# Ambition actor OOB trace\n\n");
    out.push_str(&format!("- **Reason**: {}\n", payload.dump_reason));
    out.push_str(&format!("- **Timestamp**: {}\n", payload.timestamp_label));
    out.push_str(&format!(
        "- **Frames captured**: {} / {} (cap)\n",
        payload.frame_count, payload.capacity_frames
    ));
    out.push_str(&format!("- **Current tick**: {}\n\n", payload.current_tick));

    if let Some(latest) = payload.frames.last() {
        out.push_str("## Latest frame\n\n");
        out.push_str(&format!("- Active area: `{}`\n", latest.active_area));
        out.push_str(&format!(
            "- World: size ({:.0}, {:.0}), spawn ({:.0}, {:.0})\n",
            latest.world_size.x, latest.world_size.y, latest.world_spawn.x, latest.world_spawn.y
        ));
        out.push_str(&format!("- Bodies: {}\n\n", latest.bodies.len()));
        out.push_str("| actor | kind | pos | vel | oob |\n");
        out.push_str("|---|---|---|---|---|\n");
        for b in &latest.bodies {
            out.push_str(&format!(
                "| `{}` ({}) | {} | ({:.1}, {:.1}) | ({:.1}, {:.1}) | {} |\n",
                b.actor_id,
                b.name,
                b.kind,
                b.pos.x,
                b.pos.y,
                b.vel.x,
                b.vel.y,
                b.oob.as_deref().unwrap_or(""),
            ));
        }
        out.push('\n');

        if !latest.solids.is_empty() {
            out.push_str("### World solids (geometry the bodies collide with)\n\n");
            for s in latest.solids.iter().take(40) {
                out.push_str(&format!(
                    "- `{}` `{}` ({:.0}, {:.0}) → ({:.0}, {:.0})\n",
                    s.kind, s.name, s.aabb.min.x, s.aabb.min.y, s.aabb.max.x, s.aabb.max.y,
                ));
            }
            out.push('\n');
        }
    }

    // The offending body's trajectory into the anomaly is the key diagnostic:
    // print its per-frame pos/vel/oob so the frames BEFORE it left bounds
    // are visible (a dump seconds later would only show the stuck aftermath).
    if let Some(actor_id) = reason.actor_id() {
        out.push_str(&format!("## Offender trajectory — `{actor_id}`\n\n"));
        let tail = payload
            .frames
            .len()
            .saturating_sub(crate::MARKDOWN_FRAME_SUMMARY_TAIL);
        for f in &payload.frames[tail..] {
            if let Some(b) = f.bodies.iter().find(|b| b.actor_id == actor_id) {
                out.push_str(&format!(
                    "- t={:>5} pos=({:>8.1},{:>8.1}) vel=({:>8.1},{:>8.1}) ts={:.2}{}\n",
                    f.tick,
                    b.pos.x,
                    b.pos.y,
                    b.vel.x,
                    b.vel.y,
                    f.time_scale,
                    b.oob
                        .as_deref()
                        .map(|r| format!("  OOB[{r}]"))
                        .unwrap_or_default(),
                ));
            }
        }
        out.push('\n');
    }

    out.push_str("## Hints\n\n");
    out.push_str("- Find the first frame in the offender trajectory tagged `OOB[...]`;\n");
    out.push_str("  the frames just before it show how the body left the envelope\n");
    out.push_str("  (steady drift vs. a single teleport-class jump in pos).\n");
    out.push_str("- `outside world envelope (axis)` = the soft world bound; `inside\n");
    out.push_str("  solid` = tunnelled/clipped into geometry; `absurd velocity` = a\n");
    out.push_str("  bad impulse. Compare against the other bodies' rows for context.\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(id: &str, oob: Option<&str>) -> BodyTraceSnapshot {
        BodyTraceSnapshot {
            actor_id: id.into(),
            name: id.into(),
            kind: "enemy".into(),
            pos: TracePoint::default(),
            vel: TracePoint::default(),
            size: TracePoint::default(),
            aabb: TraceAabb::default(),
            facing: 1.0,
            oob: oob.map(|s| s.into()),
        }
    }

    fn frame(bodies: Vec<BodyTraceSnapshot>) -> ActorTraceFrame {
        ActorTraceFrame {
            seq: 0,
            tick: 0,
            real_dt: 0.016,
            sim_dt: 0.016,
            time_scale: 1.0,
            game_mode: "Playing".into(),
            active_area: "arena".into(),
            world_size: TracePoint { x: 960.0, y: 768.0 },
            world_spawn: TracePoint::default(),
            bodies,
            solids: Vec::new(),
        }
    }

    #[test]
    fn with_capacity_clamps_and_rings() {
        let mut b = ActorTraceBuffer::with_capacity(0);
        assert_eq!(b.capacity_frames, 1);
        b.record(frame(vec![body("a", None)]));
        b.record(frame(vec![body("a", None)]));
        assert_eq!(b.frame_count(), 1, "capacity 1 keeps only the newest frame");
        assert_eq!(b.tick, 2, "tick advances even as frames evict");
    }

    #[test]
    fn an_oob_body_requests_exactly_one_dump_until_it_returns() {
        let mut b = ActorTraceBuffer::with_capacity(8);
        b.min_context_frames = 0; // test arming in isolation, no warm-up gate
        // In bounds: no dump.
        b.record(frame(vec![body("boss", None)]));
        assert!(b.dump_request.is_none());
        // Goes OOB: dump requested, attributed to the boss.
        b.record(frame(vec![body("boss", Some("outside world envelope (y)"))]));
        match b.dump_request.take() {
            Some(ActorDumpReason::OobAuto { actor_id, reason, .. }) => {
                assert_eq!(actor_id, "boss");
                assert_eq!(reason, "outside world envelope (y)");
            }
            other => panic!("expected OobAuto for the boss, got {other:?}"),
        }
        // Still OOB next frame: disarmed, so NO new dump (no spam).
        b.record(frame(vec![body("boss", Some("outside world envelope (y)"))]));
        assert!(b.dump_request.is_none(), "a still-OOB body must not re-dump");
        // Returns in bounds → re-armed.
        b.record(frame(vec![body("boss", None)]));
        assert!(b.oob_disarmed.is_empty(), "returning in bounds re-arms");
        // Goes OOB again → dumps again.
        b.record(frame(vec![body("boss", Some("inside solid (wall)"))]));
        assert!(b.dump_request.is_some(), "a fresh OOB after recovery re-dumps");
    }

    #[test]
    fn warmup_suppresses_dumps_until_context_accrues() {
        // A body that is OOB from the very first frame (e.g. a spawn-settling
        // transient) must NOT dump a useless 1-frame trace; the dump waits
        // until `min_context_frames` of lead-up exist.
        let mut b = ActorTraceBuffer::with_capacity(16);
        b.min_context_frames = 3;
        for _ in 0..3 {
            b.record(frame(vec![body("boss", Some("inside solid (wall)"))]));
            assert!(b.dump_request.is_none(), "no dump before warm-up completes");
        }
        // 4th record: 3 frames already buffered → context satisfied → dump.
        b.record(frame(vec![body("boss", Some("inside solid (wall)"))]));
        assert!(
            b.dump_request.is_some(),
            "dumps once enough pre-anomaly context is buffered"
        );
    }

    #[test]
    fn each_body_arms_independently() {
        let mut b = ActorTraceBuffer::with_capacity(8);
        b.min_context_frames = 0;
        // Boss goes OOB first — one dump, attributed to the boss.
        b.record(frame(vec![
            body("boss", Some("absurd velocity (9000)")),
            body("slug", None),
        ]));
        assert_eq!(b.dump_request.take().unwrap().actor_id(), Some("boss"));
        // Boss still OOB (disarmed) but the slug now goes OOB too: the slug
        // is independently armed, so it still triggers a dump.
        b.record(frame(vec![
            body("boss", Some("absurd velocity (9000)")),
            body("slug", Some("outside world envelope (x)")),
        ]));
        assert_eq!(b.dump_request.take().unwrap().actor_id(), Some("slug"));
    }
}
