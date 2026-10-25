'use strict';
(() => {
  const model = JSON.parse(document.getElementById('ecs-data').textContent);
  const records = model.sections;
  const allRecords = Object.values(records).flat();
  records.search = allRecords;
  const byId = new Map(allRecords.map(row => [row.id, row]));
  const nav = [
    ['overview', 'Overview', '◎'], ['search', 'All records', '⌕'], ['systems', 'Systems', '⌘'],
    ['components', 'Components', '⬡'], ['resources', 'Resources', '▣'],
    ['messages', 'Messages', '↔'], ['channels', 'Message channels', '⇆'], ['bundles', 'Bundles', '◫'],
    ['spawns', 'Entity spawn sites', '◇'], ['crates', 'Crates', '▦'],
    ['plugins', 'Plugins', '✚'], ['registrations', 'Registrations', '⇄'],
  ];
  const texts = {
    overview: ['ECS architecture', 'Browse indexed definitions, source-level access, and architecture relationships.'],
    search: ['All indexed records', 'Search across systems, components, crates, plugins, spawn sites, and wiring.'],
    systems: ['Systems', 'System-like Rust functions; query counts are parameter counts, not execution cost.'],
    components: ['Components', 'Types deriving Component, with inferred query uses and spawn evidence.'],
    resources: ['Resources', 'Types deriving Resource, with indexed read/write access.'],
    messages: ['Messages', 'Types deriving Message. Channel registrations may refer to additional external message types.'],
    channels: ['Message channels', 'Message-bus readers, writers, and registrations reported by crate inventories.'],
    bundles: ['Bundles', 'Types deriving Bundle in this workspace.'],
    spawns: ['Entity spawn sites', 'Static construction expressions and matched components. Not live entities or complete archetypes.'],
    crates: ['Crates', 'Workspace ECS inventories with declared internal dependency edges.'],
    plugins: ['Plugins', 'Bevy plugin declarations and their matched registration contexts.'],
    registrations: ['Registrations', 'add_systems, configure_sets, resources, messages, and plugin registration sites.'],
  };
  const fields = {
    search: [['name','Name'],['kind','Category'],['crate','Crate'],['location','Source']],
    systems: [['name','System'],['crate','Crate'],['queryCount','Queries'],['componentCount','Known components'],['resourceCount','Resources'],['registeredHere','Linked registration']],
    components: [['name','Component'],['crate','Crate'],['usageCount','Systems'],['reads','Reads'],['writes','Writes'],['filters','Filters']],
    resources: [['name','Resource'],['crate','Crate'],['usageCount','Systems'],['reads','Reads'],['writes','Writes']],
    messages: [['name','Message'],['crate','Crate'],['usageCount','Systems'],['reads','Readers'],['writes','Writers']],
    channels: [['name','Channel'],['crate','Crate'],['readersCount','Read sites'],['writersCount','Write sites'],['registeredCount','Registered sites']],
    bundles: [['name','Bundle'],['crate','Crate'],['usageCount','Systems']],
    spawns: [['name','Spawn evidence'],['crate','Crate'],['componentCount','Components'],['location','Source']],
    crates: [['name','Crate'],['systemsCount','Systems'],['componentsCount','Components'],['resourcesCount','Resources'],['spawnsCount','Spawns'],['regCount','Registrations']],
    plugins: [['name','Plugin'],['crate','Crate'],['registrationCount','Registrations'],['location','Source']],
    registrations: [['name','Kind / Context'],['crate','Crate'],['schedule','Schedule'],['linkedCount','Matched systems'],['location','Source']],
  };
  const sorting = {
    search: [['name','Name A–Z'],['crate','Crate A–Z']],
    systems: [['queryCount','Most queries'],['componentCount','Most component refs'],['resourceCount','Most resource refs'],['name','Name A–Z'],['crate','Crate A–Z']],
    components: [['usageCount','Most systems'],['reads','Most reads'],['writes','Most writes'],['filters','Most filters'],['name','Name A–Z'],['crate','Crate A–Z']],
    resources: [['usageCount','Most systems'],['writes','Most writes'],['reads','Most reads'],['name','Name A–Z']],
    messages: [['usageCount','Most systems'],['writes','Most writers'],['reads','Most readers'],['name','Name A–Z']],
    channels: [['readersCount','Most reader sites'],['writersCount','Most writer sites'],['name','Name A–Z'],['crate','Crate A–Z']],
    bundles: [['name','Name A–Z'],['crate','Crate A–Z']],
    spawns: [['name','Name A–Z'],['crate','Crate A–Z']],
    crates: [['systemsCount','Most system candidates'],['componentsCount','Most components'],['regCount','Most registrations'],['name','Name A–Z']],
    plugins: [['registrationCount','Most registrations'],['name','Name A–Z'],['crate','Crate A–Z']],
    registrations: [['name','Kind / Context'],['linkedCount','Most matched systems'],['crate','Crate A–Z']],
  };
  const state = { view:'overview', query:'', crate:'', sort:'', page:0, registered:false, selected:null, filtered:[] };
  const el = name => document.getElementById(name);
  const esc = value => String(value == null ? '' : value).replace(/[&<>"']/g, ch => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[ch]));
  const fmt = v => Number(v || 0).toLocaleString('en-US');
  const titleOf = id => byId.get(id)?.name || id;
  const openButton = (id, title, extra = '') => `<button type="button" class="item-link ${extra}" data-open="${esc(id)}" title="Inspect ${esc(title)}">${esc(title)}</button>`;
  const related = (ids, max = 36) => {
    if (!ids || !ids.length) return '<p class="no-match">No matching indexed records</p>';
    return `<div class="related-list">${ids.slice(0,max).map(id => `<button class="related-link" data-open="${esc(id)}">${esc(titleOf(id))} <span class="dim">/ ${esc(byId.get(id)?.crate || '')}</span></button>`).join('')}${ids.length > max ? `<p class="detail-note">Showing ${max} of ${ids.length} matches.</p>` : ''}</div>`;
  };
  const section = (title, content, note = '') => `<section class="detail-section"><h3>${esc(title)}</h3>${note ? `<p class="detail-note">${esc(note)}</p>` : ''}${content}</section>`;
  const chips = (items, kind = '') => `<div class="chiplist">${(items||[]).length ? items.map(v => `<span class="chip ${kind}">${esc(v)}</span>`).join('') : '<span class="no-match">None recorded</span>'}</div>`;
  const met = (n,label) => `<div class="micro-metric"><strong>${fmt(n)}</strong><span>${esc(label)}</span></div>`;
  const loc = row => `${row.file || ''}${row.line ? ':' + row.line : ''}`;
  const sourceSection = row => {
    let body = `<div class="detail-loc">${esc(loc(row))}</div>`;
    body += `<div class="detail-toolbar"><button class="ghost small" data-copy="${esc(loc(row))}">Copy source location</button>`;
    if (row.file) body += `<a class="ghost small" style="text-decoration:none" href="${encodeURI('../../' + row.file)}" target="_blank" rel="noopener">Open local file ↗</a>`;
    body += '</div>';
    if (row.excerpt?.text) {
      const numbered = row.excerpt.text.split('\n').map((line,i) => `${String(i + row.excerpt.start).padStart(5)}  ${line}`).join('\n');
      body += `<pre class="detail-code">${esc(numbered)}</pre>`;
    }
    return section('Source',body);
  };
  const getCell = (row, key) => {
    if (key === 'readersCount') return row.readers?.length || 0;
    if (key === 'writersCount') return row.writers?.length || 0;
    if (key === 'registeredCount') return row.registeredAt?.length || 0;
    if (key === 'resourceCount') return (row.resourceReads?.length || 0) + (row.resourceWrites?.length || 0);
    if (key === 'componentCount') return row.componentCount ?? row.components?.length ?? 0;
    if (key === 'registrationCount') return row.registrations?.length || 0;
    if (key === 'linkedCount') return row.systems?.length || 0;
    if (key === 'systemsCount') return row.systems?.length || 0;
    if (key === 'componentsCount') return row.components?.length || 0;
    if (key === 'resourcesCount') return row.resources?.length || 0;
    if (key === 'spawnsCount') return row.spawns?.length || 0;
    if (key === 'regCount') return row.counts?.registrations || 0;
    if (key === 'location') return loc(row);
    return row[key] ?? '';
  };
  function init() {
    el('commit').textContent = model.meta.commit ? `INDEX · ${model.meta.commit}` : 'INDEX · UNSTAMPED';
    el('footer-status').textContent = `${fmt(model.stats.crateCount)} crates · commit ${model.meta.commit || '?'} `;
    if (model.meta.stale) { el('stale').hidden = false; el('stale').textContent = `Warning: index ${model.meta.commit} differs from checked-out HEAD ${model.meta.head}. Refresh the generated indexes before relying on this view.`; }
    el('navigation').innerHTML = nav.map(([id,label,icon]) => `<button class="nav-btn" data-view="${id}"><span><span aria-hidden="true" style="font-family:var(--mono);margin-right:9px">${icon}</span>${esc(label)}</span><span>${id === 'overview' ? '' : fmt(records[id].length)}</span></button>`).join('');
    el('crate-filter').innerHTML = `<option value="">All crates</option>${records.crates.map(c => `<option value="${esc(c.name)}">${esc(c.name)}</option>`).join('')}`;
    el('search').addEventListener('input', ev => {state.query=ev.target.value.trim().toLowerCase();state.page=0;if(state.view==='overview' && state.query) state.view='search';render();});
    el('crate-filter').addEventListener('change', ev => {state.crate=ev.target.value;state.page=0;renderListing();});
    el('sort').addEventListener('change', ev => {state.sort=ev.target.value;state.page=0;renderListing();});
    el('registered-only').addEventListener('change', ev => {state.registered=ev.target.checked;state.page=0;renderListing();});
    el('prev').addEventListener('click', () => {if(state.page>0){state.page--;renderListing();}});
    el('next').addEventListener('click', () => {if((state.page+1)*75<state.filtered.length){state.page++;renderListing();}});
    el('close-detail').addEventListener('click', () => {state.selected=null;renderDetail();});
    el('export').addEventListener('click', exportCSV);
    document.addEventListener('click', ev => {
      const v = ev.target.closest('[data-view]');if(v){selectView(v.dataset.view);return;}
      const o = ev.target.closest('[data-open]');if(o){openItem(o.dataset.open);return;}
      const c = ev.target.closest('[data-copy]');if(c){copyToClipboard(c.dataset.copy,c);return;}
    });
    document.addEventListener('keydown', ev => {
      if(ev.key==='/' && !['INPUT','SELECT','TEXTAREA'].includes(document.activeElement.tagName)) {ev.preventDefault();el('search').focus();}
      if(ev.key==='Escape'){if(state.selected){state.selected=null;renderDetail();}else if(el('search').value){el('search').value='';state.query='';render();}}
    });
    const navigateHash=()=>{const raw=decodeURIComponent(location.hash.slice(1));if(raw.startsWith('item/'))openItem(raw.slice(5),false);else if(nav.some(x=>x[0]===raw))selectView(raw,false);};
    window.addEventListener('hashchange',navigateHash);
    navigateHash();render();
  }
  function selectView(view,hash=true) {
    if(!texts[view])return;
    state.view=view;state.page=0;state.crate='';state.sort='';state.registered=false;state.selected=null;
    el('crate-filter').value='';el('registered-only').checked=false;
    if(hash)history.replaceState(null,'','#'+view);
    render();
  }
  function openItem(id,hash=true){
    const row=byId.get(id);if(!row)return;
    state.selected=id;
    if(hash)history.replaceState(null,'','#item/'+encodeURIComponent(id));
    renderDetail();
  }
  function render() {
    const view=state.view;
    document.querySelectorAll('.nav-btn').forEach(btn=>{const active=btn.dataset.view===view;btn.classList.toggle('active',active);btn.setAttribute('aria-current',active?'page':'false');});
    el('crumb').textContent=`REPOSITORY INTELLIGENCE / ${view.toUpperCase()}`;
    el('page-title').textContent=texts[view][0]; el('page-subtitle').textContent=texts[view][1];
    el('overview').hidden=view!=='overview';el('listing').hidden=view==='overview';
    el('export').disabled=view==='overview';
    if(view==='overview')renderOverview();else{
      const list=sorting[view];state.sort=state.sort||list[0][0];
      el('sort').innerHTML=list.map(([id,title])=>`<option value="${esc(id)}">${esc(title)}</option>`).join('');
      el('sort').value=state.sort;
      el('registered-label').hidden=view!=='systems';
      renderListing();
    }
    renderDetail();
  }
  function statCard(n,label,tag,view){return `<button class="stat-card" data-view="${view}"><span class="stat-tag">${esc(tag)}</span><strong class="stat-value">${fmt(n)}</strong><span class="stat-label">${esc(label)}</span></button>`;}
  function renderOverview(){
    const x=model.stats;
    el('statcards').innerHTML=[
      statCard(records.systems.length,'System-like functions','SYSTEMS / STATIC','systems'),
      statCard(records.components.length,'Components','QUERY TARGETS','components'),
      statCard(records.resources.length,'Resources','SHARED STATE','resources'),
      statCard(records.spawns.length,'Entity spawn sites','ENTITY EVIDENCE','spawns'),
      statCard(records.crates.length,'Workspace crates','COMPOSITION','crates'),
      statCard(records.registrations.length,'Registrations','PLUGIN WIRING','registrations'),
      statCard(x.queryParams,'Query parameters','SYSTEM SIGNATURES','systems'),
      statCard(x.registeredMatched,'System candidates linked to registration','MATCHED REFERENCES','systems'),
    ].join('');
    const chart=(title,note,items,key,section,color='') => {
      const max=Math.max(...items.map(row=>Number(key(row))),1);
      return `<article class="chart-card"><div class="chart-heading"><h3>${esc(title)}</h3><button data-view="${section}">Browse all →</button></div><p class="chart-note">${esc(note)}</p><div class="bar-list ${color}">${items.map(row=>{
        const n=Number(key(row));const width=(n/max*100).toFixed(2)+'%';
        return `<button class="bar-item" data-open="${esc(row.id)}"><span class="bar-name" title="${esc(row.name)}">${esc(row.name)}</span><span class="bar-value">${fmt(n)}</span><span class="bar-track"><span class="bar-fill" style="display:block;--bar:${width}"></span></span></button>`;
      }).join('')}</div></article>`;
    };
    const top=(sect, key) => [...records[sect]].sort((a,b)=>key(b)-key(a)||a.name.localeCompare(b.name)).slice(0,8);
    el('charts').innerHTML=[
      chart('Systems with the most queries','Count of Query<> parameters in each function.',top('systems',s=>s.queryCount),s=>s.queryCount,'systems'),
      chart('Most referenced components','Number of system-like functions mentioning each indexed type.',top('components',c=>c.usageCount),c=>c.usageCount,'components','warm'),
      chart('Crates with the most systems','System-like functions per workspace crate.',top('crates',c=>c.systems.length),c=>c.systems.length,'crates','blue'),
      chart('Resources used by most systems','Distinct indexed readers/writers (static).',top('resources',r=>r.usageCount),r=>r.usageCount,'resources','lavender'),
    ].join('');
  }
  function renderListing(){
    const view=state.view;if(view==='overview')return;
    let rows=records[view];const query=state.query;
    if(state.crate) rows=rows.filter(row=>row.crate===state.crate);
    if(state.registered && view==='systems')rows=rows.filter(row=>row.registeredHere);
    if(query){const terms=query.split(/\s+/).filter(Boolean);rows=rows.filter(row=>{
      const hay=[row.name,row.crate,row.file,row.params,row.expression,row.schedule,row.kind,row.regKind,row.context, ...(row.queries||[]),...(row.matched||[]),...(row.labels||[])].filter(Boolean).join(' ').toLowerCase();
      return terms.every(t=>hay.includes(t));
    });}
    const sort=state.sort || sorting[view][0][0];rows=[...rows].sort((a,b)=>{
      const va=getCell(a,sort),vb=getCell(b,sort);
      const cmp=typeof va==='number'&&typeof vb==='number'?vb-va:
        typeof va==='boolean'&&typeof vb==='boolean'?Number(vb)-Number(va):String(va).localeCompare(String(vb));
      return cmp||a.name.localeCompare(b.name)||a.crate.localeCompare(b.crate);
    });
    state.filtered=rows;const count=rows.length;
    if(state.page*75>=count)state.page=Math.max(0,Math.ceil(count/75)-1);
    const visible=rows.slice(state.page*75,(state.page+1)*75);
    const columns=fields[view];
    el('thead').innerHTML='<tr>'+columns.map(([k,label])=>`<th class="${isNumberField(k)?'numeric':''}">${esc(label)}</th>`).join('')+'</tr>';
    el('tbody').innerHTML=visible.length?visible.map(row=>'<tr>'+columns.map(([k])=>{
      const value=getCell(row,k);
      if(k==='name')return `<td>${openButton(row.id,String(value))}</td>`;
      if(k==='crate')return `<td class="crate-cell">${openButton('c:'+row.crate,row.crate)}</td>`;
      if(k==='registeredHere')return `<td class="numeric ${value?'status-ok':'status-weak'}">${value?'Matched':'—'}</td>`;
      return `<td class="${isNumberField(k)?'numeric':'mono dim'}">${typeof value==='number'?fmt(value):esc(String(value))}</td>`;
    }).join('')+'</tr>').join(''):`<tr><td colspan="${columns.length}" class="empty">No records match the current filters.</td></tr>`;
    el('result-count').textContent=`${fmt(count)} of ${fmt(records[view].length)} records`;
    el('page-status').textContent=count?`${fmt(state.page*75+1)}–${fmt(Math.min((state.page+1)*75,count))} of ${fmt(count)}`:'No results';
    el('prev').disabled=state.page===0;el('next').disabled=(state.page+1)*75>=count;
  }
  function isNumberField(key){return !['name','crate','kind','registeredHere','location','schedule'].includes(key);}
  function renderDetail(){
    const panel=el('detail');
    if(!state.selected||!byId.has(state.selected)){panel.hidden=true;return;}
    panel.hidden=false;
    const row=byId.get(state.selected);const kind=row.kind;
    let html=`<div class="detail-content"><div class="detail-category">${esc(kind.toUpperCase())} / ${esc(row.crate)}</div><h2 class="detail-name">${esc(row.name)}</h2><div class="detail-loc">${esc(loc(row))}</div>`;
    if(kind==='System'){
      html+=`<div class="detail-metrics">${met(row.queryCount,'Query parameters')}${met(row.componentCount,'Known component types')}${met(row.resourceReads.length+row.resourceWrites.length,'Resource accesses')}${met(row.registrations.length,'Matched registrations')}</div>`;
      if(!row.registrations.length)html+='<p class="index-warning">No registration was linked by the static name matcher. This does not prove the function is unregistered.</p>';
      html+=section('Function parameters',`<pre class="detail-code">${esc(row.params || 'None')}</pre>`);
      html+=section(`Query signatures (${row.queries.length})`,row.queries.length?row.queries.map(x=>`<pre class="detail-code">${esc(x)}</pre>`).join(''):'<p class="no-match">No Bevy Query parameter</p>');
      const group=role=>row.mentions.filter(m=>role.includes(m.role));
      const mentionChips = mentions => mentions.length?`<div class="chiplist">${mentions.map(m=>m.targets.length?`<button class="chip selectable ${m.role==='write'?'write':m.role!=='read'?'filter':''}" data-open="${esc(m.targets[0])}" title="${esc(m.role)}">${esc(m.name)} · ${esc(m.role)}</button>`:`<span class="chip unknown" title="Type is external, generic, or ambiguous">${esc(m.name)} · ${esc(m.role)}?</span>`).join('')}</div>`:'<p class="no-match">None recorded</p>';
      html+=section('Component access',mentionChips(group(['read','write'])), 'Only Component-derived types in this workspace are linked. External or generic query types remain unresolved.');
      html+=section('Query filters',mentionChips(group(['with','without','has','added','changed'])));
      html+=section('Resource reads',refChips(row.resourceReads,'Resource',row.crate));
      html+=section('Resource writes',refChips(row.resourceWrites,'Resource',row.crate));
      html+=section('Message readers',refChips(row.messageReads,'Message',row.crate));
      html+=section('Message writers',refChips(row.messageWrites,'Message',row.crate));
      html+=section('System registrations',related(row.registrations), 'Links come from static registration-site name matching.');
    } else if(['Component','Resource','Message','Bundle'].includes(kind)){
      html+=`<div class="detail-metrics">${met(row.usageCount,'Systems linked')}${met(row.reads,'Read mentions')}${met(row.writes,'Write mentions')}${met(row.spawnIds.length,'Spawn matches')}</div>`;
      if(row.filters)html+=section('Query filter mentions',chips([`${fmt(row.filters)} static filter references`],'filter'));
      html+=section('Systems mentioning this type',related(row.systems));
      if(kind==='Component')html+=section('Spawn sites mentioning this component',related(row.spawnIds), 'Only statically matched spawn expressions are listed.');
      html+=section('Derives',chips(row.derives));
    } else if(kind==='Message channel'){
      html+=`<div class="detail-metrics">${met(row.readers.length,'Read sites')}${met(row.writers.length,'Write sites')}${met(row.registeredAt.length,'Registration sites')}${met(row.systems.length,'Linked systems')}</div>`;
      html+=section('Reader evidence',chips(row.readers),'Source-level read sites. Each label includes a file and line.');
      html+=section('Writer evidence',chips(row.writers));
      html+=section('Registration evidence',chips(row.registeredAt));
      html+=section('System links',related(row.systems));
    } else if(kind==='Spawn evidence'){
      html+=`<div class="detail-metrics">${met(row.components.length,'Known components')}${met(row.identifiers.length,'Parsed identifiers')}</div>`;
      html+=section('Matched components',related(row.components),'Matches are partial and may omit bundles, commands, or nested constructors.');
      html+=section('Name labels',chips(row.labels));
      html+=section('Spawn expression',`<pre class="detail-code long">${esc(row.expression)}</pre>`);
    } else if(kind==='Crate'){
      html+=`<div class="detail-metrics">${met(row.systems.length,'System candidates')}${met(row.components.length,'Components')}${met(row.regCount||row.counts.registrations,'Registrations')}${met(row.spawns.length,'Spawn sites')}</div>`;
      html+=section('Declared internal dependencies',row.deps.length?`<div class="related-list">${row.deps.map(d=>`<button class="related-link" data-open="${esc('c:'+d.crate)}">${esc(d.crate)} ${d.optional?'· optional':''} <span class="dim">${esc(d.kind)}</span></button>`).join('')}</div>`:'<p class="no-match">None listed</p>','Manifest edges, including optional declarations. Not the per-consumer compiled closure.');
      html+=section('Crates declaring a dependency on this crate',related(row.rdeps.map(x=>'c:'+x)));
      html+=section('Systems',related(row.systems,25));html+=section('Components',related(row.components,25));
      html+=section('Resources',related(row.resources,18));html+=section('Plugins',related(row.plugins,25));
    } else if(kind==='Plugin'){
      html+=`<div class="detail-metrics">${met(row.registrations.length,'Matched registrations')}</div>`;
      html+=section('Registration sites',related(row.registrations));
    } else if(kind==='Registration'){
      html+=`<div class="detail-metrics">${met(row.systemNames.length,'Registration references')}${met(row.systems.length,'Resolved system links')}</div>`;
      html+=section('Schedule / argument',chips([row.schedule || '(not recorded)']));
      html+=section('System expressions',chips(row.systemNames));
      html+=section('Linked system candidates',related(row.systems));
      html+=section('Sets',chips(row.sets));
      html+=section('Run conditions',chips(row.runConditions));
      html+=section('Ordering',chips(row.ordering));
      html+=section('Registration expression',`<pre class="detail-code long">${esc(row.expression)}</pre>`);
    }
    if(row.file)html+=sourceSection(row);
    html+='</div>';
    el('detail-body').innerHTML=html;
    panel.scrollTop=0;
  }
  function refChips(names,kind,crate){
    if(!names?.length)return '<p class="no-match">None recorded</p>';
    return `<div class="chiplist">${names.map(name=>{
      const options=[...(records[kind==='Resource'?'resources':'messages']||[])].filter(r=>r.name===name);
      const local=options.filter(r=>r.crate===crate);const found=local.length===1?local[0]:options.length===1?options[0]:null;
      return found?`<button class="chip selectable" data-open="${esc(found.id)}">${esc(name)}</button>`:`<span class="chip unknown">${esc(name)}</span>`;
    }).join('')}</div>`;
  }
  async function copyToClipboard(value,button){
    try{
      if(navigator.clipboard && window.isSecureContext)await navigator.clipboard.writeText(value);
      else{
        const area=document.createElement('textarea');area.value=value;area.style.position='fixed';area.style.opacity='0';document.body.appendChild(area);area.select();
        if(!document.execCommand('copy'))throw new Error('copy unavailable');area.remove();
      }
      const previous=button.textContent;button.textContent='Copied';setTimeout(()=>{button.textContent=previous;},1200);
    }catch(err){window.prompt('Copy source location',value);}
  }
  function exportCSV(){
    if(state.view==='overview')return;
    const columns=fields[state.view];const csvval=value=>{
      let v=String(value ?? '');if(/^[=+@\-]/.test(v))v="'"+v;return '"'+v.replace(/"/g,'""')+'"';
    };
    const body=[columns.map(([,name])=>csvval(name)).join(',')];
    for(const row of state.filtered)body.push(columns.map(([key])=>csvval(getCell(row,key))).join(','));
    const url=URL.createObjectURL(new Blob(['\ufeff'+body.join('\r\n')+'\r\n'],{type:'text/csv;charset=utf-8'}));
    const link=document.createElement('a');link.href=url;link.download='ambition-ecs-'+state.view+'-'+(model.meta.commit||'snapshot')+'.csv';link.click();
    setTimeout(()=>URL.revokeObjectURL(url),1000);
  }
  // No network requests, runtime access, or dependencies: data is in this file.
  init();
})();
