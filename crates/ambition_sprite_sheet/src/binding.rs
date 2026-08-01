//! The [`AnimRow`] namespace: resolving an animation name against the sheet that
//! actually has the rows.
//!
//! This is the namespace the binding boundary was built for. A sprite sheet's
//! rows are named by the generator from the rig (`idle`, `walk`, `death`), and
//! every consumer — character animator, boss animator, prop and melee-effect
//! visuals, projectiles — addresses them by string. The old resolver was
//! `SheetRecord::row_index_of(&str) -> Option<usize>`, and its callers spelled
//! the failure `?`, `unwrap_or(0)`, or `unwrap_or(1)`.
//!
//! That is how Mary-O shipped without a death animation. The rig emitted a row
//! called `death`; the policy asked for `dead`; `row_index_of` returned `None`;
//! the caller drew frame 0 of row 0 and said nothing. The bug survived a
//! playtest because an idle frame where a death should be is not obviously
//! WRONG on screen — it is just not right, which is much quieter.
//!
//! So there is no name→row lookup on `SheetRecord` any more. A consumer asks the
//! sheet for its [`Resolver`], resolves through a
//! [`BindingLedger`](ambition_platformer2d_shared_tangle::binding::BindingLedger),
//! and then indexes the rows with the resulting [`Bound`] — which it can only
//! hold because the row exists.

use ambition_platformer2d_shared_tangle::binding::{Bound, Namespace, Ref, Resolver};

use crate::{SheetRecord, SheetRow};

/// The animation rows of one sprite sheet.
///
/// Scoped to a sheet, not global: two sheets legitimately have different rows,
/// and "row `death` exists" is only ever a question about a particular sheet.
/// Build the resolver from the sheet you are about to draw.
pub struct AnimRow;

impl Namespace for AnimRow {
    const NAME: &'static str = "anim row";
}

/// An authored animation-row reference, before it has met a sheet.
pub type AnimRowRef = Ref<AnimRow>;

/// A row reference that has met its sheet and survived.
pub type BoundAnimRow = Bound<AnimRow>;

impl SheetRecord {
    /// The rows this sheet actually has, as the only thing that can resolve a
    /// row name against it.
    ///
    /// `Bound::slot()` is the row's index in [`SheetRecord::rows`], so a resolved
    /// reference indexes the authored data directly — see [`Self::row`].
    pub fn anim_rows(&self) -> Resolver<AnimRow> {
        Resolver::new(self.rows.iter().map(|row| row.animation.as_str()))
    }

    /// The row a resolved reference names.
    ///
    /// `Bound<AnimRow>` proves the row existed in SOME sheet, not in THIS one:
    /// the namespace marker names the family, never the authority that assigned
    /// the slot, so `sheet_b.row(&sheet_a_bound)` type-checks. Two sheets with
    /// different layouts would then silently return an unrelated row at the same
    /// index, or panic out of bounds.
    ///
    /// So the agreement is checked here, for real — an `assert!`, not a
    /// `debug_assert!`, because a release build is exactly where a wrong row is
    /// least visible. It costs one short string compare against a slot we are
    /// about to index anyway. Using a bound from another sheet is a programmer
    /// error, not a content typo, so it is loud rather than degradable.
    pub fn row(&self, bound: &BoundAnimRow) -> &SheetRow {
        let found = self.rows.get(bound.slot());
        assert_eq!(
            found.map(|row| row.animation.as_str()),
            Some(bound.id()),
            "sheet `{}` was indexed with a Bound<AnimRow> resolved against a different sheet \
             (slot {} holds {:?}, the binding names `{}`)",
            self.target,
            bound.slot(),
            found.map(|row| row.animation.as_str()),
            bound.id(),
        );
        &self.rows[bound.slot()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::binding::BindingLedger;

    fn sheet_with_rows(names: &[&str]) -> SheetRecord {
        let rows = names
            .iter()
            .enumerate()
            .map(|(index, name)| SheetRow {
                animation: (*name).to_owned(),
                row_index: index as u32,
                frame_count: 4,
                duration_ms: 400,
                duration_secs: 0.4,
                page: 0,
                rects: Vec::new(),
            })
            .collect();
        SheetRecord {
            target: "mary_o".to_owned(),
            image: "mary_o_spritesheet.png".to_owned(),
            images: Vec::new(),
            label_width: 0,
            frame_width: 32,
            frame_height: 32,
            y_offset: 0,
            body_metrics: None,
            tuning: None,
            rows,
        }
    }

    /// The Mary-O bug, as a fact about the engine rather than a playtest note:
    /// the sheet spells the row `death`, the policy asks for `dead`, and what
    /// comes back is a report naming the sheet's real rows — not row 0, not
    /// `None`, and not a silently substituted idle.
    #[test]
    fn a_misnamed_anim_row_is_reported_not_degraded() {
        let sheet = sheet_with_rows(&["idle", "walk", "run", "jump", "death"]);
        let rows = sheet.anim_rows();
        let mut ledger = BindingLedger::new();

        let bound = ledger.resolve(&rows, &AnimRowRef::new("dead"), "mary_o/death_policy");
        assert!(
            bound.is_none(),
            "a row the sheet does not have must not bind"
        );

        let report = ledger.finish();
        let unresolved = &report.unresolved()[0];
        assert_eq!(unresolved.namespace, "anim row");
        assert_eq!(unresolved.declared_by, "mary_o/death_policy");
        assert_eq!(unresolved.did_you_mean.as_deref(), Some("death"));
        assert!(
            unresolved.available.contains(&"death".to_owned()),
            "the report shows the row that DOES exist: {unresolved}"
        );
    }

    /// A `Bound` from another sheet is caught rather than silently returning
    /// whatever sits at the same index. The namespace marker cannot distinguish
    /// two sheets, so this is the check that makes `row` honest in release.
    #[test]
    #[should_panic(expected = "resolved against a different sheet")]
    fn a_binding_from_another_sheet_is_refused() {
        let a = sheet_with_rows(&["idle", "walk", "death"]);
        let b = sheet_with_rows(&["idle", "hurt", "run"]);
        let bound = a
            .anim_rows()
            .resolve(&AnimRowRef::new("death"), "sheet a")
            .expect("sheet a has it");
        // Same slot exists in b, holding an unrelated row — the case that used
        // to return `run` and draw the wrong animation.
        let _ = b.row(&bound);
    }

    /// A resolved row indexes the authored rows directly, in sheet order — the
    /// property that lets the name lookup disappear from every consumer.
    #[test]
    fn a_resolved_row_indexes_the_sheet_in_authored_order() {
        let sheet = sheet_with_rows(&["idle", "walk", "death"]);
        let rows = sheet.anim_rows();

        let bound = rows
            .resolve(&AnimRowRef::new("death"), "mary_o")
            .expect("the sheet has it");
        assert_eq!(bound.slot(), 2);
        assert_eq!(sheet.row(&bound).animation, "death");
        assert_eq!(sheet.row(&bound).row_index, 2);
    }
}
