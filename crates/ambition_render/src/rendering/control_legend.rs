//! A sign that names controls.
//!
//! Authored sign text can name a control by the action it fires:
//! `{action:jump}`. The sign shows the key or button the local primary seat
//! presses for that action, read from the one control prompt
//! (`ambition_sim_view::ControlPrompt::legend_pieces`). So the sign follows a
//! rebind, a pad, and the body the player drives, and it cannot name a key
//! that does nothing.
//!
//! ⭐ A CONTROL IS DRAWN AS ITS PICTURE when the seat's device has a published
//! one (`PromptEntry::glyph`, `sprites/input_glyphs/`), and as its name when it
//! has not (an `F13`, a headless app). A `Text2d` holds no image, so such a
//! sign is written as spans: each control with a picture is a placeholder span
//! of its own name, padded, and a sprite is placed on that span's laid-out box
//! ([`place_legend_glyphs`]). The name shows until the picture has loaded.
//! A view's mirrored copy of the sign keeps the names.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::{ComputedTextBlock, TextLayoutInfo};

use ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog;
use ambition_sim_view::{ControlPrompt, LegendPiece, LEGEND_ACTION_OPEN};

use super::label_layout::MirroredWorldLabel;

/// The authored text of a sign that names controls by action.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ControlLegend {
    template: String,
}

impl ControlLegend {
    /// The legend for authored sign text, if the text names a control.
    pub fn for_text(text: &str) -> Option<Self> {
        text.contains(LEGEND_ACTION_OPEN).then(|| Self {
            template: text.to_owned(),
        })
    }

    /// The text before any prompt exists: every control is `?`.
    pub fn unresolved_text(&self) -> String {
        ControlPrompt::default().fill_legend(&self.template)
    }
}

/// One span of a sign drawn in pieces. `glyph` is set on a control's
/// placeholder: the picture [`place_legend_glyphs`] draws over it.
#[derive(Component, Clone, Debug)]
pub struct LegendSpan {
    glyph: Option<Entity>,
}

/// The picture of one control on a sign, placed over its span.
#[derive(Component, Clone, Debug)]
pub struct LegendGlyph;

/// What a sign drawn in pieces shows now: each piece, and the picture drawn
/// for it. The prompt is rewritten often with the same answer, and a rebuilt
/// sign has no layout until the next frame, so a sign is rebuilt only when
/// this differs.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct LegendDrawn(Vec<(LegendPiece, Option<String>)>);

/// Write each legend sign, and each view's copy of it, from the prompt.
///
/// It writes when the prompt changes and when a legend or a copy appears. A
/// copy is spawned with its root's text, but the root's text changes later.
/// A copy always gets the names, as text.
#[allow(clippy::type_complexity)]
pub fn write_control_legends(
    mut commands: Commands,
    prompt: Option<Res<ControlPrompt>>,
    // Which pictures were published, and the loader. Absent in a headless
    // app, which draws every control by name.
    pictures: (Option<Res<Platformer2dAssetCatalog>>, Option<Res<AssetServer>>),
    mut legends: Query<(
        Entity,
        Ref<ControlLegend>,
        &mut Text2d,
        &TextFont,
        Option<&Children>,
        Option<&LegendDrawn>,
    )>,
    parts: Query<(), Or<(With<LegendSpan>, With<LegendGlyph>)>>,
    mut copies: Query<(Ref<MirroredWorldLabel>, &mut Text2d), Without<ControlLegend>>,
) {
    let Some(prompt) = prompt else { return };
    let (catalog, asset_server) = pictures;
    let new_copy = copies.iter().any(|(copy, _)| copy.is_added());
    let mut written: Vec<(Entity, String)> = Vec::new();
    for (entity, legend, mut text, font, children, shown) in &mut legends {
        if !(prompt.is_changed() || legend.is_added() || new_copy) {
            continue;
        }
        let pieces = prompt.legend_pieces(&legend.template);
        let wanted = prompt.fill_legend(&legend.template);
        // A picture is drawn only when it was published here.
        let picture = |glyph: &Option<String>| -> Option<String> {
            catalog.as_deref()?.try_image_path_for_load(glyph.as_deref()?)
        };
        let wanted_pieces = LegendDrawn(
            pieces
                .into_iter()
                .map(|piece| {
                    let path = match &piece {
                        LegendPiece::Control { glyph, .. } => picture(glyph),
                        LegendPiece::Text(_) => None,
                    };
                    (piece, path)
                })
                .collect(),
        );
        written.push((entity, wanted.clone()));
        if shown == Some(&wanted_pieces) {
            continue;
        }
        let drawn: Vec<(LegendPiece, Option<Handle<Image>>)> = wanted_pieces
            .0
            .iter()
            .map(|(piece, path)| {
                let handle = path
                    .as_ref()
                    .and_then(|path| Some(asset_server.as_deref()?.load(path.clone())));
                (piece.clone(), handle)
            })
            .collect();
        commands.entity(entity).insert(wanted_pieces);
        for child in children.into_iter().flatten() {
            if parts.contains(*child) {
                commands.entity(*child).despawn();
            }
        }
        if drawn.iter().all(|(_, handle)| handle.is_none()) {
            if text.0 != wanted {
                text.0 = wanted.clone();
            }
        } else {
            text.0.clear();
            for (piece, handle) in drawn {
                let span = match piece {
                    LegendPiece::Text(words) => words,
                    LegendPiece::Control { label, .. } if handle.is_some() => format!(" {label} "),
                    LegendPiece::Control { label, .. } => label,
                };
                let glyph = handle.map(|image| {
                    commands
                        .spawn((
                            Sprite {
                                image,
                                color: Color::NONE,
                                ..default()
                            },
                            Transform::default(),
                            LegendGlyph,
                            ChildOf(entity),
                        ))
                        .id()
                });
                commands.spawn((
                    TextSpan::new(span),
                    font.clone(),
                    TextColor(Color::NONE),
                    LegendSpan { glyph },
                    ChildOf(entity),
                ));
            }
        }
    }
    if written.is_empty() {
        return;
    }
    for (copy, mut text) in &mut copies {
        if let Some((_, wanted)) = written.iter().find(|(root, _)| *root == copy.root) {
            if &text.0 != wanted {
                text.0 = wanted.clone();
            }
        }
    }
}

