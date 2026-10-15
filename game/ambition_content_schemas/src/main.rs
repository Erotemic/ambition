//! `ambition_content_check`: the content CLI over Ambition's pack, with the
//! schemas Ambition owns beside the engine's. The package guard runs it.

fn main() -> std::process::ExitCode {
    ambition_content_cli::main_with(ambition_content_schemas::ambition_schemas)
}
