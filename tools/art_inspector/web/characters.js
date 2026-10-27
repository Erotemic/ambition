/* Character identity comes from the prepared game catalog. */
(function () {
  "use strict";
  const C = window.ART_CATALOG, I = C.inventory;
  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"})[c]);
  const enc = encodeURIComponent;
  const filters = {family: "", lineage: "", attention: ""};
  const badge = (text, warn = false) => `<span class="chip ${warn ? 'c-warn' : 'c-info'}">${esc(text)}</span>`;
  const characterLink = (key, label) => `<a href="#character/${enc(key)}">${esc(label || I.identities[key]?.display || key)}</a>`;
  const unitLink = unit => `<a href="#inventory/unit/${enc(unit.id)}">${esc(unit.id)}</a>`;
  const unitsFor = actor => actor.units.map(id => I.units[id]).filter(Boolean);
  const assetsFor = actor => unitsFor(actor).flatMap(unit => (unit.assets || []).map(id => C.assets[id]))
    .filter(asset => asset?.identities?.includes(actor.key));
  const lineagesFor = actor => [...new Set(unitsFor(actor).flatMap(unit => unit.lineages))];
  const familiesFor = actor => [...new Set(unitsFor(actor).map(unit => unit.facts.visual_family?.value || "unknown"))];
  const gapsFor = actor => [...new Set(actor.coverage.filter(row => !row.valid || !['direct', 'alias'].includes(row.resolution.state)).map(row => row.action))];
  const needsArt = actor => !actor.units.length || unitsFor(actor).some(unit => unit.runtime_defects?.length ||
    [unit.status, ...Object.values(unit.status.tiers || {})].some(status => ['missing', 'partial', 'stale'].includes(status.state)));
  const table = (heads, rows) => `<div class="tablebox"><table><thead><tr>${heads.map(h => `<th>${esc(h)}</th>`).join('')}</tr></thead><tbody>${rows.map(row => `<tr>${row.map(cell => `<td>${cell}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`;

  function card(actor) {
    const art = assetsFor(actor).find(asset => asset.cover), gaps = gapsFor(actor);
    const lineages = lineagesFor(actor).map(id => I.lineages[id]?.name || id);
    return `<a class="card character-card" data-character="${esc(actor.key)}" href="#character/${enc(actor.key)}">
      <div class="img checker">${art ? `<img src="${esc(art.cover)}" loading="lazy" alt="${esc(actor.display)}">` : '<span class="empty">No published preview</span>'}</div>
      <div class="meta"><div class="nm">${esc(actor.display)}</div><code>${esc(actor.id)}</code>
        <div class="sub">${familiesFor(actor).map(name => badge(name)).join(' ')}${actor.relations.form_of ? badge('form') : actor.relations.part_of ? badge('part') : ''}
          ${needsArt(actor) ? badge('Needs generation', true) : ''}${gaps.length ? badge('Check ' + gaps.join(', '), true) : ''}
          ${window.ArtReviews.openCount(actor.key) ? badge(window.ArtReviews.openCount(actor.key) + ' open notes', true) : ''}</div>
        <span class="character-lineage-label">${esc(lineages.length ? lineages.join(' · ') : 'Lineage unknown')}</span>
      </div></a>`;
  }

  function roster() {
    const actors = Object.values(I.identities).sort((a, b) => a.display.localeCompare(b.display) || a.key.localeCompare(b.key));
    const query = document.getElementById('q').value.trim().toLowerCase();
    const shown = actors.filter(actor => (!filters.family || familiesFor(actor).includes(filters.family)) &&
      (!filters.lineage || lineagesFor(actor).includes(filters.lineage)) &&
      (!filters.attention || (filters.attention === 'generation' ? needsArt(actor) : filters.attention === 'notes' ? window.ArtReviews.openCount(actor.key) : gapsFor(actor).length)) &&
      JSON.stringify([actor.display, actor.key, actor.catalog.tags, actor.relations, familiesFor(actor),
        lineagesFor(actor).map(id => [id, I.lineages[id]?.name])]).toLowerCase().includes(query));
    function select(name, label, values) {
      return `<label>${label} <select data-character-filter="${name}"><option value="">All</option>${values.map(([value, text]) => `<option value="${esc(value)}"${filters[name] === value ? ' selected' : ''}>${esc(text)}</option>`).join('')}</select></label>`;
    }
    return `<h1>Characters</h1><p class="lede">The current ${esc(I.profile.provider || 'game')} character catalog. Each character has its own page, including forms, parts, and characters that share art.</p>
      <div class="controls character-controls">
        ${select('family', 'Art family', [...new Set(actors.flatMap(familiesFor))].sort().map(name => [name, name]))}
        ${select('lineage', 'Lineage', [...new Set(actors.flatMap(lineagesFor))].sort().map(id => [id, I.lineages[id]?.name || id]))}
        ${select('attention', 'Art status', [['generation', 'Needs generation'], ['coverage', 'Animation gaps'], ['notes', 'Open review notes']])}
        <button id="clear-character-filters" type="button">Clear filters</button>
      </div><p id="character-count">${shown.length} of ${actors.length} current characters</p>
      ${I.profile.state !== 'available' ? '<p class="empty">The prepared game catalog is unavailable. Rebuild the inspector with its content inspection binary.</p>' : ''}
      ${shown.length ? `<div class="grid characters-grid">${shown.map(card).join('')}</div>` : '<p class="empty">No characters match these filters.</p>'}`;
  }

  function relations(actor) {
    const rows = [];
    for (const [field, label] of [['form_of', 'Form of'], ['part_of', 'Part of'], ['derived_from', 'Derived from']]) {
      const parent = actor.relations[field];
      if (parent) rows.push([label, actor.provider + ':' + parent]);
    }
    for (const other of Object.values(I.identities)) {
      if (other.provider !== actor.provider) continue;
      for (const [field, label] of [['form_of', 'Form'], ['part_of', 'Part'], ['derived_from', 'Derived character']]) {
        if (other.relations[field] === actor.id) rows.push([label, other.key]);
      }
    }
    return rows.length ? `<dl class="kv">${rows.map(([label, key]) => `<dt>${label}</dt><dd>${I.identities[key] ? characterLink(key) : esc(key)}</dd>`).join('')}</dl>` : '<p class="empty">No forms, parts, or derivations are declared.</p>';
  }

  function lineage(actor) {
    const ids = lineagesFor(actor), assets = assetsFor(actor);
    const current = assets[0] ? {...assets[0], display: actor.display} : {id: '', display: actor.display, identities: [actor.key]};
    if (!ids.length) return `<div id="character-lineage">${window.ArtLineage.render({name: actor.display, eras: [], tracks: []}, current)}</div>`;
    return `<div id="character-lineage">${ids.map(id => {
      const record = C.lineages?.[id] || I.lineages[id] || {name: id};
      return window.ArtLineage.render({...record, lineage: id}, current);
    }).join('')}</div>`;
  }

  function detail(actor, ui) {
    const units = unitsFor(actor), assets = assetsFor(actor), selected = assets[0];
    const descriptions = [...new Set([actor.catalog.gameplay_description, actor.catalog.authoring_description, selected?.description].filter(text => typeof text === 'string' && text.trim()))];
    const coverage = actor.coverage.map(row => [esc(row.action), esc(row.tier), badge(row.resolution.state, !row.valid || !['direct', 'alias'].includes(row.resolution.state)),
      esc(row.resolution.row || 'none'), row.source_available === null ? 'unknown' : String(row.source_available), row.profile_selected === null ? 'unknown' : String(row.profile_selected), row.valid ? 'valid' : 'missing or invalid']);
    return `<div class="detail character-detail"><p><a href="#characters">← Characters</a></p>
      <h1>${esc(actor.display)}</h1><p class="lede"><code>${esc(actor.key)}</code> ${familiesFor(actor).map(name => badge(name)).join(' ')}</p>
      ${descriptions.map(text => `<p class="lede">${esc(text)}</p>`).join('')}
      <div class="chips">${(actor.catalog.tags || []).map(tag => `<span>${esc(tag)}</span>`).join('')}</div>
      <p class="controls"><a href="#character-lineage" data-jump="character-lineage">Character lineage</a><a href="#character-notes" data-jump="character-notes">Review notes</a><a href="#character-coverage" data-jump="character-coverage">Animation coverage</a><a href="#character-products" data-jump="character-products">Art and generation</a></p>
      <section class="box"><h2>Forms, parts, and derivations</h2>${relations(actor)}</section>
      ${lineage(actor)}
      ${window.ArtReviews.section(actor.key)}
      <section class="box" id="character-products"><h2>Current character art</h2>
        <p>Manifest: <code>${esc(actor.catalog.manifest || 'Not declared')}</code></p>
        ${assets.length > 1 ? `<label>Published sheet <select id="character-art">${assets.map(asset => `<option value="${esc(asset.id)}">${esc(asset.art_label || asset.name)}</option>`).join('')}</select></label>` : ''}
        ${units.map(unit => `<p>${unitLink(unit)} ${badge(unit.status.state, needsArt(actor))} · ${esc(unit.game_use)}<br><code>${esc(unit.repair)}</code></p>`).join('') || '<p class="empty">No art bundle is registered for this character.</p>'}
      </section><div id="character-art-panels">${selected ? ui.assetPanels(selected) : '<p class="empty">No published art is available. This character remains in the roster.</p>'}</div>
      <section class="box" id="character-coverage"><h2>Animation coverage</h2>${table(['Action', 'Tier', 'Resolution', 'Row', 'Source', 'Profile', 'Runtime product'], coverage)}</section>
      <section class="box"><h2>Character data</h2><details><summary>Prepared catalog record</summary><pre class="evidence">${esc(JSON.stringify(actor.catalog, null, 2))}</pre></details></section></div>`;
  }

  window.renderArtCharacters = function (key, ui) {
    const app = document.getElementById('app');
    if (!I) { app.innerHTML = '<p class="empty">Rebuild the inspector to read the current game catalog.</p>'; return; }
    const actor = I.identities[key];
    app.innerHTML = key ? actor ? detail(actor, ui) : `<h1>Character unavailable</h1><p>${esc(key)} is absent from the current catalog.</p><a href="#characters">Characters</a>` : roster();
    app.querySelectorAll('[data-character-filter]').forEach(element => element.onchange = () => {
      filters[element.dataset.characterFilter] = element.value;
      window.renderArtCharacters('', ui);
    });
    const clear = document.getElementById('clear-character-filters');
    if (clear) clear.onclick = () => {
      Object.keys(filters).forEach(key => filters[key] = ''); document.getElementById('q').value = '';
      window.renderArtCharacters('', ui);
    };
    app.querySelectorAll('[data-jump]').forEach(element => element.onclick = event => {
      event.preventDefault(); document.getElementById(element.dataset.jump).scrollIntoView({behavior: 'smooth'});
    });
    if (actor) {
      window.ArtReviews.wire(app);
      const assets = assetsFor(actor);
      if (assets[0]) ui.wireAnimation(assets[0]);
      const select = document.getElementById('character-art');
      if (select) select.onchange = () => {
        const asset = C.assets[select.value];
        document.getElementById('character-art-panels').innerHTML = ui.assetPanels(asset);
        ui.wireAnimation(asset);
      };
    }
  };
})();
