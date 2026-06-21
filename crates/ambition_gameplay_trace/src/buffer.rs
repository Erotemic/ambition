//! The `GameplayTraceBuffer` resource: a rolling ring buffer of per-frame
//! snapshots and discrete events that the game's recorder systems push into. Owns
//! capacity/tick/sequence bookkeeping; the dump writers live in `dump`.

use crate::*;
use bevy::prelude::*;
use std::collections::VecDeque;

/// Top-level rolling buffer.
#[derive(Resource, Debug)]
pub struct GameplayTraceBuffer {
    pub capacity_frames: usize,
    pub capacity_events: usize,
    pub frames: VecDeque<GameplayTraceFrame>,
    pub events: VecDeque<GameplayTraceEvent>,
    pub sequence: u64,
    pub tick: u64,
    pub last_dump_path: Option<String>,
    pub last_dump_status: Option<String>,
    pub dump_request: Option<DumpReason>,
    /// Once an OOB has auto-dumped we suppress further auto-dumps until
    /// the player is no longer OOB; otherwise a single broken frame would
    /// produce 60 dump files per second.
    pub auto_dump_armed: bool,
    /// True after the very first frame has been recorded; lets us produce
    /// useful "first OOB frame" output without indexing into an empty
    /// buffer.
    pub has_recorded_any: bool,
    /// Frame-to-frame diff source for synthetic events.
    pub previous: Option<PreviousFrameSnapshot>,
    /// Frames remaining in the portal-transit suppression WINDOW. A portal
    /// crossing both snaps the player a long way (an "unexplained" position
    /// delta) AND lands it at the exit before the exit-side carve has opened
    /// (so it momentarily reads as inside-solid), each of which would auto-dump.
    /// Set to a few frames when a `BodyTeleported` fires; while > 0 BOTH the
    /// position-delta and the OOB auto-dumps are suppressed, so a normal transit
    /// never spams a trace dump. Decremented once per frame in `record_frame`.
    pub teleport_suppress_ticks: u32,
    /// Auto-dumps (OOB + teleport) are suppressed until the buffer holds at
    /// least this many frames. Skips spawn-settling transients — the player is
    /// authored with its feet a hair inside the floor, so it reads `inside
    /// solid` for a tick or two before the first collision resolve lifts it
    /// out — and guarantees a dump carries pre-anomaly lead-up instead of a
    /// useless 1-frame snapshot. Manual (F8) dumps are never gated. Mirrors the
    /// actor trace's gate (see `DEFAULT_MIN_CONTEXT_FRAMES`).
    pub min_context_frames: usize,
}

/// How many frames a portal transit suppresses trace auto-dumps for: long enough
/// to cover the transfer snap plus the exit-side settle (carve opening + any
/// collision push-out), short enough that a genuinely stuck body still dumps.
pub const PORTAL_TELEPORT_SUPPRESS_FRAMES: u32 = 8;

impl Default for GameplayTraceBuffer {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_FRAME_CAPACITY, DEFAULT_EVENT_CAPACITY)
    }
}

impl GameplayTraceBuffer {
    pub fn with_capacity(frames: usize, events: usize) -> Self {
        Self {
            capacity_frames: frames.max(1),
            capacity_events: events.max(1),
            frames: VecDeque::with_capacity(frames.max(1)),
            events: VecDeque::with_capacity(events.max(1)),
            sequence: 0,
            tick: 0,
            last_dump_path: None,
            last_dump_status: None,
            dump_request: None,
            auto_dump_armed: true,
            has_recorded_any: false,
            previous: None,
            teleport_suppress_ticks: 0,
            min_context_frames: crate::DEFAULT_MIN_CONTEXT_FRAMES,
        }
    }

    /// True once the buffer holds enough lead-up frames for an auto-dump to be
    /// worth taking (see [`Self::min_context_frames`]).
    pub fn has_min_context(&self) -> bool {
        self.frames.len() >= self.min_context_frames
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    pub fn request_dump(&mut self, reason: DumpReason) {
        if self.dump_request.is_none() {
            self.dump_request = Some(reason);
        }
    }

    pub fn push_frame(&mut self, frame: GameplayTraceFrame) {
        if self.frames.len() == self.capacity_frames {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
        self.sequence = self.sequence.saturating_add(1);
        self.tick = self.tick.saturating_add(1);
        self.has_recorded_any = true;
    }

    pub fn push_event(&mut self, event: GameplayTraceEvent) {
        if self.events.len() == self.capacity_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Drain `events` into the buffer in order.
    pub fn extend_events<I: IntoIterator<Item = GameplayTraceEvent>>(&mut self, events: I) {
        for ev in events {
            self.push_event(ev);
        }
    }

    pub fn frames(&self) -> impl Iterator<Item = &GameplayTraceFrame> {
        self.frames.iter()
    }

    pub fn events(&self) -> impl Iterator<Item = &GameplayTraceEvent> {
        self.events.iter()
    }
}

#[cfg(test)]
mod buffer_tests {
    //! The trace ring buffer: capacity is clamped to >=1, push is a
    //! bounded ring that evicts the oldest entry, and request_dump keeps
    //! the first request (so a later auto-dump can't clobber a pending
    //! reason). push_event shares the eviction logic with push_frame.
    use super::*;

    fn jump(tick: u64) -> GameplayTraceEvent {
        GameplayTraceEvent::Jump { tick }
    }

    fn event_ticks(b: &GameplayTraceBuffer) -> Vec<u64> {
        b.events()
            .map(|e| match e {
                GameplayTraceEvent::Jump { tick } => *tick,
                _ => 0,
            })
            .collect()
    }

    #[test]
    fn with_capacity_clamps_to_at_least_one() {
        let b = GameplayTraceBuffer::with_capacity(0, 0);
        assert_eq!(b.capacity_frames, 1);
        assert_eq!(b.capacity_events, 1);
    }

    #[test]
    fn push_event_is_a_bounded_ring_dropping_oldest() {
        let mut b = GameplayTraceBuffer::with_capacity(8, 2);
        b.push_event(jump(1));
        b.push_event(jump(2));
        b.push_event(jump(3)); // evicts tick 1
        assert_eq!(b.event_count(), 2);
        assert_eq!(
            event_ticks(&b),
            vec![2, 3],
            "oldest evicted, order preserved"
        );
    }

    #[test]
    fn extend_events_drains_in_order_within_capacity() {
        let mut b = GameplayTraceBuffer::with_capacity(8, 3);
        b.extend_events([jump(1), jump(2), jump(3), jump(4)]);
        assert_eq!(b.event_count(), 3);
        assert_eq!(event_ticks(&b), vec![2, 3, 4]);
    }

    #[test]
    fn request_dump_keeps_the_first_request() {
        let mut b = GameplayTraceBuffer::with_capacity(4, 4);
        assert!(b.dump_request.is_none());
        b.request_dump(DumpReason::Manual);
        b.request_dump(DumpReason::Programmatic {
            label: "later".into(),
        });
        assert!(
            matches!(b.dump_request, Some(DumpReason::Manual)),
            "the first dump request wins"
        );
    }
}
