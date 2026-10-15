//! The `mary_o_forms` schema: her power ladder, as data in the demo's pack
//! (`assets/data/mary_o_forms.ron`).
//!
//! A rung is a form she can be in: the catalog row she is drawn as, the clip
//! that shows her arrive at it from below, and the equipment row she wears
//! on it (none on the bottom rung). The ladder IS the downgrade chain: a
//! rung's row is armor that a hit spends into the row of the rung below, and
//! every form row has the one exclusive slot. So the file states the rows
//! without their `on_hit` and their slot, and [`MaryOForms::row`] builds the
//! [`EquipmentRow`] the engine equips. The cues say which sound each change
//! of form plays.

use ambition_platformer2d::characters::brain::action_set::RangedActionSpec;
use ambition_platformer2d::characters::equipment::{EquipmentGrant, EquipmentRow, OnHit, ParamModifier};
use ambition_platformer2d::content::{lowered_rules, rules_schema, SchemaRegistration};
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;
use serde::Deserialize;

/// The authored FILE kind.
pub const MARY_O_FORMS_SCHEMA: &str = "mary_o_forms";

/// Her power ladder, bottom rung first. A rung's index is its rank.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaryOForms {
    /// The exclusive equipment slot every form row occupies, so a new form
    /// replaces the old one rather than stacking on it.
    pub slot: String,
    pub ladder: Vec<FormRung>,
    /// The sound of each change of form, by the characters it goes between.
    pub cues: Vec<FormCue>,
}

/// One form she can be in.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormRung {
    /// The catalog row she is drawn as on this rung.
    pub character: String,
    /// The clip that shows her arrive at this rung from below (it is
    /// authored on this rung's sheet). The bottom rung is never arrived at
    /// from below.
    #[serde(default)]
    pub arrival: Option<FormArrival>,
    /// The equipment row she wears on this rung; none on the bottom rung.
    #[serde(default)]
    pub row: Option<FormRow>,
}

/// How she arrives at a rung from below.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum FormArrival {
    /// The silhouette changes size.
    Grow,
    /// The same size, another look.
    Transform,
}

impl FormArrival {
    pub fn anim(self) -> CharacterAnim {
        match self {
            Self::Grow => CharacterAnim::Grow,
            Self::Transform => CharacterAnim::Transform,
        }
    }
}

/// A form's equipment row, without the armor and the slot the ladder gives
/// it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormRow {
    pub id: String,
    #[serde(default)]
    pub modifiers: Vec<ParamModifier>,
    #[serde(default)]
    pub grants: Vec<EquipmentGrant>,
}

/// The sound one change of form plays.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormCue {
    pub from: String,
    pub to: String,
    pub cue: String,
}

impl MaryOForms {
    /// The top rung's rank.
    pub fn top(&self) -> u8 {
        (self.ladder.len() - 1) as u8
    }

    /// The equipment row of the rung that wears `id`: the rung's authored
    /// row, armor whose loss leaves the row of the rung below, in the form
    /// slot. `None` for an id that is not a form row.
    pub fn row(&self, id: &str) -> Option<EquipmentRow> {
        let rank = self.ladder.iter().position(|rung| rung.row.as_ref().is_some_and(|row| row.id == id))?;
        Some(self.row_at(rank))
    }

