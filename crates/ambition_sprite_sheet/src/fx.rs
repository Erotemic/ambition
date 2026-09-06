//! The effect sheets the ENGINE ships, and the one mapping it owes them.
//!
//! Twelve FX spritesheets are published beside the character art, and the
//! packed SFX bank carries one `vfx.<family>.<row>` cue for every one of their
//! 189 rows. So an effect NAME already addresses a clip and its paired sound
//! together, in the data. What the engine owes is a single mapping — *which
//! sheet holds the row called `sonic_boom`* — and [`authored_effect`] is it.
//!
//! the engine could not ship the art it draws. `spawn_explosion` reached
//! for `GameAssets.characters.props["generic_explosions"]` — a map keyed by the
//! LDtk `Prop.kind` field — and the only things that ever populated it were
//! *game* systems (Ambition's intro table, Sanic's one ring sheet). An FX sheet
//! is neither a character nor an LDtk prop; it was squatting there, and in the
//! Smash / Sanic / Mary-O apps nothing registered it at all, so every effect in
//! every one of those apps degraded to the same particle burst. The sheets are
//! declared HERE, loaded by the engine's own `load_game_assets`, and stored in
//! their own [`GameAssets`](crate::game_assets::GameAssets) slot.
//!
//! the index is built from the BAKED records, not from loaded assets.
//! `build.rs` embeds every `*_spritesheet.ron` into the binary, so "which
//! effects exist" is answerable with no Bevy world, no asset server and no
//! decode — which is what lets a roster validator that runs at install time ask
//! the same question the renderer asks at draw time, and get the same answer.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use crate::character::sheets::{record_for_sheet_key, CharacterSheetSpec, SheetTuning};

/// One published FX spritesheet: its manifest target, and the SFX cue family
/// its rows' sounds were packed under.
///
/// the family is NOT always the target minus a suffix (`generic_explosions`
/// packs under `vfx.explosion.*`), which is exactly why it is declared beside
/// the target instead of derived by string surgery.
pub struct FxSheet {
    /// Sheet manifest target — `<target>_spritesheet.ron` / `.png`.
    pub target: &'static str,
    /// Cue-name family: this sheet's row `r` sounds like `vfx.<family>.<r>`.
    pub cue_family: &'static str,
    /// Whether the sheet is decoded at boot and kept for the whole process
    /// (the generic vocabulary any effect may name) or decoded when a
    /// CHARACTER whose moveset names one of its rows is realized — a fighter's own effect art cannot fire before the fighter
    /// exists, so it rides the character's demand and the same reveal barrier.
    /// Measured 2026-09-02: all thirteen sheets (9.4 MP) were resident in
    /// every room, and a scripted capture drew a handful at most.
    pub residency: FxResidency,
}

/// When an FX sheet is decoded. See [`FxSheet::residency`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxResidency {
    /// Decoded at boot, resident for the process.
    Core,
    /// Decoded when a realized character's moveset names a row of it.
    OwnedByCharacter,
}

/// The effect art the engine ships. Four generic sheets plus the per-
/// character effect sheets, in the order they are searched.
///
/// generic first: a name that appears on a generic sheet and on a character
/// sheet resolves to the generic one. No such collision exists today (all 196
/// row names are distinct across all thirteen sheets, pinned by
/// `every_authored_effect_row_is_reachable_by_name`), so the order is a
/// tie-break rule that has never had to fire, not a policy anyone depends on.
pub const FX_SHEETS: &[FxSheet] = &[
    FxSheet {
        target: "generic_action_fx",
        cue_family: "generic_action",
        residency: FxResidency::Core,
    },
    FxSheet {
        target: "generic_world_fx",
        cue_family: "generic_world",
        residency: FxResidency::Core,
    },
    FxSheet {
        target: "generic_exotic_fx",
        cue_family: "generic_exotic",
        residency: FxResidency::Core,
    },
    FxSheet {
        target: "generic_explosions",
        cue_family: "explosion",
        residency: FxResidency::Core,
    },
    FxSheet {
        target: "george_booul_vfx",
        cue_family: "george_booul",
        residency: FxResidency::OwnedByCharacter,
    },
    FxSheet {
        target: "oiler_vfx",
        cue_family: "oiler",
        residency: FxResidency::OwnedByCharacter,
    },
    FxSheet {
        target: "pirate_admiral_vfx",
        cue_family: "pirate_admiral",
        residency: FxResidency::OwnedByCharacter,
    },
    FxSheet {
        target: "ninja_shadow_oni_leader_vfx",
        cue_family: "ninja_shadow_oni_leader",
        residency: FxResidency::OwnedByCharacter,
    },
    FxSheet {
        target: "pca_vfx",
        cue_family: "pca",
        residency: FxResidency::OwnedByCharacter,
    },
    FxSheet {
        target: "patent_clerk_vfx",
        cue_family: "patent_clerk",
        residency: FxResidency::OwnedByCharacter,
    },
    FxSheet {
        target: "carl_stargan_vfx",
        cue_family: "carl_stargan",
        residency: FxResidency::OwnedByCharacter,
    },
    FxSheet {
        target: "noether_vfx",
        cue_family: "noether",
        residency: FxResidency::OwnedByCharacter,
    },
    FxSheet {
        target: "projectile_polygon_vfx",
        cue_family: "projectile_polygon",
        residency: FxResidency::OwnedByCharacter,
    },
];

