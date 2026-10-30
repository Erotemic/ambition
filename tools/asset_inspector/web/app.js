// The Ambition asset inspector: an offline page over catalog.js (built by
// `python -m asset_index build`). Every image and source file is linked by a
// relative path from this page, so it works from file:// with no server.
(function () {
  "use strict";
  const C = window.ART_CATALOG;
  const app = document.getElementById("app");
  if (!C) { app.innerHTML = '<p class="empty">catalog.js is missing: run <code>python3 scripts/asset_inspector.py build</code>.</p>'; return; }

  const CONF = ["verified", "declared", "derived", "inferred", "reference", "unknown"];
  const CONF_TEXT = {
    verified: "the bytes match a digest the generator recorded",
    declared: "a generator's record or registry names the file; nothing vouches for its bytes",
    derived: "a reduced-quality copy of another file",
    inferred: "only a name points at the generator",
    reference: "authored or imported art that no generator makes",
    unknown: "no generator found",
  };
  const PARALLAX = { sky: 0.10, far_backplate: 0.20, near_background: 0.42, foreground_atmosphere: 0.60, foreground: 1.0 };
  const assets = C.assets, products = C.products, gens = C.generators;
  const assetList = Object.values(assets);
  const state = { q: "", family: new Set(), conf: new Set(), tag: new Set(), use: "", rel: "", sort: "name", page: 0, card: 150, lastList: [] };
  // A target identifies today's asset only when it publishes one asset.
  const byTarget = {};
  const targetAssets = {};
  assetList.forEach(a => {
    if ((a.generator || '').startsWith('target:')) (targetAssets[a.generator.slice(7)] ||= []).push(a.id);
  });
  Object.entries(targetAssets).forEach(([target, ids]) => { if (ids.length === 1) byTarget[target] = ids[0]; });
  let timers = [];

  // ---- helpers ------------------------------------------------------------------------
  const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
  const fmt = (n) => (n ?? 0).toLocaleString();
  const mb = (b) => b >= 1e9 ? (b / 1e9).toFixed(2) + " GB" : b >= 1e6 ? (b / 1e6).toFixed(1) + " MB" : b >= 1e3 ? (b / 1e3).toFixed(0) + " kB" : b + " B";
  const chip = (c) => `<span class="chip c-${esc(c)}" title="${esc(CONF_TEXT[c] || c)}">${esc(c)}</span>`;
  const fam = (f) => `<span class="chip fam">${esc(f)}</span>`;
  const enc = encodeURIComponent;
  const repoHref = (path) => C.repo_href + "/" + path.split("/").map(enc).join("/");
  const assetLink = (id, text) => `<a href="#asset/${enc(id)}">${esc(text ?? (assets[id] ? assets[id].display : id))}</a>`;
  const genLink = (id) => id ? `<a href="#gen/${enc(id)}">${esc(id)}</a>` : '<span class="chip c-unknown">none</span>';
  function source(path, line) {
    if (!path) return "";
    const loc = line ? `${path}:${line}` : path;
    const vs = C.repo_abs ? ` <a href="vscode://file/${esc(C.repo_abs)}/${esc(path)}${line ? ":" + line : ""}" title="Open in VS Code">vscode</a>` : "";
    return `<span class="srcline"><a href="${esc(repoHref(path))}" class="mono">${esc(loc)}</a>${vs} <button class="copy" data-copy="${esc(loc)}" type="button">copy</button></span>`;
  }
  function copy(text) {
    const done = () => {};
    try { navigator.clipboard.writeText(text).then(done, () => fallback(text)); } catch (e) { fallback(text); }
  }
  function fallback(text) {
    const ta = document.createElement("textarea"); ta.value = text; document.body.appendChild(ta); ta.select();
    try { document.execCommand("copy"); } catch (e) { /* the text stays selected for the reader */ }
    ta.remove();
  }
  document.addEventListener("click", (ev) => {
    const b = ev.target.closest("[data-copy]");
    if (b) { copy(b.dataset.copy); b.textContent = "copied"; setTimeout(() => (b.textContent = "copy"), 900); }
  });
  function stopTimers() { timers.forEach(clearInterval); timers = []; }
  function img(src, alt, cls) { return src ? `<img loading="lazy" src="${esc(src)}" alt="${esc(alt || "")}"${cls ? ` class="${cls}"` : ""}>` : ""; }

  // ---- search ------------------------------------------------------------------------
  const haystack = new Map();
  for (const a of assetList) {
    const g = gens[a.generator] || {};
    haystack.set(a.id, [a.id, a.name, a.display, a.family, a.character_id, (a.tags || []).join(" "), a.generator, g.source, g.module,
      (a.uses || []).map((u) => u.path + " " + (u.row || "")).join(" "), (a.placements || []).map((p) => p.level + " " + p.field).join(" "),
      JSON.stringify(a.chargen || {}), (a.layout?.rows || []).map(r => r.name).join(" "), a.products.join(" "), (a.lineages || []).map((l) => l.lineage + " " + l.name).join(" "),
      a.authorship ? a.authorship.shares.map((x) => x.who).join(" ") : ""].join(" ").toLowerCase());
  }
  function matches(a) {
    const terms = state.q.toLowerCase().split(/\s+/).filter(Boolean);
    const h = haystack.get(a.id);
    return terms.every((t) => h.includes(t));
  }
  const used = (a) => (a.uses && a.uses.length) || (a.placements && a.placements.length);

  // ---- gallery -----------------------------------------------------------------------
  const PAGE = 240;
  function gallery() {
    const pool0 = assetList.filter(matches);
    const count = (key, fn) => { const m = new Map(); pool.forEach((a) => { const k = fn(a); m.set(k, (m.get(k) || 0) + 1); }); return m; };
    const group = relGroup(state.rel);
    const pool = group ? pool0.filter((a) => group.ids.has(a.id)) : pool0;
    const families = count("family", (a) => a.family), confs = count("conf", (a) => a.confidence);
    const tags = new Map(); pool.forEach((a) => (a.tags || []).forEach((t) => tags.set(t, (tags.get(t) || 0) + 1)));
    let list = pool.filter((a) => (!state.family.size || state.family.has(a.family)) && (!state.conf.size || state.conf.has(a.confidence))
      && (!state.tag.size || [...state.tag].every((t) => (a.tags || []).includes(t)))
      && (!state.use || (state.use === "used" ? used(a) : state.use === "unused" ? !used(a) : a.findings > 0)));
    // Of two sheets of one character, the one the game draws comes first.
    const older = (a) => (a.inspection_unit && C.inventory?.units[a.inspection_unit]?.game_use === "referenced" ? 0 : 1);
    const sorts = {
      name: (a, b) => a.display.localeCompare(b.display) || older(a) - older(b),
      frames: (a, b) => b.frames - a.frames,
      animations: (a, b) => b.animations - a.animations,
      bytes: (a, b) => b.bytes - a.bytes,
      uses: (a, b) => (b.placements || []).reduce((s, p) => s + p.count, 0) - (a.placements || []).reduce((s, p) => s + p.count, 0),
      findings: (a, b) => b.findings - a.findings,
    };
    list.sort(sorts[state.sort] || sorts.name);
    state.lastList = list.map((a) => a.id);
    const pages = Math.max(1, Math.ceil(list.length / PAGE));
    state.page = Math.min(state.page, pages - 1);
    const shown = list.slice(state.page * PAGE, (state.page + 1) * PAGE);
    const facet = (group, key, n, on) => `<div class="facet${on ? " on" : ""}" data-facet="${group}" data-key="${esc(key)}"><span>${group === "conf" ? chip(key) : esc(key)}</span><span class="n">${fmt(n)}</span></div>`;
    const famHtml = [...families.entries()].sort((a, b) => b[1] - a[1]).map(([k, n]) => facet("family", k, n, state.family.has(k))).join("");
    const confHtml = CONF.filter((k) => confs.has(k)).map((k) => facet("conf", k, confs.get(k), state.conf.has(k))).join("");
    const tagList = [...tags.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
    const tagHtml = tagList.slice(0, 24).concat(tagList.slice(24).filter(([k]) => state.tag.has(k))).map(([k, n]) => facet("tag", k, n, state.tag.has(k))).join("")
      + (tagList.length > 24 ? `<details><summary class="n">${tagList.length - 24} more</summary>${tagList.slice(24).filter(([k]) => !state.tag.has(k)).map(([k, n]) => facet("tag", k, n, false)).join("")}</details>` : "");
    const useHtml = [["used", "named by the game"], ["unused", "no known use"], ["findings", "has findings"]].map(([k, t]) =>
      `<div class="facet${state.use === k ? " on" : ""}" data-facet="use" data-key="${k}"><span>${t}</span></div>`).join("");
    app.innerHTML = `<div class="layout">
      <aside class="facets"><div><h3>Family</h3>${famHtml}</div><div><h3>Generator link</h3>${confHtml}</div><div><h3>Use</h3>${useHtml}</div>${tagHtml ? `<div><h3>Tag</h3>${tagHtml}</div>` : ""}</aside>
      <div>
        ${group ? `<div class="bar relbar">Related to ${assetLink(group.asset.id)}: <b>${esc(group.title)}</b> <button id="unrel" type="button" class="copy">show all assets</button></div>` : ""}
        <div class="bar">${fmt(list.length)} of ${fmt(assetList.length)} assets
          <label>sort <select id="sort">${Object.keys(sorts).map((k) => `<option${k === state.sort ? " selected" : ""}>${k}</option>`).join("")}</select></label>
          <label>size <input id="cardsize" type="range" min="100" max="260" value="${state.card}"></label>
        </div>
        <div class="grid" style="--card:${state.card}px">${shown.map(card).join("")}</div>
        ${pages > 1 ? `<div class="pager"><button id="prev" type="button"${state.page ? "" : " disabled"}>‹ prev</button> page ${state.page + 1} of ${pages} <button id="next" type="button"${state.page < pages - 1 ? "" : " disabled"}>next ›</button></div>` : ""}
      </div></div>`;
    app.querySelectorAll("[data-facet]").forEach((el) => el.addEventListener("click", () => {
      const { facet: g, key: k } = el.dataset;
      if (g === "use") state.use = state.use === k ? "" : k;
      else { const set = g === "family" ? state.family : g === "tag" ? state.tag : state.conf; set.has(k) ? set.delete(k) : set.add(k); }
      state.page = 0; gallery();
    }));
    const unrel = app.querySelector("#unrel");
    if (unrel) unrel.addEventListener("click", () => { state.rel = ""; state.page = 0; location.hash = "#gallery"; });
    app.querySelector("#sort").addEventListener("change", (e) => { state.sort = e.target.value; gallery(); });
    app.querySelector("#cardsize").addEventListener("input", (e) => { state.card = +e.target.value; app.querySelector(".grid").style.setProperty("--card", state.card + "px"); });
    const prev = app.querySelector("#prev"), next = app.querySelector("#next");
    if (prev) prev.addEventListener("click", () => { state.page--; gallery(); window.scrollTo(0, 0); });
    if (next) next.addEventListener("click", () => { state.page++; gallery(); window.scrollTo(0, 0); });
  }
  function card(a) {
    const counts = a.animations ? `${a.animations} anim · ${fmt(a.frames)} fr` : `${a.products.length} file${a.products.length > 1 ? "s" : ""}`;
    return `<a class="card" href="#asset/${enc(a.id)}"><div class="img checker">${img(a.cover, a.display)}</div>
      <div class="meta"><div class="nm">${esc(a.display)}${a.art_label && a.art_label !== a.display ? ` <code class="gen">${esc(a.art_label)}</code>` : ""}</div><div class="sub">${chip(a.confidence)} ${esc(a.family)} · ${counts}${a.findings ? ` · <span class="chip c-warn">${a.findings}</span>` : ""}${unitChip(a)}</div></div></a>`;
  }
  function unitChip(a) {
    const unit = C.inventory?.units[a.inspection_unit];
    return unit ? ` ${chip(unit.game_use)} ${chip(unit.status.state)}` : "";
  }

  // ---- asset detail ------------------------------------------------------------------
  function assetPanels(a) {
    const g = gens[a.generator] || null;
    const left = [], right = [];
    if (a.layout && a.layout.rows && a.layout.rows.length) left.push(animationBox(a));
    else if (a.id.startsWith("parallax:")) left.push(parallaxBox(a));
    else left.push(previewBox(a));
    if (a.portraits) left.push(portraitBox(a));
    if (a.id.startsWith("skin:") || a.id.startsWith("legacy_bg:") || a.id.startsWith("ref:")) left.push(filesGrid(a));
    if (a.parts || a.body_rig) left.push(partsBox(a));
    right.push(generatorBox(a, g));
    if (a.authorship) right.push(authorshipBox(a.authorship));
    right.push(usageBox(a));
    const finds = C.findings.filter((f) => f.asset === a.id || a.products.includes(f.subject));
    if (finds.length) right.push(`<section class="box"><h2>Findings (${finds.length})</h2>${findingTable(finds)}</section>`);
    right.push(productBox(a));
    return `<div class="cols"><div>${left.join("")}</div><div>${right.join("")}</div></div>`;
  }
  function detail(id) {
    const a = assets[id];
    if (!a) { app.innerHTML = `<p class="empty">No asset ${esc(id)}.</p>`; return; }
    app.innerHTML = `<div class="detail">
      ${navBar(a)}
      <h1>${esc(a.display)}</h1>
      <p class="lede">${fam(a.family)} ${chip(a.confidence)} <code>${esc(a.id)}</code>${a.character_id ? ` · character <code>${esc(a.character_id)}</code>` : ""}${a.weakest && a.weakest !== a.confidence ? ` · weakest file: ${chip(a.weakest)}` : ""}${unitChip(a)}</p>
      ${a.description ? `<p class="lede">${esc(a.description)}</p>` : ""}
      ${a.tags && a.tags.length ? `<div class="chips" style="margin-bottom:12px">${a.tags.map((t) => `<span>${esc(t)}</span>`).join("")}</div>` : ""}
      ${a.inspection_unit ? `<p><a href="#inventory/unit/${enc(a.inspection_unit)}">Inventory, coverage, and generation evidence</a></p>` : ""}
      ${relatedBox(a)}
      ${(a.lineages || []).map((l) => lineageBox(l, a)).join("")}
      ${assetPanels(a)}</div>`;
    wireAnimation(a); wireParallax(a);
  }

  function previewBox(a) {
    const main = a.products.map((p) => products[p]).find((p) => p.thumb && p.tier === "full") || null;
    const path = a.products.find((p) => products[p] === main);
    const href = main ? main.href : null;
    return `<section class="box"><h2>Preview</h2><div class="stage checker">${href && /\.(png|webp|jpe?g|gif|svg)$/i.test(href) ? `<img src="${esc(href)}" alt="">` : img(a.cover)}</div>
      ${main ? `<div class="controls">${main.size ? `${main.size[0]}×${main.size[1]} px · ` : ""}${mb(main.bytes)} · <a href="${esc(main.href)}">open ${esc(path)}</a></div>` : ""}</section>`;
  }

  function animationBox(a) {
    const rows = a.layout.rows;
    const start = Math.max(0, rows.findIndex((r) => (a.preview_row ? r.name === a.preview_row : ["idle", "rest", "stand"].includes(r.name))));
    return `<section class="box"><h2>Animations (${rows.length} · ${fmt(a.frames)} frames · frame ${a.layout.frame.join("×")})</h2>
      <div class="stage checker" id="stage"><canvas id="player"></canvas></div>
      <div class="controls"><button id="play" type="button">pause</button><button id="step" type="button">step</button>
        <label>view <select id="framing"><option value="subject">Body</option><option value="frame">Full animation frame</option></select></label>
        <label>mode <select id="playmode"><option value="once">play once / hold</option><option value="loop">loop</option><option value="death">death → corpse</option></select></label>
        <label>frame <input id="scrub" type="range" min="0" value="0"></label>
        <label>speed <select id="speed"><option>0.25</option><option>0.5</option><option selected>1</option><option>2</option></select></label>
        <label>scale <select id="scale"><option>1</option><option selected>2</option><option>3</option></select></label>
        <a id="pagelink" href="#">full-resolution page</a>
        <span id="frameinfo" class="mono"></span></div>
      <div class="anims" id="anims">${rows.map((r, i) => `<div class="anim${i === start ? " on" : ""}" data-row="${i}"><span>${esc(r.name)}${r.mirror_of ? " ⇋" : ""}</span><span class="n">${r.frames}${r.ms ? ` · ${r.ms}ms` : ""}</span></div>`).join("")}</div>
      <p class="controls">Body view uses one camera and foot baseline for every action. Full frame retains the authored room for movement, weapons, and effects. Metadata: <code>${esc(a.layout.schema)}</code> · framing: ${esc(a.layout.framing?.basis || 'not drawn')} · feet: ${esc(a.layout.framing?.feet_basis || 'unknown')}.</p>
      <h2 style="margin-top:12px">Pose grid</h2><div class="poses" id="poses">${rows.map((r, i) => `<div class="pose" data-row="${i}"><canvas width="1" height="1" data-strip="${esc(r.strip || "")}" data-subject-strip="${esc(r.subject_strip || "")}" data-frames="${r.frames}"></canvas>${esc(r.name)}</div>`).join("")}</div></section>`;
  }

  const stripCache = new Map();
  function loadStrip(src) {
    if (!stripCache.has(src)) stripCache.set(src, new Promise((ok, bad) => { const im = new Image(); im.onload = () => ok(im); im.onerror = bad; im.src = src; }));
    return stripCache.get(src);
  }
  function wireAnimation(a) {
    stopTimers();
    const canvas = document.getElementById("player");
    if (!canvas || !a.layout) return;
    const rows = a.layout.rows;
    let [cw, ch] = a.layout.framing?.cell || [128, 128], loadVersion = 0;
    let row = Math.max(0, rows.findIndex((r) => (a.preview_row ? r.name === a.preview_row : ["idle", "rest", "stand"].includes(r.name)))), frame = 0, playing = true, strip = null, speed = 1, scale = 2;
    const info = document.getElementById("frameinfo"), ctx = canvas.getContext("2d");
    function draw() {
      if (document.getElementById("player") !== canvas) return;
      canvas.width = cw * scale; canvas.height = ch * scale; ctx.imageSmoothingEnabled = true; ctx.imageSmoothingQuality = "high"; ctx.clearRect(0, 0, canvas.width, canvas.height);
      const columns = document.getElementById("framing").value === "subject" ? rows[row].subject_columns : rows[row].strip_columns;
      if (strip) ctx.drawImage(strip, (frame % columns) * cw, Math.floor(frame / columns) * ch, cw, ch, 0, 0, cw * scale, ch * scale);
      document.getElementById("scrub").max = Math.max(0, rows[row].frames - 1);
      document.getElementById("scrub").value = frame;
      document.getElementById("play").textContent = playing ? "pause" : "play";
      info.textContent = `${rows[row].name}  ${frame + 1}/${rows[row].frames}`;
    }
    function select(i) {
      const version = ++loadVersion, subject = document.getElementById("framing").value === "subject";
      [cw, ch] = subject ? a.layout.framing?.cell || [128, 128] : a.layout.cell || [96, 96];
      row = i; frame = 0; strip = null; playing = true;
      document.querySelectorAll("#anims .anim").forEach((el) => el.classList.toggle("on", +el.dataset.row === i));
      const src = subject ? rows[i].subject_strip : rows[i].strip;
      if (src) loadStrip(src).then((im) => { if (version === loadVersion) { strip = im; draw(); } }, () => { info.textContent = "preview not drawn: run the build without --no-previews"; });
      restart();
    }
    function restart() {
      stopTimers();
      const ms = Math.max(30, (rows[row].ms || 100) / speed);
      timers.push(setInterval(() => { if (playing && strip) { const mode = document.getElementById("playmode").value;
        if (frame + 1 < rows[row].frames) frame += 1;
        else if (mode === "loop") frame = 0;
        else if (mode === "death" && rows[row].name === "death") {
          const corpse = rows.findIndex(r => r.name === "death_idle");
          if (corpse >= 0) { select(corpse); return; }
          playing = false;
        } else playing = false;
        draw(); } }, ms));
    }
    document.getElementById("anims").addEventListener("click", (e) => { const el = e.target.closest("[data-row]"); if (el) select(+el.dataset.row); });
    document.getElementById("poses").addEventListener("click", (e) => { const el = e.target.closest("[data-row]"); if (el) { select(+el.dataset.row); window.scrollTo({ top: 0 }); } });
    document.getElementById("play").addEventListener("click", () => {
      if (!playing && document.getElementById("playmode").value === "death") {
        const death = rows.findIndex(r => r.name === "death");
        if (death >= 0) { select(death); return; }
      }
      playing = !playing;
      if (playing && frame === rows[row].frames - 1) frame = 0;
      draw();
    });
    document.getElementById("step").addEventListener("click", () => { playing = false; document.getElementById("play").textContent = "play"; frame = (frame + 1) % Math.max(1, rows[row].frames); draw(); });
    document.getElementById("scrub").addEventListener("input", e => { playing = false; frame = +e.target.value; draw(); });
    document.getElementById("playmode").addEventListener("change", e => { const death = rows.findIndex(r => r.name === "death"); select(e.target.value === "death" && death >= 0 ? death : row); });
    document.getElementById("speed").addEventListener("change", (e) => { speed = +e.target.value; restart(); });
    document.getElementById("scale").addEventListener("change", (e) => { scale = +e.target.value; draw(); });
    const page0 = a.products.map((p) => products[p]).find((p) => p.role === "sheet_page:0" && p.tier === "full");
    const pageLink = document.getElementById("pagelink");
    if (page0) pageLink.href = page0.href; else pageLink.remove();
    select(row);
    // Pose grid: the middle frame of each animation, drawn when it scrolls into view.
    const io = new IntersectionObserver((entries) => entries.forEach((en) => {
      if (!en.isIntersecting) return;
      const c = en.target; io.unobserve(c);
      const subject = document.getElementById("framing").value === "subject";
      const src = subject ? c.dataset.subjectStrip : c.dataset.strip;
      const [w, h] = subject ? a.layout.framing?.cell || [128, 128] : a.layout.cell || [96, 96];
      if (!src) return;
      loadStrip(src).then((im) => {
        if (!c.isConnected || subject !== (document.getElementById("framing").value === "subject")) return;
        const n = +c.dataset.frames || 1, mid = Math.floor(n / 2);
        const row = rows[+c.parentElement.dataset.row], columns = subject ? row.subject_columns : row.strip_columns;
        c.width = w; c.height = h; c.getContext("2d").drawImage(im, (mid % columns) * w, Math.floor(mid / columns) * h, w, h, 0, 0, w, h);
      }, () => {});
    }));
    document.querySelectorAll("#poses canvas").forEach((c) => { c.classList.add("checker"); io.observe(c); });
    document.getElementById("framing").addEventListener("change", () => {
      select(row);
      document.querySelectorAll("#poses canvas").forEach(c => { c.width = 1; c.height = 1; io.observe(c); });
    });
  }

  function parallaxBox(a) {
    const layers = a.products.map((p) => [p, products[p]]).filter(([, p]) => p.role.startsWith("layer:"));
    const tiers = [...new Set(layers.map(([, p]) => p.tier))];
    return `<section class="box"><h2>Parallax layers</h2>
      <div class="stage checker"><canvas id="sky" width="960" height="540"></canvas></div>
      <div class="controls"><label>camera <input id="cam" type="range" min="-1" max="1" step="0.01" value="0"></label>
        <label>tier <select id="tier">${tiers.map((t) => `<option${t === "full" ? " selected" : ""}>${esc(t)}</option>`).join("")}</select></label></div>
      <div class="layers" id="layers">${Object.keys(PARALLAX).concat(["flat"]).map((name) => {
        const found = layers.find(([, p]) => p.role === "layer:" + name && p.tier === "full");
        return found ? `<label><input type="checkbox" data-layer="${name}"${name === "flat" ? "" : " checked"}> ${esc(name)} <span class="mono">${found[1].size ? found[1].size.join("×") : ""}</span> ${PARALLAX[name] !== undefined ? `<span class="n">factor ${PARALLAX[name]}</span>` : "(the picture a tier with parallax off draws)"}</label>` : "";
      }).join("")}</div>
      <p class="controls">Layers are tiled and offset by the camera times each layer's factor (the factors of <code>backgrounds/scenes.py: RUNTIME</code>); the game also scales each panel, so this is an approximation.</p></section>`;
  }
  function wireParallax(a) {
    const canvas = document.getElementById("sky");
    if (!canvas) return;
    const ctx = canvas.getContext("2d"), cache = new Map();
    const layerProduct = (name, tier) => a.products.map((p) => products[p]).find((p) => p.role === "layer:" + name && p.tier === tier);
    function get(src) {
      if (!cache.has(src)) cache.set(src, new Promise((ok) => { const im = new Image(); im.onload = () => ok(im); im.onerror = () => ok(null); im.src = src; }));
      return cache.get(src);
    }
    async function draw() {
      const cam = +document.getElementById("cam").value, tier = document.getElementById("tier").value;
      const on = [...document.querySelectorAll("#layers input:checked")].map((el) => el.dataset.layer);
      const order = Object.keys(PARALLAX).concat(["flat"]).filter((n) => on.includes(n));
      const ims = await Promise.all(order.map((n) => { const p = layerProduct(n, tier); return p ? get(p.href) : Promise.resolve(null); }));
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      order.forEach((name, i) => {
        const im = ims[i]; if (!im) return;
        const h = canvas.height, w = im.width * (h / im.height), f = PARALLAX[name] ?? 0;
        let x = -((cam * f * canvas.width) % w); if (x > 0) x -= w;
        for (; x < canvas.width; x += w) ctx.drawImage(im, x, 0, w, h);
      });
    }
    ["cam", "tier"].forEach((id) => document.getElementById(id).addEventListener("input", draw));
    document.getElementById("layers").addEventListener("change", draw);
    draw();
  }

  function portraitBox(a) {
    const clips = Object.entries(a.portraits.clips);
    return `<section class="box"><h2>Portraits (${clips.length} clip${clips.length > 1 ? "s" : ""})</h2><div class="thumbs">${clips.map(([n, c]) =>
      `<figure><div class="checker">${img(c.thumb, n)}</div><figcaption>${esc(n)}${n === a.portraits.default ? " (default)" : ""}${c.frames > 1 ? ` · ${c.frames} fr` : ""}</figcaption></figure>`).join("")}</div></section>`;
  }
  function filesGrid(a) {
    const files = a.products.map((p) => [p, products[p]]).filter(([, p]) => p.thumb && p.tier === "full");
    return `<section class="box"><h2>Files (${files.length})</h2><div class="thumbs">${files.map(([path, p]) =>
      `<figure><div class="checker"><a href="${esc(p.href)}">${img(p.thumb, path)}</a></div><figcaption>${esc(p.role.replace(/^(part|layer):/, ""))}${p.size ? ` · ${p.size.join("×")}` : ""} ${chip(p.provenance.confidence)}</figcaption></figure>`).join("")}</div></section>`;
  }
  function partsBox(a) {
    let html = `<section class="box"><h2>Parts and rig</h2>`;
    if (a.parts) html += `<p>${fmt(a.parts.parts)} part rasters, ${fmt(a.parts.clips)} clips. The published part names are anonymous; these are the named tracks the clips draw:</p><div class="chips">${a.parts.tracks.map((t) => `<span>${esc(t)}</span>`).join("")}</div>`;
    if (a.body_rig) {
      const kids = {}; a.body_rig.joints.forEach(([n, p]) => (kids[p || ""] = kids[p || ""] || []).push(n));
      const tree = (p, d) => (kids[p] || []).map((n) => `<div style="padding-left:${d * 14}px" class="mono">${esc(n)}</div>${tree(n, d + 1)}`).join("");
      html += `<h2 style="margin-top:10px">Body rig joints</h2>${tree("", 0)}`;
    }
    return html + `</section>`;
  }

  // ---- related assets and the way through them ---------------------------------------
  // `related` comes from the index (`asset_index/related.py`): the other forms of
  // the character, its lineage, the art made the same way. The type is the family.
  function relGroup(key) {
    if (!key) return null;
    const cut = key.lastIndexOf("|"), a = assets[key.slice(0, cut)], g = a && (a.related || [])[+key.slice(cut + 1)];
    return g ? { asset: a, title: g.title, ids: new Set([a.id, ...g.assets]) } : null;
  }
  function mini(id) {
    const r = assets[id];
    // The target id under the name: two forms of one character share a display name.
    const sub = r.name && r.name.toLowerCase() !== r.display.toLowerCase() ? `<code>${esc(r.name)}</code>` : "";
    const now = r && r.generation && r.generation.state === "current" ? " (in the game)" : r && r.generation ? " (older)" : "";
    return r ? `<a class="mini" href="#asset/${enc(id)}" title="${esc(r.family + " · " + id + now)}"><div class="checker">${img(r.cover, r.display)}</div><span>${esc(r.display)}</span>${sub}</a>` : "";
  }
  const STRIP = 24;
  function relatedBox(a) {
    const groups = (a.related || []).map((g, i) => [g, i]);
    const row = ([g, i]) => `<div class="relrow"><div class="relhead"><b>${esc(g.title)}</b> <span class="n">${fmt(g.assets.length)}</span>
        <a href="#gallery?rel=${enc(a.id + "|" + i)}">open in gallery</a></div>
      <div class="strip">${g.assets.slice(0, STRIP).map(mini).join("")}${g.assets.length > STRIP ? `<a class="mini more" href="#gallery?rel=${enc(a.id + "|" + i)}">+${fmt(g.assets.length - STRIP)} more</a>` : ""}</div></div>`;
    const near = groups.filter(([g]) => g.kind !== "tag"), tags = groups.filter(([g]) => g.kind === "tag");
    const type = `<a href="#gallery?family=${enc(a.family)}">every ${esc(a.family)} asset (${fmt(assetList.filter((x) => x.family === a.family).length)})</a>`;
    const traits = a.chargen ? ` · chargen ${Object.entries(a.chargen).map(([k, v]) => `${esc(k)} <code>${esc(v)}</code>`).join(", ")}` : "";
    return `<section class="box related"><h2>Related</h2>
      ${near.length ? `<div class="relrows">${near.map(row).join("")}</div>` : `<p class="empty">No other sheet shares a form, a game character, a generator or a chargen trait with this one${(a.lineages || []).length ? "; its own lineage is below" : ""}.</p>`}
      ${tags.length ? `<details${near.length ? "" : " open"}><summary>Same tags: ${tags.map(([g]) => esc(g.title.replace(/^Tagged /, ""))).join(", ")}</summary><div class="relrows">${tags.map(row).join("")}</div></details>` : ""}
      <p class="controls">Same type: ${type}${traits}. <kbd>←</kbd> <kbd>→</kbd> step through the list above.</p></section>`;
  }
  // The list that prev and next step through: the gallery's last list when it
  // holds this asset, else the asset's family by name.
  function navList(a) {
    if (state.lastList.includes(a.id)) return { ids: state.lastList, label: state.rel ? "in the related list" : "in the gallery list" };
    return { ids: assetList.filter((x) => x.family === a.family).sort((x, y) => x.display.localeCompare(y.display)).map((x) => x.id), label: `in ${a.family}` };
  }
  function navBar(a) {
    const nav = navList(a), i = nav.ids.indexOf(a.id), n = nav.ids.length;
    const prev = nav.ids[(i - 1 + n) % n], next = nav.ids[(i + 1) % n];
    return `<div class="bar"><a href="#gallery">‹ gallery</a> · <a id="navprev" href="#asset/${enc(prev)}" title="${esc(assets[prev].display)} (←)">‹ prev</a>
      ${fmt(i + 1)} of ${fmt(n)} ${esc(nav.label)} <a id="navnext" href="#asset/${enc(next)}" title="${esc(assets[next].display)} (→)">next ›</a></div>`;
  }

  // ---- lineage and authorship (the design history) -------------------------------------
  const CONFIDENCE_CHIP = { high: "verified", medium: "declared", low: "inferred", none: "unknown" };
  const confChip = (c) => `<span class="chip c-${CONFIDENCE_CHIP[c] || "info"}" title="authorship confidence">${esc(c)}</span>`;
  const pct = (n, total) => total ? Math.round(100 * n / total) + "%" : "";
  function shareText(shares) {
    const total = (shares || []).reduce((s, x) => s + x.lines, 0);
    return (shares || []).slice(0, 3).map((x) => `${esc(x.who)} ${pct(x.lines, total)}`).join(", ");
  }
  function lineageBox(l, a) {
    // An era's target is measured (`target`) or declared (an era `id`). The
    // last era of a target is that target's asset today.
    const targetOf = (e) => e.target || (e.ids && e.ids.length === 1 ? e.ids[0] : "");
    const last = {};
    const curatedEras = l.eras || [];
    curatedEras.forEach((e, i) => { if (targetOf(e)) last[targetOf(e)] = i; });
    const eras = curatedEras.map((e, i) => {
      const now = last[targetOf(e)] === i ? byTarget[targetOf(e)] : null;
      const gen = e.generator;
      const lines = [`<b>${esc(e.label)}</b>`, `${esc((e.date || "").slice(0, 10))} · <code>${esc(e.commit.slice(0, 9))}</code>${targetOf(e) ? ` · <code>${esc(targetOf(e))}</code>` : ""}`];
      if (e.pin) lines.push(`commit: ${esc(e.pin.label)}`);
      if (gen && gen.lines) {
        if (gen.parts.code.length) lines.push(`code: ${shareText(gen.parts.code)}`);
        if (gen.parts.art.length) lines.push(`art: ${shareText(gen.parts.art)}`);
        lines.push(confChip(gen.confidence));
      } else if (!e.in_store) lines.push("not in the history store");
      if (e.drawn_by) lines.push(`drawn by: ${esc(e.drawn_by.join(", "))}`);
      if (now) lines.push(now === a?.id ? '<span class="chip c-info">this asset</span>' : `today: ${assetLink(now)}`);
      const still = e.still ? `<a href="${esc(e.still)}" target="_blank" rel="noopener">${img(e.preview || e.still, e.label)}</a>` : '<span class="controls">Historical preview not cached</span>';
      return `<figure title="${esc(e.note || "")}"><div class="checker">${still}</div><figcaption>${lines.join("<br>")}</figcaption></figure>`;
    }).join("");
    const variations = (l.tracks || []).flatMap(track => (track.variations || []).map(v => ({...v, target: v.target || track.track})));
    const dated = entries => entries.sort((x, y) => (Date.parse(x.date) || 0) - (Date.parse(y.date) || 0));
    const unique = entries => [...new Map(entries.map(e => [(e.sha || e.commit).slice(0, 12) + ':' + (e.target || e.ids?.join('+') || ''), e])).values()];
    const all = dated(unique([...variations, ...curatedEras.filter(e => e.still).map(e => ({...e, target: targetOf(e), subject: e.note, by: e.pin?.label, holistic: true}))]));
    const major = all.filter(e => e.holistic && e.still);
    const visible = major.length ? major : all.filter(e => e.still);
    const current = a ? `<figure class="history-current"><div class="checker">${a.cover ? `<a href="#asset/${enc(a.id)}">${img(a.cover, a.display)}</a>` : '<span class="controls">Current preview unavailable</span>'}</div><figcaption><b>${a.identities?.length ? 'Current published look' : 'Selected art'}</b><br>${esc(a.display)}${C.built_at ? `<br>Inspected ${esc(C.built_at.slice(0, 10))}` : ''}</figcaption></figure>` : '';
    return `<section class="box visual-lineage" data-lineage="${esc(l.lineage || '')}"><h2>Character lineage: ${esc(l.name)}</h2>
      ${l.summary ? `<p>${esc(l.summary)}</p>` : ''}<p class="controls">Recorded looks, oldest first. Open an image to inspect the original.</p>
      <div class="thumbs lineage-previews history-summary">${visible.map(variationFig).join('')}${current}</div>
      ${!visible.length ? '<p class="empty">No earlier snapshots are cached for this lineage.</p>' : ''}
      ${all.length > visible.length ? `<details><summary>All recorded variations (${all.length})</summary><div class="thumbs lineage-previews">${all.map(variationFig).join('')}</div></details>` : ''}
      ${curatedEras.length ? `<details><summary>Curated milestones (${curatedEras.length})</summary><div class="thumbs lineage-previews">${eras}</div></details>` : ''}
      ${l.targets?.length ? `<p class="controls">Art targets: ${l.targets.map(t => `<code>${esc(t)}</code>`).join(' · ')}</p>` : ''}</section>`;
  }
  // The survey's variations of one track (`design_history/variations.yaml`):
  // the holistic looks, and every variation behind a toggle.
  function variationFig(v) {
    const lines = [v.label ? `<b>${esc(v.label)}</b>` : '', `${esc((v.date || '').slice(0, 10))} · <code>${esc(v.commit.slice(0, 9))}</code>`, esc(v.target), esc(v.by)].filter(Boolean);
    return `<figure title="${esc(v.subject || v.note || '')}"><div class="checker">${v.still ? `<a href="${esc(v.still)}" target="_blank" rel="noopener">${img(v.preview || v.still, v.label || v.subject || 'Historical look')}</a>` : '<span class="controls">Historical preview not cached</span>'}</div><figcaption>${lines.join("<br>")}</figcaption></figure>`;
  }
  window.ArtLineage = {render: lineageBox};
  function authorshipBox(w) {
    const rows = [];
    for (const part of ["code", "art"]) {
      const shares = w.parts[part] || [], total = shares.reduce((s, x) => s + x.lines, 0);
      shares.forEach((x) => rows.push(`<tr><td>${part}</td><td>${esc(x.who)}</td><td class="num">${fmt(x.lines)}</td><td class="num">${pct(x.lines, total)}</td><td>${esc(x.basis)}</td><td class="mono">${x.commits.map((c) => esc(c.slice(0, 9))).join(" ")}</td></tr>`));
    }
    const files = Object.entries(w.files);
    return `<section class="box"><h2>Authorship of the generator</h2>
      <p>${confChip(w.confidence)} ${fmt(w.lines)} lines in ${files.length} file${files.length === 1 ? "" : "s"} at renderer <code>${esc(w.commit.slice(0, 10))}</code> (files from ${esc(w.files_from || w.found_by)}). Each line's commit names its author: a ledger session (measured), a <code>Co-Authored-By</code> trailer (declared), a model named in the message, or, when no model signed the commit, GPT (no trailer: Claude Code signs its commits, GPT does not).</p>
      ${rows.length ? `<div class="tablebox"><table><tr><th>part</th><th>author</th><th class="num">lines</th><th class="num">share</th><th>basis</th><th>commits</th></tr>${rows.join("")}</table></div>` : '<p class="empty">No generator file found.</p>'}
      ${files.length ? `<details><summary>${files.length} files</summary>${files.map(([f, n]) => `<div>${source("tools/ambition_sprite2d_renderer/" + f)} <span class="controls">${fmt(n)} lines</span></div>`).join("")}</details>` : ""}</section>`;
  }

  function generatorBox(a, g) {
    const main = a.products.map((p) => products[p]).filter((p) => p.tier === "full");
    const madeBy = new Map();
    main.forEach((p) => { const m = p.provenance.made_by; if (m) madeBy.set(m.callable, m); });
    let html = `<section class="box"><h2>Generator</h2>`;
    if (!g) {
      html += `<p>${chip(a.confidence)} ${a.confidence === "reference" ? "Authored or imported art: no generator makes it." : "No generator is known for this asset. That is a finding: see the files below."}</p></section>`;
      return html;
    }
    html += `<dl class="kv"><dt>generator</dt><dd>${genLink(a.generator)} ${chip(a.confidence)}</dd>
      <dt>family</dt><dd>${esc(g.family || "")}${g.kind ? ` · ${esc(g.kind)}` : ""}${g.category ? ` · ${esc(g.category)}` : ""}</dd>
      ${g.source ? `<dt>source</dt><dd>${source(g.source)}</dd>` : ""}
      ${g.module_source && g.module_source !== g.source ? `<dt>module</dt><dd>${source(g.module_source)}</dd>` : g.module ? `<dt>module</dt><dd><code>${esc(g.module)}</code></dd>` : ""}
      ${[...madeBy.values()].length ? `<dt>drawn by</dt><dd>${[...madeBy.values()].map((m) => `<div><code>${esc(m.callable)}</code> ${source(m.source, m.line)}</div>`).join("")}</dd>` : ""}
      ${g.command ? `<dt>regenerate</dt><dd><code>${esc(g.command)}</code> <button class="copy" data-copy="${esc(g.command)}" type="button">copy</button></dd>` : ""}
      ${g.basis ? `<dt>record</dt><dd>${esc(g.basis)}${g.written_at ? ` · ${esc(g.written_at)}` : ""}${g.renderer && g.renderer.commit ? ` · renderer <code>${esc(g.renderer.commit.slice(0, 10))}</code>${g.renderer.dirty ? " (uncommitted changes)" : ""}` : ""}</dd>` : ""}
      ${g.note ? `<dt>note</dt><dd>${esc(g.note)}</dd>` : ""}
      ${g.inputs && g.inputs.length ? `<dt>inputs</dt><dd><details><summary>${g.inputs.length} files the render read</summary>${g.inputs.map((p) => `<div>${source(p)}</div>`).join("")}</details></dd>` : ""}
    </dl>`;
    const ev = main.map((p) => p.provenance.evidence).filter(Boolean)[0];
    if (ev) html += `<p class="controls">Evidence: ${esc(ev.join("; "))}</p>`;
    return html + `</section>`;
  }

  function usageBox(a) {
    const uses = a.uses || [], places = a.placements || [];
    if (!uses.length && !places.length) return `<section class="box"><h2>Used by</h2><p class="empty">No content row, LDtk placement or Rust literal names this asset. A Rust constant built at run time would not show here.</p></section>`;
    let html = `<section class="box"><h2>Used by</h2>`;
    if (places.length) {
      const total = places.reduce((s, p) => s + p.count, 0);
      html += `<p>${fmt(total)} placement${total > 1 ? "s" : ""} in ${new Set(places.map((p) => p.world)).size} world(s)</p><div class="tablebox"><table><tr><th>world</th><th>level</th><th>field</th><th class="num">n</th></tr>${places.map((p) =>
        `<tr><td><a href="${esc(repoHref(p.world))}" title="${esc(p.world)}">${esc(p.world.split("/").pop())}</a></td><td><code>${esc(p.level)}</code></td><td><code>${esc(p.field)}</code></td><td class="num">${p.count}</td></tr>`).join("")}</table></div>`;
    }
    if (uses.length) html += `<div class="tablebox" style="margin-top:8px"><table><tr><th>where</th><th>row</th><th>names</th></tr>${uses.map((u) =>
      `<tr><td>${source(u.path, u.line)}${u.kind === "rust literal" ? ' <span class="chip c-info">rust literal</span>' : ""}</td><td><code>${esc(u.row || "")}</code></td><td class="mono">${esc(u.ref)}</td></tr>`).join("")}</table></div>`;
    return html + `</section>`;
  }

  function productBox(a) {
    const rows = a.products.map((p) => [p, products[p]]).sort((x, y) => (x[1].tier === "full" ? 0 : 1) - (y[1].tier === "full" ? 0 : 1) || x[0].localeCompare(y[0]));
    return `<section class="box"><h2>Files (${rows.length})</h2><div class="tablebox"><table><tr><th>file</th><th>role</th><th>tier</th><th class="num">size</th><th>link</th></tr>${rows.map(([path, p]) =>
      `<tr><td><a href="${esc(p.href)}" class="mono">${esc(path)}</a></td><td>${esc(p.role)}</td><td>${esc(p.tier)}</td><td class="num">${p.size ? p.size.join("×") + "<br>" : ""}${mb(p.bytes)}</td><td>${chip(p.provenance.confidence)}${p.provenance.made_by ? `<br><code>${esc(p.provenance.made_by.callable.split(":").pop())}</code>` : ""}</td></tr>`).join("")}</table></div></section>`;
  }

  // ---- generators ----------------------------------------------------------------------
  function generatorList() {
    const terms = state.q.toLowerCase().split(/\s+/).filter(Boolean);
    const list = Object.entries(gens).filter(([id, g]) => terms.every((t) => (id + " " + (g.source || "") + " " + (g.family || "")).toLowerCase().includes(t)))
      .sort((x, y) => (y[1].assets || []).length - (x[1].assets || []).length || x[0].localeCompare(y[0]));
    app.innerHTML = `<div class="bar">${fmt(list.length)} generators. A generator is a sprite target, a parallax theme, a sky part, a terrain skin or a fixed tool.</div>
      <div class="tablebox"><table><tr><th>generator</th><th>family</th><th>source</th><th class="num">assets</th><th>record</th></tr>${list.slice(0, 800).map(([id, g]) =>
        `<tr><td>${genLink(id)}</td><td>${esc(g.family || "")}</td><td>${source(g.source)}</td><td class="num">${(g.assets || []).length}</td><td>${esc(g.basis || "")}</td></tr>`).join("")}</table></div>`;
  }
  function generatorDetail(id) {
    const g = gens[id];
    if (!g) { app.innerHTML = `<p class="empty">No generator ${esc(id)}.</p>`; return; }
    const made = (g.assets || []).map((x) => assets[x]).filter(Boolean);
    app.innerHTML = `<div class="detail"><div class="bar"><a href="#generators">‹ generators</a></div><h1>${esc(id)}</h1>
      <div class="cols"><div><section class="box"><h2>Makes ${made.length} asset${made.length === 1 ? "" : "s"}</h2><div class="grid" style="--card:130px">${made.map(card).join("")}</div></section></div>
      <div>${generatorBox({ products: made.flatMap((x) => x.products), confidence: made[0] ? made[0].confidence : "declared", generator: id }, g)}</div></div></div>`;
  }

  // ---- findings -------------------------------------------------------------------------
  function findingTable(list) {
    return `<div class="tablebox"><table><tr><th>finding</th><th>subject</th><th>detail</th></tr>${list.map((f) =>
      `<tr><td><span class="chip c-${esc(f.severity)}">${esc(f.code)}</span></td><td>${f.asset ? assetLink(f.asset, f.subject) : f.generator ? genLink(f.generator) : `<code>${esc(f.subject)}</code>`}</td><td>${esc(f.message)}${f.inputs ? `<details><summary>inputs</summary>${f.inputs.map((p) => `<div>${source(p)}</div>`).join("")}</details>` : ""}</td></tr>`).join("")}</table></div>`;
  }
  function findings() {
    const terms = state.q.toLowerCase().split(/\s+/).filter(Boolean);
    const codes = {};
    C.findings.forEach((f) => (codes[f.code] = (codes[f.code] || 0) + 1));
    const pick = state.code || "";
    const list = C.findings.filter((f) => (!pick || f.code === pick) && terms.every((t) => (f.code + " " + f.subject + " " + f.message).toLowerCase().includes(t)));
    app.innerHTML = `<div class="bar">${fmt(list.length)} of ${fmt(C.findings.length)} findings
      <label>kind <select id="code"><option value="">all</option>${Object.entries(codes).sort((a, b) => b[1] - a[1]).map(([c, n]) => `<option value="${esc(c)}"${c === pick ? " selected" : ""}>${esc(c)} (${n})</option>`).join("")}</select></label>
      <button id="csv" type="button">copy as CSV</button></div>${findingTable(list.slice(0, 1500))}${list.length > 1500 ? `<p class="empty">Showing 1,500; narrow the search.</p>` : ""}`;
    app.querySelector("#code").addEventListener("change", (e) => { state.code = e.target.value; findings(); });
    app.querySelector("#csv").addEventListener("click", (e) => {
      const cell = (v) => { v = String(v ?? ""); if (/^[=+@-]/.test(v)) v = "'" + v; return '"' + v.replace(/"/g, '""') + '"'; };
      copy(["code,severity,subject,message"].concat(list.map((f) => [f.code, f.severity, f.subject, f.message].map(cell).join(","))).join("\n"));
      e.target.textContent = "copied";
    });
  }

  // ---- coverage ---------------------------------------------------------------------
  function coverage() {
    const rows = Object.entries(C.coverage);
    const total = {}; rows.forEach(([, c]) => Object.entries(c).forEach(([k, n]) => (total[k] = (total[k] || 0) + n)));
    const bar = (c) => { const n = Object.values(c).reduce((s, x) => s + x, 0) || 1; return `<div class="stack">${CONF.filter((k) => c[k]).map((k) => `<span class="s-${k}" style="width:${(100 * c[k]) / n}%" title="${k}: ${c[k]}"></span>`).join("")}</div>`; };
    const sum = (c) => Object.values(c).reduce((s, x) => s + x, 0);
    const linked = sum(total) - (total.unknown || 0);
    const top = (title, list, fmtRow) => `<section class="box"><h2>${title}</h2><div class="tablebox"><table>${list.map(fmtRow).join("")}</table></div></section>`;
    const sheets = assetList.filter((a) => a.animations);
    const placed = (a) => (a.placements || []).reduce((s, p) => s + p.count, 0);
    app.innerHTML = `<p>${fmt(linked)} of ${fmt(sum(total))} files (${((100 * linked) / Math.max(1, sum(total))).toFixed(1)}%) link to a generator or are reference art. ${fmt(total.unknown || 0)} have no known generator.</p>
      <div class="legend">${CONF.map((k) => `<span>${chip(k)} ${esc(CONF_TEXT[k])}</span>`).join("")}</div>
      <div class="coverage">${rows.map(([root, c]) => `<div class="cov-row"><code>${esc(root)}</code>${bar(c)}<span class="num">${fmt(sum(c))}</span></div>`).join("")}
        <div class="cov-row"><strong>all files</strong>${bar(total)}<span class="num">${fmt(sum(total))}</span></div></div>
      <div class="tops">
        ${top("Most frames", sheets.sort((a, b) => b.frames - a.frames).slice(0, 12), (a) => `<tr><td>${assetLink(a.id)}</td><td class="num">${fmt(a.frames)} fr · ${a.animations} anim</td></tr>`)}
        ${top("Largest on disk", assetList.slice().sort((a, b) => b.bytes - a.bytes).slice(0, 12), (a) => `<tr><td>${assetLink(a.id)}</td><td class="num">${mb(a.bytes)}</td></tr>`)}
        ${top("Most placed in worlds", assetList.filter(placed).sort((a, b) => placed(b) - placed(a)).slice(0, 12), (a) => `<tr><td>${assetLink(a.id)}</td><td class="num">${fmt(placed(a))}</td></tr>`)}
        ${top("Generators by assets made", Object.entries(gens).filter(([, g]) => (g.assets || []).length).sort((a, b) => b[1].assets.length - a[1].assets.length).slice(0, 12), ([id, g]) => `<tr><td>${genLink(id)}</td><td class="num">${g.assets.length}</td></tr>`)}
      </div>`;
  }

  // ---- routing ------------------------------------------------------------------------
  function route() {
    stopTimers();
    const raw = location.hash.slice(1);
    if (raw.startsWith("gallery?")) {
      // A link into a filtered gallery: `#gallery?family=character&tag=goblin&rel=<asset>|<group>`.
      const qs = new URLSearchParams(raw.slice(8));
      state.family = new Set(qs.getAll("family")); state.tag = new Set(qs.getAll("tag")); state.conf = new Set();
      state.use = ""; state.rel = qs.get("rel") || ""; state.q = ""; q.value = ""; state.page = 0;
      history.replaceState(null, "", "#gallery");
    }
    const h = decodeURIComponent(location.hash.slice(1));
    const [view, ...rest] = h.split("/");
    const id = rest.join("/");
    document.querySelectorAll(".top .tabs a").forEach((a) => a.classList.toggle("on", a.dataset.view === (view === "asset" ? "gallery" : view === "gen" ? "generators" : view === "character" ? "characters" : view || "characters")));
    if (view === "characters" || view === "character" || !view) window.renderArtCharacters(view === "character" ? id : "", {assetPanels, wireAnimation, lineageBox});
    else if (view === "inventory") window.renderArtInventory(id || "current");
    else if (view === "reviews") window.ArtReviews.backlog();
    else if (view === "asset") detail(id);
    else if (view === "gen") generatorDetail(id);
    else if (view === "generators") generatorList();
    else if (view === "findings") findings();
    else if (view === "coverage") coverage();
    else gallery();
  }
  const q = document.getElementById("q");
  let debounce = 0;
  q.addEventListener("input", () => {
    clearTimeout(debounce);
    debounce = setTimeout(() => {
      state.q = q.value; state.page = 0;
      const v = location.hash.slice(1).split("/")[0];
      if (v === "character") location.hash = "#characters";
      else if (v === "asset" || v === "gen") location.hash = "#gallery"; else route();
    }, 120);
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "/" && document.activeElement !== q) { e.preventDefault(); q.focus(); }
    const typing = /^(INPUT|SELECT|TEXTAREA)$/.test((document.activeElement || {}).tagName || "");
    if (!typing && !e.altKey && !e.ctrlKey && !e.metaKey && (e.key === "ArrowLeft" || e.key === "ArrowRight")) {
      const link = document.getElementById(e.key === "ArrowLeft" ? "navprev" : "navnext");
      if (link) { e.preventDefault(); location.hash = link.getAttribute("href"); }
    }
    if (e.key === "Escape" && document.activeElement === q) { q.value = ""; state.q = ""; route(); }
  });
  const themeBtn = document.getElementById("theme");
  try { const t = localStorage.getItem("asset-inspector-theme"); if (t) document.documentElement.dataset.theme = t; } catch (e) { /* no storage: system theme */ }
  themeBtn.addEventListener("click", () => {
    const dark = document.documentElement.dataset.theme ? document.documentElement.dataset.theme === "dark" : matchMedia("(prefers-color-scheme: dark)").matches;
    document.documentElement.dataset.theme = dark ? "light" : "dark";
    try { localStorage.setItem("asset-inspector-theme", document.documentElement.dataset.theme); } catch (e) { /* ignore */ }
  });
  const stamp = document.getElementById("stamp");
  stamp.textContent = `Built ${C.built_at} at ${String(C.repo_commit).slice(0, 10)} (renderer ${String(C.renderer_commit).slice(0, 10)}) in ${C.build_seconds}s · ${fmt(assetList.length)} assets · ${fmt(Object.keys(products).length)} files · ${fmt(C.findings.length)} findings. Refresh: ./scripts/asset_inspector.py`;
  // What the index could not read, with its fix. The rest of the page still works.
  const profile = C.inventory?.profile;
  if (profile && profile.state !== "available") {
    const notice = document.createElement("div");
    notice.className = "stamp stale";
    notice.innerHTML = `<b>The character catalog is unavailable</b>, so Characters is empty. The Raw gallery, Generators, Inventory and lineages still show the art on disk.`
      + (profile.warnings || C.inventory.warnings || []).map((w) => `<pre>${esc(w)}</pre>`).join("");
    stamp.after(notice);
  }
  // Served: the page may be the previous index while a refresh runs.
  if (location.protocol.startsWith("http")) {
    const live = document.createElement("div");
    live.className = "stamp";
    live.hidden = true;
    stamp.after(live);
    const poll = async () => {
      let s;
      try { s = await (await fetch("/api/build-status", { cache: "no-store" })).json(); } catch (e) { return; }
      if (s.state === "building") { live.hidden = false; live.textContent = `Showing the previous index; refreshing: ${s.step}`; setTimeout(poll, 1500); }
      else if (s.state === "failed") { live.hidden = false; live.className = "stamp stale"; live.textContent = `The refresh failed; this is the previous index. ${s.step}`; }
      else if (!live.hidden) { live.innerHTML = `The refreshed index is ready. <button type="button">Reload</button>`; live.querySelector("button").onclick = () => location.reload(); }
    };
    poll();
  }
  window.addEventListener("hashchange", route);
  window.addEventListener("art-reviews-changed", route);
  route();
})();
