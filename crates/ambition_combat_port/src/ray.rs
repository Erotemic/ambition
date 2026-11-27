//! A ray: a strike along a line. Lightning, a laser, any move that hurts
//! along a line is one.
//!
//! The module says where the line starts, which way it goes, how far it may
//! reach, how wide it is and how it looks. The engine casts it: it stops at
//! the first wall and goes on through each portal pair it enters, so its
//! length is its length unfolded. It hurts as one volume from where it
//! starts, and the portal pairs cut that volume as they cut every other
//! (`BodyGeometry::pieces`): what reaches through a pair is out of its
//! other end. Its look is a name the presentation resolves, drawn from the
//! ray's published geometry; the module never draws it.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// Where something a module asks for is.
#[derive(Clone, Debug, PartialEq)]
pub enum Spot {
    /// A world point, +Y down.
    World([f32; 2]),
    /// An offset from the body the invocation ran for: it follows the body.
    Body([f32; 2]),
    /// A named point on a part the body's art draws: see [`PartSpot`].
    Part(PartSpot),
}

/// A part a body's art can draw, for a spot that follows the art.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtPart {
    /// A lit visor.
    Visor,
    /// A hand (v5): its landmark is `hand_near` or `hand_far`. The art draws
    /// no box for it, so a ray from it is the request's `width` wide.
    Hand,
}

/// A named point on a part the body's art draws in a row. The part's box
/// gives the ray width. The row is named by the request, as a
/// `DrawnRow` names it, so the spot is a function of the module's own state
/// and not of what was drawn last.
#[derive(Clone, Debug, PartialEq)]
pub struct PartSpot {
    pub part: ArtPart,
    /// The rows in order of preference: the first the body's art has.
    pub rows: Vec<String>,
    /// Seconds into that row.
    pub elapsed: f32,
    /// The row runs as a loop on this clock.
    pub looping: bool,
    /// How far in from the bounds of the drawn part its edge is, in sheet
    /// pixels: the transparent margin around the part.
    pub inset_px: f32,
    /// The named origin on this part. A missing point refuses the ray.
    pub landmark: String,
}

impl Spot {
    pub fn put(&self, out: &mut Vec<u8>) {
        match self {
            Spot::World(at) => {
                wire::put_u8(out, 0);
                wire::put_vec2(out, *at);
            }
            Spot::Body(offset) => {
                wire::put_u8(out, 1);
                wire::put_vec2(out, *offset);
            }
            Spot::Part(spot) => {
                wire::put_u8(out, 2);
                wire::put_u8(out, match spot.part {
                    ArtPart::Visor => 0,
                    ArtPart::Hand => 1,
                });
                wire::put_u32(out, spot.rows.len() as u32);
                for row in &spot.rows {
                    wire::put_str(out, row);
                }
                wire::put_f32(out, spot.elapsed);
                wire::put_bool(out, spot.looping);
                wire::put_f32(out, spot.inset_px);
                wire::put_str(out, &spot.landmark);
            }
        }
    }

    pub fn read(r: &mut WireReader<'_>) -> Result<Spot, WireError> {
        match r.u8()? {
            0 => Ok(Spot::World(r.vec2()?)),
            1 => Ok(Spot::Body(r.vec2()?)),
            2 => {
                let part = match r.u8()? {
                    0 => ArtPart::Visor,
                    1 => ArtPart::Hand,
                    tag => return Err(WireError::BadTag(tag)),
                };
                let count = r.read_len()?;
                let mut rows = Vec::new();
                for _ in 0..count {
                    rows.push(r.str()?.to_owned());
                }
                Ok(Spot::Part(PartSpot { part, rows, elapsed: r.f32()?, looping: r.bool()?, inset_px: r.f32()?, landmark: r.str()?.to_owned() }))
            }
            tag => Err(WireError::BadTag(tag)),
        }
    }

    /// A world point without a landmark query. A part needs the host's pose.
    pub fn at(&self, body: [f32; 2]) -> Option<[f32; 2]> {
        match self {
            Spot::World(at) => Some(*at),
            Spot::Body([x, y]) => Some([body[0] + x, body[1] + y]),
            Spot::Part(_) => None,
        }
    }
}

