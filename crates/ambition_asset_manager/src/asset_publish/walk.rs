//! A tiny recursive file walk shared by the publisher and the hygiene
//! validator. Returns paths relative to `root`, in sorted order, so callers get
//! deterministic output without pulling in an external walk crate.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Collect every regular file under `root`, as paths relative to `root`.
/// Returns an empty vec (not an error) when `root` does not exist, so callers
/// can treat an absent gitignored variant root as "nothing to check".
pub fn walk_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    collect(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(root, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}
