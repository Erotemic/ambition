//! The slots content joins to reset its own state when a room resets or
//! replays.
//!
//! The host anchors each set in its chain; generic machinery orders against the
//! SET and never names a content system. They live here, below the session that
//! consumes the requests, so a domain that joins one does not depend on it.

use bevy::prelude::SystemSet;

/// Room-transition slot for *content-side* reset work (named boss
/// arenas, story state). Content plugins register their reset systems in
/// this set; the host anchors the set into the room-transition chain, and
/// machinery that must run after content resets (e.g. gravity
/// reset-to-default) orders against the SET — generic plugins never name
/// a content system.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentRoomResetSet;

/// Player-input-phase slot for content systems that FOLLOW UP a closed
/// dialogue with a request (e.g. emit `RoomReplayRequested` after a
/// "try again" conversation ends). Content plugins register emitters in
/// this set; the host anchors it before the replay consumer so a request
/// lands the same frame it is emitted.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentDialogueFollowupSet;

/// Player-input-phase slot for content systems that reset *content-named*
/// per-attempt state when a `RoomReplayRequested` fires (e.g. clear a named
/// boss's persisted "cleared" record before the room replays). Content plugins
/// register their reset systems here; the host anchors the set before its
/// generic replay consumer so the content reset lands the same frame the
/// request does — the consumer never names content.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentRoomReplayResetSet;

