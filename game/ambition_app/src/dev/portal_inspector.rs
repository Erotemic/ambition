//! Custom F3 portal inspector: one app-side window that groups the portal
//! resources without changing their ownership or defaults.
//!
//! The quick `ResourceInspectorPlugin<T>` windows are useful for raw reflection,
//! but portal tuning needs a non-overlapping label column, exact field names, and
//! tooltips grounded in the resource documentation. This panel edits the same
//! live resources directly; only the dev presentation changes.

#[cfg(all(feature = "dev_tools", feature = "portal"))]
mod enabled {
    use ambition::dev_tools::dev_tools::inspector_visible;
    #[cfg(feature = "portal_render")]
    use ambition::portal::PlacedPortal;
    use ambition::portal::{PortalConvention, PortalTuning};
    #[cfg(feature = "portal_render")]
    use ambition::portal_presentation::{
        selected_portal_view_cone_debug_rows, PortalApertureLosQuality,
        PortalCameraContinuityConfig, PortalCameraContinuityHostView,
        PortalCameraContinuitySelection, PortalCameraTransitMode, PortalCaptureCameraMode,
        PortalEffectSelection, PortalViewConeConfig, PortalViewConeDebugDumpRequest,
        PortalViewConeMode, PortalViewConeSourceClipPolicy, PortalViewConeVisibilityMode,
        PortalViewer, PortalVisualEffect, PortalWorldFrame,
    };
    use bevy::prelude::*;
    use bevy_inspector_egui::bevy_egui::{
        egui, EguiContext, EguiPrimaryContextPass, PrimaryEguiContext,
    };
    use bevy_inspector_egui::DefaultInspectorConfigPlugin;

    const PORTAL_INSPECTOR_WIDTH: f32 = 920.0;
    const PORTAL_INSPECTOR_HEIGHT: f32 = 680.0;

    pub struct PortalInspectorPlugin;

    impl Plugin for PortalInspectorPlugin {
        fn build(&self, app: &mut App) {
            if !app.is_plugin_added::<DefaultInspectorConfigPlugin>() {
                app.add_plugins(DefaultInspectorConfigPlugin);
            }
            app.add_systems(
                EguiPrimaryContextPass,
                portal_inspector_ui.run_if(inspector_visible),
            );
        }
    }

