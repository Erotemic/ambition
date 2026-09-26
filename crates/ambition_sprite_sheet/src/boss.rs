//! Boss spritesheet animation, parallel to `character_sprites` but with the
//! boss generator's own animation rows (rest / floor_slam / side_sweep /
//! spike_halo / dash_echo / hit / death) instead of the standard 8-row
//! `CharacterAnim` grid.
//!
//! Bosses do not walk, run, or jump like a platforming character. Reusing
//! `CharacterAnim` would force placeholder rows or wrong labels.

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::character::{build_atlas_layout, record_for_sheet_key, RenderBasis};
use crate::SheetRecord;
use ambition_persistence::settings::VisualQualityBudget;

/// Boss animation rows in the order the generator emits them.
///
/// Boss sheets keep this vocabulary instead of
/// [`CharacterAnim`](crate::character::CharacterAnim). Boss rows name attack
/// verbs (`floor_slam`, `side_sweep`, `spike_halo`, `dash_echo`) that are also
/// keys into hurtbox/hitbox metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BossAnim {
    Rest = 0,
    FloorSlam = 1,
    SideSweep = 2,
    SpikeHalo = 3,
    DashEcho = 4,
    Hit = 5,
    Death = 6,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnimRow {
    pub frame_count: usize,
    pub duration_secs: f32,
}

/// Frame layout for a boss sheet. Like `character_sprites::CharacterSheetSpec`,
/// but rows are sparse, so each boss generator can emit a different row subset
/// (for example, the mockingbird has no `FloorSlam`/`SideSweep`). Per-target
/// anchor and scale keep bosses at the right size relative to characters.
///
/// The spec is owned and serde round-trippable, so a provider can author its
/// layout as data. Provider composition lives above this crate; this module
/// supplies only the schema and built-in fallback sheets.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BossSheetSpec {
    pub label_width: u32,
    /// Per-frame size in source-image pixels after the union-bbox crop. After
    /// a regen, resync with the manifest's `frame_size` block.
    pub frame_width: u32,
    pub frame_height: u32,
    /// Animation rows in the order the generator emits them in the PNG.
    /// Sparse: a sheet may omit any row except `Rest` (the fallback).
    pub rows: Vec<(BossAnim, AnimRow)>,
    /// Multiplier on the collision box's max dimension that gives the rendered
    /// height. Width follows the frame's aspect ratio.
    pub collision_scale: f32,
    /// Sprite anchor on the y axis, tuned per sheet.
    pub feet_anchor_y: f32,
    /// Sample inset (pixels) on each URect, so bilinear filtering does not
    /// sample neighbouring frames.
    pub frame_sample_inset: u32,
    /// True for flying bosses whose body is centred in the collision box, not
    /// anchored to its bottom. Then `collision_anchor` uses `feet_anchor_y` as
    /// the body's normalized vertical offset in the sprite quad (Bevy +Y-up;
    /// 0 = sprite centre).
    pub body_centered: bool,
    /// True when the generator drew the neutral pose facing left. The renderer
    /// computes `flip_x = (facing < 0) XOR authored_faces_left`, so a
    /// left-drawn sheet faces the correct way.
    pub authored_faces_left: bool,
}

/// Parsed boss-sheet data used by provider catalog builders and tests.
///
/// The App-local `ambition_boss_encounter::BossCatalog` is the runtime
/// authority. This crate owns no process-global override.
#[derive(Clone, Debug, Default)]
pub struct BossSheetRegistry {
    by_key: std::collections::HashMap<String, BossSheetSpec>,
}

impl BossSheetRegistry {
    /// Parse a boss-sheet RON document (`HashMap<boss_key, BossSheetSpec>`).
    pub fn from_ron(ron: &str) -> Self {
        let by_key = ron::from_str(ron).unwrap_or_else(|err| {
            panic!("boss_sheets.ron failed to deserialize as HashMap<String, BossSheetSpec>: {err}")
        });
        Self { by_key }
    }

