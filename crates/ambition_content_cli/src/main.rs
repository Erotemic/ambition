//! `ambition_content` — validate a content pack without rebuilding the engine.
//!
//! The CLI is a diagnostic FRONT DOOR, not the enforcement mechanism. The same
//! `ambition_content_pack::compile` runs in the standard test, in CI, in
//! development reload and in packaging; this binary exists so an author gets
//! the answer in milliseconds instead of a ten-minute rebuild.

use crate::{USAGE, default_registry, parse_args};

fn main() -> std::process::ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return std::process::ExitCode::SUCCESS;
    }

    let invocation = match parse_args(raw) {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("ambition_content: {error}\n\n{USAGE}");
            return std::process::ExitCode::from(2);
        }
    };

    if invocation.list_schemas {
        let registry = default_registry();
        println!("installed schemas:");
        for schema in registry.schemas() {
            println!(
                "  {} {}  (capability `{}`, {:?})\n      {}",
                schema.id, schema.version, schema.capability, schema.disposition, schema.doc
            );
        }
        println!("\ninstalled capabilities:");
        for capability in registry.capabilities() {
            println!("  {capability}");
        }
        return std::process::ExitCode::SUCCESS;
    }

    match invocation.run() {
        Ok(pack) => {
            if invocation.fingerprint_only {
                println!("{}", pack.fingerprint);
            } else {
                print!("{}", pack.summary());
            }
            std::process::ExitCode::SUCCESS
        }
        Err(failure) => {
            eprint!("{}", failure.render());
            std::process::ExitCode::from(1)
        }
    }
}