    fn portal_inspector_ui(world: &mut World) {
        let Ok(egui_context) = world
            .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
            .single(world)
        else {
            return;
        };
        let mut egui_context = egui_context.clone();

        egui::Window::new("Portal")
            .default_width(PORTAL_INSPECTOR_WIDTH)
            .default_height(PORTAL_INSPECTOR_HEIGHT)
            .min_width(PORTAL_INSPECTOR_WIDTH)
            .show(egui_context.get_mut(), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_min_width(PORTAL_INSPECTOR_WIDTH - 48.0);
                    ui.label("Field labels are exact Rust variable names. Hover a label, ?, or control for tuning notes.");
                    ui.separator();
                    mechanics_section(world, ui);
                    effects_section(world, ui);
                    camera_continuity_section(world, ui);
                    view_cones_section(world, ui);
                });
            });
    }

    fn mechanics_section(world: &mut World, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Mechanics")
            .default_open(true)
            .show(ui, |ui| {
                let Some(mut tuning) = world.get_resource_mut::<PortalTuning>() else {
                    missing_resource(ui, "PortalTuning");
                    return;
                };
                egui::Grid::new("portal_mechanics_grid")
                    .num_columns(3)
                    .spacing([14.0, 4.0])
                    .min_col_width(110.0)
                    .show(ui, |ui| {
                        convention_row(
                            ui,
                            "convention",
                            &mut tuning.convention,
                            "Active portal map convention. Reflection preserves tangents and flips normals; Rotation maps the entry-facing chart into the exit.",
                        );
                        u32_row(
                            ui,
                            "raycast_recursion_depth",
                            &mut tuning.raycast_recursion_depth,
                            "levels",
                            "Budget for portal-aware logic raycasts. Production fire traces do not recurse yet, but tests and tools can use this instead of hard-coding.",
                        );
                        f32_row(
                            ui,
                            "min_exit_speed",
                            &mut tuning.min_exit_speed,
                            1.0,
                            "world px/s",
                            "Minimum exit speed along the exit normal after a body transfers.",
                        );
                        f32_row(
                            ui,
                            "teleport_cooldown_s",
                            &mut tuning.teleport_cooldown_s,
                            0.01,
                            "seconds",
                            "Per-body anti-ping-pong latch after a transfer.",
                        );
                        f32_row(
                            ui,
                            "emission_time_s",
                            &mut tuning.emission_time_s,
                            0.01,
                            "seconds",
                            "Duration of the input guard that prevents immediate pushback into the exit wall.",
                        );
                        f32_row(
                            ui,
                            "input_held_epsilon",
                            &mut tuning.input_held_epsilon,
                            0.01,
                            "axis magnitude",
                            "Stick/axis magnitude above which movement counts as held.",
                        );
                        f32_row(
                            ui,
                            "input_warp_keep_cos",
                            &mut tuning.input_warp_keep_cos,
                            0.01,
                            "cosine",
                            "Cosine threshold before a changed held direction drops the input warp.",
                        );
                        bool_row(
                            ui,
                            "suppress_wall_abilities",
                            &mut tuning.suppress_wall_abilities,
                            "While an actor is in a portal aperture, disable wall movement abilities so carved aperture edges cannot catch them.",
                        );
                        bool_row(
                            ui,
                            "reorient_facing",
                            &mut tuning.reorient_facing,
                            "Global gate for same-wall turn-around transits to re-orient the body's facing. It is ANDed with each body's PortalPolicy reorient flag.",
                        );
                    });
            });
    }

    #[cfg(feature = "portal_render")]
    fn effects_section(world: &mut World, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Effects")
            .default_open(true)
            .show(ui, |ui| {
                let Some(mut selection) = world.get_resource_mut::<PortalEffectSelection>() else {
                    missing_resource(ui, "PortalEffectSelection");
                    return;
                };
                egui::Grid::new("portal_effects_grid")
                    .num_columns(3)
                    .spacing([14.0, 4.0])
                    .min_col_width(110.0)
                    .show(ui, |ui| {
                        effect_row(
                            ui,
                            "active",
                            &mut selection.active,
                            "Which portal visual effect is live right now. Inactive effect systems stand down; the view-cone renderer despawns capture rigs entirely so A/B profiling sees the true cost delta.",
                        );
                    });
            });
    }

    #[cfg(not(feature = "portal_render"))]
    fn effects_section(_world: &mut World, _ui: &mut egui::Ui) {}

    #[cfg(feature = "portal_render")]
    fn camera_continuity_section(world: &mut World, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Camera Continuity")
            .default_open(true)
            .show(ui, |ui| {
                if let Some(mut selection) = world.get_resource_mut::<PortalCameraContinuitySelection>()
                {
                    egui::Grid::new("portal_camera_continuity_selection_grid")
                        .num_columns(3)
                        .spacing([14.0, 4.0])
                        .min_col_width(110.0)
                        .show(ui, |ui| {
                            camera_mode_row(
                                ui,
                                "mode",
                                &mut selection.mode,
                                "How the host camera behaves around a portal transit. Pop leaves the host camera policy alone; Continuous maps the previous visible camera center through the same portal body map while the focus remains in the aperture.",
                            );
                        });
                } else {
                    missing_resource(ui, "PortalCameraContinuitySelection");
                }

                ui.add_space(4.0);

                if let Some(mut config) = world.get_resource_mut::<PortalCameraContinuityConfig>() {
                    egui::Grid::new("portal_camera_continuity_config_grid")
                        .num_columns(3)
                        .spacing([14.0, 4.0])
                        .min_col_width(110.0)
                        .show(ui, |ui| {
                            f32_row(
                                ui,
                                "roll_epsilon_radians",
                                &mut config.roll_epsilon_radians,
                                0.001,
                                "radians",
                                "Ignore camera roll below this threshold. Straight-through wall-to-wall and floor-to-ceiling transitions should not visibly perturb the camera.",
                            );
                            vec2_row(
                                ui,
                                "max_entry_screen_offset",
                                &mut config.max_entry_screen_offset,
                                1.0,
                                "world units",
                                "Maximum absolute screen offset from camera center for the entry aperture to be considered the visible seam that the continuity pass should preserve.",
                            );
                            bool_row(
                                ui,
                                "debug_log",
                                &mut config.debug_log,
                                "Emit one-line transition diagnostics on each focus transit. This logs start/skip decisions, not every frame.",
                            );
                            f32_row(
                                ui,
                                "camera_constraint_warn_pixels",
                                &mut config.camera_constraint_warn_pixels,
                                1.0,
                                "world units",
                                "Emit a constraint diagnostic when the portal-continuous camera center disagrees with the host camera center by more than this on either axis, or when the desired center needs room-bound padding.",
                            );
                            f32_row(
                                ui,
                                "overlap_warn_weight",
                                &mut config.overlap_warn_weight,
                                0.01,
                                "active weight",
                                "Treat a new transfer as overlapping a previous continuity effect when the previous effect still has at least this much active weight.",
                            );
                        });
                } else {
                    missing_resource(ui, "PortalCameraContinuityConfig");
                }
            });
    }

    #[cfg(not(feature = "portal_render"))]
    fn camera_continuity_section(_world: &mut World, _ui: &mut egui::Ui) {}

    #[cfg(feature = "portal_render")]
    fn view_cones_section(world: &mut World, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("View Cones")
            .default_open(true)
            .show(ui, |ui| {
                let mut request_dump = false;
                let debug_config = {
                    let Some(mut config) = world.get_resource_mut::<PortalViewConeConfig>() else {
                        missing_resource(ui, "PortalViewConeConfig");
                        return;
                    };

                    ui.collapsing("Mode", |ui| {
                    egui::Grid::new("portal_view_cones_mode_grid")
                        .num_columns(3)
                        .spacing([14.0, 4.0])
                        .min_col_width(320.0)
                        .show(ui, |ui| {
                            view_cone_mode_row(
                                ui,
                                "mode",
                                &mut config.mode,
                                "High-level view-window behavior. Off hides portal view windows, Static uses the authored static view_cone without viewer LOS, and Dynamic uses viewer-dependent LOS admission and shaping.",
                            );
                        });
                });

                ui.collapsing("Dynamic Visibility", |ui| {
                    egui::Grid::new("portal_view_cones_visibility_grid")
                        .num_columns(3)
                        .spacing([14.0, 4.0])
                        .min_col_width(320.0)
                        .show(ui, |ui| {
                            view_cone_visibility_mode_row(
                                ui,
                                "visibility_mode",
                                &mut config.visibility_mode,
                                "Dynamic visibility policy. Selects which LOS route may admit/open the cone and whether portal-continuity routes can shape the cone after face LOS exists or while crossing this portal's doorway.",
                            );
                            aperture_los_quality_row(
                                ui,
                                "aperture_los_quality",
                                &mut config.aperture_los_quality,
                                "Aperture LOS quality. Low is the original single center ray per viewer corner. Medium samples the left endpoint, center, and right endpoint, then averages visible samples.",
                            );
                            source_clip_policy_row(
                                ui,
                                "source_clip_policy",
                                &mut config.source_clip_policy,
                                "Policy for reconciling plan.wedge.source with the final source rect used by mesh UVs and the capture camera.",
                            );
                            capture_camera_mode_row(
                                ui,
                                "capture_camera_mode",
                                &mut config.capture_camera_mode,
                                "Capture camera policy. ConeRect frames the tight cone source rect; MappedCameraSnapshot samples from the destination-side host camera frame mapped through the portal.",
                            );
                        });
                });

                ui.collapsing("Dynamic Shape", |ui| {
                    egui::Grid::new("portal_view_cones_dynamic_shape_grid")
                        .num_columns(3)
                        .spacing([14.0, 4.0])
                        .min_col_width(240.0)
                        .show(ui, |ui| {
                            f32_row(
                                ui,
                                "dynamic_depth_close",
                                &mut config.dynamic_depth_close,
                                1.0,
                                "world px",
                                "Maximum dynamic window depth behind the surface, reached when the viewer is within dynamic_dist_close of the aperture.",
                            );
                            f32_row(
                                ui,
                                "dynamic_depth_far",
                                &mut config.dynamic_depth_far,
                                1.0,
                                "world px",
                                "Minimum dynamic window depth behind the surface, reached when the viewer is beyond dynamic_dist_far.",
                            );
                            f32_row(
                                ui,
                                "dynamic_dist_close",
                                &mut config.dynamic_dist_close,
                                1.0,
                                "world px",
                                "Viewer-to-aperture distance at or below which dynamic depth equals dynamic_depth_close.",
                            );
                            f32_row(
                                ui,
                                "dynamic_dist_far",
                                &mut config.dynamic_dist_far,
                                1.0,
                                "world px",
                                "Viewer-to-aperture distance at or beyond which dynamic depth equals dynamic_depth_far.",
                            );
                            f32_row(
                                ui,
                                "blend_rate",
                                &mut config.blend_rate,
                                0.1,
                                "per second",
                                "How quickly the window opens/closes between the minimum cone and the visible wedge using exponential approach.",
                            );
                            f32_row(
                                ui,
                                "min_depth",
                                &mut config.min_depth,
                                1.0,
                                "world px",
                                "Minimum cone depth shown once LOS admits the window. Blocked LOS hides the capture window instead of drawing the minimum through walls.",
                            );
                            f32_row(
                                ui,
                                "min_spread",
                                &mut config.min_spread,
                                0.01,
                                "per px depth",
                                "Minimum-cone side widening per pixel of depth.",
                            );
                            f32_row(
                                ui,
                                "viewer_blend",
                                &mut config.viewer_blend,
                                0.01,
                                "0=min, 1=wedge",
                                "Blend from the minimum cone toward the visible wedge. The default 1.0 follows the real visibility wedge as soon as any visibility exists; lower values are for tuning transitions.",
                            );
                        });
                });

                ui.collapsing("Half-Plane Preview", |ui| {
                    egui::Grid::new("portal_view_cones_half_plane_grid")
                        .num_columns(3)
                        .spacing([14.0, 4.0])
                        .min_col_width(240.0)
                        .show(ui, |ui| {
                            f32_row(
                                ui,
                                "half_plane_preview_full_distance",
                                &mut config.half_plane_preview_full_distance,
                                1.0,
                                "world px",
                                "Body-edge distance to the finite aperture where the art-directed half-plane preview is fully applied. Set to 0.0 for exact LOS geometry with no preview assist.",
                            );
                            f32_row(
                                ui,
                                "half_plane_preview_blend_distance",
                                &mut config.half_plane_preview_blend_distance,
                                1.0,
                                "world px",
                                "Extra directed distance before half_plane_preview_full_distance over which exact LOS geometry eases toward the half-plane preview.",
                            );
                            f32_row(
                                ui,
                                "half_plane_preview_max_lateral",
                                &mut config.half_plane_preview_max_lateral,
                                1.0,
                                "world px",
                                "Maximum lateral reach of the half-plane preview behind the portal face. This bounds near-plane preview geometry so it cannot create enormous source rects that clip into misleading textures.",
                            );
                        });
                });

                ui.collapsing("Static Shape", |ui| {
                    egui::Grid::new("portal_view_cones_static_grid")
                        .num_columns(3)
                        .spacing([14.0, 4.0])
                        .min_col_width(240.0)
                        .show(ui, |ui| {
                            f32_row(
                                ui,
                                "static_depth",
                                &mut config.static_depth,
                                1.0,
                                "world px",
                                "Static-mode window depth into the surface. Used when mode is Static.",
                            );
                            f32_row(
                                ui,
                                "static_spread",
                                &mut config.static_spread,
                                0.01,
                                "per px depth",
                                "Static-mode side widening per pixel of depth. Zero is a straight corridor.",
                            );
                        });
                });

                ui.collapsing("Capture / Draw", |ui| {
                    egui::Grid::new("portal_view_cones_capture_grid")
                        .num_columns(3)
                        .spacing([14.0, 4.0])
                        .min_col_width(240.0)
                        .show(ui, |ui| {
                            f32_row(
                                ui,
                                "texels_per_world_px",
                                &mut config.texels_per_world_px,
                                0.05,
                                "texels/world px",
                                "Capture sharpness target: texels per world pixel along the window's long axis, capped by max_resolution.",
                            );
                            u32_row(
                                ui,
                                "max_resolution",
                                &mut config.max_resolution,
                                "texture px",
                                "Hard cap on the capture texture's long side. It is a GPU memory guard.",
                            );
                            u32_row(
                                ui,
                                "recursion_depth",
                                &mut config.recursion_depth,
                                "capture levels",
                                "Portal-window capture recursion. Zero makes capture cameras see only the world layer; positive values include other portal windows and preserve current one-frame-lag recursive feedback.",
                            );
                            read_only_bool_row(
                                ui,
                                "recursion_includes_portal_windows",
                                config.recursion_depth > 0,
                                "derived",
                                "Derived runtime value from recursion_depth. False means capture cameras exclude the portal-window render layer; true means capture cameras include portal windows and can show one-frame-lag recursive feedback.",
                            );
                            f32_row(
                                ui,
                                "z",
                                &mut config.z,
                                0.01,
                                "z units",
                                "Render z of the window mesh. It sits just behind the portal rim so the doorway stays crisp, above world blocks and below actors.",
                            );
                            f32_row(
                                ui,
                                "z_proximity_span",
                                &mut config.z_proximity_span,
                                0.01,
                                "z units",
                                "Z range over which nearer portals' windows draw on top of farther ones by adding an inverse-distance bias, kept under the rim gap.",
                            );
                            color_row(
                                ui,
                                "tint",
                                &mut config.tint,
                                "Tint multiplied over the capture. It also attenuates recursion: nested portal captures multiply tint repeatedly so facing portals can fade into a tunnel instead of staying full-bright.",
                            );
                        });
                });

                    ui.collapsing("Debug", |ui| {
                        egui::Grid::new("portal_view_cones_debug_grid")
                            .num_columns(3)
                            .spacing([14.0, 4.0])
                            .min_col_width(240.0)
                            .show(ui, |ui| {
                                string_row(
                                    ui,
                                    "debug_dump_portal",
                                    &mut config.debug_dump_portal,
                                    "portal name",
                                    "Optional F8/debug-dump filter. Empty prints every portal. Enter c136, c137, or another portal name to print only that portal and its partner.",
                                );
                                bool_row(
                                    ui,
                                    "debug_outline",
                                    &mut config.debug_outline,
                                    "Draw gizmo outlines of each portal's exit sample zone and the entry window.",
                                );
                                bool_row(
                                    ui,
                                    "debug_los_rays",
                                    &mut config.debug_los_rays,
                                    "Draw the same candidate LOS rays used to decide whether the viewer can see into the portal. Reaching rays draw brightly; blocked rays truncate at the blocker.",
                                );
                            });
                        if ui
                            .button("dump_portal_view_cone_state")
                            .on_hover_text("Request one portal view-cone debug dump. Set debug_dump_portal to c136, c137, or another portal name to print only that portal pair; leave it empty to print every portal. The dump includes render.source_clipped_by_plan, render.source_clip_loss_fraction, render.entry_poly_world, and render.mapped_source_vertices.")
                            .clicked()
                        {
                            request_dump = true;
                        }
                    });
                    config.clone()
                };

                let viewer = world.get_resource::<PortalViewer>().cloned();
                let frame = world.get_resource::<PortalWorldFrame>().copied();
                let host_view = world.get_resource::<PortalCameraContinuityHostView>().cloned();
                let portals: Vec<PlacedPortal> =
                    world.query::<&PlacedPortal>().iter(world).copied().collect();
                if let Some(frame) = frame {
                    ui.collapsing("Debug Selected Pair", |ui| {
                        egui::Grid::new("portal_view_cones_selected_pair_debug_grid")
                            .num_columns(3)
                            .spacing([14.0, 4.0])
                            .min_col_width(300.0)
                            .show(ui, |ui| {
                                for row in selected_portal_view_cone_debug_rows(
                                    &debug_config,
                                    viewer.as_ref(),
                                    &frame,
                                    host_view.as_ref(),
                                    &portals,
                                ) {
                                    read_only_text_row(
                                        ui, &row.label, &row.value, row.units, row.help,
                                    );
                                }
                            });
                    });
                }

                if request_dump {
                    if let Some(mut request) = world.get_resource_mut::<PortalViewConeDebugDumpRequest>() {
                        request.request("F3 portal inspector");
                    }
                }
            });
    }

    #[cfg(not(feature = "portal_render"))]
    fn view_cones_section(_world: &mut World, _ui: &mut egui::Ui) {}

    fn missing_resource(ui: &mut egui::Ui, name: &str) {
        ui.label(format!("{name} is not present in this app state."));
    }

    fn field_label(ui: &mut egui::Ui, label: &'static str, help: &'static str) {
        field_label_text(ui, label, help);
    }

    fn field_label_text(ui: &mut egui::Ui, label: &str, help: &str) {
        ui.horizontal(|ui| {
            ui.label(label).on_hover_text(help);
            ui.label(egui::RichText::new("?").small())
                .on_hover_text(help);
        });
    }

    fn f32_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut f32,
        speed: f64,
        units: &'static str,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        ui.add(egui::DragValue::new(value).speed(speed))
            .on_hover_text(help);
        ui.label(units).on_hover_text(help);
        ui.end_row();
    }

    fn read_only_text_row(
        ui: &mut egui::Ui,
        label: &str,
        value: &str,
        units: &'static str,
        help: &'static str,
    ) {
        field_label_text(ui, label, help);
        ui.add_enabled(false, egui::Label::new(value))
            .on_hover_text(help);
        ui.label(units).on_hover_text(help);
        ui.end_row();
    }

    fn u32_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut u32,
        units: &'static str,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        ui.add(egui::DragValue::new(value).speed(1.0))
            .on_hover_text(help);
        ui.label(units).on_hover_text(help);
        ui.end_row();
    }

    fn bool_row(ui: &mut egui::Ui, label: &'static str, value: &mut bool, help: &'static str) {
        field_label(ui, label, help);
        ui.checkbox(value, "").on_hover_text(help);
        ui.label("");
        ui.end_row();
    }

    fn read_only_bool_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: bool,
        units: &'static str,
        help: &'static str,
    ) {
        let mut shown = value;
        field_label(ui, label, help);
        ui.add_enabled(false, egui::Checkbox::new(&mut shown, ""))
            .on_hover_text(help);
        ui.label(units).on_hover_text(help);
        ui.end_row();
    }

    fn string_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut String,
        units: &'static str,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        ui.add(egui::TextEdit::singleline(value).desired_width(180.0))
            .on_hover_text(help);
        ui.label(units).on_hover_text(help);
        ui.end_row();
    }

    fn vec2_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut Vec2,
        speed: f64,
        units: &'static str,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        ui.horizontal(|ui| {
            ui.label("x").on_hover_text(help);
            ui.add(egui::DragValue::new(&mut value.x).speed(speed))
                .on_hover_text(help);
            ui.label("y").on_hover_text(help);
            ui.add(egui::DragValue::new(&mut value.y).speed(speed))
                .on_hover_text(help);
        });
        ui.label(units).on_hover_text(help);
        ui.end_row();
    }

    fn color_row(ui: &mut egui::Ui, label: &'static str, value: &mut Color, help: &'static str) {
        let mut srgba = value.to_srgba();
        let mut changed = false;
        field_label(ui, label, help);
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    egui::DragValue::new(&mut srgba.red)
                        .speed(0.01)
                        .prefix("r "),
                )
                .on_hover_text(help)
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut srgba.green)
                        .speed(0.01)
                        .prefix("g "),
                )
                .on_hover_text(help)
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut srgba.blue)
                        .speed(0.01)
                        .prefix("b "),
                )
                .on_hover_text(help)
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut srgba.alpha)
                        .speed(0.01)
                        .prefix("a "),
                )
                .on_hover_text(help)
                .changed();
        });
        ui.label("sRGBA").on_hover_text(help);
        ui.end_row();
        if changed {
            *value = Color::srgba(
                srgba.red.clamp(0.0, 1.0),
                srgba.green.clamp(0.0, 1.0),
                srgba.blue.clamp(0.0, 1.0),
                srgba.alpha.clamp(0.0, 1.0),
            );
        }
    }

    fn convention_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut PortalConvention,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        egui::ComboBox::from_id_salt("portal_convention_combo")
            .selected_text(match *value {
                PortalConvention::Reflection => "Reflection",
                PortalConvention::Rotation => "Rotation",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(value, PortalConvention::Reflection, "Reflection")
                    .on_hover_text(
                        "Historical det -1 portal map: tangents are preserved and normals flip.",
                    );
                ui.selectable_value(value, PortalConvention::Rotation, "Rotation")
                    .on_hover_text(
                        "Proper det +1 portal map: the entry-facing chart rotates into the exit.",
                    );
            })
            .response
            .on_hover_text(help);
        ui.label("");
        ui.end_row();
    }

    #[cfg(feature = "portal_render")]
    fn effect_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut PortalVisualEffect,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        egui::ComboBox::from_id_salt("portal_effect_combo")
            .selected_text((*value).label())
            .show_ui(ui, |ui| {
                for effect in PortalVisualEffect::compiled() {
                    ui.selectable_value(value, *effect, (*effect).label())
                        .on_hover_text(help);
                }
            })
            .response
            .on_hover_text(help);
        ui.label("");
        ui.end_row();
    }

    #[cfg(feature = "portal_render")]
    fn camera_mode_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut PortalCameraTransitMode,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        egui::ComboBox::from_id_salt("portal_camera_mode_combo")
            .selected_text((*value).label())
            .show_ui(ui, |ui| {
                for mode in PortalCameraTransitMode::ALL {
                    ui.selectable_value(value, *mode, (*mode).label())
                        .on_hover_text(match *mode {
                            PortalCameraTransitMode::Pop => {
                                "The host camera behaves normally. If its focus teleports, the camera pops, snaps, or lerps exactly as the host camera system normally would."
                            }
                            PortalCameraTransitMode::Continuous => {
                                "Map the previous visible camera center through the same portal body map that moved the focus, preserving the screen-space offset while the focus remains in the aperture."
                            }
                        });
                }
            })
            .response
            .on_hover_text(help);
        ui.label("");
        ui.end_row();
    }

    #[cfg(feature = "portal_render")]
    fn view_cone_mode_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut PortalViewConeMode,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        egui::ComboBox::from_id_salt("portal_view_cone_mode_combo")
            .selected_text((*value).label())
            .show_ui(ui, |ui| {
                for mode in PortalViewConeMode::ALL {
                    ui.selectable_value(value, mode, mode.label())
                        .on_hover_text(match mode {
                            PortalViewConeMode::Off => {
                                "No portal view window is drawn or captured. Portal rims and body pieces still render."
                            }
                            PortalViewConeMode::Static => {
                                "Always draw the authored static view_cone. Viewer LOS is not required."
                            }
                            PortalViewConeMode::Dynamic => {
                                "Draw a viewer-dependent window when dynamic visibility admits the portal."
                            }
                        });
                }
            })
            .response
            .on_hover_text(help);
        ui.label("");
        ui.end_row();
    }

    #[cfg(feature = "portal_render")]
    fn view_cone_visibility_mode_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut PortalViewConeVisibilityMode,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        egui::ComboBox::from_id_salt("portal_view_cone_visibility_mode_combo")
            .selected_text((*value).label())
            .show_ui(ui, |ui| {
                for mode in PortalViewConeVisibilityMode::ALL {
                    ui.selectable_value(value, mode, mode.label())
                        .on_hover_text(match mode {
                            PortalViewConeVisibilityMode::FaceLosOnly => {
                                "Only direct LOS from the viewer to this portal face can admit and shape the cone. Use this for strict wall visibility diagnostics."
                            }
                            PortalViewConeVisibilityMode::FaceLosWithContinuity => {
                                "Direct face LOS admits the cone; this portal's doorway-continuity route may also admit while crossing; partner-side through-portal routes require face LOS. This is the continuity default."
                            }
                            PortalViewConeVisibilityMode::AnyPortalRoute => {
                                "Direct face, through-portal, or exit-side routes may independently admit the cone. Use this for magical/recursive portal visibility experiments."
                            }
                        });
                }
            })
            .response
            .on_hover_text(help);
        ui.label("");
        ui.end_row();
    }

    #[cfg(feature = "portal_render")]
    fn aperture_los_quality_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut PortalApertureLosQuality,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        egui::ComboBox::from_id_salt("portal_aperture_los_quality_combo")
            .selected_text(match *value {
                PortalApertureLosQuality::Low => "Low",
                PortalApertureLosQuality::Medium => "Medium",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(value, PortalApertureLosQuality::Low, "Low")
                    .on_hover_text("One line-of-sight ray per viewer corner, aimed at the lifted aperture center.");
                ui.selectable_value(value, PortalApertureLosQuality::Medium, "Medium")
                    .on_hover_text("Three line-of-sight rays per viewer corner: left endpoint, center, and right endpoint.");
            })
            .response
            .on_hover_text(help);
        ui.label("");
        ui.end_row();
    }

    #[cfg(feature = "portal_render")]
    fn source_clip_policy_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut PortalViewConeSourceClipPolicy,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        egui::ComboBox::from_id_salt("portal_source_clip_policy_combo")
            .selected_text((*value).label())
            .show_ui(ui, |ui| {
                for policy in PortalViewConeSourceClipPolicy::ALL {
                    ui.selectable_value(value, policy, policy.label())
                        .on_hover_text(match policy {
                            PortalViewConeSourceClipPolicy::AllowClip => {
                                "Diagnostic escape hatch: build from the planned entry quad even when it extends outside the active frame."
                            }
                            PortalViewConeSourceClipPolicy::ClampToFrame => {
                                "Default: clip the final entry polygon to the active frame, then derive mesh UVs and camera source rect from the same final source."
                            }
                            PortalViewConeSourceClipPolicy::FitToFrame => {
                                "Explicit fitting label for future aspect-preserving behavior; currently uses the coherent clamp-to-frame source path."
                            }
                        });
                }
            })
            .response
            .on_hover_text(help);
        ui.label("");
        ui.end_row();
    }

    #[cfg(feature = "portal_render")]
    fn capture_camera_mode_row(
        ui: &mut egui::Ui,
        label: &'static str,
        value: &mut PortalCaptureCameraMode,
        help: &'static str,
    ) {
        field_label(ui, label, help);
        egui::ComboBox::from_id_salt("portal_capture_camera_mode_combo")
            .selected_text((*value).label())
            .show_ui(ui, |ui| {
                for mode in PortalCaptureCameraMode::ALL {
                    ui.selectable_value(value, mode, mode.label())
                        .on_hover_text(match mode {
                            PortalCaptureCameraMode::ConeRect => {
                                "Frame the exact cone source rect computed from viewer visibility."
                            }
                            PortalCaptureCameraMode::MappedCameraSnapshot => {
                                "Frame the destination-side camera snapshot by mapping the host view through the portal pair."
                            }
                        });
                }
            })
            .response
            .on_hover_text(help);
        ui.label("");
        ui.end_row();
    }
}

#[cfg(not(all(feature = "dev_tools", feature = "portal")))]
mod disabled {
    use bevy::prelude::*;

    pub struct PortalInspectorPlugin;

    impl Plugin for PortalInspectorPlugin {
        fn build(&self, _app: &mut App) {}
    }
}

#[cfg(not(all(feature = "dev_tools", feature = "portal")))]
pub use disabled::PortalInspectorPlugin;
#[cfg(all(feature = "dev_tools", feature = "portal"))]
pub use enabled::PortalInspectorPlugin;
