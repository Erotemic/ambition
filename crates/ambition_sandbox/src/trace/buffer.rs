use super::*;

/// Top-level rolling buffer.
#[derive(Resource, Debug)]
pub struct GameplayTraceBuffer {
    pub(super) capacity_frames: usize,
    pub(super) capacity_events: usize,
    pub(super) frames: VecDeque<GameplayTraceFrame>,
    pub(super) events: VecDeque<GameplayTraceEvent>,
    pub(super) sequence: u64,
    pub(super) tick: u64,
    pub last_dump_path: Option<String>,
    pub last_dump_status: Option<String>,
    pub dump_request: Option<DumpReason>,
    /// Once an OOB has auto-dumped we suppress further auto-dumps until
    /// the player is no longer OOB; otherwise a single broken frame would
    /// produce 60 dump files per second.
    pub(super) auto_dump_armed: bool,
    /// True after the very first frame has been recorded; lets us produce
    /// useful "first OOB frame" output without indexing into an empty
    /// buffer.
    pub(super) has_recorded_any: bool,
    /// Frame-to-frame diff source for synthetic events.
    pub(super) previous: Option<PreviousFrameSnapshot>,
}

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
        }
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

    pub(super) fn push_frame(&mut self, frame: GameplayTraceFrame) {
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
