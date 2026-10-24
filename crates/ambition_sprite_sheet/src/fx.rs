//! The effect sheets the engine ships, and the mapping from effect name to
//! sheet row.
//!
//! The packed SFX bank has one `vfx.<family>.<row>` cue for each FX sheet row,
//! so an effect name addresses a clip and its sound together. The engine owns
//! one mapping, from a row name (for example `sonic_boom`) to its sheet:
//! [`authored_effect`].
//!
//! An FX sheet is neither a character nor an LDtk prop. The sheets are declared
//! here, loaded by the engine's `load_game_assets`, and stored in their own
//! [`GameAssets`](crate::game_assets::GameAssets) slot, so every app gets them.
//!
//! The index is built from the baked records, not from loaded assets.
//! `build.rs` embeds every `*_spritesheet.ron`, so the set of effects is known
//! with no Bevy world, asset server, or decode. An install-time roster
//! validator and the renderer therefore get the same answer.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::character::sheets::{CharacterSheetSpec, SheetTuning};

/// One published FX spritesheet: its manifest target, and the SFX cue family
/// its rows' sounds were packed under.
///
/// The family is not always the target minus a suffix (`generic_explosions`
/// packs under `vfx.explosion.*`), so it is declared, not derived.
pub struct FxSheet {
    /// Sheet manifest target — `<target>_spritesheet.ron` / `.png`.
    pub target: &'static str,
    /// Cue-name family: this sheet's row `r` sounds like `vfx.<family>.<r>`.
    pub cue_family: &'static str,
    /// Whether the sheet is decoded at boot and kept (the generic vocabulary
    /// any effect may use), or decoded when a character whose moveset names
    /// one of its rows is realized. A fighter's own effects cannot fire before
    /// the fighter exists, so they use the character's demand and reveal
    /// barrier.
    pub residency: FxResidency,
    /// Who fades an effect of this sheet out. Declared, because the art
    /// decides it and the game cannot read that from a frame.
    pub fade: FxFade,
    /// The characters an [`FxResidency::OwnedByCharacter`] sheet is decoded
    /// with besides those whose moveset names its rows: a body whose effects
    /// a conductor shows, not its moves (the giant gnu's blows).
    pub owners: &'static [String],
}

/// Who fades an effect out. See [`FxSheet::fade`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum FxFade {
    /// The game fades it, from its first instant to its last: it is at half
    /// strength at the middle of its clip.
    OverItsLife,
    /// Its frames do: the art draws its own start and its own end (it ends
    /// on a clear frame), and the game draws each frame at full strength. A
    /// solid thing that comes and goes (a fist out of a portal) is this.
    InItsFrames,
}

/// When an FX sheet is decoded. See [`FxSheet::residency`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum FxResidency {
    /// Decoded at boot, resident for the process.
    Core,
    /// Decoded when a realized character's moveset names a row of it.
    OwnedByCharacter,
}

/// The FX facts a sheet record declares (`fx` in its record): the cue family
/// its rows' sounds were packed under, its residency and who fades it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FxSheetFacts {
    pub cue_family: String,
    pub residency: FxResidency,
    pub fade: FxFade,
    #[serde(default)]
    pub owners: Vec<String>,
}

/// The effect sheets the published art declares: each sheet record with
/// `fx`, the [`FxResidency::Core`] ones first, then by key.
///
/// Core sheets come first, so a name on both a core and a character sheet
/// resolves to the core one. Today all row names are distinct
/// (`every_authored_effect_row_is_reachable_by_name`), so this is only a
/// tie-break. Built once for each sheet generation from its records, as the
/// effect index is.
pub fn fx_sheets() -> &'static [FxSheet] {
    crate::baked_sheet_rons::current().fx_sheets()
}

