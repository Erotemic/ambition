use bevy::prelude::{Res, ResMut};

use super::{ActiveRoomMetadata, RoomMusicRequest, RoomSet};

/// Mirror `RoomSet::active_metadata()` into the `ActiveRoomMetadata`
/// resource, but only when the metadata actually changes. The
/// PartialEq guard means change-detection consumers (e.g. a future
/// room-music selector) only fire when the active room's biome /
/// music_track / ambient / theme really differ — not on every frame.
pub fn sync_active_room_metadata(room_set: Res<RoomSet>, mut active: ResMut<ActiveRoomMetadata>) {
    let current = room_set.active_metadata().clone();
    if current != active.0 {
        active.0 = current;
    }
}

/// Push the active room's `music_track` into `RoomMusicRequest` so the
/// audio system knows the room-default track when no encounter
/// override is active. Empty values clear the request, falling back to
/// `sandbox_data.audio.default_music_track`.
pub fn sync_room_music_request(
    active: Res<ActiveRoomMetadata>,
    mut request: ResMut<RoomMusicRequest>,
) {
    let next = active.0.music_track.clone();
    if next != request.desired_track {
        request.desired_track = next;
    }
}