    fn row_at(&self, rank: usize) -> EquipmentRow {
        let row = self.ladder[rank].row.as_ref().expect("admission: every rung over the bottom has a row");
        let below = (rank > 0 && self.ladder[rank - 1].row.is_some()).then(|| Box::new(self.row_at(rank - 1)));
        EquipmentRow {
            id: row.id.clone(),
            modifiers: row.modifiers.clone(),
            grants: row.grants.clone(),
            on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: below }),
            exclusive_slot: Some(self.slot.clone()),
        }
    }

    /// The sound of the change of form from `from` to `to`, if it has one.
    pub fn cue(&self, from: &str, to: &str) -> Option<&str> {
        self.cues.iter().find(|cue| cue.from == from && cue.to == to).map(|cue| cue.cue.as_str())
    }

    /// The sound of arriving at the top rung from the rung below it.
    pub fn top_cue_up(&self) -> Option<&str> {
        let top = self.top() as usize;
        self.cue(&self.ladder[top - 1].character, &self.ladder[top].character)
    }

    /// The sound of leaving the top rung for the rung below it.
    pub fn top_cue_down(&self) -> Option<&str> {
        let top = self.top() as usize;
        self.cue(&self.ladder[top].character, &self.ladder[top - 1].character)
    }

    /// Each value the demo cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.slot.trim().is_empty() {
            problems.push("`slot` is empty; every form row occupies it".to_string());
        }
        if self.ladder.len() < 2 {
            problems.push(format!("`ladder` has {} rung(s); a ladder has a bottom rung and a form over it", self.ladder.len()));
        }
        for (rank, rung) in self.ladder.iter().enumerate() {
            match (rank, &rung.row, rung.arrival) {
                (0, Some(row), _) => problems.push(format!(
                    "`ladder[0].row` is `{}`; the bottom rung wears no form row (a hit there is a real hit)",
                    row.id
                )),
                (0, None, Some(_)) => {
                    problems.push("`ladder[0].arrival` is set; nothing is under the bottom rung".to_string())
                }
                (0, None, None) => {}
                (_, None, _) => problems.push(format!("`ladder[{rank}].row` is absent; each rung over the bottom wears a row")),
                (_, Some(_), None) => problems.push(format!("`ladder[{rank}].arrival` is absent; say how she arrives at it")),
                (_, Some(row), Some(_)) if row.id.trim().is_empty() => {
                    problems.push(format!("`ladder[{rank}].row.id` is empty"))
                }
                _ => {}
            }
            if rung.character.trim().is_empty() {
                problems.push(format!("`ladder[{rank}].character` is empty"));
            }
            if self.ladder[..rank].iter().any(|other| other.character == rung.character) {
                problems.push(format!("`ladder[{rank}].character` `{}` is on two rungs", rung.character));
            }
            if let Some(row) = &rung.row {
                for (index, grant) in row.grants.iter().enumerate() {
                    if let EquipmentGrant::Ranged(shot) = grant {
                        problems.extend(shot_problems(shot, &format!("ladder[{rank}].row.grants[{index}]")));
                    }
                }
                let earlier = self.ladder[..rank].iter().filter_map(|other| other.row.as_ref());
                if earlier.clone().any(|other| other.id == row.id) {
                    problems.push(format!("`ladder[{rank}].row.id` `{}` is on two rungs", row.id));
                }
            }
        }
        for (index, cue) in self.cues.iter().enumerate() {
            for (field, character) in [("from", &cue.from), ("to", &cue.to)] {
                if !self.ladder.iter().any(|rung| &rung.character == character) {
                    problems.push(format!("`cues[{index}].{field}` `{character}` is not a rung's character"));
                }
            }
            if cue.from == cue.to {
                problems.push(format!("`cues[{index}]` goes from `{}` to itself", cue.from));
            }
            if cue.cue.trim().is_empty() {
                problems.push(format!("`cues[{index}].cue` is empty"));
            }
            if self.cues[..index].iter().any(|other| other.from == cue.from && other.to == cue.to) {
                problems.push(format!("`cues[{index}]` ({} to {}) is stated twice", cue.from, cue.to));
            }
        }
        problems
    }
}

/// Each value of a shot the projectile road cannot fly, named by its field
/// under `at`.
fn shot_problems(shot: &RangedActionSpec, at: &str) -> Vec<String> {
    let mut values = vec![("speed", shot.speed), ("refire_s", shot.refire_s)];
    if let Some(flight) = &shot.flight {
        values.extend([
            ("flight.gravity", flight.gravity),
            ("flight.max_lifetime", flight.max_lifetime),
            ("flight.half_extent.0", flight.half_extent.x),
            ("flight.half_extent.1", flight.half_extent.y),
        ]);
    }
    let mut problems: Vec<String> = values
        .into_iter()
        .filter(|(_, value)| !(value.is_finite() && *value >= 0.0))
        .map(|(field, value)| format!("`{at}.{field}` is {value}; it is finite and not negative"))
        .collect();
    if shot.damage < 0 {
        problems.push(format!("`{at}.damage` is {}; it is not negative", shot.damage));
    }
    problems
}

/// The schema, for the pack's `content_pack! { schemas: [..] }`.
pub fn mary_o_forms_schema() -> SchemaRegistration {
    rules_schema(
        MARY_O_FORMS_SCHEMA,
        1,
        "mary_o_demo",
        "Her power ladder: each form's character, how she arrives at it, the \
         equipment row she wears on it (armor that a hit spends into the rung \
         below), and the sound of each change of form.",
        MaryOForms::problems,
    )
}

