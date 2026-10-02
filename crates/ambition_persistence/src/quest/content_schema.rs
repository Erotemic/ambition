//! The `quest_book` authored-content schema, owned by the quest capability.
//!
//! A pack authors its quests as a RON list of [`QuestSpec`]s. The content
//! compiler is the one reader: it checks the book and lowers it, and the
//! runtime registers what it lowered.
//!
//! The checks are the ones a serde parse cannot do. A quest with no steps
//! parses and is never shown as anything a player can do. Two quests with one
//! id parse, and the registry keeps only one of them. A step condition with an
//! empty id parses, and no event can ever match it.

use std::collections::BTreeSet;
use std::sync::Arc;

use ambition_content_pack::{
    CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource,
    RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};

use super::{QuestSpec, QuestStepCondition};

/// The capability that owns this schema.
pub const QUEST_CAPABILITY: &str = "quest";

/// The authored FILE kind: a list of quests.
pub const QUEST_BOOK_SCHEMA: &str = "quest_book";

/// The schema version this handler reads.
pub const QUEST_BOOK_VERSION: SchemaVersion = SchemaVersion(1);

/// What a prepared pack lowers a checked quest book to, in authored order.
pub type AuthoredQuestBook = Vec<QuestSpec>;

struct QuestBookSchema;

impl ContentSchemaHandler for QuestBookSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let book: AuthoredQuestBook = match ron::from_str(facet.text) {
            Ok(book) => book,
            Err(error) => {
                let code = match error.code {
                    ron::error::Error::NoSuchStructField { .. } => DiagnosticCode::UnknownField,
                    _ => DiagnosticCode::MalformedSource,
                };
                out.report(facet.diagnostic(code, format!("{error}")));
                return;
            }
        };
        declare(facet, &book, out);
        // Lower only when clean: a refused pack gives the runtime nothing.
        if !out.failed() {
            out.lower(book);
        }
    }
}

fn declare(facet: &FacetSource<'_>, book: &AuthoredQuestBook, out: &mut FacetOutcome) {
    let mut seen = BTreeSet::new();
    for quest in book {
        let id = quest.id.as_str();
        if id.trim().is_empty() || id.trim() != id {
            out.report(facet.diagnostic(
                DiagnosticCode::MalformedSource,
                format!(
                    "the quest id {id:?} is empty or has surrounding whitespace; the registry \
                     and the save key a quest by its id verbatim"
                ),
            ));
            continue;
        }
        if !seen.insert(id) {
            out.report(facet.diagnostic(
                DiagnosticCode::DuplicateIdentity,
                format!("two quests have the id {id:?}; the registry keeps only one"),
            ));
            continue;
        }
        // `Debug` is canonical here: the spec is plain fields and ordered
        // `Vec`s, with no map whose order could change.
        out.define(facet.content_id(id), format!("{quest:?}"));
        if quest.steps.is_empty() {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::MalformedSource,
                        format!("quest {id:?} has no steps"),
                    )
                    .fix("a quest advances one step at a time; author at least one step"),
            );
        }
        for (index, step) in quest.steps.iter().enumerate() {
            if condition_id(&step.condition).trim().is_empty() {
                out.report(facet.diagnostic(
                    DiagnosticCode::MalformedSource,
                    format!(
                        "quest {id:?} step {index} has a condition with an empty id; no \
                         event can match it, so the quest stops there"
                    ),
                ));
            }
        }
    }
}

fn condition_id(condition: &QuestStepCondition) -> &str {
    match condition {
        QuestStepCondition::NpcTalked(id)
        | QuestStepCondition::ItemCollected(id)
        | QuestStepCondition::BossDefeated(id)
        | QuestStepCondition::EncounterCleared(id)
        | QuestStepCondition::FlagSet(id)
        | QuestStepCondition::RoomEntered(id) => id,
    }
}

/// The quest book a prepared pack lowered, if it carries one.
pub fn lowered_quest_book(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&AuthoredQuestBook> {
    pack.lowered::<AuthoredQuestBook>(&SchemaId::new(QUEST_BOOK_SCHEMA))
}

/// The quest capability's registration, for a composition to install.
pub fn quest_book_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(QUEST_BOOK_SCHEMA),
        version: QUEST_BOOK_VERSION,
        capability: CapabilityId::new(QUEST_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "The quests a pack ships: id, title, summary, ordered steps (each a condition \
              on one advance event) and whether the quest starts when the registry starts.",
        handler: Arc::new(QuestBookSchema),
    }
}

#[cfg(test)]
mod tests;