/// The effect sheets of the records `records`.
pub(crate) fn fx_sheets_of(records: &'static std::collections::HashMap<String, crate::SheetRecord>) -> Vec<FxSheet> {
    let mut sheets: Vec<FxSheet> = records
        .iter()
        // A tier copy (`<target>.<marker>`) is the same sheet reduced, not
        // another effect sheet.
        .filter(|(key, _)| !crate::sprite_tree::TIERS.iter().any(|(_, marker)| key.strip_suffix(marker).is_some_and(|rest| rest.ends_with('.'))))
        .filter_map(|(key, record)| {
            let facts = record.fx.as_ref()?;
            Some(FxSheet {
                target: key.as_str(),
                cue_family: facts.cue_family.as_str(),
                residency: facts.residency,
                fade: facts.fade,
                owners: facts.owners.as_slice(),
            })
        })
        .collect();
    sheets.sort_by_key(|sheet| (sheet.residency != FxResidency::Core, sheet.target));
    sheets
}

/// The sheet a target names, if the engine ships one.
pub fn fx_sheet(target: &str) -> Option<&'static FxSheet> {
    fx_sheets().iter().find(|sheet| sheet.target == target)
}

/// The sheets decoded at boot: every [`FxResidency::Core`] target, in table order.
pub fn core_fx_targets() -> impl Iterator<Item = &'static str> {
    fx_sheets()
        .iter()
        .filter(|sheet| sheet.residency == FxResidency::Core)
        .map(|sheet| sheet.target)
}

/// The character-owned sheets that hold a set of effect names: what a realized
/// character's moveset requires. Effects on core sheets are already resident
/// and are not returned. Unknown names are not an error here;
/// `MoveSpec::presentation_problems` reports them.
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

/// The character-owned sheets whose record names `character_id` an owner:
/// decoded when that character is, whatever its moveset names.
pub fn owned_fx_sheets_of(character_id: &str) -> BTreeSet<&'static str> {
    fx_sheets()
        .iter()
        .filter(|sheet| sheet.residency == FxResidency::OwnedByCharacter && sheet.owners.iter().any(|o| o == character_id))
        .map(|sheet| sheet.target)
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
    /// Its index in that sheet's rows, from
    /// [`SheetRecord::first_bound_row`](crate::SheetRecord::first_bound_row).
    /// Never `unwrap_or(0)`: a name with no row gives `None`.
    pub slot: usize,
    /// The packed-bank cue name paired with this row, `vfx.<family>.<name>`.
    pub cue: String,
    pub frame_count: usize,
    /// Seconds per frame, as authored.
    pub duration_secs: f32,
    /// Who fades it out: its sheet's rule.
    pub fade: FxFade,
}

impl AuthoredEffect {
    /// How long the clip runs, once, at its authored rate.
    pub fn clip_secs(&self) -> f32 {
        self.frame_count as f32 * self.duration_secs
    }
}

/// Every authored effect, by name. Built once for each sheet generation
/// from its records.
pub fn authored_effects() -> &'static BTreeMap<&'static str, AuthoredEffect> {
    crate::baked_sheet_rons::current().authored_effects()
}

/// The effects of the effect sheets `sheets`, read from the records
/// `records` of the same generation.
pub(crate) fn authored_effects_of(
    sheets: &[FxSheet],
    records: &'static std::collections::HashMap<String, crate::SheetRecord>,
) -> BTreeMap<&'static str, AuthoredEffect> {
    {
        let mut index = BTreeMap::new();
        for sheet in sheets {
            // A sheet the build did not bake is absent, like any missing
            // manifest target. The pin test below says it must not happen.
            let Some(record) = records.get(sheet.target) else {
                continue;
            };
            for row in &record.rows {
                let name = row.animation.as_str();
                // Ask the sheet for the row by name; never fall back to index 0.
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
                    fade: sheet.fade,
                });
            }
        }
        index
    }
}

/// The effect `name` addresses, if any sheet has a row by that name.
pub fn authored_effect(name: &str) -> Option<&'static AuthoredEffect> {
    authored_effects().get(name)
}

/// Is `name` an effect the shipped art can draw?
/// A pure, world-free check for content validators, answered by the sheets
/// themselves.
pub fn is_authored_effect(name: &str) -> bool {
    authored_effects().contains_key(name)
}

