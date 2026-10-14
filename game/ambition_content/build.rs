//! Embed Ambition's pack in a build that has no source tree: the list is
//! `assets/pack.ron` itself (`ambition_content_pack::embed`).

fn main() {
    ambition_content_pack::embed::embed_pack("assets");
}
