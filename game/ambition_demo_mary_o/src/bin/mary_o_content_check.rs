//! `mary_o_content_check`: the content CLI over a pack, with the schema this
//! demo owns (`mary_o_rules`) beside the engine's. The generic CLI refuses
//! the demo's pack: no installed capability owns its schema.

fn main() -> std::process::ExitCode {
    ambition_platformer2d::content::check_pack_main(ambition_demo_mary_o::pack::PACK.game_schemas())
}
