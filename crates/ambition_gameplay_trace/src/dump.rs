//! Dump writers: serialize a `GameplayTraceBuffer` to a timestamped markdown +
//! JSON pair (`write_dump`, path/label helpers). The markdown is a human-readable
//! tail summary; the JSON is the full payload the replay harness reads back.

use crate::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};

static DUMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn next_dump_sequence() -> u64 {
    DUMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

#[derive(Serialize, Debug)]
struct DumpPayload<'a> {
    schema_version: u32,
    timestamp_unix: u64,
    timestamp_label: String,
    dump_reason: String,
    capacity_frames: usize,
    capacity_events: usize,
    frame_count: usize,
    event_count: usize,
    sequence: u64,
    current_tick: u64,
    frames: &'a [GameplayTraceFrame],
    events: &'a [GameplayTraceEvent],
}

/// Pure path-formatting helper. Does not touch the filesystem so tests
/// stay fast and the function is safe to call when the dump directory
/// doesn't yet exist.
pub fn dump_paths(dir: &Path, timestamp_label: &str) -> (PathBuf, PathBuf) {
    let stem = format!("ambition_gameplay_trace_{timestamp_label}");
    let json = dir.join(format!("{stem}.json"));
    let md = dir.join(format!("{stem}.md"));
    (json, md)
}

/// Format a unique, lexically-sortable label for a dump filename.
///
/// Format: `{secs:010}-{nanos:09}-{seq:06}_{Dd}d{HH}h{MM}m{SS}s`.
/// The `seq` segment is a process-wide atomic counter, so two dumps
/// taken in the same nanosecond still get distinct paths. Lexical
/// order matches chronological order so `ls -1` lists dumps in the
/// order they were taken.
pub fn timestamp_label_with_seq(ts: SystemTime, seq: u64) -> String {
    let dur = ts.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let nanos = dur.subsec_nanos();
    let total_minutes = secs / 60;
    let seconds = secs % 60;
    let total_hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    let total_days = total_hours / 24;
    let hours = total_hours % 24;
    format!(
        "{secs:010}-{nanos:09}-{seq:06}_{}d{:02}h{:02}m{:02}s",
        total_days, hours, minutes, seconds
    )
}

/// Convenience wrapper used by `write_dump`: pulls a fresh sequence
/// counter and formats `ts` against it. Tests can call
/// `timestamp_label_with_seq` directly with explicit sequences to
/// pin behavior.
pub fn timestamp_label(ts: SystemTime) -> String {
    timestamp_label_with_seq(ts, next_dump_sequence())
}

/// Convert the buffer into a `DumpPayload` and write JSON + Markdown to
/// `dir`. Returns the JSON path on success.
pub fn write_dump(
    buffer: &GameplayTraceBuffer,
    reason: &DumpReason,
    dir: &Path,
) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let now = SystemTime::now();
    let label = timestamp_label(now);
    let (json_path, md_path) = dump_paths(dir, &label);

    let frames_slice: Vec<GameplayTraceFrame> = buffer.frames.iter().cloned().collect();
    let events_slice: Vec<GameplayTraceEvent> = buffer.events.iter().cloned().collect();

    let payload = DumpPayload {
        schema_version: 1,
        timestamp_unix: now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        timestamp_label: label.clone(),
        dump_reason: reason.label(),
        capacity_frames: buffer.capacity_frames,
        capacity_events: buffer.capacity_events,
        frame_count: frames_slice.len(),
        event_count: events_slice.len(),
        sequence: buffer.sequence,
        current_tick: buffer.tick,
        frames: &frames_slice,
        events: &events_slice,
    };
    let json_body = serde_json::to_string_pretty(&payload)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(&json_path, json_body)?;

    let md_body = render_markdown(&payload);
    fs::write(&md_path, md_body)?;

    Ok(json_path)
}

