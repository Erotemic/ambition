//! The lines Ambition shows, as its pack states them (`data/ambition_text.ron`,
//! schema `ambition_text`, owned by `ambition_content_schemas::text`): its
//! launcher row and the cover while the first frame is prepared. A content
//! reload restates them (`reload::publish_ambition_text`).

use ambition_content_pack::ShownText;
use ambition_content_schemas::text::lowered_ambition_text;
use bevy::prelude::{App, Resource};

/// The lines the selected pack states.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct AmbitionShownText(pub ShownText);

/// Insert the lines the selected pack states.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    app.insert_resource(AmbitionShownText(ambition_text_of(&pack)));
}

/// The lines `pack` states. Ambition's pack states them, so a pack without
/// them is refused here, with the file named.
pub fn ambition_text_of(pack: &ambition_content_pack::PreparedContentPack) -> ShownText {
    lowered_ambition_text(pack).expect("Ambition's pack states its lines (`data/ambition_text.ron`)")
}
