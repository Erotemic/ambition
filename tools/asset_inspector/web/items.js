/* The overview and the item kinds beside sprites: music, sound effects, fonts and worlds.
   Every item has one shape (`asset_index/kinds.py`), so one list and one detail page serve all. */
(function () {
  "use strict";
  const C = window.ART_CATALOG;
  const items = C.items || {};
  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"})[c]);
  const app = () => document.getElementById("app");
  const query = () => document.getElementById("q").value.trim().toLowerCase();
  const KINDS = {music: "Music", sfx: "Sound effects", font: "Fonts", world: "Worlds"};
  const VIEW = {music: "music", sfx: "sound", font: "fonts", world: "worlds"};
  const STATE_CLASS = {present: "c-ok", missing: "c-warn", stale: "c-warn", unregistered: "c-info", excluded: "c-info", unknown: "c-info"};
  const chip = state => `<span class="chip ${STATE_CLASS[state] || "c-info"}">${esc(state)}</span>`;
  const editor = (path, line) => C.repo_abs ? `vscode://file/${C.repo_abs}/${path}${line ? ":" + line : ""}` : `${C.repo_href}/${path}`;
  const source = (path, line) => `<a href="${esc(editor(path, line))}" title="Open in the editor"><code>${esc(path)}${line ? ":" + line : ""}</code></a>`;
  const itemLink = (id, text) => items[id] ? `<a href="#item/${encodeURIComponent(id)}">${esc(text || items[id].display)}</a>` : `<code>${esc(id)}</code>`;
  const notes = key => (window.ArtReviews ? window.ArtReviews.openFor(key) : 0);
  // One player for the whole page, docked at the bottom: it says what is playing and links to it.
  const bar = document.createElement("div");
  bar.className = "now-playing"; bar.hidden = true;
  bar.innerHTML = `<span class="np-label"></span><audio controls preload="none"></audio><button type="button" class="ghost np-close" title="Stop">✕</button>`;
  document.body.append(bar);
  const player = bar.querySelector("audio"), label = bar.querySelector(".np-label");
  const title = document.title;
  // `current` is the item id and the exact file: one track can offer several renders.
  let current = null, currentItem = null;
  const played = {};  // furthest second heard, by file, for music reviews
  const keyOf = (id, href) => id + "|" + (href || items[id]?.media?.href || "");
  function sync() {
    const playing = current && !player.paused;
    document.title = current ? `${playing ? "▶" : "⏸"} ${items[currentItem]?.display || currentItem} · ${title}` : title;
    document.querySelectorAll("[data-play]").forEach(b => {
      const on = keyOf(b.dataset.play, b.dataset.href) === current;
      b.textContent = on && playing ? "⏸" : "▶";
      b.closest("tr")?.classList.toggle("playing", on);
    });
  }
  function play(id, href, note) {
    const item = items[id];
    if (!item?.media || item.media.type !== "audio") return;
    const key = keyOf(id, href);
    if (current === key) { player.paused ? player.play() : player.pause(); return; }
    current = key; currentItem = id;
    player.src = href || item.media.href; player.dataset.href = href || item.media.href;
    const what = note || (item.media.unpublished ? "the music tool's unpublished render" : "");
    label.innerHTML = `<b>Now playing</b> ${itemLink(id)} <small>${esc(KINDS[item.kind] || item.kind)}${what ? " · " + esc(what) : ""}</small>`;
    bar.hidden = false; document.body.classList.add("has-player");
    player.play();
  }
  ["play", "pause", "ended"].forEach(type => player.addEventListener(type, sync));
  player.addEventListener("timeupdate", () => {
    const href = player.dataset.href, row = played[href] || (played[href] = {seconds: 0, fraction: 0});
    row.seconds = Math.max(row.seconds, player.currentTime);
    if (player.duration) row.fraction = Math.max(row.fraction, player.currentTime / player.duration);
  });
  bar.querySelector(".np-close").onclick = () => { player.pause(); player.removeAttribute("src"); current = null; bar.hidden = true; document.body.classList.remove("has-player"); sync(); };
  document.addEventListener("click", event => {
    const button = event.target.closest("[data-play]");
    if (button) { event.preventDefault(); play(button.dataset.play, button.dataset.href, button.dataset.note); }
  });
  window.addEventListener("hashchange", () => setTimeout(sync, 0));

  function families() {
    const libraries = C.families?.libraries || [];
    if (!libraries.length) return `<p class="empty">Producer status is unavailable. ${esc(C.families?.notice || "")}</p>`;
    return libraries.map(lib => `<h3>${esc(lib.id)} <small><code>${esc(lib.manifest)}</code></small></h3>
      <table class="tbl"><thead><tr><th>Family</th><th>State</th><th>Units</th><th>Why</th><th>Make it</th></tr></thead><tbody>
      ${Object.entries(lib.families).map(([name, f]) => `<tr><td>${esc(name)}</td><td>${chip(f.state)}</td><td>${esc(f.units)}</td>
        <td>${(f.stale_units || []).slice(0, 2).map(u => `<details><summary>${esc(u.reason)}</summary>${(u.changed || []).slice(0, 12).map(p => `<div><code>${esc(p)}</code></div>`).join("")}</details>`).join("") || (f.unchecked_units ? `${esc(f.unchecked_units)} unit(s) unchecked` : "")}</td>
        <td>${f.state === "present" ? "" : `<code>${esc(fixFor(name))}</code>`}</td></tr>`).join("")}</tbody></table>`).join("");
  }
  function fixFor(family) {
    const kind = Object.values(C.kinds || {}).find(k => k.family === family);
    return kind?.fix || C.families?.make || "scripts/setup/generated_content.sh";
  }

  function search(terms) {
    const hit = text => terms.every(t => text.toLowerCase().includes(t));
    const found = Object.values(items).filter(i => hit(i.id + " " + i.display + " " + i.tags.join(" ") + " " + (i.score?.description || "")));
    const characters = Object.entries(C.inventory?.identities || {}).filter(([key, a]) => hit(key + " " + a.display + " " + (a.catalog?.tags || []).join(" ")));
    const images = Object.values(C.assets || {}).filter(a => hit(a.id + " " + a.display + " " + (a.tags || []).join(" ")));
    app().innerHTML = `<h1>Search <small>${esc(terms.join(" "))}</small></h1>
      <section class="box"><h2>Characters (${characters.length})</h2>${characters.slice(0, 60).map(([key, a]) => `<a href="#character/${encodeURIComponent(key)}">${esc(a.display)}</a>`).join(" · ") || '<p class="empty">None.</p>'}</section>
      <section class="box"><h2>Images (${images.length})</h2>${images.slice(0, 60).map(a => `<a href="#asset/${encodeURIComponent(a.id)}">${esc(a.display)}</a>`).join(" · ") || '<p class="empty">None.</p>'}
        ${images.length > 60 ? '<p><a href="#gallery">All matches in Images</a></p>' : ""}</section>
      <section class="box"><h2>Music, sound, fonts and worlds (${found.length})</h2>${found.length ? table(found.slice(0, 300), true) : '<p class="empty">None.</p>'}</section>`;
  }

  function overview() {
    const terms = query().split(/\s+/).filter(Boolean);
    if (terms.length) return search(terms);
    const I = C.inventory;
    const cards = [];
    if (I) {
      const ids = Object.values(I.identities || {});
      cards.push({title: "Characters", href: "#characters", n: ids.length,
        line: I.profile.state === "available" ? `${Object.values(I.units).filter(u => u.status?.state === "missing").length} art bundles missing, ${Object.values(I.units).filter(u => ["partial", "stale"].includes(u.status?.state)).length} stale, ${Object.values(I.units).filter(u => u.status?.state === "possibly-stale").length} possibly stale` : "catalog unavailable"});
    }
    const images = Object.values(C.assets || {});
    cards.push({title: "Images", href: "#gallery", n: images.length, line: `${images.filter(a => a.findings).length} with findings`});
    for (const [kind, title] of Object.entries(KINDS)) {
      const k = C.kinds?.[kind];
      if (!k) continue;
      const states = Object.entries(k.states).map(([s, n]) => `${n} ${s}`).join(", ");
      cards.push({title, href: "#" + VIEW[kind], n: k.count, line: (states || "none") + (k.notices.length ? " · notice" : "")});
    }
    const reviewCount = window.ArtReviews ? window.ArtReviews.openCount() : 0;
    cards.push({title: "Review notes", href: "#reviews", n: reviewCount, line: "open notes"});
    const attention = Object.values(items).filter(i => i.state !== "present").sort((a, b) => a.kind.localeCompare(b.kind) || a.name.localeCompare(b.name));
    const notices = Object.entries(C.kinds || {}).flatMap(([kind, k]) => k.notices.map(n => `<p class="stamp stale">${esc(KINDS[kind] || kind)}: ${esc(n)}</p>`));
    app().innerHTML = `<h1>Assets</h1>
      <p class="lede">Everything the selected game ships or declares, what is missing or stale, and how to make it. Open any item to play or view it, see its source and where the game uses it, and leave a review note.</p>
      ${notices.join("")}
      <div class="cards">${cards.map(c => `<a class="card overview-card" href="${c.href}"><b>${esc(c.title)}</b><span class="big">${esc(c.n)}</span><span class="n">${esc(c.line)}</span></a>`).join("")}</div>
      <section class="box"><h2>Generated content by producer</h2>${families()}</section>
      <section class="box"><h2>Needs attention (${attention.length})</h2>
        ${attention.length ? table(attention.slice(0, 200), true) : '<p class="empty">Every music, sound, font and world item is present.</p>'}</section>`;
  }

  function table(rows, withKind) {
    return `<table class="tbl"><thead><tr><th></th>${withKind ? "<th>Kind</th>" : ""}<th>Name</th><th>State</th><th>Uses</th><th>Review</th><th>Notes</th></tr></thead><tbody>
      ${rows.map(i => `<tr><td>${i.media?.type === "audio" ? `<button type="button" class="ghost" data-play="${esc(i.id)}" title="Play ${esc(i.display)}">▶</button>` : ""}</td>
        ${withKind ? `<td>${esc(KINDS[i.kind] || i.kind)}</td>` : ""}
        <td>${itemLink(i.id)}<br><small><code>${esc(i.name)}</code></small></td><td>${chip(i.state)}</td><td>${i.uses.length || ""}</td>
        <td>${i.reviews ? `${esc(i.reviews.score)} ${esc(i.reviews.label || "")}${i.reviews.score_changed ? " ⚠" : ""}` : ""}</td>
        <td>${notes(i.id) ? `${notes(i.id)} open` : ""}${i.reasons.length ? `<small>${esc(i.reasons[0])}</small>` : ""}</td></tr>`).join("")}</tbody></table>`;
  }

  const filters = {};
  function list(kind) {
    const all = Object.values(items).filter(i => i.kind === kind);
    const k = C.kinds?.[kind] || {notices: [], states: {}};
    const terms = query().split(/\s+/).filter(Boolean);
    const f = filters[kind] || (filters[kind] = {state: "", used: ""});
    let rows = all.filter(i => terms.every(t => (i.id + " " + i.display + " " + i.tags.join(" ") + " " + (i.score?.description || "")).toLowerCase().includes(t)));
    if (f.state) rows = rows.filter(i => i.state === f.state);
    if (f.used) rows = rows.filter(i => f.used === "used" ? i.uses.length : f.used === "unused" ? !i.uses.length : f.used === "reviewed" ? i.reviews : !i.reviews);
    const facet = (group, key, label) => `<button type="button" class="ghost${f[group] === key ? " on" : ""}" data-filter="${group}" data-key="${esc(key)}">${esc(label)}</button>`;
    app().innerHTML = `<h1>${esc(KINDS[kind])} <small>${rows.length} of ${all.length}</small></h1>
      ${k.notices.map(n => `<p class="stamp stale">${esc(n)}</p>`).join("")}
      <p class="controls">${facet("state", "", "all")}${Object.entries(k.states).map(([s, n]) => facet("state", s, `${s} (${n})`)).join("")}
        · ${facet("used", "", "any use")}${facet("used", "used", "used in game text")}${facet("used", "unused", "no use found")}
        ${kind === "music" ? facet("used", "reviewed", "reviewed") + facet("used", "unreviewed", "never reviewed") : ""}
        · make: <code>${esc(k.fix || "")}</code></p>
      ${rows.length ? table(rows.slice(0, 1000), false) : '<p class="empty">Nothing matches.</p>'}`;
    app().querySelectorAll("[data-filter]").forEach(b => b.onclick = () => { f[b.dataset.filter] = b.dataset.key; list(kind); });
    sync();
  }

  function media(item) {
    if (!item.media) return "";
    if (item.media.type === "audio") return `<p><button type="button" class="play-big" data-play="${esc(item.id)}">▶</button> Play ${esc(item.display)}</p>${item.media.unpublished ? '<p class="empty">This is the music tool\'s newest render; the game does not have it yet.</p>' : ""}`;
    if (item.media.type === "font") {
      const family = "asset-font-" + item.name.replace(/[^A-Za-z0-9]/g, "_");
      return `<style>@font-face { font-family: "${family}"; src: url("${esc(item.media.href)}"); }</style>
        <p style="font-family:'${family}'; font-size: 32px; margin: 8px 0">The quick brown fox jumps over the lazy dog</p>
        <p style="font-family:'${family}'; font-size: 18px">ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz 0123456789 !?&amp;@#</p>`;
    }
    return "";
  }

  function details(item) {
    if (item.kind === "world") {
      const rooms = item.details.rooms || [];
      return `<h2>Rooms (${rooms.length})</h2><table class="tbl"><thead><tr><th>Room</th><th>Size</th><th>Music</th><th>Fight music</th><th>Parallax</th><th>Biome</th></tr></thead><tbody>
        ${rooms.map(r => `<tr><td>${esc(r.id)}</td><td>${esc((r.size || []).join("×"))}</td><td>${r.music_track ? itemLink("music:" + r.music_track, r.music_track) : ""}</td>
          <td>${r.fight_music_track ? itemLink("music:" + r.fight_music_track, r.fight_music_track) : ""}</td><td>${esc(r.parallax_theme || "")}</td><td>${esc(r.biome || "")}</td></tr>`).join("")}</tbody></table>`;
    }
    const rows = Object.entries(item.details).filter(([, v]) => v !== null && v !== "" && !(Array.isArray(v) && !v.length));
    return rows.length ? `<h2>Details</h2><table class="tbl">${rows.map(([k, v]) => `<tr><th>${esc(k.replace(/_/g, " "))}</th><td>${esc(Array.isArray(v) ? v.join(", ") : typeof v === "object" ? JSON.stringify(v) : v)}</td></tr>`).join("")}</table>` : "";
  }

  const served = location.protocol.startsWith("http");
  function reviewForm(item, target) {
    const k = C.kinds.music || {}, tags = k.issue_tags || [], [low, high] = k.score_range || [1, 10];
    const old = (item.reviews?.history || []).find(h => h.version === target.version_id);
    return `<form class="music-review" data-item="${esc(item.id)}" data-version="${esc(target.version_id)}">
      <p><button type="button" class="ghost" data-play="${esc(item.id)}" data-href="${esc(target.href)}" data-note="${esc(target.roles.join(", "))}">▶</button>
        <b>${esc(target.roles.join(" · "))}</b> <code>${esc(target.version_id)}</code>
        ${old ? ` · reviewed <b>${esc(old.score)}</b> ${esc(old.label || "")}` : " · not reviewed"}</p>
      ${served ? `<div class="controls"><label>Score <input name="score" type="number" min="${low}" max="${high}" step="0.1" required value="${esc(old?.score ?? "")}"></label>
        <span class="n">1–3 replace · 3–5 major polish · 5–7 acceptable · 7–9 strong · 9–10 standout</span></div>
        <textarea name="notes" rows="3" placeholder="What works, what to change">${esc(old?.notes || "")}</textarea>
        <div class="controls">${tags.map(t => `<label><input type="checkbox" name="issues" value="${esc(t)}"${old?.issues?.includes(t) ? " checked" : ""}> ${esc(t)}</label>`).join(" ")}</div>
        <p class="controls"><button type="submit">${old ? "Update review" : "Save review"}</button> <span class="save-state" role="status"></span></p>`
        : '<p class="empty">Open the page with <code>./scripts/asset_inspector.py</code> to save a review.</p>'}</form>`;
  }

  // The score's own words: its description and notes, its form, and what plays it.
  function scoreBox(item) {
    const sc = item.score;
    if (!sc) return "";
    const facts = [sc.key, sc.bpm && `${sc.bpm} BPM`, sc.meter, sc.form, sc.one_shot && "one-shot"].filter(Boolean);
    const groups = {};
    sc.instruments.forEach(i => (groups[i.source] ||= []).push(i));
    const summary = Object.entries(groups).map(([source, rows]) => `${rows.length} ${esc(source)}`).join(" · ");
    return `<section class="box"><h2>Score</h2>
      ${sc.title && sc.title !== item.display ? `<p><b>${esc(sc.title)}</b> <small>(the score's title)</small></p>` : ""}
      ${sc.description ? `<p class="review-text">${esc(sc.description)}</p>` : '<p class="empty">The score has no description.</p>'}
      ${facts.length ? `<p>${facts.map(esc).join(" · ")}</p>` : ""}
      ${sc.sections.length ? `<p>Sections: ${sc.sections.map(esc).join(" → ")}</p>` : ""}
      ${sc.notes.length ? `<details${sc.description ? "" : " open"}><summary>Notes (${sc.notes.length})</summary>${sc.notes.map(n => `<p class="review-text">${esc(n)}</p>`).join("")}</details>` : ""}
      <h3>Instruments (${sc.instruments.length})</h3><p class="n">${summary}</p>
      <table class="tbl"><thead><tr><th>Part</th><th>Sound</th><th>Source</th></tr></thead><tbody>
        ${sc.instruments.map(i => `<tr><td>${esc(i.name)}${i.group && i.group !== i.name ? ` <small>${esc(i.group)}</small>` : ""}</td><td>${esc(i.sound)}${i.notes ? `<small>${esc(i.notes)}</small>` : ""}</td><td>${esc(i.source)}</td></tr>`).join("")}</tbody></table></section>`;
  }

  function musicReviews(item) {
    const r = item.reviews;
    if (item.kind !== "music") return "";
    const targets = item.review_targets || [];
    const history = r ? `<p>Latest: <b>${esc(r.score)}</b> ${esc(r.label || "")} · ${esc(r.reviewed_at || "")}</p>
      ${r.score_changed ? '<p class="stamp stale">The score changed after this review: the review is of an older version.</p>' : ""}
      ${r.notes ? `<p class="review-text">${esc(r.notes)}</p>` : ""}
      ${r.history.length > 1 ? `<details><summary>All ${r.history.length} reviews</summary>${r.history.map(h => `<p><b>${esc(h.score)}</b> ${esc(h.label || "")} · ${esc(h.reviewed_at || "")} · <code>${esc(h.version || "")}</code><br>${esc(h.notes)}</p>`).join("")}</details>` : ""}` : '<p class="empty">Never reviewed.</p>';
    return `<section class="box"><h2>Music reviews (${r ? r.count : 0})</h2>${history}
      ${targets.length ? `<h3>Rate a render</h3><p class="n">A review rates one exact render. Listen to it here; how far you listened is saved with the review.</p>${targets.map(t => reviewForm(item, t)).join("")}`
        : '<p class="empty">The music tool has no render of this score to review.</p>'}</section>`;
  }

  function wireMusicReviews() {
    document.querySelectorAll("form.music-review").forEach(form => form.onsubmit = async event => {
      event.preventDefault();
      const item = items[form.dataset.item], target = item.review_targets.find(t => t.version_id === form.dataset.version);
      const state = form.querySelector(".save-state"), button = form.querySelector("[type=submit]");
      const data = new FormData(form), heard = played[target.href] || {seconds: 0, fraction: 0};
      button.disabled = true; state.textContent = "Saving…";
      try {
        const response = await fetch("/api/music-reviews", {method: "POST", headers: {"Content-Type": "application/json"}, body: JSON.stringify({
          item: item.id, version_id: target.version_id, score: Number(data.get("score")), notes: data.get("notes") || "",
          issues: data.getAll("issues"), furthest_played_seconds: heard.seconds, furthest_played_fraction: heard.fraction})});
        const result = await response.json();
        if (!response.ok) throw new Error(result.error || "Could not save the review.");
        // Show it now; the next index reads it from the music tool's review file.
        const rows = (item.reviews ||= {count: 0, history: [], tool: "music"}).history.filter(h => h.version !== result.version_id);
        rows.push(result.review);
        Object.assign(item.reviews, result.review, {count: rows.length, history: rows, score_changed: false});
        detail(item.id);
        document.querySelector(`form[data-version="${CSS.escape(target.version_id)}"] .save-state`).textContent = `Saved to ${result.path}`;
      } catch (error) { state.textContent = error.message; button.disabled = false; }
    });
  }

  function detail(id) {
    const item = items[id];
    if (!item) { app().innerHTML = `<h1>Unknown item</h1><p><code>${esc(id)}</code> is not in this index.</p><a href="#overview">Overview</a>`; return; }
    const usedBy = item.uses.map(u => `<li>${u.item ? itemLink(u.item) + " · " : ""}${u.line ? source(u.file, u.line) : esc(u.where)}${u.field ? ` <small>${esc(u.field)}</small>` : ""}</li>`).join("");
    app().innerHTML = `<p><a href="#${VIEW[item.kind]}">${esc(KINDS[item.kind] || item.kind)}</a></p>
      <h1>${esc(item.display)} ${chip(item.state)}</h1><p><code>${esc(item.id)}</code> ${item.tags.map(t => `<span class="chip">${esc(t)}</span>`).join(" ")}</p>
      ${item.reasons.map(r => `<p class="stamp stale">${esc(r)}</p>`).join("")}
      ${item.fix ? `<p>Make it: <code>${esc(item.fix)}</code></p>` : ""}
      ${media(item)}
      ${scoreBox(item)}
      <div class="cols">
        <section class="box"><h2>Source</h2>${item.sources.length ? item.sources.map(p => `<p>${source(p)}</p>`).join("") : '<p class="empty">No source is known.</p>'}</section>
        <section class="box"><h2>Files</h2>${item.files.length ? item.files.map(f => `<p><a href="${esc(f.href)}"><code>${esc(f.path)}</code></a> <small>${esc(Math.round(f.bytes / 1024))} KiB</small></p>`).join("") : '<p class="empty">Not generated.</p>'}</section>
      </div>
      <section class="box"><h2>Used by (${item.uses.length})</h2>${usedBy ? `<ul>${usedBy}</ul>` : '<p class="empty">No use found in game text or worlds.</p>'}</section>
      ${item.related.length ? `<section class="box"><h2>Related</h2><p>${item.related.map(r => itemLink(r, r)).join(" · ")}</p></section>` : ""}
      ${(() => { const d = details(item); return d ? `<section class="box">${d}</section>` : ""; })()}
      ${musicReviews(item)}
      ${window.ArtReviews ? window.ArtReviews.section(item.id) : ""}`;
    if (window.ArtReviews) window.ArtReviews.wire();
    wireMusicReviews();
    sync();
  }

  window.AssetItems = {overview, list, detail, VIEW};
})();
