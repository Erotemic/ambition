//! Embed host packs and stamp the application build.

fn main() {
    build_label();
    ambition_content_pack::embed::embed_pack("assets", &[ambition_sprite_part_format::compile_metadata]);
    ambition_content_pack::embed::embed_pack("assets/host", &[ambition_sprite_part_format::compile_metadata]);
}

fn git(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git").args(args).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The build label is compiled in only where the binary cannot report its own
/// build: an explicit `AMBITION_BUILD_ID` (CI, a release), or a browser or
/// Android build (no executable file to read). A native development build has
/// no compiled label: the pause menu reads the executable's own build time at
/// runtime (`menu::build_label`). A compiled stamp that watched every
/// workspace package and the Git index rebuilt and relinked the app for edits
/// outside its dependency graph and for a `git add` (runtime authoring audit
/// 2026-11-14, B01).
fn build_label() {
    println!("cargo:rerun-if-env-changed=AMBITION_BUILD_ID");
    println!("cargo:rerun-if-env-changed=AMBITION_HIDE_BUILD_ID");
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let deployed = target_arch == "wasm32" || target_os == "android";
    if let Ok(id) = std::env::var("AMBITION_BUILD_ID") {
        println!("cargo:rustc-env=AMBITION_BUILD_LABEL=Build {id}");
        return;
    }
    if !deployed {
        return;
    }
    // A deployed build is made rarely and must say what it is: the stamp, the
    // revision, and whether the tree was clean, compiled in.
    for path in ["HEAD", "index", "packed-refs"] {
        if let Some(path) = git(&["rev-parse", "--git-path", path]) {
            if std::path::Path::new(&path).exists() { println!("cargo:rerun-if-changed={path}"); }
        }
    }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .expect("the build clock is after the Unix epoch").as_secs();
    let revision = git(&["rev-parse", "--short=10", "HEAD"]).unwrap_or_else(|| "source".into());
    let dirty = git(&["status", "--porcelain", "--untracked-files=no", "--ignore-submodules=dirty"])
        .is_some_and(|status| !status.is_empty());
    println!("cargo:rustc-env=AMBITION_BUILD_LABEL=Build {stamp} / {revision}{}", if dirty { "+" } else { "" });
}
