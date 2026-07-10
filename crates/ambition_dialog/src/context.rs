//! **Who is talking to whom** — the identity context of one conversation.
//!
//! A dialogue is not a property of the NPC alone. The same pedestal in the Hall
//! of Characters should say something different to a visitor than to the
//! character standing on it, and a possessed body should not strike up a
//! conversation with itself. [`DialogueContext`] is the engine's answer: at
//! interact-dispatch it records the SPEAKER (the body the player is driving),
//! the LISTENER (the body being talked to), and whether they are the same.
//!
//! Content branches on the three Yarn variables the bridge publishes from it:
//!
//! ```yarn
//! title: hall_pirate_admiral
//! ---
//! <<if $speaker_is_self>>
//!     Admiral: ...
//! <<endif>>
//! ===
//! ```
//!
//! **Ids, never display names.** A display name is a localization artifact and
//! two characters may share one; the id is the identity.

use std::collections::BTreeSet;

use bevy::prelude::Resource;

/// The suffix identifying a dialogue's SELF branch — the node content authors
/// for "the speaker is the listener". See [`DialogueNodeIndex::entry_node`].
///
/// `__` is the project's existing sub-node convention (the content validator
/// already splits known dialogue ids on it).
pub const SELF_NODE_SUFFIX: &str = "__self";

/// The identity context of one conversation. Built at interact-dispatch, carried
/// on the pending-start request, and published to Yarn by the bridge.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DialogueContext {
    /// The driven body's id: a possessed actor's id, or — for the home avatar —
    /// the character id it is wearing. Empty for scripted dialogue.
    pub speaker_id: String,
    /// The id of the body being talked to. Empty for scripted dialogue.
    pub listener_id: String,
    /// The speaker IS the listener: the player possessed this body, or wears the
    /// character this body is. Both cases reduce to one id comparison.
    pub speaker_is_self: bool,
}

impl DialogueContext {
    /// The context of an in-world interaction between two identified bodies.
    pub fn between(speaker_id: impl Into<String>, listener_id: impl Into<String>) -> Self {
        let speaker_id = speaker_id.into();
        let listener_id = listener_id.into();
        // A pair of empty ids is "nobody talking to nobody", not self-talk.
        let speaker_is_self = !speaker_id.is_empty() && speaker_id == listener_id;
        Self {
            speaker_id,
            listener_id,
            speaker_is_self,
        }
    }

    /// A conversation with no in-world speaker: a cutscene, an intro beat, a
    /// system message. Never self.
    pub fn scripted() -> Self {
        Self::default()
    }
}

/// The set of Yarn nodes the compiled project declares.
///
/// Published by the Yarn bridge when the runner spawns. It exists so the
/// SIMULATION can ask "did content author a self branch for this dialogue?"
/// without depending on `bevy_yarnspinner` — the interaction is suppressed
/// before it happens, rather than opening a dialogue box and closing it.
///
/// `BTreeSet`, not `HashSet`: this answers a question the sim branches on, and
/// sim order may never depend on a hash (ADR 0023).
#[derive(Resource, Clone, Debug, Default)]
pub struct DialogueNodeIndex {
    nodes: BTreeSet<String>,
    /// False until a compiled Yarn project has been seen. An unpopulated index
    /// knows nothing, and an index that knows nothing must not suppress
    /// anything — headless sims and the frames before the runner spawns would
    /// silently eat every self-interaction.
    populated: bool,
}

impl DialogueNodeIndex {
    /// Record the compiled project's node names. Idempotent.
    pub fn populate(&mut self, names: impl IntoIterator<Item = String>) {
        self.nodes = names.into_iter().collect();
        self.populated = true;
    }

    pub fn is_populated(&self) -> bool {
        self.populated
    }

    pub fn contains(&self, node: &str) -> bool {
        self.nodes.contains(node)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The node this conversation should actually enter, or `None` when the
    /// interaction is SUPPRESSED.
    ///
    /// - Not talking to yourself: the dialogue's own node.
    /// - Talking to yourself, and content authored `<id>__self`: that node.
    /// - Talking to yourself, and it did not: **suppressed**. The engine's
    ///   default is that a body has nothing to say to itself. Content opts in.
    /// - Index unpopulated (no Yarn project — headless, tests, the frames
    ///   before the runner spawns): the dialogue's own node. Not knowing is not
    ///   grounds for silently dropping a player's interaction.
    pub fn entry_node(&self, dialogue_id: &str, speaker_is_self: bool) -> Option<String> {
        if !speaker_is_self || !self.populated {
            return Some(dialogue_id.to_string());
        }
        let self_node = format!("{dialogue_id}{SELF_NODE_SUFFIX}");
        self.contains(&self_node).then_some(self_node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_is_an_id_comparison_not_a_name_one() {
        assert!(DialogueContext::between("npc_emmy", "npc_emmy").speaker_is_self);
        assert!(!DialogueContext::between("npc_emmy", "npc_admiral").speaker_is_self);
    }

    /// Scripted dialogue has no speaker. Two absent ids are not the same body.
    #[test]
    fn nobody_talking_to_nobody_is_not_self() {
        assert!(!DialogueContext::between("", "").speaker_is_self);
        assert!(!DialogueContext::scripted().speaker_is_self);
    }

    #[test]
    fn a_normal_conversation_enters_its_own_node() {
        let mut index = DialogueNodeIndex::default();
        index.populate(["hall_admiral".to_string()]);
        assert_eq!(
            index.entry_node("hall_admiral", false).as_deref(),
            Some("hall_admiral")
        );
    }

    #[test]
    fn talking_to_yourself_enters_the_self_branch_when_content_authored_one() {
        let mut index = DialogueNodeIndex::default();
        index.populate(["hall_admiral".to_string(), "hall_admiral__self".to_string()]);
        assert_eq!(
            index.entry_node("hall_admiral", true).as_deref(),
            Some("hall_admiral__self")
        );
    }

    /// The engine default: a body has nothing to say to itself.
    #[test]
    fn talking_to_yourself_is_suppressed_without_a_self_branch() {
        let mut index = DialogueNodeIndex::default();
        index.populate(["hall_admiral".to_string()]);
        assert_eq!(index.entry_node("hall_admiral", true), None);
    }

    /// An index that has never seen a Yarn project must not suppress: headless
    /// sims and the pre-spawn frames would swallow the interaction.
    #[test]
    fn an_unpopulated_index_never_suppresses() {
        let index = DialogueNodeIndex::default();
        assert!(!index.is_populated());
        assert_eq!(
            index.entry_node("hall_admiral", true).as_deref(),
            Some("hall_admiral")
        );
    }
}
