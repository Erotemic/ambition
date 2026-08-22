//! The items capability's authored-content SCHEMA registration.
//!
//! `ambition_items` owns the item catalog, so it — not a central content enum,
//! not the compiler — says how an `items.ron` is read, what identities it mints,
//! and what is wrong with it.
//!
//! ## The file is POSITIONAL, and that is the interesting check
//!
//! `items.ron` is a `Vec<ItemMeta>` in grid order: slot index = `row * 6 + col`,
//! index 0 is `PortalGun`, index 23 is `ReservedSlot`. The index IS the binding
//! to the [`crate::Item`] discriminant — there is no key in the file tying a row
//! to the item it re-authors.
//!
//! so deleting one row does not remove one item, it renames twenty-three.
//! Every row after the gap shifts up a slot and silently re-authors the wrong
//! item: the axe's description on the javelin, and so on down the grid, with no
//! parse error and no missing reference. `from_ron` accepted it happily and the
//! fallback path made it worse — a short file leaves the tail resolving to
//! built-in defaults, so the grid looks populated. [`ROW_COUNT_IS_THE_GRID`]
//! is that refusal.
//!
//! This is the class of bug the compiler is for: not a typo that fails to
//! parse, but a well-formed file that means something other than what was
//! intended.

use std::sync::Arc;

use ambition_content_pack::{
    CapabilityId, ContentId, ContentKind, ContentSchemaHandler, DiagnosticCode, FacetOutcome,
    FacetSource, RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};

use crate::{ItemCatalog, ItemMeta, ITEM_COUNT};

/// The capability that owns every schema in this module.
pub const ITEMS_CAPABILITY: &str = "items";

/// The authored FILE kind.
pub const ITEM_CATALOG_SCHEMA: &str = "item_catalog";
/// One authored item row.
pub const ITEM_SCHEMA: &str = "item";

/// The schema version this handler reads.
pub const ITEM_CATALOG_VERSION: SchemaVersion = SchemaVersion(1);

/// Named so the refusal message can point at the reason rather than the number.
const ROW_COUNT_IS_THE_GRID: &str =
    "the item grid is fixed at ITEM_COUNT slots and the file is positional, so the row count is \
     part of the schema, not a length that happens to match";

/// Typed reference marker, for shipped Rust consumers pointing at an item.
pub struct ItemRef;
impl ContentKind for ItemRef {
    const SCHEMA: &'static str = ITEM_SCHEMA;
    const NOUN: &'static str = "item";
}

struct ItemCatalogSchema;

impl ContentSchemaHandler for ItemCatalogSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let rows: Vec<ItemMeta> = match ron::from_str(facet.text) {
            Ok(rows) => rows,
            Err(error) => {
                // Match the ron VARIANT, not the message text: the message is a
                // rendering detail and pinning it makes the diagnostic depend on
                // ron's release notes.
                let code = match error.code {
                    ron::error::Error::NoSuchStructField { .. } => DiagnosticCode::UnknownField,
                    _ => DiagnosticCode::MalformedSource,
                };
                out.report(facet.diagnostic(code, format!("{error}")));
                return;
            }
        };

        declare(facet, &rows, out);

        // LOWER only when clean — a caller must never receive a runtime value
        // out of a pack that was refused.
        if !out.failed() {
            out.lower(ItemCatalog::from_rows(rows));
        }
    }
}