/// The request port marker for a held ray.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — hold a ray from `from` along `toward` (any length; it
///   is the direction), as far as `reach` or the first wall, `width` wide,
///   through the portal pairs it enters. A ray from a [`Spot::Part`] the art
///   draws leaves the whole part: it is as wide as the part is tall, and
///   `width` is its width when the art draws no box for the part (a hand). It hurts what it meets for
///   `damage`, launching by `knockback` (a multiplier over the victim's
///   feel-tuned launch). `launch_dir` selects the axes; absence launches away
///   from the source. It is drawn as `look`.
/// * **Owner** — `ambition_combat::extension`, with the held damage boxes:
///   the same slots, generations and lifetimes. Held and asked again, it is
///   cast again: it follows its `from` and `toward`.
/// * **Scope and grant** — the body the invocation ran for owns it, on its
///   effective side; a body with no faction has its rays refused.
/// * **Time** — offered in `technique_execution` and `boss_conduct`.
/// * **Read model** — world units, +Y down.
/// * **Replay** — as the held damage boxes: the adapter's record is rollback
///   state on the body, and the ray is a hitbox.
pub struct HeldRayPort;

#[derive(Clone, Debug, PartialEq)]
pub struct HeldRay {
    /// The module's name for the ray. Slots are per entry, shared with the
    /// held damage boxes.
    pub slot: u32,
    /// A new generation in a slot replaces the ray.
    pub generation: u32,
    pub from: Spot,
    pub toward: [f32; 2],
    pub reach: f32,
    pub width: f32,
    pub damage: i32,
    pub damage_types: Vec<String>,
    pub knockback: f32,
    pub launch_dir: Option<crate::LaunchDirection>,
    pub lifetime_s: f32,
    /// How it is drawn: a name of the presentation's ray looks (`"strike"`,
    /// `"force"`, `"laser"`).
    pub look: String,
    /// Optional arc to a second target after the ray does damage.
    pub jump: Option<RayJump>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayJump {
    pub chance: f32,
    pub range: f32,
    pub damage: i32,
}

impl RayJump {
    pub fn is_valid(self) -> bool {
        (0.0..=1.0).contains(&self.chance)
            && self.range.is_finite()
            && self.range > 0.0
            && self.damage > 0
    }
}

impl Port for HeldRayPort {
    const KEY: PortKey = PortKey::new("ambition.combat.held_ray", 7);
    const ROLE: PortRole = PortRole::Request;
    type Value = HeldRay;

