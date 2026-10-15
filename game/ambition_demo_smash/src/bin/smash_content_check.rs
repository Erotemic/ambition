//! `smash_content_check`: the content CLI over a pack, with the schemas this
//! demo owns (`smash_stages`, `smash_rules`) beside the engine's. The generic
//! CLI refuses the demo's pack: no installed capability owns its schemas.

fn main() -> std::process::ExitCode {
    ambition_platformer2d::content::check_pack_main(ambition_demo_smash::smash_pack::PACK.game_schemas())
}