/// Draw each control's picture over its span, in the sign's colour.
///
/// After the placement pass, which is the only writer of the sign's colour
/// (its fade): every span and picture takes that colour, and every span the
/// sign's font, here. A picture is
/// placed on its span's run box from the sign's last layout, by the transform
/// Bevy draws the text with (`top_left + (x, -y) / scale_factor`), and its
/// height is the line's. Until its image has loaded, the span shows the name.
#[allow(clippy::type_complexity)]
pub fn place_legend_glyphs(
    // Absent in a composition that draws no images; then no sign has a
    // picture, and every name shows.
    images: Option<Res<Assets<Image>>>,
    signs: Query<
        (&TextLayoutInfo, &ComputedTextBlock, &Anchor, &TextColor, &TextFont, &Children),
        With<ControlLegend>,
    >,
    mut spans: Query<(&LegendSpan, &mut TextColor, &mut TextFont), Without<ControlLegend>>,
    mut glyphs: Query<(&mut Sprite, &mut Transform), With<LegendGlyph>>,
) {
    let Some(images) = images else { return };
    for (layout, block, anchor, color, font, children) in &signs {
        let top_left = (Anchor::TOP_LEFT.0 - anchor.as_vec()) * layout.size;
        let scale = layout.scale_factor.max(f32::EPSILON).recip();
        for child in children {
            let Ok((span, mut span_color, mut span_font)) = spans.get_mut(*child) else { continue };
            // A span keeps its own font, and the sign's arrives after the
            // span was written (`apply_world_label_fonts` writes labels only).
            if *span_font != *font {
                *span_font = font.clone();
            }
            let Some(glyph) = span.glyph else {
                span_color.set_if_neq(*color);
                continue;
            };
            let run = layout
                .run_geometry
                .iter()
                .find(|run| block.entities().get(run.section_index).is_some_and(|e| e.entity == *child));
            let Ok((mut sprite, mut transform)) = glyphs.get_mut(glyph) else { continue };
            let loaded = images.get(&sprite.image).map(|image| image.size_f32());
            let (Some(run), Some(image_size)) = (run, loaded) else {
                // Not laid out or not loaded yet: the name shows.
                span_color.set_if_neq(*color);
                sprite.color = Color::NONE;
                continue;
            };
            let center = run.bounds.center();
            let height = run.bounds.height() * scale;
            let width = (height * image_size.x / image_size.y.max(1.0)).min(run.bounds.width() * scale);
            transform.translation = (top_left + Vec2::new(center.x, -center.y) * scale).extend(0.01);
            sprite.custom_size = Some(Vec2::new(width, width * image_size.y / image_size.x.max(1.0)));
            sprite.color = Color::WHITE.with_alpha(color.0.alpha());
            span_color.set_if_neq(TextColor(Color::NONE));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sim_view::{ActionId, ControlContextKind, ControlSlot, PromptEntry};

    fn prompt(jump: &str) -> ControlPrompt {
        ControlPrompt {
            context: ControlContextKind::Gameplay,
            entries: vec![PromptEntry {
                action: ActionId::new("jump"),
                slot: ControlSlot::Jump,
                label: "Jump".to_owned(),
                visual: None,
                binding: Some(jump.to_owned()),
                glyph: None,
                ready: true,
            }],
            menu_confirm: None,
        }
    }

    /// The sign and each view's copy of it name the key the prompt binds, and
    /// both follow a rebind.
    #[test]
    fn a_legend_sign_and_its_copies_follow_the_prompt() {
        let mut app = App::new();
        app.add_systems(Update, write_control_legends);
        app.insert_resource(prompt("Z"));
        let legend = ControlLegend::for_text("{action:jump}: JUMP").expect("names a control");
        let sign = app
            .world_mut()
            .spawn((Text2d::new(legend.unresolved_text()), legend))
            .id();
        app.update();
        let copy = app
            .world_mut()
            .spawn((
                Text2d::new("?: JUMP"),
                MirroredWorldLabel { root: sign },
            ))
            .id();
        app.update();
        let text = |app: &App, entity: Entity| app.world().get::<Text2d>(entity).unwrap().0.clone();
        assert_eq!(text(&app, sign), "Z: JUMP");
        assert_eq!(text(&app, copy), "Z: JUMP", "a new copy did not get the sign's text");

        app.insert_resource(prompt("K"));
        app.update();
        assert_eq!(text(&app, sign), "K: JUMP", "the sign kept the old key after a rebind");
        assert_eq!(text(&app, copy), "K: JUMP", "the copy kept the old key after a rebind");
    }

    /// Plain sign text is not a legend.
    #[test]
    fn text_that_names_no_control_is_not_a_legend() {
        assert_eq!(ControlLegend::for_text("LOOP"), None);
    }
}
