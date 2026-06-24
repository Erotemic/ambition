#!/usr/bin/env python3
from __future__ import annotations
import argparse, json, importlib.util
from pathlib import Path
from typing import List, Tuple
import numpy as np
from PIL import Image, ImageDraw, ImageFont

Rect = Tuple[int, int, int, int]


def font(size: int):
    for p in [
        '/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf',
        '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',
    ]:
        try:
            return ImageFont.truetype(p, size)
        except Exception:
            pass
    return ImageFont.load_default()


def import_and_render(script_path: Path) -> Image.Image:
    spec = importlib.util.spec_from_file_location('pca_v14_sheet', str(script_path))
    if spec is None or spec.loader is None:
        raise RuntimeError(f'Could not import {script_path}')
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)  # type: ignore[union-attr]
    out = Path(mod.make_sheet())
    return Image.open(out).convert('RGBA')


def border_bg(arr: np.ndarray) -> np.ndarray:
    rgb = arr[:, :, :3].astype(np.float32)
    strips = [rgb[:4, :, :], rgb[-4:, :, :], rgb[:, :4, :], rgb[:, -4:, :]]
    return np.median(np.concatenate([s.reshape(-1, 3) for s in strips], axis=0), axis=0)


def foreground_mask(im: Image.Image, threshold: float = 33.0) -> np.ndarray:
    arr = np.array(im.convert('RGBA'))
    bg = border_bg(arr)
    rgb = arr[:, :, :3].astype(np.float32)
    dist = np.sqrt(((rgb - bg[None, None, :]) ** 2).sum(axis=2))
    return (dist > threshold) & (arr[:, :, 3] > 8)


def local_valid_mask(size: Tuple[int, int], roi: Rect, exclude: List[Rect]) -> np.ndarray:
    w, h = size
    valid = np.ones((h, w), dtype=bool)
    rx0, ry0, _, _ = roi
    for ex in exclude:
        x0, y0, x1, y1 = ex
        lx0, ly0 = max(0, x0 - rx0), max(0, y0 - ry0)
        lx1, ly1 = min(w, x1 - rx0), min(h, y1 - ry0)
        if lx1 > lx0 and ly1 > ly0:
            valid[ly0:ly1, lx0:lx1] = False
    return valid


def diff_metrics(tc: Image.Image, cc: Image.Image, valid: np.ndarray):
    ta = np.array(tc.convert('RGBA'))
    ca = np.array(cc.convert('RGBA'))
    tm = foreground_mask(tc) & valid
    cm = foreground_mask(cc) & valid
    union = tm | cm
    inter = tm & cm
    if union.sum():
        mad = float(np.abs(ta[:, :, :3].astype(np.int16) - ca[:, :, :3].astype(np.int16))[union].mean())
        iou = float(inter.sum() / union.sum())
    else:
        mad = 0.0
        iou = 1.0
    return {
        'mean_abs_rgb_union': mad,
        'coverage_iou': iou,
        'target_only_pixels': int((tm & ~cm).sum()),
        'candidate_only_pixels': int((cm & ~tm).sum()),
        'overlap_pixels': int(inter.sum()),
        'union_pixels': int(union.sum()),
    }, tm, cm


def overlay(tc: Image.Image, tm: np.ndarray, cm: np.ndarray, valid: np.ndarray) -> Image.Image:
    inter = tm & cm
    t_only = tm & ~cm
    c_only = cm & ~tm
    out = Image.new('RGBA', tc.size, (18, 20, 22, 255))
    under = Image.blend(tc.convert('RGBA'), out, 0.72)
    out.alpha_composite(under)
    arr = np.array(out)
    arr[t_only] = np.array([255, 0, 255, 235], dtype=np.uint8)
    arr[c_only] = np.array([0, 235, 255, 235], dtype=np.uint8)
    arr[inter] = np.array([245, 245, 245, 245], dtype=np.uint8)
    arr[~valid] = np.array([80, 60, 0, 230], dtype=np.uint8)
    return Image.fromarray(arr, 'RGBA')


