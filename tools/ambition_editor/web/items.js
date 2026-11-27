/* The overview and the item kinds beside sprites: music, sound effects, fonts and worlds.
   Every item has one shape (`asset_index/kinds.py`), so one list and one detail page serve all. */
(function () {
  "use strict";
  const C = window.ART_CATALOG;
  if (!C) return;
  const items = C.items;
  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"})[c]);
  const app = () => document.getElementById("app");
  const query = () => document.getElementById("q").value.trim().toLowerCase();
  const KINDS = {music: "Music", sfx: "Sound effects", font: "Fonts", world: "Worlds"};
  const VIEW = {music: "music", sfx: "sound", font: "fonts", world: "worlds"};
  const STATE_CLASS = {present: "c-ok", missing: "c-warn", stale: "c-warn", unregistered: "c-info", excluded: "c-info", unknown: "c-info"};
  const chip = state => `<span class="chip ${STATE_CLASS[state] || "c-info"}">${esc(state)}</span>`;
  const editor = (path, line) => C.repo_abs ? `vscode://file/${C.repo_abs}/${path}${line ? ":" + line : ""}` : `${C.repo_href}/${path}`;
  const source = (path, line) => `<a href="${esc(editor(path, line))}" title="Open in the editor"><code>${esc(path)}${line ? ":" + line : ""}</code></a>`;
  const itemLink = (id, text) => Object.hasOwn(C.item_labels, id) ? `<a href="#item/${encodeURIComponent(id)}">${esc(text || C.item_labels[id])}</a>` : `<code>${esc(id)}</code>`;
  const R = () => window.AssetReviews;
  // One player for the whole page, docked at the bottom: it says what is playing and links to it.
  const bar = document.createElement("div");
  bar.className = "now-playing"; bar.hidden = true;
  bar.innerHTML = `<span class="np-label"></span>
    <button type="button" class="ghost np-prev" title="Previous section ( [ )">⏮</button><audio controls preload="none"></audio><button type="button" class="ghost np-next" title="Next section ( ] )">⏭</button>
    <button type="button" class="ghost np-close" title="Stop">✕</button>
    <div class="np-track" title="Click to play from here"><div class="np-sections"></div><div class="np-head"></div></div>`;
  document.body.append(bar);
  const player = bar.querySelector("audio"), label = bar.querySelector(".np-label");
  const track = bar.querySelector(".np-track"), sectionsEl = bar.querySelector(".np-sections"), head = bar.querySelector(".np-head");
  const title = document.title;
  // `current` is the item id and the exact file: one track can offer several renders.
  let current = null, currentItem = null;
  const played = {};  // furthest second heard, by file, for music reviews
  const keyOf = (id, href) => id + "|" + (href || items[id]?.media?.href || "");
  function sync() {
    const playing = current && !player.paused;
    document.title = current ? `${playing ? "▶" : "⏸"} ${items[currentItem]?.display || currentItem} · ${title}` : title;
    document.querySelectorAll("[data-play]:not([data-section])").forEach(b => {
      const on = keyOf(b.dataset.play, b.dataset.href) === current;
      b.textContent = on && playing ? "⏸" : "▶";
      b.closest("tr")?.classList.toggle("playing", on);
    });
  }
  function play(id, href, note, section) {
    const item = items[id];
    if (!item?.media || item.media.type !== "audio") return;
    const key = keyOf(id, href);
    if (current === key) {
      if (section) seekSection(section);
      else player.paused ? player.play() : player.pause();
      return;
    }
    current = key; currentItem = id; chapters = [];
    player.src = href || item.media.href; player.dataset.href = href || item.media.href;
    const what = note || (item.media.unpublished ? "the music tool's unpublished render" : "");
    label.innerHTML = `<b>Now playing</b> ${itemLink(id)} <small>${esc(KINDS[item.kind] || item.kind)}${what ? " · " + esc(what) : ""}</small>`;
    bar.hidden = false; document.body.classList.add("has-player");
    drawTrack();
    const src = player.src;
    chaptersOf(player.dataset.href).then(rows => {
      if (player.src !== src) return;  // another file started meanwhile
      chapters = rows; drawTrack();
      if (section) seekSection(section);
    });
    player.play();
  }

  /* The sections of the file playing, from its own chapter comments: the
     renderer writes `CHAPTERnnn` / `CHAPTERnnnNAME` (render/export.py), the
     chapters VLC lists. A file without them shows a plain seek strip. */
  let chapters = [];
  const chapterCache = new Map();
  function chaptersOf(href) {
    if (!chapterCache.has(href)) chapterCache.set(href, readHead(href).then(parseChapters).catch(() => []));
    return chapterCache.get(href);
  }
  // The first 256 KiB of a file, which hold its comment header.
  async function readHead(href, limit = 262144) {
    const response = await fetch(href, {headers: {Range: `bytes=0-${limit - 1}`}});
    if (!response.ok) return new Uint8Array();
    const reader = response.body.getReader(), parts = [];
    let size = 0;
    while (size < limit) {
      const {done, value} = await reader.read();
      if (done) break;
      parts.push(value); size += value.length;
    }
    reader.cancel().catch(() => {});  // a server that ignored the range sends the whole file
    const out = new Uint8Array(size);
    let at = 0;
    for (const part of parts) { out.set(part, at); at += part.length; }
    return out;
  }
  // Packet `wanted` of an Ogg stream: its segments, joined across pages.
  function oggPacket(bytes, wanted) {
    let pos = 0, packet = 0, parts = [];
    while (pos + 27 <= bytes.length && bytes[pos] === 0x4f && bytes[pos + 1] === 0x67 && bytes[pos + 2] === 0x67 && bytes[pos + 3] === 0x53) {
      const count = bytes[pos + 26], table = pos + 27;
      let data = table + count;
      for (let i = 0; i < count; i++) {
        const lacing = bytes[table + i];
        if (packet === wanted) parts.push(bytes.subarray(data, data + lacing));
        data += lacing;
        if (lacing < 255) {
          if (packet === wanted) return parts.reduce((all, part) => { const joined = new Uint8Array(all.length + part.length); joined.set(all); joined.set(part, all.length); return joined; }, new Uint8Array());
          packet++;
        }
      }
      pos = data;
    }
    return null;
  }
  function parseChapters(bytes) {
    const packet = oggPacket(bytes, 1);  // Vorbis and Opus both carry their comments in the second packet
    if (!packet) return [];
    const ascii = (from, n) => String.fromCharCode(...packet.subarray(from, from + n));
    const start = packet[0] === 3 && ascii(1, 6) === "vorbis" ? 7 : ascii(0, 8) === "OpusTags" ? 8 : -1;
    if (start < 0) return [];
    const view = new DataView(packet.buffer, packet.byteOffset, packet.byteLength), text = new TextDecoder();
    let at = start + 4 + view.getUint32(start, true);
    const count = view.getUint32(at, true), tags = {};
    at += 4;
    for (let i = 0; i < count && at + 4 <= packet.length; i++) {
      const n = view.getUint32(at, true), entry = text.decode(packet.subarray(at + 4, at + 4 + n));
      at += 4 + n;
      const cut = entry.indexOf("=");
      if (cut > 0) tags[entry.slice(0, cut).toUpperCase()] = entry.slice(cut + 1);
    }
    const rows = [];
    for (let n = 1; tags[`CHAPTER${String(n).padStart(3, "0")}`]; n++) {
      const key = `CHAPTER${String(n).padStart(3, "0")}`;
      const [h, m, s] = tags[key].split(":").map(Number);
      const name = tags[key + "NAME"] || tags[key + "ID"] || `section ${n}`;
      if (![h, m, s].some(Number.isNaN)) rows.push({id: tags[key + "ID"] || name, name, at: h * 3600 + m * 60 + s});
    }
    return rows.sort((a, b) => a.at - b.at);
  }

  const clock = t => `${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, "0")}`;
  // The strip under the player: one block per section, as long as it plays.
  function drawTrack() {
    const length = player.duration;
    if (!isFinite(length) || !length) { sectionsEl.innerHTML = ""; head.style.left = "0"; return; }
    const form = Object.fromEntries((items[currentItem]?.score?.form_sections || []).map(row => [row.id, row]));
    sectionsEl.innerHTML = chapters.map((c, i) => {
      const end = chapters[i + 1]?.at ?? length, role = form[c.id]?.role, energy = form[c.id]?.energy;
      return `<div class="np-section" style="left:${c.at / length * 100}%;width:${(end - c.at) / length * 100}%;--e:${energy ?? 0.5}"
        title="${esc(`${c.id}${c.name !== c.id ? ": " + c.name : ""} · ${clock(c.at)}–${clock(end)}${role ? " — " + role : ""}`)}"><button type="button" class="np-name" data-at="${c.at}">${esc(c.id)}</button></div>`;
    }).join("");
    moveHead();
  }
  function moveHead() {
    const length = player.duration, t = player.currentTime;
    if (!isFinite(length) || !length) return;
    head.style.left = `${t / length * 100}%`;
    const now = chapters.findLastIndex(c => c.at <= t + 0.05);
    sectionsEl.querySelectorAll(".np-section").forEach((b, i) => b.classList.toggle("now", i === now));
  }
  function seekTo(t) {
    player.currentTime = Math.max(0, Math.min(t, (player.duration || t) - 0.05));
    if (player.paused) player.play();
    moveHead();
  }
  function seekSection(id) {
    const row = chapters.find(c => c.id === id || c.name === id);
    if (!row) return;
    if (isFinite(player.duration)) seekTo(row.at);
    else player.addEventListener("loadedmetadata", () => seekTo(row.at), {once: true});
  }
  // To the previous section start, or to this one's when more than two seconds in.
  function step(direction) {
    if (!current || !chapters.length) return;
    const t = player.currentTime, now = Math.max(0, chapters.findLastIndex(c => c.at <= t + 0.05));
    const target = direction > 0 ? chapters[now + 1] : (t - chapters[now].at > 2 ? chapters[now] : chapters[Math.max(0, now - 1)]);
    if (target) seekTo(target.at);
  }
  track.addEventListener("click", event => {
    if (!isFinite(player.duration)) return;
    // A section's name starts it; anywhere else plays from that point.
    const name = event.target.closest(".np-name"), box = track.getBoundingClientRect();
    seekTo(name ? +name.dataset.at : (event.clientX - box.left) / box.width * player.duration);
  });
  track.addEventListener("mousemove", event => {
    if (!isFinite(player.duration)) return;
    const box = track.getBoundingClientRect();
    track.title = `Play from ${clock((event.clientX - box.left) / box.width * player.duration)}`;
  });
  bar.querySelector(".np-prev").onclick = () => step(-1);
  bar.querySelector(".np-next").onclick = () => step(1);
  document.addEventListener("keydown", event => {
    const active = document.activeElement || {};
    if (/^(INPUT|SELECT|TEXTAREA)$/.test(active.tagName || "") || active.isContentEditable || event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.key === "[" || event.key === "]") { event.preventDefault(); step(event.key === "]" ? 1 : -1); }
  });
  ["loadedmetadata", "durationchange"].forEach(type => player.addEventListener(type, drawTrack));
  player.addEventListener("timeupdate", moveHead);
  player.addEventListener("seeked", moveHead);
  ["play", "pause", "ended"].forEach(type => player.addEventListener(type, sync));
  player.addEventListener("timeupdate", () => {
    const href = player.dataset.href, row = played[href] || (played[href] = {seconds: 0, fraction: 0});
    row.seconds = Math.max(row.seconds, player.currentTime);
    if (player.duration) row.fraction = Math.max(row.fraction, player.currentTime / player.duration);
  });
  bar.querySelector(".np-close").onclick = () => { player.pause(); player.removeAttribute("src"); current = null; bar.hidden = true; document.body.classList.remove("has-player"); sync(); };
  document.addEventListener("click", event => {
    const button = event.target.closest("[data-play]");
    if (button) { event.preventDefault(); play(button.dataset.play, button.dataset.href, button.dataset.note, button.dataset.section); }
  });
  window.addEventListener("hashchange", () => setTimeout(sync, 0));
  // How far a file was heard, for a review of it.
  const heard = href => ({...(played[href] || {seconds: 0, fraction: 0})});

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
      <section class="box"><h2>Music, sound, fonts and worlds (${found.length})</h2>${found.length ? table("items-found", found, {kind: true, limit: 300}) : '<p class="empty">None.</p>'}</section>`;
    wireTables(() => search(terms));
  }

  function overview() {
    const terms = query().split(/\s+/).filter(Boolean);
    if (terms.length) return search(terms);
    const counts = C.counts, status = C.unit_status;
    const cards = [{title: "Characters", href: "#characters", n: counts.characters,
      line: C.inventory.profile.state === "available" ? `${status.missing || 0} art bundles missing, ${(status.partial || 0) + (status.stale || 0)} stale, ${status["possibly-stale"] || 0} possibly stale` : "catalog unavailable"}];
    for (const [key, c] of Object.entries(C.categories || {}).sort((a, b) => a[1].order - b[1].order)) {
      if (key === "characters" || !(c.assets || 0)) continue;
      cards.push({title: c.label, href: "#cat/" + key, n: c.assets, line: key === "other" ? "no rule classified these: worth a look"
        : `${c.used || 0} used by the game${c.tier_copies ? ` · ${c.tier_copies} tier copies` : ""}`});
    }
    for (const [kind, title] of Object.entries(KINDS)) {
      const k = C.kinds?.[kind];
      if (!k) continue;
      const states = Object.entries(k.states).map(([s, n]) => `${n} ${s}`).join(", ");
      cards.push({title, href: "#" + VIEW[kind], n: k.count, line: (states || "none") + (k.notices.length ? " · notice" : "")});
    }
    const {reviews, backlog} = R().counts();
    cards.push({title: "Reviews", href: "#reviews", n: reviews, line: `${backlog} need work`});
    const attention = Object.values(C.attention).sort((a, b) => a.kind.localeCompare(b.kind) || a.name.localeCompare(b.name));
    const notices = Object.entries(C.kinds || {}).flatMap(([kind, k]) => k.notices.map(n => `<p class="stamp stale">${esc(KINDS[kind] || kind)}: ${esc(n)}</p>`));
    app().innerHTML = `<h1>Assets</h1>
      <p class="lede">Everything the selected game ships or declares, what is missing or stale, and how to make it. Open any item to play or view it, see its source and where the game uses it, and review it.</p>
      ${notices.join("")}
      <div class="cards">${cards.map(c => `<a class="card overview-card" href="${c.href}"><b>${esc(c.title)}</b><span class="big">${esc(c.n)}</span><span class="n">${esc(c.line)}</span></a>`).join("")}</div>
      <section class="box"><h2>Generated content by producer</h2>${families()}</section>
      <section class="box"><h2>Needs attention (${attention.length})</h2>
        ${attention.length ? table("items-attention", attention, {kind: true, limit: 200}) : '<p class="empty">Every music, sound, font and world item is present.</p>'}</section>`;
    wireTables(overview);
  }

  /* One table of items, sorted by a click on a column header (`table_sort.js`).
     Each table (by `id`) keeps its own sort while the page is open. */
  const sorts = {};
  const useCount = item => item.uses_count ?? item.uses.length;
  function table(id, rows, {kind: withKind = false, added: withAdded = false, limit = 1000, by = "", dir = 1} = {}) {
    const state = sorts[id] || (sorts[id] = {by, dir});
    const cols = {
      play: {label: ""},
      ...(withKind ? {kind: {label: "Kind", value: i => KINDS[i.kind] || i.kind}} : {}),
      name: {label: "Name", value: i => i.display},
      ...(withAdded ? {added: {label: "Added (Git)", title: "First score commit in available Git history", value: i => Date.parse(i.added_at) || null, first: -1}} : {}),
      state: {label: "State", value: i => i.state},
      uses: {label: "Uses", value: i => useCount(i) || null, first: -1},
      review: {label: "Review", title: "The newest review's score", value: i => R().latest(i.id)?.score ?? null, first: -1},
      why: {label: "Why", value: i => i.reasons[0] || null},
    };
    rows = TableSort.apply(rows, cols, state).slice(0, limit);
    return `<table class="tbl" id="${esc(id)}"><thead><tr>${TableSort.headers(cols, state)}</tr></thead><tbody>
      ${rows.map(i => `<tr><td>${i.media?.type === "audio" ? `<button type="button" class="ghost" data-play="${esc(i.id)}" title="Play ${esc(i.display)}">▶</button>` : ""}</td>
        ${withKind ? `<td>${esc(KINDS[i.kind] || i.kind)}</td>` : ""}
        <td>${itemLink(i.id)}<br><small><code>${esc(i.name)}</code></small></td>${withAdded ? `<td>${i.added_at ? `<time datetime="${esc(i.added_at)}" title="${esc(i.added_at)}">${esc(i.added_at.slice(0, 10))}</time>` : '<small>Unknown</small>'}</td>` : ""}<td>${chip(i.state)}</td><td>${useCount(i) || ""}</td>
        <td>${R().chip(R().latest(i.id))}${i.reviews?.score_changed ? ' <span title="The score changed after this review">⚠</span>' : ""}</td>
        <td>${i.reasons.length ? `<small>${esc(i.reasons[0])}</small>` : ""}</td></tr>`).join("")}</tbody></table>`;
  }

  function wireTables(redraw) {
    for (const [id, state] of Object.entries(sorts)) TableSort.wire(document.getElementById(id), state, redraw);
  }

  const filters = {};
  function list(kind) {
    const all = Object.values(items).filter(i => i.kind === kind);
    const k = C.kinds?.[kind] || {notices: [], states: {}};
    const terms = query().split(/\s+/).filter(Boolean);
    const f = filters[kind] || (filters[kind] = {state: "", used: ""});
    let rows = all.filter(i => terms.every(t => (i.id + " " + i.display + " " + i.tags.join(" ") + " " + (i.score?.description || "")).toLowerCase().includes(t)));
    if (f.state) rows = rows.filter(i => i.state === f.state);
    if (f.used) rows = rows.filter(i => f.used === "used" ? i.uses.length : f.used === "unused" ? !i.uses.length : f.used === "reviewed" ? R().latest(i.id) : !R().latest(i.id));
    // By name, and a click on a column header sorts by that column.
    rows.sort((a, b) => a.display.localeCompare(b.display) || a.id.localeCompare(b.id));
    const facet = (group, key, label) => `<button type="button" class="ghost${f[group] === key ? " on" : ""}" data-filter="${group}" data-key="${esc(key)}">${esc(label)}</button>`;
    app().innerHTML = `<h1>${esc(KINDS[kind])} <small>${rows.length} of ${all.length}</small></h1>
      ${k.notices.map(n => `<p class="stamp stale">${esc(n)}</p>`).join("")}
      <p class="controls">${facet("state", "", "all")}${Object.entries(k.states).map(([s, n]) => facet("state", s, `${s} (${n})`)).join("")}
        · ${facet("used", "", "any use")}${facet("used", "used", "used in game text")}${facet("used", "unused", "no use found")}
        ${facet("used", "reviewed", "reviewed") + facet("used", "unreviewed", "never reviewed")}
        · make: <code>${esc(k.fix || "")}</code></p>
      ${rows.length ? table("items-" + kind, rows, {added: kind === "music"}) : '<p class="empty">Nothing matches.</p>'}`;
    app().querySelectorAll("[data-filter]").forEach(b => b.onclick = () => { f[b.dataset.filter] = b.dataset.key; list(kind); });
    wireTables(() => list(kind));
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
      const rooms = item.details.rooms || [], pv = item.preview;
      const anchor = id => "room-" + String(id).replace(/[^A-Za-z0-9_-]/g, "_");
      // Boxes over the map image link each room to its row, placed as fractions of the map.
      const map = pv ? (() => { const [x0, y0, w, h] = pv.bounds; return `<div class="world-map">
        <a href="${esc(pv.world)}" title="Open the full-size map"><img src="${esc(pv.world)}" alt="Map of ${esc(item.display)}"></a>
        ${rooms.map(r => `<a class="room-box" title="${esc(r.id)}" href="#item/${encodeURIComponent(item.id)}" data-room="${esc(anchor(r.id))}"
          style="left:${(r.at[0] - x0) / w * 100}%;top:${(r.at[1] - y0) / h * 100}%;width:${r.size[0] / w * 100}%;height:${r.size[1] / h * 100}%"></a>`).join("")}</div>
        <p class="n">Collision, water and climbable cells and entities, drawn by the LDtk tool. Click a room to see it below; a dashed room is on another depth.</p>`; })() : "";
      return `<h2>Map</h2>${map || '<p class="empty">No preview.</p>'}
        <h2>Rooms (${rooms.length})</h2><table class="tbl"><thead><tr><th>Preview</th><th>Room</th><th>Size</th><th>Music</th><th>Fight music</th><th>Parallax</th><th>Biome</th></tr></thead><tbody>
        ${rooms.map(r => `<tr id="${esc(anchor(r.id))}"><td>${pv?.rooms?.[r.id] ? `<a href="${esc(pv.rooms[r.id])}" title="Open ${esc(r.id)} full size"><img class="room-thumb" loading="lazy" src="${esc(pv.rooms[r.id])}" alt=""></a>` : ""}</td><td>${esc(r.id)}</td><td>${esc((r.size || []).join("×"))}</td><td>${r.music_track ? itemLink("music:" + r.music_track, r.music_track) : ""}</td>
          <td>${r.fight_music_track ? itemLink("music:" + r.fight_music_track, r.fight_music_track) : ""}</td><td>${esc(r.parallax_theme || "")}</td><td>${esc(r.biome || "")}</td></tr>`).join("")}</tbody></table>`;
    }
    const rows = Object.entries(item.details).filter(([, v]) => v !== null && v !== "" && !(Array.isArray(v) && !v.length));
    return rows.length ? `<h2>Details</h2><table class="tbl">${rows.map(([k, v]) => `<tr><th>${esc(k.replace(/_/g, " "))}</th><td>${esc(Array.isArray(v) ? v.join(", ") : typeof v === "object" ? JSON.stringify(v) : v)}</td></tr>`).join("")}</table>` : "";
  }

  // What is typed in a music review form, by render, until it is saved: the page
  // draws itself again when the reviews arrive or change, and that must not
  // clear a review being written.
  const musicDrafts = new Map();
  function reviewForm(item, target) {
    const k = C.kinds.music || {}, tags = k.issue_tags || [], [low, high] = k.score_range || [1, 10];
    const old = R().rowsFor(item.id).find(row => row.version_id === target.version_id);
    const draft = musicDrafts.get(target.version_id);
    const typed = {score: draft?.score ?? old?.score ?? "", notes: draft?.notes ?? old?.notes ?? "",
      issues: draft?.issues ?? old?.issues ?? []};
    return `<form class="music-review" data-item="${esc(item.id)}" data-version="${esc(target.version_id)}">
      <p><button type="button" class="ghost" data-play="${esc(item.id)}" data-href="${esc(target.href)}" data-note="${esc(target.roles.join(", "))}">▶</button>
        <b>${esc(target.roles.join(" · "))}</b> <code>${esc(target.version_id)}</code>
        ${old ? ` · reviewed ${R().chip(old)}` : " · not reviewed"}</p>
      ${R().writable() ? `<div class="controls"><label>Score <input name="score" type="number" min="${low}" max="${high}" step="0.1" required value="${esc(typed.score)}"></label>
        <span class="n">1–3 replace · 3–5 major polish · 5–7 acceptable · 7–9 strong · 9–10 standout</span></div>
        <textarea name="notes" rows="3" placeholder="What works, what to change">${esc(typed.notes)}</textarea>
        <div class="controls">${tags.map(t => `<label><input type="checkbox" name="issues" value="${esc(t)}"${typed.issues.includes(t) ? " checked" : ""}> ${esc(t)}</label>`).join(" ")}</div>
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
      ${sc.form_sections?.length && item.media?.type === "audio" ? `<h3>Form</h3><p class="n">▶ plays from that section; <kbd>[</kbd> and <kbd>]</kbd> step between sections while it plays.</p>
        <table class="tbl form-sections"><tbody>${sc.form_sections.map(row => `<tr><td><button type="button" class="ghost" data-play="${esc(item.id)}" data-section="${esc(row.id)}" title="Play from ${esc(row.id)}">▶</button></td>
          <td><b>${esc(row.id)}</b></td><td>${row.energy != null ? `<span class="energy" style="--e:${row.energy}" title="energy ${row.energy}"></span>` : ""}</td><td>${esc(row.role || "")}</td></tr>`).join("")}</tbody></table>`
        : sc.sections.length ? `<p>Sections: ${sc.sections.map(esc).join(" → ")}</p>` : ""}
      ${sc.notes.length ? `<details${sc.description ? "" : " open"}><summary>Notes (${sc.notes.length})</summary>${sc.notes.map(n => `<p class="review-text">${esc(n)}</p>`).join("")}</details>` : ""}
      <h3>Instruments (${sc.instruments.length})</h3><p class="n">${summary}</p>
      <table class="tbl"><thead><tr><th>Part</th><th>Sound</th><th>Source</th></tr></thead><tbody>
        ${sc.instruments.map(i => `<tr><td>${esc(i.name)}${i.group && i.group !== i.name ? ` <small>${esc(i.group)}</small>` : ""}</td><td>${esc(i.sound)}${i.notes ? `<small>${esc(i.notes)}</small>` : ""}</td><td>${esc(i.source)}</td></tr>`).join("")}</tbody></table></section>`;
  }

  function musicReviews(item) {
    if (item.kind !== "music") return "";
    if (!R().ready()) return R().loading();
    const rows = R().rowsFor(item.id), targets = item.review_targets || [], last = rows[0];
    const history = last ? `<p>Latest: ${R().chip(last)} · ${esc(last.reviewed_at)}${last.relation !== "current" ? ` · ${esc(last.relation)} version` : ""}</p>
      ${item.reviews?.score_changed ? '<p class="stamp stale">The score changed after this review: the review is of an older version.</p>' : ""}
      ${last.notes ? `<p class="review-text">${esc(last.notes)}</p>` : ""}
      ${rows.length > 1 ? `<details><summary>All ${rows.length} reviews</summary>${rows.map(h => `<p>${R().chip(h)} · ${esc(h.reviewed_at)} · <code>${esc(h.version_id)}</code><br>${esc(h.notes)}</p>`).join("")}</details>` : ""}` : '<p class="empty">Never reviewed.</p>';
    return `<section class="box" id="asset-reviews"><h2>Reviews (${rows.length})</h2>
      <p class="n">The music tool's review files: its review window (<code>${esc(item.reviews?.command || "review_music.sh")}</code>) shows the same reviews.</p>${history}
      ${targets.length ? `<h3>Rate a render</h3><p class="n">A review rates one exact render. Listen to it here; how far you listened is saved with the review.</p>${targets.map(t => reviewForm(item, t)).join("")}`
        : '<p class="empty">The music tool has no render of this score to review.</p>'}</section>`;
  }

  function wireMusicReviews() {
    document.querySelectorAll("form.music-review").forEach(form => form.oninput = form.onchange = () => {
      const data = new FormData(form);
      musicDrafts.set(form.dataset.version, {score: data.get("score"), notes: data.get("notes") || "", issues: data.getAll("issues")});
    });
    document.querySelectorAll("form.music-review").forEach(form => form.onsubmit = async event => {
      event.preventDefault();
      const item = items[form.dataset.item], target = item.review_targets.find(t => t.version_id === form.dataset.version);
      const state = form.querySelector(".save-state"), button = form.querySelector("[type=submit]");
      const data = new FormData(form), heard = played[target.href] || {seconds: 0, fraction: 0};
      button.disabled = true; state.textContent = "Saving…";
      try {
        const saved = await R().save({id: item.id, version_id: target.version_id, score: Number(data.get("score")), notes: data.get("notes") || "",
          issues: data.getAll("issues"), furthest_played_seconds: heard.seconds, furthest_played_fraction: heard.fraction});
        if (item.reviews) item.reviews.score_changed = false;  // the new review rates the current score
        musicDrafts.delete(target.version_id);
        R().changed();
        const again = document.querySelector(`form[data-version="${CSS.escape(target.version_id)}"] .save-state`);
        if (again) again.textContent = `Saved to ${saved.path}`;
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
      ${item.kind === "world" ? `<section class="box">${details(item)}</section>` : ""}
      <div class="cols">
        <section class="box"><h2>Source</h2>${item.sources.length ? item.sources.map(p => `<p>${source(p)}</p>`).join("") : '<p class="empty">No source is known.</p>'}</section>
        <section class="box"><h2>Files</h2>${item.files.length ? item.files.map(f => `<p><a href="${esc(f.href)}"><code>${esc(f.path)}</code></a> <small>${esc(Math.round(f.bytes / 1024))} KiB</small></p>`).join("") : '<p class="empty">Not generated.</p>'}</section>
      </div>
      <section class="box"><h2>Used by (${item.uses.length})</h2>${usedBy ? `<ul>${usedBy}</ul>` : '<p class="empty">No use found in game text or worlds.</p>'}</section>
      ${item.related.length ? `<section class="box"><h2>Related</h2><p>${item.related.map(r => itemLink(r, r)).join(" · ")}</p></section>` : ""}
      ${(() => { const d = item.kind === "world" ? "" : details(item); return d ? `<section class="box">${d}</section>` : ""; })()}
      ${item.kind === "music" ? musicReviews(item) : R().section(item.id, {kind: item.kind, audio: item.media?.type === "audio" ? item.media.href : null,
        parts: item.kind === "world" ? (item.details.rooms || []).map(room => room.id) : [], partLabel: "Room"})}`;
    R().wire();
    wireMusicReviews();
    document.querySelectorAll(".room-box").forEach(box => box.onclick = event => {
      event.preventDefault();
      const row = document.getElementById(box.dataset.room);
      if (row) { row.scrollIntoView({behavior: "smooth", block: "center"}); row.classList.add("playing"); setTimeout(() => row.classList.remove("playing"), 1500); }
    });
    sync();
  }

  window.AssetItems = {overview, list, detail, heard, VIEW};
})();
