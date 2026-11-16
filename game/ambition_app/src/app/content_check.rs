//! The shipped binary's own content validator.
//!
//! `ambition_game_bin content <pack-dir> [options]` is the content CLI
//! (`ambition_content_cli`) with the schemas this game compiles its pack
//! against, so an author validates a pack with the binary they run it with
//! and builds nothing. `ambition_game_bin --schema-identity` prints the
//! identity of the registry the running game compiles against; the CLI's
//! `--schema-identity` prints the same value when the two install the same
//! schemas, so a separately shipped validator can be checked against the
//! runtime it claims to match.

/// Run the content command the arguments ask for; `None` starts the game.
pub fn run_if_asked(args: &[String]) -> Option<std::process::ExitCode> {
    match args.first().map(String::as_str) {
        Some("content") => Some(ambition_platformer2d::content::check_pack_main_with_args(
            args[1..].to_vec(),
            ambition_content_schemas::ambition_schemas,
        )),
        Some("--schema-identity") => {
            println!("{}", ambition_content::pack::pack_schemas().identity());
            Some(std::process::ExitCode::SUCCESS)
        }
        _ => None,
    }
}