/// The sheet spec for an FX target, addressed by ROW rather than by pose.
///
/// Do not use [`try_load_spec_for_target`](crate::character::sheets::try_load_spec_for_target)
/// here. It refuses a sheet with no `idle` row, which is correct for the
/// character path that indexes by
/// [`CharacterAnim`](crate::character::CharacterAnim). Most FX sheets have no
/// `idle`. An effect row is addressed by name, so this loads the spec without
/// a pose.
pub fn fx_sheet_spec(target: &str) -> Option<CharacterSheetSpec> {
    crate::character::sheets::try_load_row_addressed_spec(target, &FX_TUNING)
}

#[cfg(test)]
mod tests {
    /// The effect sheets are the published records that declare `fx`, and no
    /// other: a new effect sheet is a publish, with no Rust edit. Core sheets
    /// come first, so a name on a core and a character sheet resolves to the
    /// core one.
    #[test]
    fn the_effect_sheets_are_the_published_records_that_declare_fx() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../ambition_platformer2d_actor_monolith/assets/sprites");
        let mut declared: Vec<String> = std::fs::read_dir(root)
            .expect("the published sprite root is readable")
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let text = std::fs::read_to_string(&path).ok()?;
                let key = path.file_name()?.to_str()?.strip_suffix("_spritesheet.ron")?.to_owned();
                text.contains("\n    fx: Some(").then_some(key)
            })
            .collect();
        declared.sort();
        let mut indexed: Vec<String> = fx_sheets().iter().map(|sheet| sheet.target.to_owned()).collect();
        let core = core_fx_targets().count();
        assert!(core > 0, "control: the engine ships core effect sheets");
        assert!(
            fx_sheets()[..core].iter().all(|sheet| sheet.residency == FxResidency::Core),
            "the core sheets are not first"
        );
        indexed.sort();
        assert_eq!(indexed, declared, "the effect sheets are not the records that declare `fx`");
    }

    use super::*;
    use crate::character::sheets::record_for_sheet_key;

    /// The residency split, asked through the seam a realization uses: a
    /// moveset naming an owned row owes that sheet; core rows and unknown
    /// names owe nothing.
    #[test]
    fn owned_sheets_are_owed_by_the_rows_that_name_them() {
        let owned: Vec<&FxSheet> = fx_sheets()
            .iter()
            .filter(|sheet| sheet.residency == FxResidency::OwnedByCharacter)
            .collect();
        assert!(!owned.is_empty() && core_fx_targets().count() + owned.len() == fx_sheets().len());
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
    /// Set equality both ways against the baked records. A row the index cannot
    /// reach is art that cannot be drawn. An index entry with no row resolves
    /// to nothing at draw time.
    #[test]
    fn every_authored_effect_row_is_reachable_by_name() {
        let mut from_sheets: Vec<(&str, &str)> = Vec::new();
        for sheet in fx_sheets() {
            let record = record_for_sheet_key(sheet.target)
                .unwrap_or_else(|| panic!("`{}` is a shipped FX sheet", sheet.target));
            for row in &record.rows {
                from_sheets.push((row.animation.as_str(), sheet.target));
            }
        }
        assert_eq!(
            from_sheets.len(),
            // 189, plus the Projectile Polygon's six charge rows (his neutral
            // special charges at the muzzle, which follows his aim and lasts
            // while the button is held, so it cannot be a character row), plus
            // the trapdoor rows, plus GNU-ton's three rows of the giant's fist
            // out of a portal (his smash attacks on a platform-fighter stage),
            // plus the three stages of the PCA's charge, plus the four rows
            // of GNU-ton's blows in his fight (the glint, the streak, the
            // quake of the hammer, the impact), plus five more of his fight
            // (two blows that meet, his power, the launch, a push in space,
            // the golden apple of its end).
            211,
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

    /// The name addresses the sound too; that pairing lets the vocabulary be
    /// one string.
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

    /// The FX specs load without an `idle` row, which the character path
    /// requires.
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