def make_diagnostics(target: Image.Image, candidate: Image.Image, specs: dict, out_path: Path):
    report = {'rois': {}}
    panels = []
    for name, meta in specs.items():
        roi = tuple(meta['roi'])
        exclude = [tuple(x) for x in meta.get('exclude', [])]
        tc = target.crop(roi).convert('RGBA')
        cc = candidate.crop(roi).convert('RGBA')
        valid = local_valid_mask(tc.size, roi, exclude)
        m, tm, cm = diff_metrics(tc, cc, valid)
        ov = overlay(tc, tm, cm, valid)
        report['rois'][name] = {'roi': list(roi), 'exclude': [list(x) for x in exclude], 'diff': m}

        pad = 8
        colw = max(tc.width, cc.width, ov.width)
        h = tc.height + 52
        panel = Image.new('RGBA', (colw * 3 + pad * 4, h), (10, 12, 13, 255))
        d = ImageDraw.Draw(panel)
        d.text((pad, 4), name, fill=(255, 255, 255, 255), font=font(13))
        d.text((pad, 22), f"iou={m['coverage_iou']:.3f}  diff={m['mean_abs_rgb_union']:.1f}  t-only={m['target_only_pixels']}  c-only={m['candidate_only_pixels']}", fill=(210, 210, 210, 255), font=font(11))
        for idx, (title, im) in enumerate([('target', tc), ('candidate', cc), ('overlay', ov)]):
            x = pad + idx * (colw + pad)
            d.text((x, 38), title, fill=(180, 180, 180, 255), font=font(11))
            panel.alpha_composite(im, (x, 52))
            if exclude and idx < 2:
                dd = ImageDraw.Draw(panel, 'RGBA')
                rx0, ry0, _, _ = roi
                for ex in exclude:
                    x0, y0, x1, y1 = ex
                    dd.rectangle((x + x0 - rx0, 52 + y0 - ry0, x + x1 - rx0, 52 + y1 - ry0), fill=(255, 180, 0, 55), outline=(255, 210, 0, 220), width=2)
        panels.append(panel)

    width = max(p.width for p in panels)
    height = 46 + sum(p.height + 10 for p in panels)
    sheet = Image.new('RGBA', (width, height), (7, 8, 9, 255))
    d = ImageDraw.Draw(sheet)
    d.text((10, 8), 'PCA v14 polygon-pass masked ROI diagnostics', fill=(255, 255, 255, 255), font=font(18))
    d.text((10, 31), 'magenta=target-only, cyan=candidate-only, white=overlap, yellow=excluded mask', fill=(230, 230, 230, 255), font=font(12))
    y = 46
    for p in panels:
        sheet.alpha_composite(p, (0, y))
        y += p.height + 10
    sheet.save(out_path)
    return report


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--target', type=Path, default=Path('/mnt/data/perfect_cellular_automaton_target_reference.png'))
    ap.add_argument('--candidate', type=Path, default=Path('/mnt/data/perfect_cellular_automaton_pil_concept_sheet_v14.png'))
    ap.add_argument('--candidate-script', type=Path, default=None)
    ap.add_argument('--roi-specs', type=Path, default=Path('/mnt/data/pca_roi_specs_v14.json'))
    ap.add_argument('--out-json', type=Path, default=Path('/mnt/data/perfect_cellular_automaton_fit_report_v14.json'))
    ap.add_argument('--out-image', type=Path, default=Path('/mnt/data/perfect_cellular_automaton_fit_diagnostics_v14.png'))
    args = ap.parse_args()

    target = Image.open(args.target).convert('RGBA')
    candidate = import_and_render(args.candidate_script) if args.candidate_script else Image.open(args.candidate).convert('RGBA')
    if candidate.size != target.size:
        candidate = candidate.resize(target.size, Image.Resampling.LANCZOS)

    specs = json.loads(args.roi_specs.read_text())['rois']
    report = {
        'target': str(args.target),
        'candidate': str(args.candidate),
        'roi_specs': str(args.roi_specs),
        'notes': [
            'Metrics are computed only inside fixed ROI windows and outside any exclude sub-rectangles.',
            'mean_abs_rgb_union is lower-is-better; coverage_iou is higher-is-better.',
            'This v14 pass prioritizes corrected alignment windows and polygon edits rather than optimizer-only nudges.'
        ],
    }
    diag = make_diagnostics(target, candidate, specs, args.out_image)
    report.update(diag)
    args.out_json.write_text(json.dumps(report, indent=2) + '\n')

    ranked = sorted(report['rois'].items(), key=lambda kv: kv[1]['diff']['coverage_iou'])
    print(args.out_json)
    print(args.out_image)
    print('lowest IoU ROIs:')
    for name, item in ranked[:5]:
        d = item['diff']
        print(f"  {name:12s} iou={d['coverage_iou']:.3f} diff={d['mean_abs_rgb_union']:.1f} t-only={d['target_only_pixels']} c-only={d['candidate_only_pixels']}")


if __name__ == '__main__':
    main()