fn declare(facet: &FacetSource<'_>, rows: &[ItemMeta], out: &mut FacetOutcome) {
    if rows.len() != ITEM_COUNT {
        out.report(
            facet
                .diagnostic(
                    DiagnosticCode::MalformedSource,
                    format!(
                        "the item catalog has {} rows; the grid has {ITEM_COUNT} slots",
                        rows.len()
                    ),
                )
                .fix(format!(
                    "{ROW_COUNT_IS_THE_GRID}. A row is a SLOT: to blank one, keep the row and \
                     empty its fields — deleting it shifts every later item up a slot and \
                     re-authors the wrong ones"
                )),
        );
        // Identities below are keyed by dialog_id, not by index, so they stay
        // meaningful; but nothing downstream should trust an unaligned grid.
    }

    let mut owners: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();

    for (index, row) in rows.iter().enumerate() {
        let dialog_id = row.dialog_id.trim();

        // Refuse the un-normalized spelling rather than silently rewriting it, so the id an
        // author reads in the file is the id scripts use.
        //
        // Compare normalization against the raw authored value; pre-trimming would
        // hide the whitespace this validation is meant to reject.
        let raw = row.dialog_id.as_str();
        let normalized: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect();
        if !raw.trim().is_empty() && raw != normalized {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::MalformedProviderBinding,
                        format!(
                            "item at grid slot {index} has `dialog_id: \"{raw}\"`, which no \
                             script can reach — lookups normalize to `{normalized}`"
                        ),
                    )
                    .at_field("dialog_id")
                    .fix(format!(
                        "spell it `{normalized}`: lowercase, alphanumerics only"
                    )),
            );
        }

        // The authoring id is the identity. An empty one cannot be referred to
        // by `condition("inventory.holds", ...)` at all, so the row is unreachable content.
        if dialog_id.is_empty() {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::MalformedProviderBinding,
                        format!("item at grid slot {index} has an empty `dialog_id`"),
                    )
                    .at_field("dialog_id")
                    .fix(
                        "give it the stable lowercase authoring id scripts use — \
                         `condition(\"inventory.holds\", \"portalgun\")` names a dialog_id, and an empty one is a row \
                         nothing can name",
                    ),
            );
            continue;
        }

        let id = ContentId::new(facet.namespace, &SchemaId::new(ITEM_SCHEMA), dialog_id);
        // THE SLOT IS PART OF THE ROW'S IDENTITY, so it must be in the
        // canonical form. The pack fingerprint sorts definitions by content
        // id, so a per-row canonical keyed only by `dialog_id` made SWAPPING two
        // complete rows a no-op for the fingerprint — while swapping exactly
        // which metadata belongs to which `Item` enum variant. This file is
        // positional; the whole reason the row COUNT is checked is that the
        // index is the binding. The same hole the music track ORDER had.
        out.define(id.clone(), format!("slot={index}\n{}", canonical(row)));

        // Two rows answering one `inventory.holds` is an authority conflict, not a
        // duplicate: every script asking the question gets whichever the lookup
        // reaches first.
        if let Some(first) = owners.insert(dialog_id, index) {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::ConflictingModuleContribution,
                        format!("grid slots {first} and {index} share `dialog_id` `{dialog_id}`"),
                    )
                    .about(id.clone())
                    .at_field("dialog_id")
                    .fix(
                        "give one of them its own id — a dialog_id is what a script asks for, \
                         so two owners is a conflict rather than a duplicate",
                    ),
            );
        }

        if row.display_name.trim().is_empty() {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::MalformedProviderBinding,
                        format!("item `{dialog_id}` has an empty `display_name`"),
                    )
                    .about(id.clone())
                    .at_field("display_name"),
            );
        }

        // `None` means "not equippable"; `Some("")` means somebody started
        // wiring one and stopped. The first is a decision, the second is a
        // half-edit that equips nothing and reports nothing.
        if row
            .held_item_id
            .as_deref()
            .is_some_and(|h| h.trim().is_empty())
        {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::MalformedProviderBinding,
                        format!("item `{dialog_id}` has an empty `held_item_id`"),
                    )
                    .about(id.clone())
                    .at_field("held_item_id")
                    .fix("name the HeldItem it grants, or use `None` to say it grants nothing"),
            );
        }
    }
}

/// The canonical form a row contributes to the pack fingerprint. Semantic, not
/// the authored bytes: reflowing a comment must not move the fingerprint.
fn canonical<T: serde::Serialize>(value: &T) -> String {
    ron::ser::to_string(value).unwrap_or_else(|error| format!("<uncanonicalizable: {error}>"))
}

/// The catalog a prepared pack lowered to, if it carries one — the runtime's
/// load path.
pub fn lowered_item_catalog(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&ItemCatalog> {
    pack.lowered::<ItemCatalog>(&SchemaId::new(ITEM_CATALOG_SCHEMA))
}

#[cfg(test)]
mod tests;

/// The items capability's registration, for a composition to install.
pub fn item_catalog_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(ITEM_CATALOG_SCHEMA),
        version: ITEM_CATALOG_VERSION,
        capability: CapabilityId::new(ITEMS_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "The item grid's authored flavor and wiring, one row per slot in grid order. \
              Defines `item` identities keyed by `dialog_id`.",
        handler: Arc::new(ItemCatalogSchema),
    }
}