/// Her power ladder, as the demo's pack states it (`data/mary_o_forms.ron`).
pub fn mary_o_forms() -> &'static MaryOForms {
    lowered_rules::<MaryOForms>(crate::pack::PACK.prepared(), MARY_O_FORMS_SCHEMA)
        .expect("the demo's pack states her power ladder (`data/mary_o_forms.ron`)")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rung(character: &str, arrival: Option<FormArrival>, row: Option<&str>) -> FormRung {
        FormRung {
            character: character.into(),
            arrival,
            row: row.map(|id| FormRow { id: id.into(), modifiers: Vec::new(), grants: Vec::new() }),
        }
    }

    fn forms() -> MaryOForms {
        MaryOForms {
            slot: "form".into(),
            ladder: vec![
                rung("small", None, None),
                rung("tall", Some(FormArrival::Grow), Some("wand")),
                rung("fire", Some(FormArrival::Transform), Some("beacon")),
            ],
            cues: vec![FormCue { from: "small".into(), to: "tall".into(), cue: "grow".into() }],
        }
    }

    /// The ladder is the downgrade chain: each form row is armor that leaves
    /// the row of the rung below, in the form slot.
    #[test]
    fn a_form_row_downgrades_into_the_rung_below() {
        let forms = forms();
        assert!(forms.problems().is_empty(), "{:?}", forms.problems());
        let beacon = forms.row("beacon").expect("a form row");
        assert_eq!(beacon.exclusive_slot.as_deref(), Some("form"));
        let Some(OnHit::ConsumeAsArmor { downgrade_to: Some(wand) }) = beacon.on_hit else {
            panic!("the beacon is armor that leaves the wand: {:?}", beacon.on_hit);
        };
        assert_eq!(*wand, forms.row("wand").unwrap());
        assert_eq!(wand.on_hit, Some(OnHit::ConsumeAsArmor { downgrade_to: None }), "the wand's loss leaves nothing");
        assert!(forms.row("small").is_none(), "a character is not a row");
    }

    #[test]
    fn a_ladder_the_demo_cannot_use_is_named_by_its_field() {
        let mut no_row = forms();
        no_row.ladder[1].row = None;
        let mut bottom_row = forms();
        bottom_row.ladder[0].row = Some(FormRow { id: "x".into(), modifiers: Vec::new(), grants: Vec::new() });
        let mut stray_cue = forms();
        stray_cue.cues[0].to = "giant".into();
        for (forms, field) in [
            (no_row, "`ladder[1].row` is absent"),
            (bottom_row, "`ladder[0].row` is `x`"),
            (stray_cue, "`cues[0].to` `giant`"),
        ] {
            let problems = forms.problems();
            assert!(problems.iter().any(|problem| problem.starts_with(field)), "{field}: {problems:?}");
        }
    }

    /// The demo plays its pack's `mary_o_forms.ron`: an edit to the file is
    /// the ladder the next start plays, and a ladder the demo cannot use is
    /// refused at admission with its file and its field named.
    #[test]
    fn her_ladder_is_her_pack_file() {
        use ambition_platformer2d::content::EmbeddedPack;

        let authored = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data/mary_o_forms.ron"))
            .expect("her ladder file is readable");
        let root = std::env::temp_dir().join(format!("mary_o_forms_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            format!(
                r#"(id: "mary_o_forms_probe", version: "0.1.0", namespace: "probe", requires: [],
                    sources: [(path: "data/mary_o_forms.ron", schema: "{MARY_O_FORMS_SCHEMA}", version: 1)])"#
            ),
        )
        .unwrap();
        let compile = |text: &str| {
            std::fs::write(root.join("data/mary_o_forms.ron"), text).unwrap();
            EmbeddedPack::rooted(Box::leak(root.display().to_string().into_boxed_str()), None)
                .with_schemas(|| vec![mary_o_forms_schema()])
                .compile()
        };
        let forms_of = |text: &str| {
            let prepared = compile(text).unwrap_or_else(|failure| panic!("{}", failure.render()));
            lowered_rules::<MaryOForms>(&prepared, MARY_O_FORMS_SCHEMA).expect("the probe pack lowers the ladder").clone()
        };
        let spark = |forms: &MaryOForms| {
            forms
                .row("cinder_beacon")
                .expect("the beacon is on the ladder")
                .grants
                .into_iter()
                .find_map(|grant| match grant {
                    EquipmentGrant::Ranged(shot) => Some(shot),
                    _ => None,
                })
                .expect("the beacon grants the spark")
        };

        let file = forms_of(&authored);
        assert_eq!(&file, mary_o_forms(), "the demo plays the file");
        assert_eq!(spark(&file).damage, 4);

        let damage = "damage: 4,";
        assert_eq!(authored.matches(damage).count(), 1, "the probe edits the spark's damage");
        assert_eq!(spark(&forms_of(&authored.replace(damage, "damage: 7,"))).damage, 7);
        let cue = r#"cue: "mary_o.revert.big_to_small""#;
        assert_eq!(authored.matches(cue).count(), 1, "the probe edits one cue");
        assert_eq!(
            forms_of(&authored.replace(cue, r#"cue: "mary_o.revert.other""#)).cue("mary_o_tall", "mary_o"),
            Some("mary_o.revert.other")
        );

        // A spark that the road cannot fly, and a row on the bottom rung.
        let lifetime = "max_lifetime: 1.5,";
        assert_eq!(authored.matches(lifetime).count(), 1, "the probe edits the spark's lifetime");
        let bottom = r#"(character: "mary_o"),"#;
        assert_eq!(authored.matches(bottom).count(), 1, "the probe edits the bottom rung");
        for (text, field) in [
            (authored.replace(lifetime, "max_lifetime: -1.0,"), "`ladder[2].row.grants[0].flight.max_lifetime`"),
            (authored.replace(bottom, r#"(character: "mary_o", row: Some((id: "extra"))),"#), "`ladder[0].row`"),
        ] {
            let refused = match compile(&text) {
                Ok(_) => panic!("{field}: the ladder was admitted"),
                Err(failure) => failure.render(),
            };
            assert!(refused.contains(field), "the refusal names the field {field}: {refused}");
            assert!(refused.contains("mary_o_forms.ron"), "the refusal names the file: {refused}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