    fn encode(v: &HeldRay, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<HeldRay, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ray_round_trips() {
        let ray = HeldRay {
            slot: 5,
            generation: 2,
            from: Spot::Part(PartSpot {
                part: ArtPart::Visor,
                rows: vec!["eye_laser".into(), "shoot".into()],
                elapsed: 0.25,
                looping: true,
                inset_px: 3.0,
                landmark: "eye".into(),
            }),
            toward: [0.6, -0.8],
            reach: 460.0,
            width: 30.0,
            damage: 1,
            damage_types: vec!["example.electric".into(), "example.fire".into()],
            knockback: 1.3,
            launch_dir: Some(crate::LaunchDirection::World([-1.0, -0.4])),
            lifetime_s: 0.3,
            look: "force".into(),
            jump: Some(RayJump { chance: 0.4, range: 88.0, damage: 1 }),
        };
        let mut out = Vec::new();
        HeldRayPort::encode(&ray, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(HeldRayPort::decode(&mut r).unwrap(), ray);
        r.finish().unwrap();
        use ambition_extension_sdk::script::ScriptPort;
        assert_eq!(HeldRayPort::from_script(&HeldRayPort::to_script(&ray)).unwrap(), ray);
    }

    #[test]
    fn a_spot_is_a_world_point_or_rides_its_body() {
        assert_eq!(Spot::World([5.0, 6.0]).at([100.0, 100.0]), Some([5.0, 6.0]));
        assert_eq!(Spot::Body([5.0, 6.0]).at([100.0, 100.0]), Some([105.0, 106.0]));
    }
}

impl ambition_extension_sdk::structure::Field<ArtPart> for ArtPart {
    fn shape() -> ambition_extension_sdk::structure::Shape {
        use ambition_extension_sdk::structure::{CaseShape, Shape};
        Shape::Choice(vec![
            CaseShape { name: "visor", payload: None, script: "\"visor\"" },
            CaseShape { name: "hand", payload: None, script: "\"hand\"" },
        ])
    }
    fn put(v: &ArtPart, out: &mut Vec<u8>) {
        wire::put_u8(out, match v { ArtPart::Visor => 0, ArtPart::Hand => 1 });
    }
    fn read(r: &mut WireReader<'_>) -> Result<ArtPart, WireError> {
        match r.u8()? { 0 => Ok(ArtPart::Visor), 1 => Ok(ArtPart::Hand), tag => Err(WireError::BadTag(tag)) }
    }
    fn to_script(v: &ArtPart) -> ambition_extension_sdk::script::ScriptValue {
        ambition_extension_sdk::script::ScriptValue::Text(match v { ArtPart::Visor => "visor", ArtPart::Hand => "hand" }.into())
    }
    fn from_script(v: &ambition_extension_sdk::script::ScriptValue) -> Result<ArtPart, String> {
        match v.as_text()? { "visor" => Ok(ArtPart::Visor), "hand" => Ok(ArtPart::Hand), _ => Err("unknown art part".into()) }
    }
}

ambition_extension_sdk::port_record! {
    PartSpot {
        part: ArtPart,
        rows: Vec<String>,
        elapsed: f32 = 0.0,
        looping: bool = false,
        inset_px: f32 = 0.0,
        landmark: String,
    }
}

impl ambition_extension_sdk::structure::Field<Spot> for Spot {
    fn shape() -> ambition_extension_sdk::structure::Shape {
        use ambition_extension_sdk::structure::{CaseShape, Shape};
        Shape::Choice(vec![
            CaseShape { name: "world", payload: Some(Shape::Vec2), script: "#{ world: [x, y] }" },
            CaseShape { name: "body", payload: Some(Shape::Vec2), script: "#{ body: [x, y] }: offset from the body" },
            CaseShape { name: "part", payload: Some(PartSpot::shape()), script: "#{ part: #{ .. } }: a named point on drawn art" },
        ])
    }
    fn put(v: &Spot, out: &mut Vec<u8>) { v.put(out); }
    fn read(r: &mut WireReader<'_>) -> Result<Spot, WireError> { Spot::read(r) }
    fn to_script(v: &Spot) -> ambition_extension_sdk::script::ScriptValue {
        use ambition_extension_sdk::script::ScriptValue;
        match v {
            Spot::World(at) => ScriptValue::map([("world", ScriptValue::vec2(*at))]),
            Spot::Body(at) => ScriptValue::map([("body", ScriptValue::vec2(*at))]),
            Spot::Part(part) => ScriptValue::map([("part", PartSpot::to_script(part))]),
        }
    }
    fn from_script(v: &ambition_extension_sdk::script::ScriptValue) -> Result<Spot, String> {
        use ambition_extension_sdk::structure::record_fields;
        let fields = record_fields(v, &["world", "body", "part"])?;
        let [(kind, value)] = fields else { return Err("a spot needs exactly one of world, body, or part".into()); };
        match kind.as_str() {
            "world" => <[f32; 2]>::from_script(value).map(Spot::World),
            "body" => <[f32; 2]>::from_script(value).map(Spot::Body),
            "part" => PartSpot::from_script(value).map(Spot::Part),
            _ => unreachable!(),
        }
    }
}

ambition_extension_sdk::port_record! {
    RayJump {
        /// Probability from zero to one for each damage hit.
        chance: f32,
        /// Maximum distance from the body that took damage.
        range: f32,
        damage: i32,
    }
}

ambition_extension_sdk::port_record! {
    HeldRay {
        slot: u32 = 0,
        generation: u32,
        from: Spot,
        toward: [f32; 2],
        reach: f32,
        width: f32,
        damage: i32,
        /// Game-defined IDs for this damage amount.
        damage_types: Vec<String> = Vec::new(),
        knockback: f32 = 1.0,
        launch_dir: Option<crate::LaunchDirection> = None,
        lifetime_s: f32,
        look: String,
        /// Settings for one jump after a body takes damage. The jump stops at solid walls.
        jump: Option<RayJump> = None,
    }
}
ambition_extension_sdk::script_port!(HeldRayPort);
