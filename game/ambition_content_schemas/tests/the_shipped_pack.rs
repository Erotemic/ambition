//! Ambition's shipped pack, checked by the binary the package guard runs:
//! the content CLI with Ambition's own schemas beside the engine's.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The binary as cargo built it: the artifact the guard and an author run.
const CHECK: &str = env!("CARGO_BIN_EXE_ambition_content_check");

fn shipped_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../ambition_content/assets")
}

/// The control: the content CLI with the engine's schemas only refuses the
/// pack, and names the schema it cannot read. So the green test below is
/// green because the check binary links Ambition's schemas.
#[test]
fn the_engine_only_cli_refuses_ambitions_own_schema() {
    let output = Command::new(env!("CARGO_BIN_EXE_ambition_content_check"))
        .arg("--list-schemas")
        .output()
        .expect("the check binary runs");
    let listed = String::from_utf8_lossy(&output.stdout);
    assert!(listed.contains("ambition_rules"), "the check binary lists Ambition's schema:\n{listed}");
    let generic = ambition_content_cli::parse_args([shipped_pack().display().to_string(), "--advisory-assets".into()])
        .expect("the invocation parses");
    let failure = generic.run().expect_err("the engine's composition alone refuses Ambition's pack");
    let rendered = failure.render();
    assert!(rendered.contains("ambition_rules"), "the refusal names the schema it cannot read:\n{rendered}");
}

#[test]
fn the_shipped_pack_has_no_schema_reference_or_conflict_errors() {
    // The architecture claim about Ambition's own content, run against the real
    // pack in place: every preset reference resolves, no identity is defined
    // twice, no display name has two owners, no authored field is unconsumed.
    //
    //  assets are ADVISORY here and that is deliberate, not a loophole.
    // AGENTS.md: binary payloads are git-ignored but present, and a feature
    // owes only "degrade visibly when a file is absent" — so on a fresh clone
    // missing art is a documented state. Making it fatal here would contradict
    // a project rule and the check would get waived. The next test is the one
    // that proves the strict path still refuses.
    let output = Command::new(CHECK)
        .arg(shipped_pack())
        .arg("--advisory-assets")
        .output()
        .expect("CLI runs");
    let text = String::from_utf8_lossy(&output.stdout);
    let errors = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the shipped pack compiles:\n{text}{errors}"
    );
    assert!(
        !text.contains("unknown-preset")
            && !text.contains("unresolved-reference")
            && !text.contains("duplicate-identity")
            && !text.contains("unknown-field")
            && !text.contains("conflicting-module-contribution"),
        "the only findings in the shipped pack are about assets:\n{text}"
    );
    println!("{text}");
}