    pub fn get(&self, key: &str) -> Option<&BossSheetSpec> {
        self.by_key.get(key)
    }
}

// `feet_anchor_y` matches the body-metrics measurement. After a regen, resync
// with the manifest's `body_metrics.feet_anchor_norm.y`.
pub static BOSS_SHEET: std::sync::LazyLock<BossSheetSpec> =
    std::sync::LazyLock::new(|| BossSheetSpec {
        label_width: 100,
        frame_width: 128,
        frame_height: 128,
        rows: vec![
            (
                BossAnim::Rest,
                AnimRow {
                    frame_count: 8,
                    duration_secs: 0.120,
                },
            ),
            (
                BossAnim::FloorSlam,
                AnimRow {
                    frame_count: 7,
                    duration_secs: 0.082,
                },
            ),
            (
                BossAnim::SideSweep,
                AnimRow {
                    frame_count: 7,
                    duration_secs: 0.072,
                },
            ),
            (
                BossAnim::SpikeHalo,
                AnimRow {
                    frame_count: 8,
                    duration_secs: 0.092,
                },
            ),
            (
                BossAnim::DashEcho,
                AnimRow {
                    frame_count: 7,
                    duration_secs: 0.062,
                },
            ),
            (
                BossAnim::Hit,
                AnimRow {
                    frame_count: 5,
                    duration_secs: 0.090,
                },
            ),
            (
                BossAnim::Death,
                AnimRow {
                    frame_count: 8,
                    duration_secs: 0.110,
                },
            ),
        ],
        // A slightly smaller scale keeps bosses from overpowering the scene.
        collision_scale: 1.6,
        feet_anchor_y: -0.336,
        frame_sample_inset: 1,
        body_centered: false,
        authored_faces_left: false,
    });

impl BossSheetSpec {
    fn row_index(&self, anim: BossAnim) -> Option<usize> {
        self.rows.iter().position(|(row_anim, _)| *row_anim == anim)
    }

    /// Whether the sprite is flipped to face `facing`. This is the one source
    /// for the renderer's `Sprite::flip_x`: art faces +x, so a leftward
    /// `facing` flips it, unless `authored_faces_left` inverts the rule.
    pub fn flip_x(&self, facing: f32) -> bool {
        (facing < 0.0) ^ self.authored_faces_left
    }

    /// Resolve a requested animation against this sheet's rows. Falls back to
    /// `Rest` if the row is not shipped. A sheet without `Rest` would crash the
    /// indexer; if a future sheet omits `Rest`, make it fail with an error.
    pub fn resolve_anim(&self, anim: BossAnim) -> BossAnim {
        if self.row_index(anim).is_some() {
            anim
        } else {
            BossAnim::Rest
        }
    }

    /// The `Hit` row cell to draw for a hit reaction with `remaining_secs` of
    /// flash left, or `None` when this sheet ships no `Hit` row.
    ///
    /// Presentation only: the sim cursor never selects `Hit`, so boss geometry
    /// does not depend on flash length. The row plays forward and stops on its
    /// last frame.
    pub fn hit_reaction_frame(&self, remaining_secs: f32) -> Option<(BossAnim, usize)> {
        let (_, row) = self.rows.iter().find(|(anim, _)| *anim == BossAnim::Hit)?;
        if row.frame_count == 0 || remaining_secs <= 0.0 {
            return None;
        }
        let frames_left = if row.duration_secs > 0.0 {
            (remaining_secs / row.duration_secs).ceil() as usize
        } else {
            1
        };
        let frame = row.frame_count.saturating_sub(frames_left.max(1));
        Some((BossAnim::Hit, frame))
    }

    pub(crate) fn row(&self, anim: BossAnim) -> AnimRow {
        let resolved = self.resolve_anim(anim);
        let idx = self
            .row_index(resolved)
            .expect("boss sprite sheet must define a Rest row");
        self.rows[idx].1
    }

