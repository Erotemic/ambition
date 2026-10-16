//! The `yarn_dialogue` schema: each Yarn file of the game's dialogue is a
//! source of the pack (`(path: "dialogue/sandbox/intro.yarn", schema:
//! "yarn_dialogue", version: 1)`).
//!
//! The manifest is the one list of the dialogue files, so a new file is a
//! line in `pack.ron` and no Rust build, and a build without a source tree
//! embeds each file as it embeds every source. Admission reads each node's
//! `title:` and its closing `===`: a node with no end, a file with no node
//! and a title that two nodes use are refused with the file named. The Yarn
//! compiler itself runs when the game loads the project.

use std::sync::Arc;

use ambition_content_pack::{
    AggregateOutcome, Aggregation, CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource,
    LoweredFragment, RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};

/// The authored FILE kind: one Yarn file.
pub const YARN_DIALOGUE_SCHEMA: &str = "yarn_dialogue";

/// One Yarn file: its path as the manifest declares it, its text, and the
/// title of each of its nodes in file order, as admission read them. A
/// reader that needs the node ids reads `titles`, so the ids have one parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YarnFile {
    pub path: String,
    pub text: String,
    pub titles: Vec<String>,
}

/// Every Yarn file of a pack, in manifest order.
pub type YarnFiles = Vec<YarnFile>;

/// Each node of a Yarn file, in file order, as `(title, text)`, and each
/// problem with the file's nodes.
///
/// A node is a header (its `title:` line) and a body, and the body ends at a
/// line that is `===`. Yarn reads a `---` line as the end of the header. The
/// text is the node's lines from its title to its `===`: what the game plays.
pub fn yarn_nodes(text: &str) -> (Vec<(String, String)>, Vec<String>) {
    let mut nodes = Vec::new();
    let mut problems = Vec::new();
    let mut open: Option<(String, usize, Vec<&str>)> = None;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if let Some(title) = line.strip_prefix("title:") {
            if let Some((previous, at, _)) = open.take() {
                problems.push(format!("node `{previous}` (line {at}) has no closing `===` before the next title"));
            }
            let title = title.trim().to_string();
            if title.is_empty() {
                problems.push(format!("line {}: a `title:` with no name", index + 1));
            }
            open = Some((title, index + 1, vec![raw]));
        } else if line == "===" {
            match open.take() {
                Some((title, _, mut lines)) => {
                    lines.push(raw);
                    nodes.push((title, lines.join("\n")));
                }
                None => problems.push(format!("line {}: a `===` that closes no node", index + 1)),
            }
        } else if let Some((_, _, lines)) = open.as_mut() {
            lines.push(raw);
        }
    }
    if let Some((title, at, _)) = open {
        problems.push(format!("node `{title}` (line {at}) has no closing `===`"));
    }
    if nodes.is_empty() && problems.is_empty() {
        problems.push("the file has no node (no `title:` line)".to_string());
    }
    (nodes, problems)
}

struct YarnDialogue;

impl ContentSchemaHandler for YarnDialogue {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let (nodes, problems) = yarn_nodes(facet.text);
        for problem in problems {
            out.report(facet.diagnostic(DiagnosticCode::MalformedSource, problem));
        }
        // A node's title is the id a dialogue starts by, so two nodes with one
        // title, in one file or in two, are one identity defined twice. The
        // canonical form is the node's text: the pack's identity is built from
        // it, so an edit to a spoken line is a new pack.
        let named: Vec<&(String, String)> = nodes.iter().filter(|(title, _)| !title.is_empty()).collect();
        for (title, text) in &named {
            out.define(facet.content_id_in(YARN_DIALOGUE_SCHEMA, title.clone()), text.clone());
        }
        out.lower(YarnFile {
            path: facet.declared_path.to_string(),
            text: facet.text.to_string(),
            titles: named.iter().map(|(title, _)| title.clone()).collect(),
        });
    }

    fn aggregate(&self, fragments: &[LoweredFragment<'_>], out: &mut AggregateOutcome) -> Aggregation {
        let files: YarnFiles = fragments.iter().filter_map(|fragment| fragment.get::<YarnFile>().cloned()).collect();
        out.lower(files);
        Aggregation::Defined
    }
}

/// The schema, for the pack's schema list.
pub fn yarn_dialogue_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(YARN_DIALOGUE_SCHEMA),
        version: SchemaVersion(1),
        capability: CapabilityId::new(crate::AMBITION_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "One Yarn file of the game's dialogue: its nodes, each a title and a body closed by `===`.",
        handler: Arc::new(YarnDialogue),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_node_with_no_end_and_a_file_with_no_node_are_named() {
        let (nodes, problems) = yarn_nodes("// a comment\ntitle: a\n---\nhello\n===\ntitle: b\n---\nbye\n===\n");
        assert_eq!(
            nodes,
            [("a".to_string(), "title: a\n---\nhello\n===".to_string()), ("b".to_string(), "title: b\n---\nbye\n===".to_string())]
        );
        assert!(problems.is_empty(), "{problems:?}");
        let (_, problems) = yarn_nodes("title: a\n---\nhello\ntitle: b\n---\nbye\n===\n");
        assert_eq!(problems, ["node `a` (line 1) has no closing `===` before the next title"]);
        let (_, problems) = yarn_nodes("title: a\n---\nhello\n===\n===\n");
        assert_eq!(problems, ["line 5: a `===` that closes no node"]);
        let (_, problems) = yarn_nodes("title: a\n---\nhello\n");
        assert_eq!(problems, ["node `a` (line 1) has no closing `===`"]);
        let (_, problems) = yarn_nodes("just prose\n");
        assert_eq!(problems, ["the file has no node (no `title:` line)"]);
    }
}
