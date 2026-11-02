"""Show owner commands without starting services or changing authored data."""
from __future__ import annotations

import html
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'scripts'))
from lib.tool_catalog import commands


def write_tools(repo: Path, out: Path) -> None:
    escape = html.escape
    cards = []
    for row in commands(repo):
        docs = os.path.relpath(repo / row['docs'], out)
        link = f'<a href="{escape(row["url"], quote=True)}">Open tool</a> · ' if row.get('url') else ''
        details = ''.join(f'<dt>{escape(label)}</dt><dd>{escape(row[key])}</dd>' for label, key in (
            ('Owner', 'owner'), ('Availability', 'availability'), ('Inputs', 'inputs'),
            ('Outputs', 'outputs'), ('Changes', 'mutation'), ('Build and reuse', 'build')))
        cards.append(f'<section><h2>{escape(row["title"])}</h2>'
                     f'<p>{link}<a href="{escape(docs, quote=True)}">Owner guide</a></p>'
                     f'<p>Command</p><pre>{escape(row["command"])}</pre>'
                     f'<p>Help</p><pre>{escape(row["help"])}</pre><dl>{details}</dl></section>')
    (out / 'tools.html').write_text(
        '<!doctype html><html lang="en"><meta charset="utf-8">'
        '<meta name="viewport" content="width=device-width, initial-scale=1">'
        '<title>Ambition Editor · Tools</title><link rel="stylesheet" href="app.css">'
        '<header class="top"><a class="brand" href="index.html#overview">Ambition Editor</a>'
        '<nav class="tabs"><a href="index.html#overview">Assets and reviews</a></nav></header>'
        '<main><h1>Authoring and inspection tools</h1>'
        '<p>Run commands from the repository root. Start a service before opening its link. '
        'ECS pages need an existing browser build. Source presence does not prove runtime readiness.</p>'
        + ''.join(cards) + '</main></html>', encoding='utf-8')
