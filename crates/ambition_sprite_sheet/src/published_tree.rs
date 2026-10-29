//! Intern content identities and read native metadata text.

use std::path::PathBuf;

pub(crate) fn kept(text: impl AsRef<str>) -> &'static str {
    static IDENTITIES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<&'static str>>> = std::sync::OnceLock::new();
    let mut identities = IDENTITIES.get_or_init(Default::default).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let text = text.as_ref();
    if let Some(held) = identities.get(text) { return held; }
    let held = &*Box::leak(text.to_owned().into_boxed_str());
    identities.insert(held);
    held
}

pub(crate) fn texts(entries: Vec<(String, PathBuf)>) -> Vec<(&'static str, String)> {
    entries.into_iter().filter_map(|(key, path)| match std::fs::read_to_string(&path) {
        Ok(text) => Some((kept(key), text)),
        Err(error) => {
            tracing::warn!("{}: {error}", path.display());
            None
        }
    }).collect()
}
