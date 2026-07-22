//! Gameplay flight-recorder format — the reusable, content-free core of the
//! trace recorder.
//!
//! A rolling ring buffer of per-frame player snapshots ([`GameplayTraceFrame`])
//! and discrete gameplay events ([`GameplayTraceEvent`]), plus the markdown/JSON
//! [`write_dump`] writers. The game's simulation systems (the recorder /
//! OOB-detector, which read live player + world state) fill the buffer; this
//! crate owns only the FORMAT, so the headless replay harness can read a dump
//! without depending on the game.
//!
//! Extracted from `ambition_actors` (`dev/trace/`); the recording systems
//! (`detect` / `systems` / `plugin`) stay in the game next to the sim state they
//! sample.

mod actor_trace;
mod buffer;
mod dump;
mod model;
mod policy;

pub use actor_trace::*;
pub use buffer::*;
pub use dump::*;
pub use model::*;
pub use policy::*;

/// Default rolling capacity (frames) for a fresh [`GameplayTraceBuffer`].
pub const DEFAULT_FRAME_CAPACITY: usize = 240;
/// Default rolling capacity (discrete events) for a fresh buffer.
pub const DEFAULT_EVENT_CAPACITY: usize = 240;
/// How many of the most-recent frames the markdown dump summarizes.
pub const MARKDOWN_FRAME_SUMMARY_TAIL: usize = 120;
/// How many of the most-recent events the markdown dump lists.
pub const MARKDOWN_EVENT_TAIL: usize = 100;
