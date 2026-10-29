//! Embed the packs selected by the host game.

fn main() {
    ambition_content_pack::embed::embed_pack("assets");
    ambition_content_pack::embed::embed_pack("assets/host");
}
