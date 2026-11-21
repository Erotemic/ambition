//! The view-owned camera policy adapter. Gameplay never reads its state.
use crate::camera_snapshot::CameraSnapshotResolveInput;
use ambition_extension_host::ViewRecords;
use ambition_platformer2d_core as ae;
use ambition_world_port::camera::*;
use bevy::prelude::*;

#[derive(Component, Default, Debug)]
pub struct CameraPolicyState {
    pub frame: u64,
    pub was_snapping: bool,
    pub program: String,
    pub records: ViewRecords,
    pub last_intent: Option<CameraIntent>,
    pub fault: Option<String>,
    pub last_observation: Option<CameraPolicyInput>,
}

pub fn facts(
    input: &CameraSnapshotResolveInput<'_>,
    state: Option<&ambition_camera_presentation::CameraEaseState>,
) -> CameraPolicyInput {
    use ambition_platformer2d_world::rooms::{
        CameraActivationSample as Sample, CameraRegionSelection as Selection,
    };
    let focus = input.focus;
    let region = state
        .and_then(|state| state.active_composition)
        .and_then(|index| input.camera_zones.get(index));
    let safe = input
        .screen_framing
        .filter(|framing| framing.active)
        .map(|framing| framing.subject_safe_region);
    CameraPolicyInput {
        room: String::new(),
        room_instance: 0,
        room_size: input.world.size.into(),
        subjects: vec![CameraSubject {
            id: String::new(),
            previous_position: None,
            position: focus.center_world.into(),
            physical_position: focus.tick_center_world.into(),
            size: focus.size.into(),
            standing_size: focus.base_size.into(),
            velocity: focus.velocity_world.into(),
            down: input.subject_down.unwrap_or(ae::Vec2::Y).into(),
            grounded: focus.grounded,
            facing: focus.facing,
            look: focus.look,
        }],
        viewport: input.viewport_px.into(),
        view_size: crate::camera_snapshot::fit_camera_view(
            input.base_view * state.map_or(1.0, |state| state.live_scale) * focus.scale,
            input.viewport_px,
            input.aspect_policy,
        )
        .0
        .into(),
        safe_min: safe.map_or([0.0, 0.0], |region| region.min.into()),
        safe_max: safe.map_or([1.0, 1.0], |region| region.max.into()),
        previous_center: state
            .filter(|state| state.target_initialized)
            .map_or(focus.center_world, |state| state.live_target_world)
            .into(),
        previous_region: region.map_or_else(String::new, |zone| zone.id.clone()),
        regions: input
            .camera_zones
            .iter()
            .map(|zone| CameraRegion {
                id: zone.id.clone(),
                activation_min: zone.aabb.min.into(),
                activation_max: zone.aabb.max.into(),
                priority: zone.priority,
                blend: match zone.composition.selection {
                    Selection::Exclusive => RegionSelection::Exclusive,
                    Selection::BlendX => RegionSelection::BlendX,
                    Selection::BlendY => RegionSelection::BlendY,
                    Selection::BlendEdge => RegionSelection::BlendEdge,
                },
                blend_width: zone.composition.blend_width,
                sample: match zone.composition.sample {
                    Sample::Center => ActivationSample::Center,
                    Sample::Feet => ActivationSample::Feet,
                    Sample::FullBody => ActivationSample::FullBody,
                },
                enter_margin: zone.composition.enter_margin,
                exit_margin: zone.composition.exit_margin,
                latched: zone.composition.latched,
                bounds: {
                    use ambition_platformer2d_world::rooms::{CameraBoundsSpace, CameraClampMode};
                    let bounds = zone.composition.bounds.unwrap_or(zone.aabb);
                    let rect = CameraRect {
                        min: bounds.min.into(),
                        max: bounds.max.into(),
                    };
                    match zone.clamp_mode {
                        CameraClampMode::None => BoundsRequest::Unbounded,
                        CameraClampMode::RoomBounds => BoundsRequest::Footprint(CameraRect {
                            min: [0.0, 0.0],
                            max: input.world.size.into(),
                        }),
                        CameraClampMode::ZoneBounds => {
                            if zone.composition.bounds_space == CameraBoundsSpace::Centers {
                                BoundsRequest::Centers(rect)
                            } else {
                                BoundsRequest::Footprint(rect)
                            }
                        }
                    }
                },
                target_offset: zone.target_offset.into(),
                target_x: zone.composition.target_x,
                target_y: zone.composition.target_y,
                zoom: zone.effective_zoom(),
                response_hz: zone.easing_hz.unwrap_or(8.0),
                tracking: if zone.composition.tracking
                    == ambition_platformer2d_world::rooms::CameraTracking::Grounded
                {
                    TrackingRequest::Grounded
                } else {
                    TrackingRequest::Follow
                },
            })
            .collect(),
        reset: input.snap_camera,
        continuity: if input.chart_transit.is_some() {
            CameraContinuity::ChartTransit
        } else if input.snap_camera {
            CameraContinuity::Placed
        } else {
            CameraContinuity::Continuous
        },
        director: input.cutscene.map(|camera| CameraDirector {
            from: camera.from,
            to: camera.to,
            progress: camera.progress,
        }),
    }
}

pub fn validate(
    intent: &CameraIntent,
    input: &CameraPolicyInput,
) -> Result<(), ambition_extension_sdk::Fault> {
    let fail = |message: &str| ambition_extension_sdk::Fault::Module(message.to_owned().into());
    for value in intent
        .target
        .iter()
        .flatten()
        .chain(intent.target_x.iter())
        .chain(intent.target_y.iter())
        .chain(intent.zoom.iter())
        .chain(intent.response_hz.iter())
        .chain(intent.transition_seconds.iter())
    {
        if !value.is_finite() {
            return Err(fail("camera values must be finite"));
        }
    }
    if intent.zoom.is_some_and(|zoom| zoom <= 0.0)
        || intent.response_hz.is_some_and(|hz| hz < 0.0)
        || intent
            .transition_seconds
            .is_some_and(|seconds| seconds < 0.0)
    {
        return Err(fail("invalid camera zoom, response or tracking"));
    }
    if let BoundsRequest::Footprint(rect) | BoundsRequest::Centers(rect) = &intent.bounds {
        if rect
            .min
            .iter()
            .chain(rect.max.iter())
            .any(|value| !value.is_finite())
            || rect.min[0] > rect.max[0]
            || rect.min[1] > rect.max[1]
            || (matches!(intent.bounds, BoundsRequest::Footprint(_))
                && (rect.min[0] == rect.max[0] || rect.min[1] == rect.max[1]))
        {
            return Err(fail("camera bounds must be finite and ordered"));
        }
    }
    let mut total = 0.0;
    for (index, weight) in intent.regions.iter().enumerate() {
        if !weight.weight.is_finite()
            || weight.weight < 0.0
            || !input.regions.iter().any(|region| region.id == weight.id)
            || intent.regions[..index]
                .iter()
                .any(|previous| previous.id == weight.id)
        {
            return Err(fail("invalid or duplicate camera region weight"));
        }
        total += weight.weight;
    }
    if total > 1.00001 {
        return Err(fail("camera region weights exceed one"));
    }
    Ok(())
}
