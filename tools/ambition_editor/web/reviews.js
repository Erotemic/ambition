/* Asset reviews: one format for every kind (`asset_index/reviews.py`). A review
   scores one exact version from 1 to 10 with notes and issue tags, as the music
   tool's review does; music reviews are the music tool's own files. */
(function () {
  "use strict";
  const C = window.ART_CATALOG;
  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"})[c]);
  const served = ["http:", "https:"].includes(location.protocol);
  const EMPTY = {reviews: [], versions: {}, backlog: {count: 0, rows: []}, issue_tags: {}, problems: [],
    scale: {min: 1, max: 10}, polish_threshold: 6, rubric: []};
  let report = {...EMPTY, ...(C.reviews || {})}, writable = false, failure = "";
  const drafts = new Map();
  const KIND_LABEL = {character: "Character", image: "Image", music: "Music", sfx: "Sound effect", font: "Font", world: "World"};

  const rowsFor = id => report.reviews.filter(row => row.id === id);  // newest first
  const latest = id => rowsFor(id)[0] || null;
  const link = row => `#${row.kind === "character" ? "character" : row.kind === "image" ? "asset" : "item"}/${encodeURIComponent(row.id)}`;
  function chip(row) {
    if (!row) return "";
    const tone = row.score < report.polish_threshold ? "c-warn" : row.score >= 7 ? "c-ok" : "c-info";
    return `<span class="chip ${tone}" title="${esc(row.label)}${row.relation !== "current" ? " (" + esc(row.relation) + " version)" : ""}">${esc(row.score)} ${esc(row.label)}</span>`;
  }
  const relation = row => row.relation === "current" ? "" : ` <span class="chip c-info">${esc(row.relation === "older" ? "older version" : row.relation)}</span>`;
  const needsWork = id => report.backlog.rows.filter(row => !id || row.id === id);

  function changed() {
    const tab = document.querySelector('[data-view="reviews"]');
    if (tab) tab.textContent = `Reviews (${report.backlog.count})`;
    window.dispatchEvent(new Event("asset-reviews-changed"));
  }
  async function request(method, data) {
    const response = await fetch("/api/asset-reviews", {method, cache: "no-store", headers: {"Content-Type": "application/json"}, ...(data ? {body: JSON.stringify(data)} : {})});
    const result = await response.json();
    if (!response.ok) throw new Error(result.error || "Could not save the review.");
    report = {...EMPTY, ...result}; writable = true; failure = "";
    return result;
  }
  // Save one review. `data` is {id, version_id, score, notes, issues, part, furthest_played_*}.
  // The caller calls `changed()` to show it.
  async function save(data) {
    return (await request("POST", data)).saved;
  }

  function rubric() {
    return report.rubric.map(band => `<span title="${esc(band.description)}">${esc(band.low)}–${esc(Math.ceil(band.high))} ${esc(band.label.toLowerCase())}</span>`).join(" · ");
  }
  function reviewHtml(row) {
    return `<article class="review-note"><p>${chip(row)}${relation(row)}${row.part ? ` · <b>${esc(row.part)}</b>` : ""} · <time>${esc(row.reviewed_at.slice(0, 16).replace("T", " "))}</time>
        · <code title="${esc(row.version_id)}">${esc(row.version_id.slice(0, 12))}</code></p>
      ${row.notes ? `<p class="review-text">${esc(row.notes)}</p>` : ""}
      ${row.issues.length ? `<div class="chips">${row.issues.map(tag => `<span>${esc(tag)}</span>`).join("")}</div>` : ""}</article>`;
  }

  /* The review box of one subject. `opts`: kind, parts (names) and partLabel,
     audio (the href the page plays, to save how far it was heard). */
  function section(id, opts = {}) {
    const kind = opts.kind, version = report.versions[id], rows = rowsFor(id);
    const tags = report.issue_tags[kind] || [], parts = opts.parts || [];
    const draft = drafts.get(id) || {};
    const part = draft.part ?? "";
    const mine = rows.find(row => row.version_id === version && row.part === part);
    const value = (name, fallback) => draft[name] ?? (mine ? mine[name] : fallback);
    const issues = draft.issues ?? mine?.issues ?? [];
    const work = needsWork(id);
    const form = !version ? `<p class="empty">Nothing of this asset is published to review. Make it, then rebuild the inspector.</p>`
      : !writable ? `<p class="empty">To save a review, open the page with <code>./scripts/asset_inspector.py</code>. ${esc(failure)}</p>`
      : `<form class="asset-review" data-id="${esc(id)}" data-kind="${esc(kind)}"${opts.audio ? ` data-audio="${esc(opts.audio)}"` : ""}>
        <div class="controls"><label>Score <input name="score" type="number" min="${report.scale.min}" max="${report.scale.max}" step="0.1" required value="${esc(value("score", ""))}"></label>
          ${parts.length ? `<label>${esc(opts.partLabel || "Part")} <select name="part"><option value="">All of it</option>${parts.map(name => `<option${name === part ? " selected" : ""}>${esc(name)}</option>`).join("")}</select></label>` : ""}
          <span class="n">${rubric()}</span></div>
        <textarea name="notes" rows="3" maxlength="20000" placeholder="What works, and what to change">${esc(value("notes", ""))}</textarea>
        <div class="controls">${tags.map(tag => `<label><input type="checkbox" name="issues" value="${esc(tag)}"${issues.includes(tag) ? " checked" : ""}> ${esc(tag)}</label>`).join(" ")}</div>
        <p class="controls"><button type="submit">${mine ? "Update review" : "Save review"}</button> <span class="save-state" role="status"></span></p></form>`;
    return `<section class="box" id="asset-reviews"><h2>Reviews (${rows.length})${work.length ? ` <span class="chip c-warn">${work.length} need work</span>` : ""}</h2>
      <p class="n">A review rates this exact version${version ? ` (<code title="${esc(version)}">${esc(version.slice(0, 12))}</code>)` : ""}. After the asset changes, the review stays with the version it rated. Below ${esc(report.polish_threshold)}, it is in the <a href="#reviews">work queue</a>.</p>
      ${form}
      ${rows.length ? rows.map(reviewHtml).join("") : '<p class="empty">Never reviewed.</p>'}</section>`;
  }

  function wire(root = document.getElementById("app")) {
    root.querySelectorAll("form.asset-review").forEach(form => {
      const id = form.dataset.id;
      const read = () => ({part: form.elements.part?.value ?? "", score: form.elements.score.value, notes: form.elements.notes.value,
        issues: [...form.querySelectorAll('[name="issues"]:checked')].map(box => box.value)});
      form.oninput = () => drafts.set(id, read());
      if (form.elements.part) form.elements.part.onchange = () => {
        // Another part has its own review: show it, or an empty form.
        drafts.set(id, {part: form.elements.part.value});
        changed();
      };
      form.onsubmit = async event => {
        event.preventDefault();
        const state = form.querySelector(".save-state"), button = form.querySelector("[type=submit]");
        const heard = form.dataset.audio && window.AssetItems?.heard ? window.AssetItems.heard(form.dataset.audio) : {seconds: 0, fraction: 0};
        const data = read();
        button.disabled = true; state.textContent = "Saving…";
        try {
          const saved = await save({id, version_id: report.versions[id], ...data, score: Number(data.score),
            furthest_played_seconds: heard.seconds, furthest_played_fraction: heard.fraction});
          drafts.set(id, {part: data.part});
          changed();
          const again = document.querySelector(`form.asset-review[data-id="${CSS.escape(id)}"] .save-state`);
          if (again) again.textContent = `Saved to ${saved.path}`;
        } catch (error) { state.textContent = error.message; button.disabled = false; }
      };
    });
  }

  const pageFilter = {kind: "", sort: "newest"};
  function page() {
    const query = document.getElementById("q").value.trim().toLowerCase();
    const hit = row => !query || JSON.stringify([row.id, row.title, row.notes, row.issues, row.part]).toLowerCase().includes(query);
    const work = report.backlog.rows.filter(hit);
    let all = report.reviews.filter(row => hit(row) && (!pageFilter.kind || row.kind === pageFilter.kind));
    if (pageFilter.sort !== "newest") all = [...all].sort((a, b) => (pageFilter.sort === "lowest" ? a.score - b.score : b.score - a.score) || a.id.localeCompare(b.id));
    const kinds = [...new Set(report.reviews.map(row => row.kind))].sort();
    const subject = row => `<a href="${link(row)}">${esc(row.title)}</a><br><small><code>${esc(row.id)}</code></small>`;
    const first = text => esc((text || "").split("\n")[0].slice(0, 220));
    document.getElementById("app").innerHTML = `<h1>Reviews</h1>
      <p class="lede">One format for every asset: a score from ${esc(report.scale.min)} to ${esc(report.scale.max)} (${rubric()}), notes and issue tags, for one exact version. Music reviews are the music tool's files, so its review window shows the same reviews.</p>
      ${report.problems?.length ? `<div class="stamp stale"><b>Some review files do not read.</b>${report.problems.map(p => `<pre>${esc(p)}</pre>`).join("")}</div>` : ""}
      ${!writable ? `<p class="empty">This page shows the reviews saved when it was built. Open it with <code>./scripts/asset_inspector.py</code> to save reviews and see new ones. ${esc(failure)}</p>` : ""}
      <section class="box"><h2>Needs work (${work.length})</h2>
        <p>The newest review of each asset (or of its part) that scored below ${esc(report.polish_threshold)}. <b>fix</b>: it rated the current version. <b>re-review</b>: the asset changed after the review, so look or listen again.</p>
        <p class="controls"><button id="export-review-backlog" type="button">Export as JSON</button> <code>python3 scripts/asset_inspector.py reviews backlog --json</code></p>
        ${work.length ? `<table class="tbl"><thead><tr><th>Action</th><th>Score</th><th>Kind</th><th>Asset</th><th>Part</th><th>Notes</th></tr></thead><tbody>
          ${work.map(row => `<tr><td><span class="chip ${row.action === "fix" ? "c-warn" : "c-info"}">${esc(row.action)}</span></td><td>${chip(row)}</td><td>${esc(KIND_LABEL[row.kind] || row.kind)}</td>
            <td>${subject(row)}</td><td>${esc(row.part)}</td><td>${first(row.notes)}${row.issues.length ? `<div class="chips">${row.issues.map(t => `<span>${esc(t)}</span>`).join("")}</div>` : ""}</td></tr>`).join("")}</tbody></table>`
          : '<p class="empty">No review asks for work.</p>'}</section>
      <section class="box"><h2>All reviews (${all.length})</h2>
        <p class="controls"><button type="button" class="ghost${pageFilter.kind ? "" : " on"}" data-review-kind="">all kinds</button>${kinds.map(kind => `<button type="button" class="ghost${pageFilter.kind === kind ? " on" : ""}" data-review-kind="${esc(kind)}">${esc(KIND_LABEL[kind] || kind)}</button>`).join("")}
          <label> · Sort <select id="review-sort">${[["newest", "Newest"], ["lowest", "Lowest score"], ["highest", "Highest score"]].map(([key, label]) => `<option value="${key}"${pageFilter.sort === key ? " selected" : ""}>${label}</option>`).join("")}</select></label></p>
        ${all.length ? `<table class="tbl"><thead><tr><th>Score</th><th>Kind</th><th>Asset</th><th>Part</th><th>Version</th><th>Reviewed</th><th>Notes</th></tr></thead><tbody>
          ${all.slice(0, 1000).map(row => `<tr><td>${chip(row)}</td><td>${esc(KIND_LABEL[row.kind] || row.kind)}</td><td>${subject(row)}</td><td>${esc(row.part)}</td>
            <td>${esc(row.relation)}</td><td><time>${esc(row.reviewed_at.slice(0, 10))}</time></td><td>${first(row.notes)}</td></tr>`).join("")}</tbody></table>`
          : '<p class="empty">No reviews match.</p>'}</section>`;
    document.getElementById("export-review-backlog").onclick = () => {
      const url = URL.createObjectURL(new Blob([JSON.stringify(report.backlog, null, 2)], {type: "application/json"}));
      const a = document.createElement("a"); a.href = url; a.download = "asset-review-backlog.json"; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
    };
    document.querySelectorAll("[data-review-kind]").forEach(button => button.onclick = () => { pageFilter.kind = button.dataset.reviewKind; page(); });
    document.getElementById("review-sort").onchange = event => { pageFilter.sort = event.target.value; page(); };
  }

  window.AssetReviews = {section, wire, page, save, changed, latest, chip, rowsFor, needsWork, report: () => report};
  setTimeout(async () => {
    if (served) {
      try { await request("GET"); }
      catch (error) { failure = "The review service is unavailable."; }
    }
    changed();
  }, 0);
})();
