/* Character identity comes from the prepared game catalog. */
(function () {
  "use strict";
  const C = window.ART_CATALOG;
  if (!C) return;
  const I = C.inventory;
  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"})[c]);
  const enc = encodeURIComponent;
  const defaultFilters = {family: "", lineage: "", attention: "", view: "", sort: "added-newest", importance: "", role: "", history: ""};
  const filters = {...defaultFilters};
  const badge = (text, warn = false) => `<span class="chip ${warn ? 'c-warn' : 'c-info'}">${esc(text)}</span>`;
  const characterLink = (key, label) => `<a href="#character/${enc(key)}">${esc(label || I.identities[key]?.display || key)}</a>`;
  const unitLink = unit => `<a href="#inventory/unit/${enc(unit.id)}">${esc(unit.id)}</a>`;
  const unitsFor = actor => actor.units.map(id => I.units[id]).filter(Boolean);
  const assetsFor = actor => unitsFor(actor).flatMap(unit => (unit.assets || []).map(id => C.assetDetails[id]))
    .filter(asset => asset?.identities?.includes(actor.key))
    .map(asset => ({...asset, cover: actor.preview?.asset === asset.id ? actor.preview.cover : asset.cover, preview_row: actor.catalog.rest_row}));
  const lineagesFor = actor => actor.lineages || [];
  const lineageRecord = id => C.lineages?.[id] || I.lineages[id] || {name: id};
  function lineageChain(id) {
    const result = [], seen = new Set();
    while (id && !seen.has(id)) { seen.add(id); result.unshift(id); id = lineageRecord(id).continues; }
    return result;
  }
  function lineageSections(actor) {
    const chains = lineagesFor(actor).map(lineageChain);
    const origins = [...new Set(chains.flatMap(path => path.flatMap(id => lineageRecord(id).split_from || [])))];
    return {chains, origins};
  }
  const historyCounts = new Map();
  function lineageItemCount(actor) {
    if (!historyCounts.has(actor.key)) {
      const {chains, origins} = lineageSections(actor);
      const records = [...new Set([...chains.flat(), ...origins])].map(lineageRecord);
      historyCounts.set(actor.key, window.ArtLineage.items({
        eras: records.flatMap(record => record.eras || []),
        tracks: records.flatMap(record => record.tracks || []),
      }).length);
    }
    return historyCounts.get(actor.key);
  }
  const familiesFor = actor => I.identities[actor.key].art_families;
  const gapsFor = actor => actor.coverage_gaps;
  // A review can name an action or a published animation row.
  const animations = actor => [...new Set([...actor.coverage.map(row => row.action), ...assetsFor(actor).flatMap(asset => (asset.layout?.rows || []).map(row => row.name))])].sort();
  const importance = actor => actor.catalog.cast?.importance || 'Unclassified';
  const importanceLevels = ['Primary', 'Secondary', 'Tertiary', 'Background', 'Unclassified'];
  const hasBossTag = actor => (actor.catalog.tags || []).includes('boss');
  function rolesFor(actor) {
    const roles = actor.catalog.cast?.roles || {};
    return hasBossTag(actor) && !roles.encounter?.length ? {...roles, encounter: ['boss']} : roles;
  }
  const roleKey = (group, role) => JSON.stringify([group, role]);
  const roleLabel = role => role.replace(/[-_]+/g, ' ').replace(/\b[a-z]/g, letter => letter.toUpperCase());
  function matchesRole(actor, roles) {
    if (!filters.role) return true;
    if (filters.role === 'other-characters') {
      return !hasBossTag(actor) && !(roles.encounter || []).some(role => ['boss', 'mini-boss', 'miniboss'].includes(role));
    }
    return Object.entries(roles).some(([group, labels]) => labels.some(label => roleKey(group, label) === filters.role));
  }
  function castBadges(actor) {
    const level = importance(actor);
    const roles = Object.entries(rolesFor(actor)).flatMap(([group, labels]) => labels.map(label =>
      `<span class="chip" title="${esc(roleLabel(group))}">${esc(roleLabel(label))}</span>`));
    return [level !== 'Unclassified' ? badge(level) : '', ...roles].filter(Boolean).join(' ');
  }
  const needsArt = actor => I.identities[actor.key].needs_art;
  const table = (heads, rows) => `<div class="tablebox"><table><thead><tr>${heads.map(h => `<th>${esc(h)}</th>`).join('')}</tr></thead><tbody>${rows.map(row => `<tr>${row.map(cell => `<td>${cell}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`;

  const dateValue = value => Number.isFinite(Date.parse(value)) ? Date.parse(value) : null;
  const dateLabel = value => dateValue(value) === null ? 'Unknown' : new Date(value).toISOString().slice(0, 10);
  const generationLabel = actor => dateLabel(actor.generated_at) + (actor.generated_at_evidence === 'file-mtime' ? ' (file date)' : '');
  const byName = (a, b) => a.display.localeCompare(b.display) || a.key.localeCompare(b.key);
  function compare(a, b, groupDates) {
    if (!filters.sort) return byName(a, b);
    const field = filters.sort.startsWith('added') ? 'added_at' : 'generated_at';
    const value = actor => dateValue(groupDates && field === 'generated_at' ? groupDates.get(actor.key)?.generated_at : actor[field]);
    const left = value(a), right = value(b);
    if (left === null || right === null) return (left === null) - (right === null) || byName(a, b);
    return (left - right) * (filters.sort.endsWith('oldest') ? 1 : -1) || byName(a, b);
  }

  function card(actor) {
    const gaps = gapsFor(actor), history = lineageItemCount(actor);
    const lineages = lineagesFor(actor).map(id => I.lineages[id]?.name || id);
    const labels = castBadges(actor);
    return `<a class="card character-card" data-character="${esc(actor.key)}" href="#character/${enc(actor.key)}">
      <div class="img checker">${window.ArtCatalog.previews() ? actor.cover ? `<img src="${esc(actor.cover)}" loading="lazy" alt="${esc(actor.display)}">` : '<span class="empty">No published preview</span>' : ''}</div>
      <div class="meta"><div class="nm">${esc(actor.display)}</div><code>${esc(actor.id)}</code>
        ${labels ? `<div class="sub">${labels}</div>` : ''}
        <div class="sub">${familiesFor(actor).map(name => badge(name)).join(' ')}${actor.relations.form_of ? badge('form') : actor.relations.part_of ? badge('part') : ''}
          ${history ? badge(history + ' lineage items') : ''}
          ${needsArt(actor) ? badge('Needs generation', true) : ''}${gaps.length ? badge('Check ' + gaps.join(', '), true) : ''}
          ${window.AssetReviews.chip(window.AssetReviews.latest(actor.key))}</div>
        ${filters.sort ? `<div class="sub">${filters.sort.startsWith('added') ? 'Added to game: ' + dateLabel(actor.added_at) : 'Art generated: ' + generationLabel(actor)}</div>` : ''}
        <span class="character-lineage-label">${esc(lineages.length ? lineages.join(' · ') : 'Lineage unknown')}</span>
      </div></a>`;
  }

  function roster() {
    const actors = Object.values(I.identities).sort((a, b) => compare(a, b));
    const query = document.getElementById('q').value.trim().toLowerCase();
    const roles = new Map(actors.map(actor => [actor.key, rolesFor(actor)]));
    const shown = actors.filter(actor => (!filters.importance || importance(actor) === filters.importance)
      && matchesRole(actor, roles.get(actor.key)) && (!filters.family || familiesFor(actor).includes(filters.family)) &&
      (!filters.lineage || lineagesFor(actor).includes(filters.lineage)) &&
      (filters.history === '' || lineageItemCount(actor) > Number(filters.history)) &&
      (!filters.attention || (filters.attention === 'generation' ? needsArt(actor) : filters.attention === 'reviews' ? window.AssetReviews.workCount(actor.key) : gapsFor(actor).length)) &&
      JSON.stringify([actor.display, actor.key, actor.catalog.tags, actor.relations, familiesFor(actor), importance(actor),
        Object.entries(roles.get(actor.key)).flatMap(([group, labels]) => [roleLabel(group), ...labels.map(roleLabel)]),
        lineagesFor(actor).map(id => [id, I.lineages[id]?.name])]).toLowerCase().includes(query));
    const matched = new Set(shown.map(actor => actor.key));
    const groupDates = new Map(I.character_groups.map(group => {
      const latest = group.members.map(key => I.identities[key]).filter(actor => dateValue(actor.generated_at) !== null)
        .sort((a, b) => dateValue(b.generated_at) - dateValue(a.generated_at))[0];
      return [group.root, latest];
    }));
    const groups = I.character_groups.filter(group => group.members.some(key => matched.has(key)))
      .sort((a, b) => compare(I.identities[a.root], I.identities[b.root], groupDates));
    const filteringCast = Boolean(filters.importance || filters.role);
    const grouped = group => {
      const children = group.members.filter(key => key !== group.root && (!filteringCast || matched.has(key)))
        .sort((a, b) => compare(I.identities[a], I.identities[b]));
      const work = group.members.reduce((sum, key) => sum + window.AssetReviews.workCount(key), 0);
      const open = query || filters.attention || (filteringCast && !matched.has(group.root));
      return `<article class="character-group" data-character-root="${esc(group.root)}">${card({...I.identities[group.root], generated_at: groupDates.get(group.root)?.generated_at, generated_at_evidence: groupDates.get(group.root)?.generated_at_evidence})}
        ${children.length ? `<details class="character-variants"${open ? ' open' : ''}><summary>${children.length} forms and parts${work ? ` · ${work} reviews need work` : ''}</summary>
          <div class="character-children">${children.map(key => card(I.identities[key])).join('')}</div></details>` : ''}</article>`;
    };
    function select(name, label, values) {
      return `<label>${label} <select data-character-filter="${name}"><option value="">${name === 'view' ? 'Characters with forms and parts' : name === 'sort' ? 'Name (A–Z)' : 'All'}</option>${values.map(([value, text]) => `<option value="${esc(value)}"${filters[name] === value ? ' selected' : ''}>${esc(text)}</option>`).join('')}</select></label>`;
    }
    const roleOptions = new Map();
    for (const entries of roles.values()) {
      for (const [group, labels] of Object.entries(entries)) {
        if (!roleOptions.has(group)) roleOptions.set(group, new Set());
        for (const role of labels) roleOptions.get(group).add(role);
      }
    }
    const roleOption = (value, text) => `<option value="${esc(value)}"${filters.role === value ? ' selected' : ''}>${esc(text)}</option>`;
    const roleSelect = `<label>Role <select data-character-filter="role">
      ${roleOption('', 'All')}${roleOption('other-characters', 'Exclude bosses')}
      ${[...roleOptions.keys()].sort().filter(group => roleOptions.get(group).size).map(group =>
        `<optgroup label="${esc(roleLabel(group))}">${[...roleOptions.get(group)].sort().map(role => roleOption(roleKey(group, role), roleLabel(role))).join('')}</optgroup>`).join('')}
      </select></label>`;
    return `<h1>Characters</h1><p class="lede">The current ${esc(I.profile.provider || 'game')} character and boss roster. Forms and parts are under their character. Each entry has a detail page. Shared ancestry does not merge distinct characters.</p>
      <div class="controls character-controls">
        ${select('sort', 'Sort by', [['added-newest', 'Added to game (newest first)'], ['added-oldest', 'Added to game (oldest first)'], ['generated-newest', 'Art generated (newest first)'], ['generated-oldest', 'Art generated (oldest first)']])}
        ${select('view', 'Show', [['entries', 'All entries']])}
        ${select('importance', 'Importance', importanceLevels.map(level => [level, level]))}
        ${roleSelect}
        ${select('family', 'Art family', [...new Set(actors.flatMap(familiesFor))].sort().map(name => [name, name]))}
        ${select('lineage', 'Lineage', [...new Set(actors.flatMap(lineagesFor))].sort().map(id => [id, I.lineages[id]?.name || id]))}
        <label title="Count distinct recorded looks, including earlier lineage branches. Excludes the current published preview.">Lineage items &gt; <input type="number" min="0" step="1" placeholder="Any" aria-label="Lineage item threshold" data-character-filter="history" value="${esc(filters.history)}"></label>
        ${select('attention', 'Art status', [['generation', 'Needs generation'], ['coverage', 'Animation gaps'], ['reviews', 'Reviews that need work']])}
        <button id="clear-character-filters" type="button">Clear filters</button>
      </div><p id="character-count">${filters.view ? `${shown.length} of ${actors.length} entries` : `${groups.length} of ${I.character_groups.length} characters · ${actors.length} entries`}</p>
      ${I.profile.state !== 'available' ? '<p class="empty">The prepared game catalog is unavailable. Rebuild the inspector with its content inspection binary.</p>' : ''}
      ${filters.sort ? '<p class="sub">Added dates use the first catalog entry in available Git history. Generation dates use render receipts, or published image file dates when receipts are absent. Copying or restoring files can change file dates. Unknown dates sort last. Groups use their main character’s added date and the latest art date across their forms and parts.</p>' : ''}
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
    const record = lineageRecord;
    const {chains, origins} = lineageSections(actor);
    const relatives = Object.values(I.identities).filter(other => other.key !== actor.key &&
      !other.relations.part_of && !other.relations.form_of &&
      lineagesFor(other).some(id => lineageChain(id).some(branch => (record(branch).split_from || []).some(origin => origins.includes(origin)))));
    const ancestry = origins.length ? `<section class="box shared-origin"><h2>Shared origin</h2>
      <p>This character split from an earlier design.${relatives.length ? ` Related characters: ${relatives.map(other => characterLink(other.key)).join(' · ')}.` : ''}</p>
      ${origins.map(id => window.ArtLineage.render({...record(id), lineage: id}, null)).join('')}</section>` : '';
    if (!ids.length) return `<div id="character-lineage">${window.ArtLineage.render({name: actor.display, eras: [], tracks: []}, current)}</div>`;
    return `<div id="character-lineage">${ids.map((id, index) => {
      const path = chains[index].map(record), leaf = record(id);
      return window.ArtLineage.render({...leaf, lineage: id,
        eras: path.flatMap(item => item.eras || []), tracks: path.flatMap(item => item.tracks || []),
        maintained: path.flatMap(item => item.maintained || [])}, current);
    }).join('')}${ancestry}</div>`;
  }

  function detail(actor, ui) {
    const units = unitsFor(actor), assets = assetsFor(actor), selected = assets[0];
    const descriptions = [...new Set([actor.catalog.gameplay_description, actor.catalog.authoring_description, selected?.description].filter(text => typeof text === 'string' && text.trim()))];
    const sameArt = actor.catalog.manifest ? Object.values(I.identities).filter(other => other.key !== actor.key && other.catalog.manifest === actor.catalog.manifest) : [];
    const coverage = actor.coverage.map(row => [esc(row.action), esc(row.tier), badge(row.resolution.state, !row.valid || !['direct', 'alias'].includes(row.resolution.state)),
      esc(row.resolution.row || 'none'), row.source_available === null ? 'unknown' : String(row.source_available), row.profile_selected === null ? 'unknown' : String(row.profile_selected), row.valid ? 'valid' : 'missing or invalid']);
    return `<div class="detail character-detail"><p><a href="#characters">← Characters</a></p>
      <h1>${esc(actor.display)}</h1><p class="lede"><code>${esc(actor.key)}</code> ${familiesFor(actor).map(name => badge(name)).join(' ')}</p>
      ${descriptions.map(text => `<p class="lede">${esc(text)}</p>`).join('')}
      <div class="chips">${castBadges(actor)} ${(actor.catalog.tags || []).map(tag => `<span>${esc(tag)}</span>`).join('')}</div>
      <p class="controls"><a href="#character-lineage" data-jump="character-lineage">Character lineage</a><a href="#asset-reviews" data-jump="asset-reviews">Reviews</a><a href="#character-coverage" data-jump="character-coverage">Animation coverage</a><a href="#character-products" data-jump="character-products">Art and generation</a></p>
      <div id="character-reference">${selected ? ui.referenceBox(selected) : ''}</div>
      ${sameArt.length ? `<p>Entries using the same art: ${sameArt.map(other => characterLink(other.key)).join(' · ')}</p>` : ''}
      <dl class="kv"><dt>Added to game</dt><dd>${esc(dateLabel(actor.added_at))}</dd><dt>Art generated</dt><dd>${esc(generationLabel(actor))}</dd></dl>
      <section class="box"><h2>Forms, parts, and derivations</h2>${relations(actor)}</section>
      ${lineage(actor)}
      ${window.AssetReviews.section(actor.key, {kind: 'character', parts: animations(actor), partLabel: 'Animation'})}
      <section class="box" id="character-products"><h2>Current character art</h2>
        <p>Manifest: <code>${esc(actor.catalog.manifest || 'Not declared')}</code></p>
        ${assets.length > 1 ? `<label>Published sheet <select id="character-art">${assets.map(asset => `<option value="${esc(asset.id)}">${esc(asset.art_label || asset.name)}</option>`).join('')}</select></label>` : ''}
        ${units.map(unit => `<p>${unitLink(unit)} ${badge(unit.status.state, needsArt(actor))} · ${esc(unit.game_use)}<br><code>${esc(unit.repair)}</code></p>`).join('') || '<p class="empty">No art bundle is registered for this character.</p>'}
      </section><div id="character-art-panels">${selected ? ui.assetPanels(selected, {references: false}) : '<p class="empty">No published art is available. This character remains in the roster.</p>'}</div>
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
    const actor = C.characterDetails[key];
    app.innerHTML = key ? actor ? detail(actor, ui) : `<h1>Character unavailable</h1><p>${esc(key)} is absent from the current catalog.</p><a href="#characters">Characters</a>` : roster();
    app.querySelectorAll('[data-character-filter]').forEach(element => element.onchange = () => {
      if (element.type === 'number' && !element.checkValidity()) return;
      filters[element.dataset.characterFilter] = element.value;
      window.renderArtCharacters('', ui);
    });
    const clear = document.getElementById('clear-character-filters');
    if (clear) clear.onclick = () => {
      Object.assign(filters, defaultFilters);
      document.getElementById('q').value = '';
      window.renderArtCharacters('', ui);
    };
    app.querySelectorAll('[data-jump]').forEach(element => element.onclick = event => {
      event.preventDefault(); document.getElementById(element.dataset.jump).scrollIntoView({behavior: 'smooth'});
    });
    if (actor) {
      window.AssetReviews.wire(app);
      const assets = assetsFor(actor);
      if (assets[0]) ui.wireAnimation(assets[0]);
      const select = document.getElementById('character-art');
      if (select) select.onchange = () => {
        const asset = assets.find(item => item.id === select.value);
        document.getElementById('character-reference').innerHTML = ui.referenceBox(asset);
        document.getElementById('character-art-panels').innerHTML = ui.assetPanels(asset, {references: false});
        ui.wireAnimation(asset);
      };
    }
  };
})();
