/* Character identity comes from the prepared game catalog. */
(function () {
  "use strict";
  const C = window.ART_CATALOG, I = C.inventory;
  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"})[c]);
  const enc = encodeURIComponent;
  const filters = {family: "", lineage: "", attention: "", view: ""};
  const badge = (text, warn = false) => `<span class="chip ${warn ? 'c-warn' : 'c-info'}">${esc(text)}</span>`;
  const characterLink = (key, label) => `<a href="#character/${enc(key)}">${esc(label || I.identities[key]?.display || key)}</a>`;
  const unitLink = unit => `<a href="#inventory/unit/${enc(unit.id)}">${esc(unit.id)}</a>`;
  const unitsFor = actor => actor.units.map(id => I.units[id]).filter(Boolean);
  const assetsFor = actor => unitsFor(actor).flatMap(unit => (unit.assets || []).map(id => C.assets[id]))
    .filter(asset => asset?.identities?.includes(actor.key))
    .map(asset => ({...asset, cover: actor.preview?.asset === asset.id ? actor.preview.cover : asset.cover, preview_row: actor.catalog.rest_row}));
  const lineagesFor = actor => actor.lineages || [];
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
    const matched = new Set(shown.map(actor => actor.key));
    const groups = I.character_groups.filter(group => group.members.some(key => matched.has(key)))
      .sort((a, b) => I.identities[a.root].display.localeCompare(I.identities[b.root].display));
    const grouped = group => {
      const children = group.members.filter(key => key !== group.root);
      const notes = group.members.reduce((sum, key) => sum + window.ArtReviews.openCount(key), 0);
      return `<article class="character-group" data-character-root="${esc(group.root)}">${card(I.identities[group.root])}
        ${children.length ? `<details class="character-variants"${query || filters.attention ? ' open' : ''}><summary>${children.length} forms and parts${notes ? ` · ${notes} open notes` : ''}</summary>
          <div class="character-children">${children.map(key => card(I.identities[key])).join('')}</div></details>` : ''}</article>`;
    };
    function select(name, label, values) {
      return `<label>${label} <select data-character-filter="${name}"><option value="">${name === 'view' ? 'Characters with forms and parts' : 'All'}</option>${values.map(([value, text]) => `<option value="${esc(value)}"${filters[name] === value ? ' selected' : ''}>${esc(text)}</option>`).join('')}</select></label>`;
    }
    return `<h1>Characters</h1><p class="lede">The current ${esc(I.profile.provider || 'game')} character catalog. Forms and parts are under their character. Each catalog entry has a detail page. Shared ancestry does not merge distinct characters.</p>
      <div class="controls character-controls">
        ${select('view', 'Show', [['entries', 'All catalog entries']])}
        ${select('family', 'Art family', [...new Set(actors.flatMap(familiesFor))].sort().map(name => [name, name]))}
        ${select('lineage', 'Lineage', [...new Set(actors.flatMap(lineagesFor))].sort().map(id => [id, I.lineages[id]?.name || id]))}
        ${select('attention', 'Art status', [['generation', 'Needs generation'], ['coverage', 'Animation gaps'], ['notes', 'Open review notes']])}
        <button id="clear-character-filters" type="button">Clear filters</button>
      </div><p id="character-count">${filters.view ? `${shown.length} of ${actors.length} catalog entries` : `${groups.length} of ${I.character_groups.length} characters · ${actors.length} catalog entries`}</p>
      ${I.profile.state !== 'available' ? '<p class="empty">The prepared game catalog is unavailable. Rebuild the inspector with its content inspection binary.</p>' : ''}
      ${shown.length ? `<div class="grid characters-grid">${filters.view ? shown.map(card).join('') : groups.map(grouped).join('')}</div>` : '<p class="empty">No characters match these filters.</p>'}`;
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
    const record = id => C.lineages?.[id] || I.lineages[id] || {name: id};
    function chain(id) {
      const result = [], seen = new Set();
      while (id && !seen.has(id)) { seen.add(id); result.unshift(id); id = record(id).continues; }
      return result;
    }
    const chains = ids.map(chain);
    const origins = [...new Set(chains.flatMap(path => path.flatMap(id => record(id).split_from || [])))];
    const relatives = Object.values(I.identities).filter(other => other.key !== actor.key &&
      !other.relations.part_of && !other.relations.form_of &&
      lineagesFor(other).some(id => chain(id).some(branch => (record(branch).split_from || []).some(origin => origins.includes(origin)))));
    const ancestry = origins.length ? `<section class="box shared-origin"><h2>Shared origin</h2>
      <p>This character split from an earlier design.${relatives.length ? ` Related characters: ${relatives.map(other => characterLink(other.key)).join(' · ')}.` : ''}</p>
      ${origins.map(id => window.ArtLineage.render({...record(id), lineage: id}, null)).join('')}</section>` : '';
    if (!ids.length) return `<div id="character-lineage">${window.ArtLineage.render({name: actor.display, eras: [], tracks: []}, current)}</div>`;
    return `<div id="character-lineage">${ids.map((id, index) => {
      const path = chains[index].map(record), leaf = record(id);
      return window.ArtLineage.render({...leaf, lineage: id,
        eras: path.flatMap(item => item.eras || []), tracks: path.flatMap(item => item.tracks || [])}, current);
    }).join('')}${ancestry}</div>`;
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
    if (I.profile.state !== 'available') {
      app.innerHTML = `<h1>Characters unavailable</h1><p>The game catalog could not be read. The notice above names the problem and its fix; then run <code>./scripts/asset_inspector.py</code> again.</p>
        <p>The art itself is indexed: browse the <a href="#gallery">All images</a>, <a href="#inventory/all">Inventory</a> or <a href="#generators">Generators</a>.</p>`;
      return;
    }
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
        const asset = assets.find(item => item.id === select.value);
        document.getElementById('character-art-panels').innerHTML = ui.assetPanels(asset);
        ui.wireAnimation(asset);
      };
    }
  };
})();