/// The sheet a target names, if the engine ships one.
pub fn fx_sheet(target: &str) -> Option<&'static FxSheet> {
    FX_SHEETS.iter().find(|sheet| sheet.target == target)
}

/// The sheets decoded at boot: every [`FxResidency::Core`] target, in table order.
pub fn core_fx_targets() -> impl Iterator<Item = &'static str> {
    FX_SHEETS
        .iter()
        .filter(|sheet| sheet.residency == FxResidency::Core)
        .map(|sheet| sheet.target)
}

/// The character-owned sheets a set of effect names lives on — what a realized
/// character's moveset makes the engine owe. Effects on core sheets are
/// already resident and are not returned; unknown names are not an error
/// here (`MoveSpec::presentation_problems` reports them where they are
/// authored).
pub fn owned_fx_sheets_named_by<'a>(
    effects: impl IntoIterator<Item = &'a str>,
) -> BTreeSet<&'static str> {
    let index = authored_effects();
    effects
        .into_iter()
        .filter_map(|name| index.get(name))
        .filter(|effect| {
            fx_sheet(effect.sheet)
                .is_some_and(|sheet| sheet.residency == FxResidency::OwnedByCharacter)
        })
        .map(|effect| effect.sheet)
        .collect()
}

/// FX sheets render at their authored frame size; nothing collides with them.
const FX_TUNING: SheetTuning = SheetTuning::new(1.00, 2);

/// One authored effect: where its art is, and what it sounds like.
#[derive(Clone, Debug, PartialEq)]
pub struct AuthoredEffect {
    /// The authored row name, which IS the effect's name.
    pub name: &'static str,
    /// The sheet manifest target holding it.
    pub sheet: &'static str,
    /// Its index in that sheet's rows — resolved through
    /// [`SheetRecord::first_bound_row`](crate::SheetRecord::first_bound_row),
    /// the seam built so an authored clip can be drawn without an engine enum
    /// variant. never `unwrap_or(0)`: a name with no row answers `None`.
    pub slot: usize,
    /// The packed-bank cue name paired with this row, `vfx.<family>.<name>`.
    pub cue: String,
    pub frame_count: usize,
    /// Seconds per frame, as authored.
    pub duration_secs: f32,
}

impl AuthoredEffect {
    /// How long the clip runs, once, at its authored rate.
    pub fn clip_secs(&self) -> f32 {
        self.frame_count as f32 * self.duration_secs
    }
}

/// Every authored effect, by name. Built once from the baked sheet records.
pub fn authored_effects() -> &'static BTreeMap<&'static str, AuthoredEffect> {
    static INDEX: OnceLock<BTreeMap<&'static str, AuthoredEffect>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index = BTreeMap::new();
        for sheet in FX_SHEETS {
            // A sheet the build did not bake is simply absent — the same
            // degradation as any other missing manifest target, and the pin
            // test below is what says it should never happen for these thirteen.
            let Some(record) = record_for_sheet_key(sheet.target) else {
                continue;
            };
            for row in &record.rows {
                let name = row.animation.as_str();
                // The seam, used the way it was built to be used: ask the sheet
                // for the row, take the slot it proves, never index 0.
                let Some(bound) = record.first_bound_row([name]) else {
                    continue;
                };
                index.entry(name).or_insert_with(|| AuthoredEffect {
                    name,
                    sheet: sheet.target,
                    slot: bound.slot(),
                    cue: format!("vfx.{}.{}", sheet.cue_family, name),
                    frame_count: row.frame_count as usize,
                    duration_secs: row.duration_secs,
                });
            }
        }
        index
    })
}

/// The effect `name` addresses, if any sheet has a row by that name.
pub fn authored_effect(name: &str) -> Option<&'static AuthoredEffect> {
    authored_effects().get(name)
}

/// Is `name` an effect the shipped art can draw?
///
/// The oracle a content validator wants: pure, world-free, and answered by the
/// sheets themselves rather than by a Rust table transcribed from them.
pub fn is_authored_effect(name: &str) -> bool {
    authored_effects().contains_key(name)
}

