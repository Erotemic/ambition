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

/// Stamp builds for distribution. Native development reads the executable's
/// time at runtime, so Git metadata changes do not cause a relink.
fn build_label() {
    println!("cargo:rerun-if-env-changed=AMBITION_BUILD_ID");
    println!("cargo:rerun-if-env-changed=AMBITION_HIDE_BUILD_ID");
    println!("cargo:rerun-if-env-changed=AMBITION_STAMP_BUILD");
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let deployed = target_arch == "wasm32" || target_os == "android"
        || std::env::var_os("CARGO_FEATURE_DESKTOP_PLAYER").is_some()
        || matches!(std::env::var("AMBITION_STAMP_BUILD").as_deref(), Ok("1"));
    let id = std::env::var("AMBITION_BUILD_ID").ok();
    if !deployed && id.is_none() {
        return;
    }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .expect("the build clock is after the Unix epoch").as_secs();
    println!("cargo:rustc-env=AMBITION_BUILD_EPOCH={stamp}");
    if let Some(id) = id {
        println!("cargo:rustc-env=AMBITION_BUILD_LABEL={id}");
        return;
    }
    let head_ref = git(&["symbolic-ref", "--quiet", "HEAD"]);
    for name in ["HEAD", "index", "packed-refs"].into_iter().chain(head_ref.as_deref()) {
        if let Some(path) = git(&["rev-parse", "--git-path", name]) {
            if std::path::Path::new(&path).exists() { println!("cargo:rerun-if-changed={path}"); }
        }
    }
    let revision = git(&["rev-parse", "--short=10", "HEAD"]).unwrap_or_else(|| "source".into());
    let dirty = git(&["status", "--porcelain", "--untracked-files=no", "--ignore-submodules=dirty"])
        .is_some_and(|status| !status.is_empty());
    println!("cargo:rustc-env=AMBITION_BUILD_LABEL={revision}{}", if dirty { "+" } else { "" });
}
