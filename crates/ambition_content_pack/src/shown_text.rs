//! A schema for a game's shown text: each line the game shows (a banner, a
//! label, a card), by key, as data in its pack.
//!
//! A line in Rust costs a build to reword. Here it is a file the game reads
//! at start. The keys are the game's: it declares each line it shows and the
//! holes the line can fill ([`TextKeys`]), so admission refuses a line the
//! game reads and the file does not state, a line the game never reads (a
//! typing mistake), and a hole the game does not fill, each named by its key.
//!
//! A key names a line, not its words, so a later catalog of languages can
//! state the same keys. There is one language now.

use std::collections::BTreeMap;
use std::fmt::Display;

use serde::Deserialize;

use crate::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};

/// One line a game shows: its key, and the holes it can fill (`{name}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextKey {
    pub key: &'static str,
    pub holes: &'static [&'static str],
}

impl TextKey {
    /// A line with no hole.
    pub const fn plain(key: &'static str) -> Self {
        Self { key, holes: &[] }
    }

    /// A line that fills `holes`.
    pub const fn with(key: &'static str, holes: &'static [&'static str]) -> Self {
        Self { key, holes }
    }
}

/// The lines a game shows: the keys its text file must state.
pub trait TextKeys: Send + Sync + 'static {
    const KEYS: &'static [TextKey];
}

/// A game's shown text: each line by its key.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ShownText(BTreeMap<String, String>);

impl ShownText {
    /// The line `key`. Admission refused a file without it, so a key the
    /// game did not declare is the only way to miss, and that is a defect in
    /// the game: it panics with the key named.
    pub fn line(&self, key: &str) -> &str {
        self.0
            .get(key)
            .unwrap_or_else(|| panic!("the shown text states no line `{key}`; add the key to the game's `TextKeys`"))
    }

    /// The line `key` with each `{hole}` filled.
    pub fn fill(&self, key: &str, holes: &[(&str, &dyn Display)]) -> String {
        let mut line = self.line(key).to_string();
        for (hole, value) in holes {
            line = line.replace(&format!("{{{hole}}}"), &value.to_string());
        }
        line
    }

    /// Each line of `keys` the file does not state, each line it states that
    /// is not one of `keys`, and each hole a line names that its key does not
    /// fill.
    pub fn problems(&self, keys: &[TextKey]) -> Vec<String> {
        let mut problems = Vec::new();
        for declared in keys {
            if !self.0.contains_key(declared.key) {
                problems.push(format!("`{}` is not stated; the game shows this line", declared.key));
            }
        }
        for (key, line) in &self.0 {
            let Some(declared) = keys.iter().find(|declared| declared.key == key) else {
                problems.push(format!("`{key}` is not a line the game shows"));
                continue;
            };
            for hole in holes_of(line) {
                if !declared.holes.contains(&hole) {
                    problems.push(format!(
                        "`{key}` names the hole `{{{hole}}}`, which the game does not fill (it fills {:?})",
                        declared.holes
                    ));
                }
            }
        }
        problems
    }
}

/// Each `{word}` of `line`: a word of letters, digits and underscores.
fn holes_of(line: &str) -> Vec<&str> {
    let mut holes = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) if close > 0 && after[..close].chars().all(|c| c.is_ascii_alphanumeric() || c == '_') => {
                holes.push(&after[..close]);
                rest = &after[close + 1..];
            }
            _ => rest = after,
        }
    }
    holes
}

fn problems_for<K: TextKeys>(text: &ShownText) -> Vec<String> {
    text.problems(K::KEYS)
}

/// The schema of a game's shown text, with the keys of `K`.
pub fn shown_text_schema<K: TextKeys>(id: &'static str, capability: &str, doc: &'static str) -> SchemaRegistration {
    rules_schema::<ShownText>(id, 1, capability, doc, problems_for::<K>)
}

/// The shown text the pack states in its `id` source, or `None` when it
/// declares no such source.
pub fn lowered_shown_text<'pack>(pack: &'pack PreparedContentPack, id: &str) -> Option<&'pack ShownText> {
    lowered_rules::<ShownText>(pack, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, AssetsUnchecked, CompileFailure, ContentPackDraft, SchemaRegistry};

    struct ProbeKeys;
    impl TextKeys for ProbeKeys {
        const KEYS: &'static [TextKey] = &[TextKey::plain("go"), TextKey::with("winner", &["name"])];
    }

    fn compiled(text: &str) -> Result<PreparedContentPack, CompileFailure> {
        let mut registry = SchemaRegistry::new();
        registry
            .register(shown_text_schema::<ProbeKeys>("probe_text", "probe", "a probe's shown text"))
            .expect("one registration");
        let draft = ContentPackDraft::from_manifest_ron(
            r#"(id: "probe", version: "0.1.0", namespace: "probe", requires: [],
                sources: [(path: "text.ron", schema: "probe_text", version: 1)])"#,
            [("text.ron".to_string(), text.to_string())],
        )?;
        compile(&draft, &registry, &AssetsUnchecked)
    }

    /// The file's lines are the lines shown, with their holes filled; a
    /// line the game shows and the file leaves out, a line the game never
    /// shows, and a hole the game does not fill are each refused with the
    /// file and the key named.
    #[test]
    fn the_shown_text_is_the_files_and_each_refusal_names_its_key() {
        let pack = compiled(r#"{"go": "GO!", "winner": "WINNER: {name}"}"#).unwrap_or_else(|failure| panic!("{}", failure.render()));
        let text = lowered_shown_text(&pack, "probe_text").expect("the probe states its text");
        assert_eq!(text.line("go"), "GO!");
        assert_eq!(text.fill("winner", &[("name", &"Alice")]), "WINNER: Alice");
        // A hole the game fills may be left out: the line is the author's.
        let pack = compiled(r#"{"go": "GO!", "winner": "A WINNER"}"#).unwrap_or_else(|failure| panic!("{}", failure.render()));
        assert_eq!(lowered_shown_text(&pack, "probe_text").unwrap().fill("winner", &[("name", &"Alice")]), "A WINNER");

        for (text, says) in [
            (r#"{"winner": "WINNER: {name}"}"#, "`go` is not stated"),
            (r#"{"go": "GO!", "winner": "WINNER: {name}", "goo": "GOO"}"#, "`goo` is not a line the game shows"),
            (r#"{"go": "GO!", "winner": "WINNER: {nmae}"}"#, "`winner` names the hole `{nmae}`"),
        ] {
            let rendered = compiled(text).expect_err(text).render();
            assert!(rendered.contains(says) && rendered.contains("text.ron"), "{text}: {rendered}");
        }
    }

    #[test]
    fn a_hole_is_a_word_in_braces() {
        assert_eq!(holes_of("{a} and {b_2}, not { c } nor {}"), ["a", "b_2"]);
    }
}
