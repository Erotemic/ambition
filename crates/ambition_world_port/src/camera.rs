//! Camera observations and framing intent. These values contain no engine handles.

use ambition_extension_sdk::{
    structure::Field,
    wire::{WireError, WireReader},
    Port, PortKey, PortRole,
};

ambition_extension_sdk::port_choices! {
    #[derive(Default)]
    pub enum RegionSelection { #[default] Exclusive = 0 => "exclusive", BlendX = 1 => "blend_x", BlendY = 2 => "blend_y", BlendEdge = 3 => "blend_edge" }
}
ambition_extension_sdk::port_choices! {
    #[derive(Default)]
    pub enum ActivationSample { #[default] Center = 0 => "center", Feet = 1 => "feet", FullBody = 2 => "full_body" }
}
ambition_extension_sdk::port_choices! {
    #[derive(Default)]
    pub enum TrackingRequest { #[default] Authored = 0 => "authored", Follow = 1 => "follow", Grounded = 2 => "grounded" }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CameraRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}
ambition_extension_sdk::port_record! { CameraRect { min: [f32;2], max: [f32;2] } }
#[derive(Clone, Debug, Default, PartialEq)]
pub enum BoundsRequest {
    #[default]
    Inherit,
    Unbounded,
    Footprint(CameraRect),
    Centers(CameraRect),
}
impl Field<BoundsRequest> for BoundsRequest {
    fn shape() -> ambition_extension_sdk::structure::Shape {
        use ambition_extension_sdk::structure::{CaseShape, Shape};
        Shape::Choice(vec![
            CaseShape {
                name: "inherit",
                payload: None,
                script: "inherit",
            },
            CaseShape {
                name: "unbounded",
                payload: None,
                script: "unbounded",
            },
            CaseShape {
                name: "footprint",
                payload: Some(CameraRect::shape()),
                script: "#{footprint: #{min: [x,y], max: [x,y]}}",
            },
            CaseShape {
                name: "centers",
                payload: Some(CameraRect::shape()),
                script: "#{centers: #{min: [x,y], max: [x,y]}}",
            },
        ])
    }
    fn put(value: &Self, out: &mut Vec<u8>) {
        use ambition_extension_sdk::wire::put_u8;
        match value {
            Self::Inherit => put_u8(out, 0),
            Self::Unbounded => put_u8(out, 1),
            Self::Footprint(rect) => {
                put_u8(out, 2);
                CameraRect::put(rect, out);
            }
            Self::Centers(rect) => {
                put_u8(out, 3);
                CameraRect::put(rect, out);
            }
        }
    }
    fn read(r: &mut WireReader<'_>) -> Result<Self, WireError> {
        match r.u8()? {
            0 => Ok(Self::Inherit),
            1 => Ok(Self::Unbounded),
            2 => CameraRect::read(r).map(Self::Footprint),
            3 => CameraRect::read(r).map(Self::Centers),
            tag => Err(WireError::BadTag(tag)),
        }
    }
    fn to_script(value: &Self) -> ambition_extension_sdk::script::ScriptValue {
        use ambition_extension_sdk::script::ScriptValue as V;
        match value {
            Self::Inherit => V::Text("inherit".into()),
            Self::Unbounded => V::Text("unbounded".into()),
            Self::Footprint(rect) => V::map([("footprint", CameraRect::to_script(rect))]),
            Self::Centers(rect) => V::map([("centers", CameraRect::to_script(rect))]),
        }
    }
    fn from_script(value: &ambition_extension_sdk::script::ScriptValue) -> Result<Self, String> {
        use ambition_extension_sdk::script::ScriptValue as V;
        match value {
            V::Text(name) if name == "inherit" => Ok(Self::Inherit),
            V::Text(name) if name == "unbounded" => Ok(Self::Unbounded),
            V::Map(fields) if fields.len() == 1 => {
                let (name, value) = &fields[0];
                let rect = CameraRect::from_script(value)?;
                match name.as_str() {
                    "footprint" => Ok(Self::Footprint(rect)),
                    "centers" => Ok(Self::Centers(rect)),
                    _ => Err("unknown camera bounds".into()),
                }
            }
            _ => Err("expected inherit, unbounded, footprint or centers camera bounds".into()),
        }
    }
}

ambition_extension_sdk::port_choices! {
    #[derive(Default)]
    pub enum CameraContinuity { #[default] Continuous=0=>"continuous", Placed=1=>"placed", RoomChanged=2=>"room_changed", SubjectChanged=3=>"subject_changed", ChartTransit=4=>"chart_transit" }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraSubject {
    pub id: String,
    pub previous_position: Option<[f32; 2]>,
    pub position: [f32; 2],
    pub physical_position: [f32; 2],
    pub size: [f32; 2],
    pub standing_size: [f32; 2],
    pub velocity: [f32; 2],
    pub down: [f32; 2],
    pub grounded: bool,
    pub facing: f32,
    pub look: f32,
}
ambition_extension_sdk::port_record! { CameraSubject {
    id: String, previous_position: Option<[f32; 2]>,
    position: [f32; 2], physical_position: [f32; 2], size: [f32; 2],
    standing_size: [f32; 2], velocity: [f32; 2], down: [f32; 2], grounded: bool, facing: f32, look: f32,
} }

#[derive(Clone, Debug, PartialEq)]
pub struct CameraRegion {
    pub id: String,
    pub activation_min: [f32; 2],
    pub activation_max: [f32; 2],
    pub priority: i32,
    pub blend: RegionSelection,
    pub blend_width: f32,
    pub sample: ActivationSample,
    pub enter_margin: f32,
    pub exit_margin: f32,
    pub latched: bool,
    pub bounds: BoundsRequest,
    pub target_offset: [f32; 2],
    pub target_x: Option<f32>,
    pub target_y: Option<f32>,
    pub zoom: f32,
    pub response_hz: f32,
    pub tracking: TrackingRequest,
}
ambition_extension_sdk::port_record! { CameraRegion {
    id: String, activation_min: [f32; 2], activation_max: [f32; 2], priority: i32,
    blend: RegionSelection, blend_width: f32, sample: ActivationSample, enter_margin: f32, exit_margin: f32, latched: bool,
    bounds: BoundsRequest, target_offset: [f32;2],
    target_x: Option<f32>, target_y: Option<f32>, zoom: f32, response_hz: f32, tracking: TrackingRequest,
} }

#[derive(Clone, Debug, PartialEq)]
pub struct CameraDirector {
    pub from: Option<[f32; 2]>,
    pub to: Option<[f32; 2]>,
    pub progress: f32,
}
ambition_extension_sdk::port_record! { CameraDirector { from: Option<[f32;2]>, to: Option<[f32;2]>, progress: f32 } }

#[derive(Clone, Debug, PartialEq)]
pub struct CameraPolicyInput {
    pub room: String,
    pub room_instance: u64,
    pub room_size: [f32; 2],
    pub subjects: Vec<CameraSubject>,
    pub viewport: [f32; 2],
    pub view_size: [f32; 2],
    pub safe_min: [f32; 2],
    pub safe_max: [f32; 2],
    pub previous_center: [f32; 2],
    pub previous_region: String,
    pub regions: Vec<CameraRegion>,
    pub reset: bool,
    pub continuity: CameraContinuity,
    pub director: Option<CameraDirector>,
}
ambition_extension_sdk::port_record! { CameraPolicyInput {
    /// The authored room ID.
    room: String,
    /// The live occurrence of this room.
    room_instance: u64,
    /// Room dimensions in world units, with +Y down.
    room_size: [f32;2],
    /// All watched bodies, sorted by stable simulation identity.
    subjects: Vec<CameraSubject>,
    /// The physical viewport dimensions in pixels.
    viewport: [f32; 2],
    /// The fitted visible size in world units before this policy runs.
    view_size: [f32; 2],

    /// The safe screen rectangle minimum, as a fraction of the viewport.
    safe_min: [f32; 2],
    /// The safe screen rectangle maximum, as a fraction of the viewport.
    safe_max: [f32; 2],
    /// The last presented camera center in world units.
    previous_center: [f32; 2],
    /// The last dominant region ID, or empty.
    previous_region: String,

    /// Authored activation and composition facts.
    regions: Vec<CameraRegion>,
    /// The view scope has changed or was placed.
    reset: bool,
    /// The kind of movement since the previous frame.
    continuity: CameraContinuity,
    /// The active room cutscene camera, if any.
    director: Option<CameraDirector>,
} }

#[derive(Clone, Debug, PartialEq)]
pub struct RegionWeight {
    pub id: String,
    pub weight: f32,
}
ambition_extension_sdk::port_record! { RegionWeight { id: String, weight: f32 } }

#[derive(Clone, Debug, PartialEq)]
pub struct CameraIntent {
    pub regions: Vec<RegionWeight>,
    pub target: Option<[f32; 2]>,
    pub zoom: Option<f32>,
    pub target_x: Option<f32>,
    pub target_y: Option<f32>,
    pub response_hz: Option<f32>,
    pub transition_seconds: Option<f32>,
    pub transition_id: Option<u32>,
    pub bounds: BoundsRequest,
    /// Authored uses the selected composition tracking mode.
    pub tracking: TrackingRequest,
    pub allow_look: bool,
    pub keep_visible: bool,
    pub cut: bool,
    pub honor_director: bool,
}
impl Default for CameraIntent {
    fn default() -> Self {
        Self {
            regions: Vec::new(),
            target: None,
            zoom: None,
            target_x: None,
            target_y: None,
            response_hz: None,
            transition_seconds: None,
            transition_id: None,
            bounds: BoundsRequest::Inherit,
            tracking: TrackingRequest::Authored,
            allow_look: true,
            keep_visible: true,
            cut: false,
            honor_director: true,
        }
    }
}
ambition_extension_sdk::port_record! { CameraIntent {
    regions: Vec<RegionWeight> = Vec::new(), target: Option<[f32; 2]> = None, zoom: Option<f32> = None,
    target_x: Option<f32> = None, target_y: Option<f32> = None, response_hz: Option<f32> = None,
    transition_seconds: Option<f32> = None, transition_id: Option<u32> = None,
    bounds: BoundsRequest = BoundsRequest::Inherit,
    tracking: TrackingRequest = TrackingRequest::Authored, allow_look: bool = true,
    keep_visible: bool = true, cut: bool = false, honor_director: bool = true,
} }

pub struct CameraPolicyPort;
impl Port for CameraPolicyPort {
    const KEY: PortKey = PortKey::new("ambition.camera.policy", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = CameraPolicyInput;
    fn encode(v: &Self::Value, out: &mut Vec<u8>) {
        CameraPolicyInput::put(v, out);
    }
    fn decode(r: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        CameraPolicyInput::read(r)
    }
}
ambition_extension_sdk::script_port!(CameraPolicyPort);
pub struct CameraIntentPort;
impl Port for CameraIntentPort {
    const KEY: PortKey = PortKey::new("ambition.camera.intent", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = CameraIntent;
    fn encode(v: &Self::Value, out: &mut Vec<u8>) {
        CameraIntent::put(v, out);
    }
    fn decode(r: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        CameraIntent::read(r)
    }
}
ambition_extension_sdk::script_port!(CameraIntentPort);

/// Select regions from physical facts. Tracking anchors never enter this calculation.
pub fn native_policy(input: &CameraPolicyInput) -> CameraIntent {
    let mut candidates = input
        .regions
        .iter()
        .filter_map(|region| {
            let was = !input.reset && region.id == input.previous_region;
            let margin = if was {
                -region.exit_margin
            } else {
                region.enter_margin
            };
            let mut weight: Option<f32> = None;
            for subject in &input.subjects {
                let sample_at = |at: [f32; 2]| {
                    if region.sample == ActivationSample::Feet {
                        let extent = (subject.down[0].abs() * subject.size[0]
                            + subject.down[1].abs() * subject.size[1])
                            * 0.5;
                        [
                            at[0] + subject.down[0] * extent,
                            at[1] + subject.down[1] * extent,
                        ]
                    } else {
                        at
                    }
                };
                let at = sample_at(subject.physical_position);
                let min = [
                    region.activation_min[0] + margin,
                    region.activation_min[1] + margin,
                ];
                let max = [
                    region.activation_max[0] - margin,
                    region.activation_max[1] - margin,
                ];
                let inside = (0..2).all(|axis| {
                    let half = if region.sample == ActivationSample::FullBody {
                        subject.size[axis] * 0.5
                    } else {
                        0.0
                    };
                    at[axis] - half >= min[axis] && at[axis] + half <= max[axis]
                });
                let crossed = region.latched
                    && !input.reset
                    && input.continuity == CameraContinuity::Continuous
                    && subject
                        .previous_position
                        .is_some_and(|previous| crosses(sample_at(previous), at, min, max));
                if !inside && !crossed && !(was && region.latched) {
                    continue;
                }
                let at = sample_at(subject.position);
                let u = match region.blend {
                    RegionSelection::Exclusive => 1.0,
                    RegionSelection::BlendX | RegionSelection::BlendY => {
                        let axis = if region.blend == RegionSelection::BlendX {
                            0
                        } else {
                            1
                        };
                        (at[axis] - region.activation_min[axis]) / region.blend_width.max(1.0)
                    }
                    _ => {
                        (0..2)
                            .map(|axis| {
                                (at[axis] - region.activation_min[axis])
                                    .min(region.activation_max[axis] - at[axis])
                            })
                            .fold(f32::INFINITY, f32::min)
                            / region.blend_width.max(1.0)
                    }
                }
                .clamp(0.0, 1.0);
                let u = u * u * (3.0 - 2.0 * u);
                weight = Some(weight.map_or(u, |was| was.max(u)));
            }
            weight.map(|weight| (region, weight))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|(a, _), (b, _)| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));
    let mut intent = CameraIntent {
        allow_look: true,
        keep_visible: true,
        ..Default::default()
    };
    let Some((first, _)) = candidates.first() else {
        return intent;
    };
    let priority = first.priority;
    if let Some((region, _)) = candidates
        .iter()
        .take_while(|(region, _)| region.priority == priority)
        .find(|(region, _)| region.blend == RegionSelection::Exclusive)
    {
        intent.regions.push(RegionWeight {
            id: region.id.clone(),
            weight: 1.0,
        });
        return intent;
    }
    intent.regions = candidates
        .into_iter()
        .take_while(|(region, _)| region.priority == priority)
        .map(|(region, weight)| RegionWeight {
            id: region.id.clone(),
            weight,
        })
        .collect();
    let total: f32 = intent.regions.iter().map(|weight| weight.weight).sum();
    if total > 1.0 {
        for weight in &mut intent.regions {
            weight.weight /= total;
        }
    }
    intent
}

fn crosses(from: [f32; 2], to: [f32; 2], min: [f32; 2], max: [f32; 2]) -> bool {
    let (mut enter, mut exit) = (0.0_f32, 1.0_f32);
    for axis in 0..2 {
        let delta = to[axis] - from[axis];
        if delta == 0.0 {
            if from[axis] < min[axis] || from[axis] > max[axis] {
                return false;
            }
            continue;
        }
        let (a, b) = (
            (min[axis] - from[axis]) / delta,
            (max[axis] - from[axis]) / delta,
        );
        enter = enter.max(a.min(b));
        exit = exit.min(a.max(b));
        if enter > exit {
            return false;
        }
    }
    true
}

pub const DEFAULT_POLICY: &str = "ambition::camera/follow";

pub fn module() -> ambition_extension_sdk::ModuleDescriptor {
    use ambition_extension_sdk::*;
    fn follow(inv: &mut Invocation<'_>) -> Result<(), Fault> {
        let intent = native_policy(inv.trigger::<CameraPolicyPort>()?);
        inv.submit::<CameraIntentPort>(intent)
    }
    ModuleDescriptor {
        key: ModuleKey::new("ambition", "camera"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: "ambition_world_port".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            build: None,
        },
        schemas: vec![],
        entries: vec![EntryDescriptor {
            key: "follow".into(),
            phase: phases::CAMERA_POLICY,
            trigger: TriggerBinding {
                port: CameraPolicyPort::KEY,
                selector: "view".into(),
            },
            reads: vec![],
            writes: vec![],
            requests: vec![CameraIntentPort::KEY],
            after: vec![],
            replies: vec![],
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(follow),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> CameraPolicyInput {
        CameraPolicyInput {
            room: "room".into(),
            room_instance: 1,
            room_size: [800.0, 672.0],
            subjects: vec![CameraSubject {
                id: "body".into(),
                previous_position: None,
                position: [400.0, 240.0],
                physical_position: [400.0, 240.0],
                size: [24.0, 32.0],
                standing_size: [24.0, 32.0],
                velocity: [0.0, 0.0],
                down: [0.0, 1.0],
                grounded: false,
                facing: 1.0,
                look: 0.0,
            }],
            viewport: [800.0, 600.0],
            view_size: [568.0, 426.0],
            safe_min: [0.0, 0.0],
            safe_max: [1.0, 1.0],
            previous_center: [400.0, 464.0],
            previous_region: String::new(),
            reset: false,
            continuity: CameraContinuity::Continuous,
            director: None,
            regions: vec![CameraRegion {
                id: "upper".into(),
                activation_min: [0.0, 0.0],
                activation_max: [800.0, 256.0],
                priority: 10,
                blend: RegionSelection::Exclusive,
                blend_width: 128.0,
                sample: ActivationSample::Feet,
                enter_margin: 0.0,
                exit_margin: 8.0,
                latched: false,
                bounds: BoundsRequest::Footprint(CameraRect {
                    min: [0.0, 0.0],
                    max: [800.0, 672.0],
                }),
                target_offset: [0.0, 0.0],
                target_x: None,
                target_y: Some(240.0),
                zoom: 1.0,
                response_hz: 8.0,
                tracking: TrackingRequest::Follow,
            }],
        }
    }
    #[test]
    fn airborne_entry_uses_physical_feet_and_not_the_drawn_lead() {
        let mut input = input();
        input.subjects[0].physical_position[1] = 244.0;
        input.subjects[0].position[1] = 232.0;
        assert!(native_policy(&input).regions.is_empty());
        input.subjects[0].physical_position[1] = 236.0;
        assert_eq!(native_policy(&input).regions[0].id, "upper");
        input.view_size = [1200.0, 900.0];
        assert_eq!(native_policy(&input).regions[0].id, "upper");
    }
    #[test]
    fn hysteresis_and_latched_sweeps_have_explicit_lifetimes() {
        let mut input = input();
        input.previous_region = "upper".into();
        input.subjects[0].physical_position[1] = 244.0;
        assert_eq!(native_policy(&input).regions[0].id, "upper");
        input.subjects[0].physical_position[1] = 250.0;
        assert!(native_policy(&input).regions.is_empty());
        input.regions[0].latched = true;
        assert_eq!(native_policy(&input).regions[0].id, "upper");
        input.reset = true;
        assert!(native_policy(&input).regions.is_empty());
        input.reset = false;
        input.previous_region.clear();
        input.subjects[0].previous_position = Some([400.0, 300.0]);
        input.subjects[0].physical_position = [400.0, -100.0];
        assert_eq!(native_policy(&input).regions[0].id, "upper");
    }
    #[test]
    fn every_watched_body_can_select_a_region_and_ties_are_stable() {
        let mut input = input();
        let mut second = input.subjects[0].clone();
        input.subjects[0].physical_position[1] = 400.0;
        second.id = "second".into();
        input.subjects.push(second);
        assert_eq!(native_policy(&input).regions[0].id, "upper");
        let mut another = input.regions[0].clone();
        another.id = "another".into();
        input.regions.push(another);
        assert_eq!(native_policy(&input).regions[0].id, "another");
        input.regions.reverse();
        assert_eq!(native_policy(&input).regions[0].id, "another");
    }
    #[test]
    fn camera_intents_have_one_native_wire_and_script_meaning() {
        use ambition_extension_sdk::structure::Field;
        let intent = CameraIntent {
            bounds: BoundsRequest::Centers(CameraRect {
                min: [20.0, 30.0],
                max: [40.0, 50.0],
            }),
            target_y: Some(42.0),
            transition_seconds: Some(0.4),
            transition_id: Some(7),
            ..Default::default()
        };
        let mut bytes = Vec::new();
        CameraIntent::put(&intent, &mut bytes);
        let from_wire =
            ambition_extension_sdk::wire::decode_all(&bytes, CameraIntent::read).unwrap();
        assert_eq!(intent, from_wire);
        assert_eq!(
            intent,
            CameraIntent::from_script(&CameraIntent::to_script(&intent)).unwrap()
        );
    }
}