fn render_markdown(payload: &DumpPayload<'_>) -> String {
    let mut out = String::new();
    out.push_str("# Ambition gameplay trace\n\n");
    out.push_str(&format!("- **Reason**: {}\n", payload.dump_reason));
    out.push_str(&format!(
        "- **Timestamp**: {} (unix {})\n",
        payload.timestamp_label, payload.timestamp_unix
    ));
    out.push_str(&format!(
        "- **Frames captured**: {} / {} (cap)\n",
        payload.frame_count, payload.capacity_frames
    ));
    out.push_str(&format!(
        "- **Events captured**: {} / {} (cap)\n",
        payload.event_count, payload.capacity_events
    ));
    out.push_str(&format!("- **Current tick**: {}\n\n", payload.current_tick));

    if let Some(latest) = payload.frames.last() {
        out.push_str("## Latest frame\n\n");
        out.push_str(&format!("- Active area: `{}`\n", latest.active_area));
        out.push_str(&format!(
            "- Player pos: ({:.2}, {:.2})\n",
            latest.player.pos.x, latest.player.pos.y
        ));
        out.push_str(&format!(
            "- Player vel: ({:.2}, {:.2})\n",
            latest.player.vel.x, latest.player.vel.y
        ));
        out.push_str(&format!(
            "- Player AABB: ({:.1}, {:.1}) → ({:.1}, {:.1})\n",
            latest.player.aabb.min.x,
            latest.player.aabb.min.y,
            latest.player.aabb.max.x,
            latest.player.aabb.max.y
        ));
        out.push_str(&format!(
            "- Last safe pos: ({:.2}, {:.2})\n",
            latest.player.last_safe_pos.x, latest.player.last_safe_pos.y
        ));
        out.push_str(&format!(
            "- Locomotion: `{}`  Body: `{}`\n",
            latest.player.locomotion, latest.player.body_mode
        ));
        out.push_str(&format!("- on_ground: {}\n", latest.player.on_ground));
        out.push_str(&format!("- on_wall: {}\n", latest.player.on_wall));
        out.push_str(&format!(
            "- attack: attacking={} ability_enabled={} hitstun={:.3} invuln={:.3} (pressed={})\n",
            latest.player.attacking,
            latest.player.attack_ability_enabled,
            latest.player.hitstun_timer,
            latest.player.damage_invuln_timer,
            latest.controls.attack_pressed,
        ));
        out.push_str(&format!(
            "- World size: ({:.0}, {:.0})\n",
            latest.world_size.x, latest.world_size.y
        ));
        out.push('\n');

        if !latest.nearby_collision.is_empty() {
            out.push_str("## Nearby collision (around latest pos)\n\n");
            for shape in latest.nearby_collision.iter().take(16) {
                out.push_str(&format!(
                    "- `{}` `{}` ({:.2}, {:.2}) → ({:.2}, {:.2}) — d={:.1}\n",
                    shape.kind,
                    shape.name,
                    shape.aabb.min.x,
                    shape.aabb.min.y,
                    shape.aabb.max.x,
                    shape.aabb.max.y,
                    shape.distance,
                ));
            }
            out.push('\n');
        }
    }

    let oob_first = payload
        .events
        .iter()
        .find(|e| matches!(e, GameplayTraceEvent::OobDetected { .. }));
    if let Some(GameplayTraceEvent::OobDetected { tick, reason, pos }) = oob_first {
        out.push_str("## First OOB event in window\n\n");
        out.push_str(&format!(
            "- tick {tick}: `{reason}` at ({:.2}, {:.2})\n\n",
            pos.x, pos.y
        ));
    }

    out.push_str(&format!(
        "## Frames (last {} of {})\n\n",
        payload.frames.len().min(MARKDOWN_FRAME_SUMMARY_TAIL),
        payload.frames.len()
    ));
    let frames_tail_start = payload
        .frames
        .len()
        .saturating_sub(MARKDOWN_FRAME_SUMMARY_TAIL);
    for f in &payload.frames[frames_tail_start..] {
        out.push_str(&format!(
            "- t={:>5} pos=({:>7.1},{:>7.1}) vel=({:>7.1},{:>7.1}) gnd={} loco={} body={} dt={:.4} ts={:.2}\n",
            f.tick,
            f.player.pos.x,
            f.player.pos.y,
            f.player.vel.x,
            f.player.vel.y,
            f.player.on_ground,
            f.player.locomotion,
            f.player.body_mode,
            f.real_dt,
            f.time_scale,
        ));
    }
    out.push('\n');

    out.push_str(&format!(
        "## Events (last {} of {})\n\n",
        payload.events.len().min(MARKDOWN_EVENT_TAIL),
        payload.events.len()
    ));
    let events_tail_start = payload.events.len().saturating_sub(MARKDOWN_EVENT_TAIL);
    for ev in &payload.events[events_tail_start..] {
        out.push_str(&format!("- t={:>5} `{}` :: ", ev.tick(), ev.label()));
        match ev {
            GameplayTraceEvent::InputEdge { action, .. } => out.push_str(action),
            GameplayTraceEvent::PlayerModeChanged { from, to, .. } => {
                out.push_str(&format!("{from} → {to}"));
            }
            GameplayTraceEvent::Jump { .. } => out.push_str("jump"),
            GameplayTraceEvent::DoubleJump { .. } => out.push_str("double jump"),
            GameplayTraceEvent::Dash { .. } => out.push_str("dash"),
            GameplayTraceEvent::Blink {
                from,
                to,
                precision,
                ..
            } => out.push_str(&format!(
                "({:.1},{:.1}) → ({:.1},{:.1}) precision={}",
                from.x, from.y, to.x, to.y, precision
            )),
            GameplayTraceEvent::Attack { kind, .. } => out.push_str(kind),
            GameplayTraceEvent::Damage { source, amount, .. } => {
                out.push_str(&format!("{source} {amount}"));
            }
            GameplayTraceEvent::RoomTransition { from, to, .. } => {
                out.push_str(&format!("{from} → {to}"));
            }
            GameplayTraceEvent::OobDetected { reason, pos, .. } => {
                out.push_str(&format!("{reason} @ ({:.1},{:.1})", pos.x, pos.y));
            }
            GameplayTraceEvent::CollisionCorrection {
                before,
                after,
                reason,
                nearby_after,
                state_flips,
                ..
            } => {
                out.push_str(&format!(
                    "({:.1},{:.1}) → ({:.1},{:.1}) [{reason}]",
                    before.x, before.y, after.x, after.y
                ));
                if !state_flips.is_empty() {
                    out.push_str(" flips=[");
                    out.push_str(&state_flips.join(", "));
                    out.push(']');
                }
                if !nearby_after.is_empty() {
                    out.push_str(" snapped_near=[");
                    let parts: Vec<String> = nearby_after
                        .iter()
                        .take(3)
                        .map(|s| {
                            format!(
                                "{} ({:.0},{:.0})→({:.0},{:.0}) d={:.1}",
                                s.kind,
                                s.aabb.min.x,
                                s.aabb.min.y,
                                s.aabb.max.x,
                                s.aabb.max.y,
                                s.distance,
                            )
                        })
                        .collect();
                    out.push_str(&parts.join("; "));
                    out.push(']');
                }
            }
            GameplayTraceEvent::Sfx { label, .. } | GameplayTraceEvent::Vfx { label, .. } => {
                out.push_str(label)
            }
            GameplayTraceEvent::Reset { .. } => out.push_str("reset"),
            GameplayTraceEvent::Death { .. } => out.push_str("death"),
            GameplayTraceEvent::Projectile {
                kind,
                event,
                damage,
                ..
            } => {
                out.push_str(&format!("{kind} {event} dmg={damage}"));
            }
        }
        out.push('\n');
    }
    out.push('\n');

    out.push_str("## Hints\n\n");
    out.push_str("- Compare `last_safe_pos` to the OOB position; the difference\n");
    out.push_str("  hints at whether the player tunneled, blinked, or fell.\n");
    out.push_str("- Look for the latest `Blink` / `Dash` / `RoomTransition` event\n");
    out.push_str("  before the OOB. Tunneling under a one-way platform is the\n");
    out.push_str("  most common cause of the active OOB bug.\n");
    out.push_str("- This trace is *not* a deterministic replay yet: timing\n");
    out.push_str("  jitter and audio/VFX subscribers can vary across runs.\n");
    out
}

/// Default dump directory relative to the current working directory.
pub fn default_dump_dir() -> PathBuf {
    PathBuf::from("debug_traces")
}
