/* The CLI and this page consume the same derived owner report. */
(function () {
  "use strict";
  const C = window.ART_CATALOG, I = C.inventory;
  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"})[c]);
  const enc = encodeURIComponent, chosen = new Set(), filters = {};
  const link = (uid, label) => `<a href="#inventory/unit/${enc(uid)}">${esc(label || uid)}</a>`;
  const value = fact => fact?.value ?? "unknown";
  const chip = text => `<span class="chip">${esc(text)}</span>`;
  const json = object => `<pre class="evidence">${esc(JSON.stringify(object, null, 2))}</pre>`;
  const repoLink = path => `<a href="${esc(C.repo_href + '/' + path.split('/').map(enc).join('/'))}">${esc(path)}</a>`;
  const table = (heads, rows) => `<div class="tablebox"><table><thead><tr>${heads.map(h => `<th>${esc(h)}</th>`).join('')}</tr></thead><tbody>${rows.map(row => `<tr>${row.map(cell => `<td>${cell}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`;
  const needs = unit => !!unit.runtime_defects?.length || ["missing","partial","stale"].includes(unit.status?.state) || Object.values(unit.status?.tiers || {}).some(t => ["missing","partial","stale"].includes(t.state));
  const tabs = [["current","Current game"],["needs","Needs generation"],["unused","Registered with no known use"],["all","All targets"],["concepts","Concepts"],["history","History"],["unregistered","Unregistered products"],["runtime","Live snapshot"]];
  let stop = () => {};
  function coverage(unit) {
    const rows = unit.identities.flatMap(cid => I.identities[cid].coverage.map(c => [esc(I.identities[cid].display), esc(c.action), esc(c.tier),
      c.source_available === null ? 'unknown' : String(c.source_available), c.profile_selected === null ? 'unknown' : String(c.profile_selected),
      chip(c.resolution.state), esc(c.resolution.row), esc((c.resolution.steps || []).join(' → ')), c.valid ? 'valid' : 'missing or invalid']));
    return table(['Body','Action','Tier','Source','Profile','Resolution','Row','Steps','Runtime product'], rows);
  }
  function unitView(unit) {
    if (!unit) return '<p>Unit unavailable.</p>';
    const source = unit.generator.source;
    const studioSpec = source?.split('ambition_sprite2d_renderer/ambition_sprite2d_renderer/')[1]?.replace(/^data\//, '');
    const studio = studioSpec?.endsWith('.chargen.yaml') ? `<a href="http://localhost:8765/#open=${enc(studioSpec)}&pose=death">Source Studio: death</a> (start <code>./scripts/chargen_studio.sh --open</code>)` : '';
    return `<h1>${esc(unit.id)}</h1><p>${chip(unit.kind)} ${chip(unit.game_use)} ${chip(unit.registration)} ${chip(unit.status.state)}</p>
      <p>Publish: <code>${esc(unit.repair)}</code></p><p>${source ? repoLink(source) : 'Source path unknown'} ${studio}</p>
      <p>${(unit.assets || []).map(a => `<a href="#asset/${enc(a)}">Published preview: ${esc(C.assets[a].name)}</a>`).join(' · ')}</p>
      <h2>Identities and relations</h2>${table(['Identity','Game label','Art manifest','Relations','Tags'], unit.identities.map(cid => { const a = I.identities[cid]; return [`<a href="#character/${enc(cid)}">${esc(cid)}</a>`,esc(a.display),esc(a.catalog.manifest),json(a.relations),esc((a.catalog.tags || []).join(' '))]; }))}
      <h2>Metadata</h2>${table(['Fact','Value','Evidence','Owner'], Object.entries(unit.facts).map(([name,f]) => [esc(name),esc(value(f)),esc(f.state + ' / ' + f.evidence),esc(f.source)]))}
      <h2>Required products</h2>${table(['Member','Required'], unit.status.required.map(p => [repoLink(unit.required_paths?.[p] || p), 'yes']))}
      <details><summary>Optional outputs</summary>${json(unit.optional)}</details><h2>Generation and tiers</h2>${json(unit.status)}<h3>Runtime metadata defects</h3>${json(unit.runtime_defects || [])}<h2>Dependents</h2>${json(unit.dependents)}
      <h2>Animation coverage</h2>${coverage(unit)}<h2>Preview YAML / runtime RON</h2>${json(unit.preview_comparison || 'Preview metadata was not scanned in this report')}
      <h2>Lineage</h2>${unit.lineages.map(name => `<h3>${esc(name)}</h3>${json(I.lineages[name])}`).join('') || '<p>unknown</p>'}
      <details><summary>Accepted receipt and source support</summary>${json({receipt:unit.receipt,source:unit.source})}</details>`;
  }
  function identityRows(units) {
    const included = new Set(units.map(u => u.id));
    const ids = Object.values(I.identities).filter(a => a.units.some(u => included.has(u)));
    const children = new Map();
    for (const actor of ids) {
      const parent = actor.relations.form_of || actor.relations.part_of;
      if (parent) { const key = actor.provider + ':' + parent; if (!children.has(key)) children.set(key,[]); children.get(key).push(actor); }
    }
    const seen = new Set();
    function show(actor) {
      if (seen.has(actor.key)) return '';
      seen.add(actor.key);
      const cover=(I.units[actor.units[0]]?.assets || []).map(id=>C.assets[id]).find(a=>a.identities?.includes(actor.key))?.cover;
      return `<li>${cover?`<img src="${esc(cover)}" loading="lazy" width="36" height="48" style="object-fit:contain;vertical-align:middle" alt="">`:''}<b><a href="#character/${enc(actor.key)}">${esc(actor.display)}</a></b> <code>${esc(actor.key)}</code> ${actor.relations.form_of ? chip('form') : actor.relations.part_of ? chip('part') : ''}
        ${actor.units.map(uid => link(uid)).join(' · ')} ${actor.relations.derived_from ? `<span>derived from ${esc(actor.relations.derived_from)} (independent identity)</span>` : ''}
        ${(children.get(actor.key) || []).length ? `<details><summary>Forms and parts</summary><ul>${children.get(actor.key).map(show).join('')}</ul></details>` : ''}</li>`;
    }
    return `<details open><summary>Game identities, forms and parts (${ids.length})</summary><ul>${ids.filter(a => !a.relations.form_of && !a.relations.part_of).map(show).join('')}${ids.filter(a => !seen.has(a.key)).map(show).join('')}</ul></details>`;
  }
  function compare() {
    const area = document.getElementById('comparison');
    const selected = [...chosen].flatMap(uid => {
      const u=I.units[uid]; if(!u) return [];
      const looks=(u.assets || []).map(id=>C.assets[id]).filter(a=>a.layout);
      return looks.length?looks.map(a=>({...u,compare_asset:a})):[u];
    });
    area.innerHTML = `<h2>Compare published action (${selected.length})</h2><p><label>action <input id="compare-action" value="death"></label> <label>view <select id="compare-framing"><option value="subject">Body</option><option value="frame">Full animation frame</option></select></label> <label>clock <select id="compare-clock"><option value="time">elapsed time</option><option value="phase">common phase</option></select></label> <label>scale <select id="compare-scale"><option>1</option><option selected>2</option><option>3</option></select></label> <button id="compare-play">restart / play</button> <button id="compare-pause">pause</button> <input id="compare-scrub" aria-label="Comparison time" type="range" min="0" max="5000" value="0"></p><div class="thumbs">${selected.map((u,i) => `<figure><canvas id="compare-${i}" class="checker"></canvas><figcaption>${link(u.id, u.compare_asset?.display || u.id)} ${esc(u.compare_asset?.name || "")} <span id="compare-state-${i}"></span></figcaption></figure>`).join('')}</div>`;
    let origin = performance.now(), elapsed = 0, playing = true, generation = 0;
    const images=new Map();
    function load(src) { if(!images.has(src)) images.set(src,new Promise(resolve=>{const im=new Image();im.onload=()=>resolve(im);im.onerror=()=>resolve(null);im.src=src;})); return images.get(src); }
    const get = id => document.getElementById(id);
    async function draw() {
      const request = ++generation, action = get('compare-action').value;
      const cells = selected.map(u => {
        const asset = u.compare_asset;
        const manifest=asset?.products.find(path=>C.products[path].role==='sheet_record' && C.products[path].tier==='full');
        const resolution = u.published.find(p => p.tier === 'full' && manifest && p.path.endsWith('/'+manifest.split('/').slice(1).join('/')))?.actions[action];
        const row = asset?.layout.rows.find(r => r.name === (resolution?.row || action));
        return {u,asset,row,resolution};
      });
      const period = Math.max(1,...cells.map(c => c.row ? c.row.frames * (c.row.ms || 100) : 0));
      await Promise.all(cells.map(async (c,i) => {
        const canvas = get(`compare-${i}`); if (!canvas) return;
        get(`compare-state-${i}`).textContent = c.resolution ? `${c.resolution.state}: ${c.resolution.row}` : 'runtime resolution unknown';
        const subject = get('compare-framing').value === 'subject';
        const src = subject ? c.row?.subject_strip : c.row?.strip;
        if (!src) { canvas.width=1;canvas.height=1;return; }
        const image = await load(src);
        if (request !== generation || !image) return;
        const [w,h] = subject ? c.asset.layout.framing.cell : c.asset.layout.cell, scale = +get('compare-scale').value;
        canvas.width=w*scale;canvas.height=h*scale;
        const frame = Math.min(c.row.frames-1,Math.floor(get('compare-clock').value === 'phase' ? elapsed/period*c.row.frames : elapsed/(c.row.ms || 100)));
        const columns = subject ? c.row.subject_columns : c.row.strip_columns;
        canvas.getContext('2d').drawImage(image,(frame%columns)*w,Math.floor(frame/columns)*h,w,h,0,0,w*scale,h*scale);
      }));
    }
    const timer = setInterval(() => { if (playing) { elapsed=performance.now()-origin; get('compare-scrub').value=Math.min(5000,elapsed); draw(); } },100);
    stop = () => { generation++; clearInterval(timer); };
    get('compare-play').onclick=()=>{elapsed=0;origin=performance.now();playing=true;draw();};
    get('compare-pause').onclick=()=>{playing=false;};
    get('compare-scrub').oninput=e=>{playing=false;elapsed=+e.target.value;draw();};
    ['compare-action','compare-clock','compare-scale','compare-framing'].forEach(id=>get(id).onchange=()=>{origin=performance.now();elapsed=0;draw();});
    draw();
  }
  window.renderArtInventory = function (route) {
    stop(); stop=()=>{};
    const app=document.getElementById('app');
    if (!I) { app.innerHTML='<p>Rebuild the inspector to obtain owner reports.</p>'; return; }
    let body='', mode=route.split('/')[0];
    const head=`<nav class="tabs inventory-tabs">${tabs.map(([key,label])=>`<a href="#inventory/${key}"${mode===key?' class="on"':''}>${label}</a>`).join('')}</nav><p>Provider ${esc(I.profile.provider || 'unavailable')} · pack ${esc(I.profile.pack || 'unknown')} · content <code>${esc(I.profile.fingerprint || 'unavailable')}</code> · ${I.complete?'owner reports complete':'owner reports incomplete'}</p>${I.warnings.length?`<details><summary>Owner notices (${I.warnings.length})</summary>${json(I.warnings)}</details>`:''}`;
    if(mode==='unit') body=unitView(I.units[route.slice(5)]);
    else if(mode==='concepts') body=table(['Concept','Products'],Object.entries(I.concepts).map(([key,ids])=>[esc(key),ids.map(uid=>link(uid)).join(' · ')]));
    else if(mode==='history') {
      body=Object.entries(C.lineages || I.lineages).filter(([key,row])=>JSON.stringify([key,row]).toLowerCase().includes(document.getElementById('q').value.toLowerCase())).map(([key,row])=>{
        const actor=Object.values(I.identities).find(actor=>actor.lineages?.includes(key) && !actor.relations.part_of && !actor.relations.form_of);
        const current=actor?.preview ? {...C.assets[actor.preview.asset], display:actor.display, cover:actor.preview.cover} :
          Object.values(C.assets).find(asset=>asset.identities?.length && asset.lineages?.some(lineage=>lineage.lineage===key));
        return window.ArtLineage.render(row, current);
      }).join('');
    }
    else if(mode==='unregistered') body=table(['Product','Registration','Declared producer','Lineage'],I.local_products.filter(p=>['unregistered','unknown'].includes(p.registration)).map(p=>[esc(p.path),chip(p.registration),esc((p.provenance || []).map(r=>r.generator.id).join(', ') || 'unknown'),esc(p.lineages.join(', ') || 'unknown')]));
    else if(mode==='runtime') { const r=I.runtime; body=r?`<h2>Live snapshot</h2><p>Content ${r.content_fingerprint===I.profile.fingerprint?'matches':'differs or unavailable'} · tick ${esc(r.tick)} · loaded sheet generation ${esc(r.sheet_generation)}</p>${table(['Character','Target','Requested pose / clip','Resolved row / frame','Tier','Local product'],r.visuals.map(v=>[esc(v.character_id),esc(v.target),esc(v.requested_pose+' / '+v.requested_clip),esc(v.resolved_row+' / '+v.frame),esc(v.quality_tier),(() => {const u=Object.values(I.units).find(u=>u.published.some(p=>p.key===v.sheet_key)); return u?link(u.id)+' · '+(u.published.some(p=>p.fingerprint===v.manifest_fingerprint)?'loaded manifest matches local product':'loaded manifest differs from local products'):'unknown';})() ]))}${json(r)}`:'<p>Build with <code>--runtime-snapshot PATH</code>. Capture one with <code>capture_scene --art-snapshot</code>. A local report cannot establish live state.</p>'; }
    else {
      let units=Object.values(I.units).filter(u=>mode==='current'?['referenced','available-look'].includes(u.game_use):mode==='needs'?needs(u):mode==='unused'?u.game_use==='no-known-use':true);
      const fields=['generation_status','game_use','registration','kind','generator_family','visual_family','species','body_base','identity_kind','metadata_state','lineage','tag','animation_gap'];
      const field=(u,f)=>f==='generation_status'?[u.status.state]:f==='game_use'?[u.game_use]:f==='registration'?[u.registration]:f==='kind'?[u.kind]:f==='generator_family'?[u.generator_family || u.generator.family]:f==='lineage'?(u.lineages.length?u.lineages:['unknown']):f==='tag'?u.identities.flatMap(cid=>I.identities[cid].catalog.tags || []):f==='metadata_state'?[...new Set(Object.values(u.facts).map(f=>f.state))]:f==='animation_gap'?u.identities.flatMap(cid=>I.identities[cid].coverage.filter(c=>!c.valid || !['direct','alias'].includes(c.resolution.state)).map(c=>c.action)):[value(u.facts[f])];
      const facetHtml=fields.map(f=>{const values=[...new Set(units.flatMap(u=>field(u,f)))].sort();return `<label>${esc(f)} <select data-inv-facet="${f}"><option value="">all</option>${values.map(v=>`<option${filters[f]===v?' selected':''}>${esc(v)}</option>`).join('')}</select></label>`;}).join(' ');
      units=units.filter(u=>fields.every(f=>!filters[f] || field(u,f).includes(filters[f])) && JSON.stringify([u.id,u.kind,u.facts,u.generator,u.source,u.lineages,u.identities.map(cid=>I.identities[cid])]).toLowerCase().includes(document.getElementById('q').value.toLowerCase()));
      body=`<div class="controls">${facetHtml}</div><p>${units.length} bundles <button id="compare-chosen">Compare selected action</button></p>${identityRows(units)}${table(['Compare','Bundle','Kind','Game use','Generation','Tier gaps','Family','Lineage'],units.map(u=>[`<input type="checkbox" data-choose="${esc(u.id)}"${chosen.has(u.id)?' checked':''}>`,link(u.id),esc(u.kind),chip(u.game_use),chip(u.status.state),esc(Object.entries(u.status.tiers || {}).filter(([,v])=>v.state!=='current').map(([k,v])=>k+': '+v.state).join(', ')),esc(value(u.facts.visual_family)),esc(u.lineages.join(', ') || 'unknown')]))}<div id="comparison"></div>`;
    }
    app.innerHTML=head+body;
    app.querySelectorAll('[data-inv-facet]').forEach(el=>el.onchange=()=>{filters[el.dataset.invFacet]=el.value;window.renderArtInventory(route);});
    app.querySelectorAll('[data-choose]').forEach(el=>el.onchange=()=>{if(el.checked)chosen.add(el.dataset.choose);else chosen.delete(el.dataset.choose);});
    const button=document.getElementById('compare-chosen');if(button)button.onclick=()=>{stop();compare();};
  };
  window.addEventListener('hashchange',()=>{stop();stop=()=>{};});
})();