/// The sheet spec for an FX target, addressed by ROW rather than by pose.
///
/// not [`try_load_spec_for_target`](crate::character::sheets::try_load_spec_for_target),
/// which refuses a sheet with no `idle` row — correctly, because the character
/// path indexes by [`CharacterAnim`](crate::character::CharacterAnim) and a
/// sheet it cannot ask for an idle pose is one it cannot draw. Twelve of the
/// thirteen FX sheets have no `idle`, and the odd one only appeared to: five
/// aliases inside `CharacterAnim::from_name` spelled `classic_burst` as *Idle*,
/// `burst_round` as *Walk*, `shockwave` as *Run*. Those are gone. An effect row
/// is addressed by its name, so this loads the spec without asking for a pose.
pub fn fx_sheet_spec(target: &str) -> Option<CharacterSheetSpec> {
    crate::character::sheets::try_load_row_addressed_spec(target, &FX_TUNING)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The residency split, asked through the seam a realization uses: a
    /// moveset naming an owned row owes that sheet; core rows and unknown
    /// names owe nothing.
    #[test]
    fn owned_sheets_are_owed_by_the_rows_that_name_them() {
        let owned: Vec<&FxSheet> = FX_SHEETS
            .iter()
            .filter(|sheet| sheet.residency == FxResidency::OwnedByCharacter)
            .collect();
        assert!(!owned.is_empty() && core_fx_targets().count() + owned.len() == FX_SHEETS.len());
        let sheet = owned[0];
        let row = record_for_sheet_key(sheet.target).unwrap().rows[0]
            .animation
            .clone();
        let core_row = record_for_sheet_key(core_fx_targets().next().unwrap())
            .unwrap()
            .rows[0]
            .animation
            .clone();
        assert_eq!(
            owned_fx_sheets_named_by([row.as_str(), core_row.as_str(), "kaboom"]),
            BTreeSet::from([sheet.target]),
            "one owned row → its sheet; the core row is resident already; the unknown name is \
             a moveset validation problem, not a demand"
        );
        assert!(owned_fx_sheets_named_by([core_row.as_str()]).is_empty());
    }

    /// Every row of every shipped FX sheet is reachable by its own name, and
    /// nothing else is.
    ///
    /// Set equality BOTH ways against the baked records: a row the index cannot
    /// reach is art that ships and cannot be drawn (the defect this module
    /// exists to end), and an index entry with no row is a name that resolves to
    /// nothing at draw time.
    #[test]
    fn every_authored_effect_row_is_reachable_by_name() {
        let mut from_sheets: Vec<(&str, &str)> = Vec::new();
        for sheet in FX_SHEETS {
            let record = record_for_sheet_key(sheet.target)
                .unwrap_or_else(|| panic!("`{}` is a shipped FX sheet", sheet.target));
            for row in &record.rows {
                from_sheets.push((row.animation.as_str(), sheet.target));
            }
        }
        assert_eq!(
            from_sheets.len(),
            // 189 + the Projectile Polygon's six charge rows. His neutral
            // special charges at the MUZZLE, which cannot be baked into a
            // character row: it follows his cannon as he aims and it lasts as
            // long as the button is down. + the trapdoor's, which arrived with
            // the renderer that draws one.
            196,
            "the shipped FX vocabulary changed size; if that is intended, say so here"
        );

        let index = authored_effects();
        for (name, sheet) in &from_sheets {
            let effect = index
                .get(name)
                .unwrap_or_else(|| panic!("`{name}` ships on `{sheet}` and must be addressable"));
            assert_eq!(effect.sheet, *sheet);
            assert_eq!(
                record_for_sheet_key(effect.sheet).unwrap().rows[effect.slot].animation,
                *name,
                "the slot must index the row it names"
            );
        }
        assert_eq!(
            index.len(),
            from_sheets.len(),
            "the index carries a name no sheet has"
        );
    }

    /// The name addresses the sound too. One example spelled out, because
    /// the pairing is the whole reason the vocabulary can be a single string.
    #[test]
    fn an_effect_name_addresses_its_paired_cue() {
        let boom = authored_effect("sonic_boom").expect("generic_exotic_fx ships it");
        assert_eq!(boom.sheet, "generic_exotic_fx");
        assert_eq!(boom.cue, "vfx.generic_exotic.sonic_boom");

        let classic = authored_effect("classic_burst").expect("generic_explosions ships it");
        assert_eq!(classic.sheet, "generic_explosions");
        assert_eq!(
            classic.cue, "vfx.explosion.classic_burst",
            "the explosion sheet packs under `explosion`, not `generic_explosions`"
        );

        assert!(!is_authored_effect("sonik_boom"), "a typo names nothing");
    }

    /// The FX specs load without an `idle` row — the property that made the
    /// other eleven sheets unloadable through the character path.
    #[test]
    fn an_fx_sheet_loads_without_a_pose_row() {
        let spec = fx_sheet_spec("generic_exotic_fx").expect("baked");
        assert!(
            spec.clip_slot(["sonic_boom"]).is_some(),
            "the row is addressable on the loaded spec"
        );
        assert!(
            !spec.maps(crate::character::CharacterAnim::Idle),
            "an effect sheet has no idle pose, and does not need one"
        );
    }
}
