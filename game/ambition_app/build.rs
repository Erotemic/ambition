//! Embed the packs selected by the host game.

fn main() {
    ambition_content_pack::embed::embed_pack("assets", &[ambition_sprite_part_format::compile_metadata]);
    ambition_content_pack::embed::embed_pack("assets/host", &[ambition_sprite_part_format::compile_metadata]);
}