    /// Index into the backing [`SheetRecord`]'s rows for `anim` (after `Rest`
    /// fallback). The const lists rows in PNG order, so a row's position in
    /// `self.rows` is its record-row index.
    pub fn record_row(&self, anim: BossAnim) -> usize {
        let resolved = self.resolve_anim(anim);
        self.row_index(resolved)
            .expect("boss sprite sheet must define a Rest row")
    }

    /// A [`SheetRecord`] view of this const grid, for when no published sheet
    /// RON exists (subdir bosses, headless, tests). Rows carry no rects, so the
    /// shared frame algebra derives each cell from the grid stride.
    pub fn synth_record(&self, image: &str) -> SheetRecord {
        let rows = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, (anim, row))| crate::SheetRow {
                animation: format!("{anim:?}"),
                row_index: i as u32,
                frame_count: row.frame_count as u32,
                duration_ms: (row.duration_secs * 1000.0) as u32,
                duration_secs: row.duration_secs,
                page: 0,
                rects: Vec::new(),
            })
            .collect();
        SheetRecord {
            // A synthesized record has no key and no rig target.
            key: String::new(),
            target: String::new(),
            image: image.to_string(),
            images: Vec::new(),
            label_width: self.label_width,
            frame_width: self.frame_width,
            frame_height: self.frame_height,
            y_offset: 0,
            body_metrics: None,
            tuning: None,
            // Keep the drawn facing, as a published sheet RON would.
            authored_faces_left: self.authored_faces_left,
            rows,
        }
    }

    pub fn frame_count(&self, anim: BossAnim) -> usize {
        self.row(anim).frame_count
    }

    pub fn render_size(&self, collision: Vec2) -> Vec2 {
        // Height from collision; width keeps the frame's aspect ratio.
        let height = collision.x.max(collision.y).max(8.0) * self.collision_scale;
        let width = height * (self.frame_width as f32 / self.frame_height as f32);
        Vec2::new(width, height)
    }

    pub fn anchor(&self) -> Anchor {
        Anchor(Vec2::new(0.0, self.feet_anchor_y))
    }

    /// Anchor that places the boss's feet on the bottom of the collision box.
    /// See `character_sprites::feet_anchor_for` for the derivation.
    ///
    /// For `body_centered` sheets (flying bosses), skip the feet term and use
    /// `feet_anchor_y` as the body-centre offset, so the quad does not hang
    /// below the AABB.
    pub fn collision_anchor(&self, collision: Vec2) -> Anchor {
        if self.body_centered {
            return Anchor(Vec2::new(0.0, self.feet_anchor_y));
        }
        let render_height = collision.x.max(collision.y).max(8.0) * self.collision_scale;
        let half_collision_y = collision.y * 0.5;
        let ay = self.feet_anchor_y + half_collision_y / render_height;
        Anchor(Vec2::new(0.0, ay))
    }
}

/// One page image's texture + atlas layout for a boss sheet. Length-1 for the
/// common single-PNG boss; one per page when the sheet is split to stay within
/// the GPU texture limit. Mirrors `character_sprites::CharacterSpritePage`.
#[derive(Clone)]
pub struct BossSpritePage {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

#[derive(Clone)]
pub struct BossSpriteAsset {
    /// Per-page texture + layout. `pages[0]` is the primary image; the renderer
    /// swaps to the active frame's page for split sheets.
    pub pages: Vec<BossSpritePage>,
    /// The backing sheet record (published RON, or a grid-only view of the
    /// const). The shared frame algebra reads pages, flat indices, and trim
    /// from it.
    pub record: SheetRecord,
    pub spec: BossSheetSpec,
}

impl BossSpriteAsset {
    /// Primary (page-0) texture handle — the spawn-time sprite image.
    pub fn texture(&self) -> Handle<Image> {
        self.pages[0].texture.clone()
    }

