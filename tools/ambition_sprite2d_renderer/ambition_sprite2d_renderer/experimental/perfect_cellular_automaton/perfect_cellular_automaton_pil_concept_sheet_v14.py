#!/usr/bin/env python3
from __future__ import annotations
import json
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
W,H=1448,1086
DATA_PATH=Path(__file__).with_name('perfect_cellular_automaton_pose_polygons_v14.json')
OUT=Path(__file__).with_name('perfect_cellular_automaton_pil_concept_sheet_v14.png')
with DATA_PATH.open() as f: DATA=json.load(f)
PAL=DATA['palette']; ORDER=['black','dark_green','green','lime','purple','cream2','cream']
def font(size:int):
    for path in ['/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf','/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf','/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf']:
        try: return ImageFont.truetype(path,size)
        except Exception: pass
    return ImageFont.load_default()
def color(name): return tuple(PAL[name])
def draw_bg(d):
    for y in range(H):
        t=y/max(1,H-1); c0=PAL['bg0']; c1=PAL['bg1']; c=tuple(int(c0[i]*(1-t)+c1[i]*t) for i in range(3))+(255,)
        d.line((0,y,W,y),fill=c)
def rect(d,box,fill,outline=None,width=1):
    d.rectangle(box,fill=color(fill) if isinstance(fill,str) else fill,outline=color(outline) if isinstance(outline,str) else outline,width=width)
def label(d,text,xy,size=18,fill='text',anchor=None):
    d.text(xy,text,font=font(size),fill=color(fill) if isinstance(fill,str) else fill,anchor=anchor)
def draw_cells(d,cx,cy,scale=1.0,rows=5,cols=4,pattern=None):
    if pattern is None: pattern=[[1,1,1,1],[1,0,1,1],[1,0,0,1],[1,1,1,1],[1,1,0,1]]
    cell=14*scale; gap=5*scale; w=cols*cell+(cols-1)*gap; h=rows*cell+(rows-1)*gap; x0=cx-w/2; y0=cy-h/2
    for r in range(rows):
        for c in range(cols):
            x=x0+c*(cell+gap); y=y0+r*(cell+gap)
            rect(d,(x,y,x+cell,y+cell),'lime' if pattern[r][c] else 'dark_green')
def draw_layout(d):
    x0,y0=1080,64; items=[('BLACK','black'),('DARK GREEN','dark_green'),('GREEN','green'),('LIME','lime'),('PURPLE','purple'),('CREAM','cream'),('LIGHT CREAM','cream2')]
    for i,(txt,key) in enumerate(items):
        y=y0+i*45; rect(d,(x0,y,x0+48,y+33),key,outline=(245,245,245,255),width=2); label(d,txt,(x0+68,y+6),17)
    label(d,'AUTOMATON GRID',(1144,390),16); draw_cells(d,1148,469,2.1)
    for txt,x in [('IDLE',88),('WALK 1',288),('WALK 2',506),('ATTACK',708),('JUMP',909),('AIR',1109),('LAND',1298)]: label(d,txt,(x,642),17,anchor='mm')
    for i,txt in enumerate(['- ALL PARTS ARE SIMPLE POLYGONS','- EXPLICIT FRONT EYE FIX (V14)','- CLEAR SILHOUETTE AT SMALL SIZES']): label(d,txt,(22,996+i*28),17)
def draw_base_pose(d,name):
    pose=DATA['poses'][name]; x0,y0,_,_=pose['roi']
    for key in ORDER:
        for pts in pose['polygons'].get(key,[]):
            pts2=[(x0+p[0],y0+p[1]) for p in pts]
            if len(pts2)>=3: d.polygon(pts2,fill=color(key),outline=(0,0,0,255))
def draw_part_overlays(d,name):
    pose=DATA['poses'][name]; x0,y0,_,_=pose['roi']
    overlays=sorted(DATA.get('part_overlays',{}).get(name,[]), key=lambda x:x['order'])
    for part in overlays:
        bx0,by0,_,_=part['box']
        for key in ORDER:
            for pts in part['polygons'].get(key,[]):
                pts2=[(x0+bx0+p[0],y0+by0+p[1]) for p in pts]
                if len(pts2)>=3: d.polygon(pts2,fill=color(key),outline=None)
def draw_manual_eyes(d,name):
    pose=DATA['poses'][name]; x0,y0,_,_=pose['roi']
    for item in DATA.get('manual_eyes',{}).get(name,[]):
        pts=[(x0+p[0], y0+p[1]) for p in item['points']]
        d.polygon(pts,fill=color(item['color']),outline=None)
def draw_pose(d,name):
    draw_base_pose(d,name); draw_part_overlays(d,name); draw_manual_eyes(d,name)
def make_sheet():
    img=Image.new('RGBA',(W,H),tuple(PAL['bg0'])); d=ImageDraw.Draw(img,'RGBA'); draw_bg(d); draw_layout(d)
    for name in ['top_front','top_side','top_back','pose_idle','pose_walk_1','pose_walk_2','pose_attack','pose_jump','pose_air','pose_land']:
        draw_pose(d,name)
    img.save(OUT); return OUT
if __name__=='__main__': print(make_sheet())
