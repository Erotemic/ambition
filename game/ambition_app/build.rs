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

fn build_label() {
    println!("cargo:rerun-if-env-changed=AMBITION_BUILD_ID");
    println!("cargo:rerun-if-env-changed=AMBITION_HIDE_BUILD_ID");
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(workspace) = manifest.parent().and_then(|parent| parent.parent()) {
        for family in ["crates", "game"] {
            if let Ok(entries) = std::fs::read_dir(workspace.join(family)) {
                for entry in entries.flatten() {
                    let member = entry.path();
                    if !member.join("Cargo.toml").is_file() { continue; }
                    for source in ["src", "build.rs", "Cargo.toml"] {
                        let source = member.join(source);
                        if source.exists() {
                            println!("cargo:rerun-if-changed={}", source.display());
                        }
                    }
                }
            }
        }
    }
    for path in ["HEAD", "index", "packed-refs"] {
        if let Some(path) = git(&["rev-parse", "--git-path", path]) {
            if std::path::Path::new(&path).exists() { println!("cargo:rerun-if-changed={path}"); }
        }
    }
    if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"]) {
        if let Some(path) = git(&["rev-parse", "--git-path", &reference]) {
            let reference = std::path::Path::new(&path);
            if reference.exists() {
                println!("cargo:rerun-if-changed={path}");
            } else if let Some(parent) = reference.parent().filter(|parent| parent.exists()) {
                println!("cargo:rerun-if-changed={}", parent.display());
            }
        }
    }
    let label = std::env::var("AMBITION_BUILD_ID").unwrap_or_else(|_| {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .expect("the build clock is after the Unix epoch").as_secs();
        let revision = git(&["rev-parse", "--short=10", "HEAD"]).unwrap_or_else(|| "source".into());
        let dirty = git(&["status", "--porcelain", "--untracked-files=no", "--ignore-submodules=dirty"])
            .is_some_and(|status| !status.is_empty());
        format!("{stamp} / {revision}{}", if dirty { "+" } else { "" })
    });
    println!("cargo:rustc-env=AMBITION_BUILD_LABEL=Build {label}");
}
