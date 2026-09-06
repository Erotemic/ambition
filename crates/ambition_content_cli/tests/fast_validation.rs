//! The acceptance criterion for the whole content-compiler program:
//!
//! > valid character addition → validates without rebuilding Rust
//!
//! These tests spawn the already-built binary and edit content in a temporary
//! directory the binary has never seen. Successful validation therefore proves
//! that content changes do not require rebuilding Rust.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// The binary as cargo built it. Spawning this — rather than calling the
/// library — is the whole point: it is the artifact an author actually runs.
const CLI: &str = env!("CARGO_BIN_EXE_ambition_content");

/// Ambition's real, shipped catalog.
fn shipped_catalog() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../game/ambition_content/assets/data/character_catalog.ron")
}

fn shipped_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game/ambition_content/assets")
}

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// A pack holding a COPY of the shipped catalog. Real content, so the
    /// timing below is the timing an author sees.
    fn with_shipped_catalog(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("ambition_fast_validation/{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).expect("temp pack");
        std::fs::copy(shipped_catalog(), root.join("data/character_catalog.ron"))
            .expect("the shipped catalog is readable");
        std::fs::write(
            root.join("pack.ron"),
            r#"(
                id: "ambition",
                version: "0.1.0",
                namespace: "ambition",
                requires: [],
                sources: [
                    (path: "data/character_catalog.ron", schema: "character_catalog", version: 1),
                ],
            )"#,
        )
        .expect("write manifest");
        Self { root }
    }

    fn catalog(&self) -> PathBuf {
        self.root.join("data/character_catalog.ron")
    }

    fn read_catalog(&self) -> String {
        std::fs::read_to_string(self.catalog()).expect("read catalog")
    }

    fn write_catalog(&self, text: &str) {
        std::fs::write(self.catalog(), text).expect("write catalog");
    }

    /// Run the built binary. Returns (exit code, stdout+stderr, elapsed).
    fn validate(&self, extra: &[&str]) -> (i32, String, Duration) {
        let started = Instant::now();
        let output = Command::new(CLI)
            .arg(&self.root)
            // Assets are not what these tests are about, and the sandbox has
            // none: this keeps the measurement about SCHEMA + REFERENCES.
            .arg("--no-asset-check")
            .args(extra)
            .output()
            .expect("the CLI is built and runnable");
        let elapsed = started.elapsed();
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        (output.status.code().unwrap_or(-1), text, elapsed)
    }
}

#[test]
fn validating_a_character_edit_does_not_rebuild_rust() {
    let sandbox = Sandbox::with_shipped_catalog("edit_cycle");

    // 1. The shipped catalog, as authored.
    let (code, first, baseline) = sandbox.validate(&[]);
    assert_eq!(code, 0, "the shipped catalog validates:\n{first}");
    assert!(
        first.contains("× character"),
        "it reports the cast:\n{first}"
    );

    // 2. ADD a character — the exact act the acceptance criterion names. No
    //    cargo runs between here and step 3.
    let added = sandbox.read_catalog().replacen(
        "characters: {",
        r#"characters: {
        "npc_test_newcomer": (
            display_name: "Test Newcomer",
            spritesheet: "sprites/goblin_spritesheet.png",
            manifest: "sprites/goblin_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "peaceful",
            tags: [],
        ),"#,
        1,
    );
    assert_ne!(added, sandbox.read_catalog(), "the edit applied");
    sandbox.write_catalog(&added);

    let (code, second, after_add) = sandbox.validate(&[]);
    assert_eq!(
        code, 0,
        "a valid character addition validates — WITHOUT a rebuild:\n{second}"
    );
    assert!(
        second.contains("Test Newcomer") || !second.contains("error"),
        "no errors after a valid addition:\n{second}"
    );

    // 3. And it is a DIFFERENT answer, not a cached one. If the binary had
    //    embedded the catalog it would report the old fingerprint forever,
    //    which is precisely the coupling this whole program removes.
    let fingerprint_of = |text: &str| {
        text.lines()
            .next()
            .and_then(|line| line.split("fingerprint ").nth(1))
            .map(str::trim)
            .unwrap_or("<none>")
            .to_string()
    };
    assert_ne!(
        fingerprint_of(&first),
        fingerprint_of(&second),
        "adding a character moved the pack fingerprint — the binary is reading the file, not a \
         copy compiled into itself"
    );

    // 4. A BROKEN edit is caught, in the same cycle, at the same cost.
    sandbox.write_catalog(&added.replacen(
        r#"default_brain: "stand_still","#,
        r#"default_brain: "stand_stil","#,
        1,
    ));
    let (code, third, after_break) = sandbox.validate(&[]);
    assert_eq!(code, 1, "a typo is refused:\n{third}");
    assert!(
        third.contains("unknown-preset") && third.contains("npc_test_newcomer"),
        "and the refusal names the preset AND the character that named it:\n{third}"
    );
    assert!(
        third.contains("did you mean `stand_still`?"),
        "and answers the typo rather than only rejecting it:\n{third}"
    );

    // The measurement the program asked us to track. Not a threshold assertion
    // — a number in the log, because the claim is "milliseconds, not minutes"
    // and an author reading this test should see the actual figure.
    println!(
        "validation latency (real shipped catalog, {} chars):\n  \
         baseline {:?}\n  after adding a character {:?}\n  after breaking it {:?}",
        added.len(),
        baseline,
        after_add,
        after_break
    );
    // One order-of-magnitude sanity bound. A ~10 minute rebuild is 600_000 ms;
    // this asserts three orders of magnitude better than that, which no cargo
    // invocation could ever satisfy.
    assert!(
        after_add < Duration::from_secs(2),
        "validation took {after_add:?} — the point of this path is that it is not a build"
    );
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
    let output = Command::new(CLI)
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

#[test]
fn the_strict_asset_path_still_refuses_on_real_content() {
    // Copy the shipped catalog into a pack with no assets so strict validation
    // has genuinely unresolved references independent of the shipped pack's
    // current completeness.
    let sandbox = Sandbox::with_shipped_catalog("strict_assets");
    let output = Command::new(CLI)
        .arg(&sandbox.root)
        .output()
        .expect("CLI runs");
    let errors = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "strict mode refuses a pack whose art is absent:\n{errors}"
    );
    assert!(
        errors.contains("missing-asset"),
        "and says WHY it refused:\n{errors}"
    );
}

#[test]
fn the_tool_lists_what_it_installs() {
    // An agent's first question is "what may I author?", and the answer must
    // not be "read the engine source".
    let output = Command::new(CLI)
        .arg("--list-schemas")
        .output()
        .expect("CLI runs");
    let text = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        text.contains("character_catalog") && text.contains("characters"),
        "the schema and its owning capability are both named:\n{text}"
    );
}