    /// Primary (page-0) atlas layout handle.
    pub fn layout(&self) -> Handle<TextureAtlasLayout> {
        self.pages[0].layout.clone()
    }

    /// Page-local flat atlas index of `(anim, frame)` via the shared algebra.
    pub fn flat_index(&self, anim: BossAnim, frame: usize) -> usize {
        self.record
            .flat_index_in_page(self.spec.record_row(anim), frame)
    }
}

/// Build the boss sprite asset for the gradient sentinel sheet. Returns `None`
/// if the catalog disables the asset or the profile skips optional images.
/// Callers then use the static `EntitySprite::BossCore` image, or a coloured
/// rectangle.
pub fn load_boss_sprite_in(
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    label: &str,
    sheet: BossSheetSpec,
    quality: Option<&VisualQualityBudget>,
) -> Option<BossSpriteAsset> {
    load_named_boss_sprite_via_catalog(catalog, asset_server, layouts, label, sheet, quality)
}

/// Derive the published sheet's RON record key (its file root) from the resolved
/// PNG asset path, e.g. `sprites/flying_spaghetti_monster_boss_spritesheet.png`
/// → `flying_spaghetti_monster_boss`, or `sprites/gnu_ton_boss/...png` →
/// `gnu_ton_boss`. This is the key [`record_for_sheet_key`]
/// indexes baked sheets by.
pub fn boss_ron_target(path: &str) -> Option<&str> {
    let stem = path.rsplit('/').next()?.strip_suffix("_spritesheet.png")?;
    // GNU-ton's body and hands textures share one packed atlas layout. Both
    // filenames resolve to the `gnu_ton_boss` record, so both use the same flat
    // index and trim.
    Some(
        stem.strip_suffix("_body")
            .or_else(|| stem.strip_suffix("_hands"))
            .unwrap_or(stem),
    )
}

/// The baked record key for a resolved boss PNG path, carrying the quality
/// variant suffix when the path lives in a `sprites_<suffix>/` folder. A base
/// path yields `<stem>`; a variant path yields `<stem>.<suffix>` so
/// [`record_for_sheet_key`] resolves the matching scaled
/// record (whose rects address the resized PNG).
fn boss_record_key(path: &str) -> Option<String> {
    let stem = boss_ron_target(path)?;
    let suffix = path.split('/').find_map(|segment| match segment {
        "sprites_0_5x" => Some("0_5x"),
        "sprites_0_25x" => Some("0_25x"),
        "sprites_potato" => Some("potato"),
        _ => None,
    });
    Some(match suffix {
        Some(suffix) => format!("{stem}.{suffix}"),
        None => stem.to_string(),
    })
}

/// True when a published sheet record matches the const's row set, so it can
/// drive the pixels. The const owns the [`BossAnim`] row order and frame
/// counts, so each const row needs a matching record row with enough frames
/// and rects. Otherwise the boss renders from the const grid (see
/// [`BossSheetSpec::synth_record`]).
pub fn record_aligns_with_const(record: &SheetRecord, spec: &BossSheetSpec) -> bool {
    if record.rows.len() < spec.rows.len() {
        return false;
    }
    spec.rows.iter().enumerate().all(|(i, (_, row))| {
        let rec = &record.rows[i];
        (rec.frame_count as usize) >= row.frame_count && rec.rects.len() >= row.frame_count
    })
}

pub fn load_named_boss_sprite_via_catalog(
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    label: &str,
    spec: BossSheetSpec,
    quality: Option<&VisualQualityBudget>,
) -> Option<BossSpriteAsset> {
    let id = ambition_asset_manager::platformer_assets::ids::boss_sprite(label);
    // Use a scaled variant PNG only when its variant record was also baked and
    // matches the const, so the atlas rects address the resolution that loads.
    // Otherwise use the base PNG and record. Gameplay geometry reads the base
    // record. `boss_record_key` carries the scale suffix for a variant folder.
    let variant_path = quality
        .filter(|q| q.sprites.prefer_scaled_variants)
        .map(|q| q.sprites.resolution_scale)
        .filter(|scale| *scale != ambition_persistence::settings::TextureResolutionScale::Full)
        .and_then(|scale| {
            let variant_id = ambition_asset_manager::platformer_assets::scaled_asset_id(
                &id,
                scale.asset_id_suffix(),
            )?;
            let path = catalog.try_path_for_load(&variant_id)?;
            let record = record_for_sheet_key(boss_record_key(&path)?.as_str())?;
            record_aligns_with_const(record, &spec).then_some(path)
        });
    let Some(path) = variant_path.or_else(|| catalog.try_path_for_load(&id)) else {
        eprintln!(
            "[boss_sprites] {label} spritesheet missing under {} profile (id {id}) — falling back to entity sprite",
            catalog.profile().label(),
        );
        return None;
    };
    // Prefer the published sheet RON, so atlas, page splits, trim, and aspect
    // follow the texture. The const still owns the row mapping and tuning.
    // Without a matching baked record (subdir bosses, headless, tests), build a
    // grid-only record from the const. Both use the same frame algebra.
    let mut spec = spec;
    let record = boss_record_key(&path)
        .as_deref()
        .and_then(record_for_sheet_key)
        .filter(|record| record_aligns_with_const(record, &spec))
        .map(|record| {
            spec.frame_width = record.frame_width;
            spec.frame_height = record.frame_height;
            spec.label_width = record.label_width;
            record.clone()
        })
        .unwrap_or_else(|| spec.synth_record(&boss_filename_of(&path)));

    let pages = build_boss_pages(&record, &spec, &path, asset_server, layouts);
    Some(BossSpriteAsset {
        pages,
        record,
        spec,
    })
}

/// Final path component of a resolved boss PNG path (the synthetic record's
/// `image`), e.g. `sprites/gnu_ton_boss/gnu_ton_boss_spritesheet.png` →
/// `gnu_ton_boss_spritesheet.png`.
fn boss_filename_of(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

/// Build one `(texture, layout)` per page image, mirroring the character
/// loader: page 0 uses the catalog-resolved path; sibling pages resolve their
/// filename (from the record's `images` list) against page 0's directory. Each
/// page's layout comes from the shared `atlas_page` algebra.
fn build_boss_pages(
    record: &SheetRecord,
    spec: &BossSheetSpec,
    path: &str,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> Vec<BossSpritePage> {
    let parent = path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let page_count = record.page_count().max(1);
    (0..page_count)
        .map(|page| {
            let page_path = if page == 0 {
                path.to_string()
            } else {
                let file = record.page_image(page);
                if parent.is_empty() {
                    file.to_string()
                } else {
                    format!("{parent}/{file}")
                }
            };
            BossSpritePage {
                texture: crate::game_assets::load_sheet_image(asset_server, "boss-sheet", page_path),
                layout: layouts.add(build_atlas_layout(
                    &record.atlas_page(page, spec.frame_sample_inset),
                )),
            }
        })
        .collect()
}

/// Render-side boss texture addresser. It does not own the animation cursor:
/// the sim owns it ([`BossAnimFrame`], advanced by `drive_boss_animators` and
/// published in `BossFrameIndex`). This type only turns a published
/// `(anim, frame)` into an atlas cell, page, and trimmed size. So the drawn
/// frame and the frame-tied strike geometry are always the same sim frame.
/// (`CharacterAnimator` advances locally because character hitboxes are not
/// frame-tied.) It holds only the GPU binding, set once at sprite upgrade.
#[derive(Component)]
pub struct BossAnimator {
    pub spec: BossSheetSpec,
    /// Backing sheet record, cloned from the asset.
    pub record: SheetRecord,
    /// Per-page texture and layout handles, so the renderer can swap them when
    /// the frame is on another page. Length 1 for a single-PNG boss.
    pub pages: Vec<BossSpritePage>,
    /// Base render size and feet anchor set at spawn, so a trimmed sheet can
    /// compute the per-frame `custom_size` and anchor. `None` until set.
    pub render_basis: Option<RenderBasis>,
}

impl BossAnimator {
    pub fn new(asset: &BossSpriteAsset) -> Self {
        Self {
            spec: asset.spec.clone(),
            record: asset.record.clone(),
            pages: asset.pages.clone(),
            render_basis: None,
        }
    }

    pub fn with_render_basis(mut self, render_size: Vec2, feet_anchor: Vec2) -> Self {
        self.render_basis = Some(RenderBasis {
            render_size,
            feet_anchor,
        });
        self
    }

    /// Page-local flat atlas index of the published `(anim, frame)`. A pure
    /// lookup; the sim owns the cursor.
    pub fn flat_index(&self, anim: BossAnim, frame: usize) -> usize {
        self.record
            .flat_index_in_page(self.spec.record_row(anim), frame)
    }

    /// True when the sheet is split across more than one page image, so the
    /// renderer must select the active frame's page. Single-page sheets skip it.
    pub fn is_paged(&self) -> bool {
        self.pages.len() > 1
    }

    /// The page image index `(anim, frame)` draws from (per-frame: a packed
    /// animation can span pages).
    pub fn page_of(&self, anim: BossAnim, frame: usize) -> u32 {
        self.record.frame_page_of(self.spec.record_row(anim), frame)
    }

    /// Per-frame `(custom_size, anchor)` for `(anim, frame)`, or `None` when the
    /// sheet is untrimmed (or no basis is set) — callers then keep the fixed
    /// spawn-time size/anchor, so untrimmed boss sheets are unaffected.
    pub fn render_of(&self, anim: BossAnim, frame: usize) -> Option<(Vec2, Vec2)> {
        if !self.record.is_trimmed() {
            return None;
        }
        let basis = self.render_basis.as_ref()?;
        let trim = self.record.frame_trim(self.spec.record_row(anim), frame);
        Some(crate::trimmed_render(
            &trim,
            basis.render_size,
            basis.feet_anchor,
        ))
    }
}

fn non_looping(anim: BossAnim) -> bool {
    matches!(
        anim,
        BossAnim::FloorSlam
            | BossAnim::SideSweep
            | BossAnim::SpikeHalo
            | BossAnim::DashEcho
            | BossAnim::Hit
            | BossAnim::Death
    )
}

/// Gameplay phase currently driving a boss animation row.
///
/// Separate from [`BossAnim`]: one row can serve both telegraph and strike, as
/// separate plays of the clip. Keeping the phase in the animator identity makes
/// frames, authored boxes, and debug overlays advance together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossAnimDrivePhase {
    Rest,
    Windup,
    Active,
    Death,
}

/// Sim-owned boss animation frame cursor.
///
/// Gameplay geometry reads the derived frame sample this cursor publishes;
/// render-side [`BossAnimator`] only carries texture handles and mirrors this
/// state to draw the same frame.
#[derive(Component, Clone, Debug)]
pub struct BossAnimFrame {
    pub spec: BossSheetSpec,
    pub current: BossAnim,
    pub drive_phase: BossAnimDrivePhase,
    pub frame: usize,
    pub elapsed: f32,
    pub clip_held: bool,
}

impl BossAnimFrame {
    pub fn new(spec: BossSheetSpec) -> Self {
        Self {
            spec,
            current: BossAnim::Rest,
            drive_phase: BossAnimDrivePhase::Rest,
            frame: 0,
            elapsed: 0.0,
            clip_held: false,
        }
    }

    pub fn request_for_phase(&mut self, anim: BossAnim, drive_phase: BossAnimDrivePhase) {
        if self.current == anim && self.drive_phase == drive_phase {
            return;
        }
        self.current = anim;
        self.drive_phase = drive_phase;
        self.frame = 0;
        self.elapsed = 0.0;
        self.clip_held = false;
    }

    pub fn reset(&mut self) {
        self.request_for_phase(BossAnim::Rest, BossAnimDrivePhase::Rest);
    }

    pub fn tick(&mut self, dt: f32) -> usize {
        let row = self.spec.row(self.current);
        if row.frame_count == 0 || row.duration_secs <= 0.0 {
            return self.frame;
        }
        if self.clip_held {
            return self.frame;
        }
        self.elapsed += dt;
        while self.elapsed >= row.duration_secs {
            self.elapsed -= row.duration_secs;
            if self.frame + 1 >= row.frame_count {
                if non_looping(self.current) {
                    self.frame = row.frame_count - 1;
                    self.clip_held = true;
                    break;
                } else {
                    self.frame = 0;
                }
            } else {
                self.frame += 1;
            }
        }
        self.frame
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BossAnimState {
    pub alive: bool,
    pub attack_active: bool,
    pub attack_windup: bool,
    /// Profile-resolved animation to play during windup, when the
    /// gameplay layer can map the boss's active profile onto this
    /// sheet's row vocabulary. `None` keeps the generic fallback.
    pub windup_anim: Option<BossAnim>,
    /// Profile-resolved animation to play during the strike.
    pub active_anim: Option<BossAnim>,
    pub pattern_timer: f32,
    /// The side the boss is DRAWN toward: -1.0 = left, +1.0 = right. Always
    /// +1.0 for a boss that declares no left/right variant (`Unmirrored`).
    pub facing: f32,
    pub pos: Vec2,
}

impl BossAnimState {
    pub fn drive_phase(self) -> BossAnimDrivePhase {
        if !self.alive {
            return BossAnimDrivePhase::Death;
        }
        if self.attack_windup {
            return BossAnimDrivePhase::Windup;
        }
        if self.attack_active {
            return BossAnimDrivePhase::Active;
        }
        BossAnimDrivePhase::Rest
    }
}

pub fn pick_boss_anim(state: BossAnimState) -> BossAnim {
    if !state.alive {
        return BossAnim::Death;
    }
    if state.attack_windup {
        return state.windup_anim.unwrap_or(BossAnim::SpikeHalo);
    }
    if state.attack_active {
        if let Some(anim) = state.active_anim {
            return anim;
        }
        // Rotate attack clips for variety; the AI uses one pattern today.
        let bucket = (state.pattern_timer.abs() as i32) % 3;
        return match bucket {
            0 => BossAnim::FloorSlam,
            1 => BossAnim::SideSweep,
            _ => BossAnim::DashEcho,
        };
    }
    BossAnim::Rest
}

#[cfg(test)]
mod hit_reaction_tests {
    use super::*;

    /// The hit reaction is drawn from the flash alone: the row plays forward
    /// as the flash runs out, and a sheet without a `Hit` row keeps the cursor.
    #[test]
    fn the_hit_row_plays_forward_as_the_flash_runs_out() {
        let sheet = &*BOSS_SHEET;
        let row = sheet.row(BossAnim::Hit);
        let full = row.duration_secs * row.frame_count as f32;
        assert_eq!(sheet.hit_reaction_frame(full), Some((BossAnim::Hit, 0)));
        assert_eq!(
            sheet.hit_reaction_frame(row.duration_secs * 0.5),
            Some((BossAnim::Hit, row.frame_count - 1)),
            "the last slice of the flash shows the last frame"
        );
        assert_eq!(sheet.hit_reaction_frame(0.0), None, "no flash, no overlay");

        let mut bare = sheet.clone();
        bare.rows.retain(|(anim, _)| *anim != BossAnim::Hit);
        assert_eq!(bare.hit_reaction_frame(full), None);
    }
}
