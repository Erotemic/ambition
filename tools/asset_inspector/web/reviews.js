/* Review files remain readable by the page and by offline workers. */
(function () {
  "use strict";
  const C = window.ART_CATALOG;
  const pageSubjects = C.reviews?.subjects || {};
  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"})[c]);
  let report = C.reviews || {subjects: {}, notes: [], backlog: {open_count: 0, characters: []}}, writable = false, failure = '';
  const drafts = new Map();
  const openCount = key => report.notes.filter(note => note.status === 'open' && (!key || note.subject.character === key)).length;
  function changed() {
    document.querySelector('[data-view="reviews"]').textContent = `Review notes (${openCount()})`;
    window.dispatchEvent(new Event('art-reviews-changed'));
  }
  async function request(method, path, data) {
    const response = await fetch('/api/art-reviews' + path, {method, cache: 'no-store', headers: {'Content-Type': 'application/json'}, ...(data ? {body: JSON.stringify(data)} : {})});
    const result = await response.json();
    if (!response.ok) throw new Error(result.error || 'Could not save review notes.');
    report = result; writable = true; failure = ''; changed();
  }
  function noteHtml(note) {
    return `<article class="review-note"><p><span class="chip ${note.priority === 'high' ? 'c-warn' : 'c-info'}">${esc(note.priority)}</span> ${esc(note.status)} · ${esc(note.generation_relation)} generation <code>${esc(note.subject.generation_id.slice(0, 12))}</code>${note.animation ? ' · ' + esc(note.animation) : ''}</p>
      <p class="review-text">${esc(note.text)}</p><p class="controls"><time>${esc(note.created_at)}</time> <code>${esc(note.id)}</code></p>
      ${note.resolution ? `<p class="review-text">Resolution: ${esc(note.resolution)}</p>` : ''}
      ${writable ? `<p class="controls">${note.status === 'open' ? `<input data-resolution="${esc(note.id)}" aria-label="Resolution summary" placeholder="Fix summary (optional)">` : ''}<button type="button" data-review-status="${note.status === 'open' ? 'resolved' : 'open'}" data-note="${esc(note.id)}">${note.status === 'open' ? 'Resolve' : 'Reopen'}</button></p>` : ''}</article>`;
  }
  function section(key) {
    const subject = pageSubjects[key], notes = report.notes.filter(note => note.subject.character === key);
    const actor = C.inventory.identities[key];
    const rows = [...new Set([...actor.coverage.map(row => row.action), ...actor.units.flatMap(uid =>
      (C.inventory.units[uid].assets || []).flatMap(id => C.assets[id].identities?.includes(key) ? (C.assets[id].layout?.rows || []).map(row => row.name) : []))])].sort();
    const draft = drafts.get(key) || {text: '', priority: 'normal', animation: ''};
    return `<section class="box" id="character-notes"><h2>Review notes (${notes.filter(note => note.status === 'open').length} open)</h2>
      <p>Generation <code>${esc(subject?.generation_id.slice(0, 12) || 'unavailable')}</code>. Notes keep the reviewed product hashes after art changes.</p>
      ${!writable ? `<p class="empty">To save notes, run <code>python3 scripts/asset_inspector.py serve --open</code>. ${esc(failure)}</p>` : ''}
      <form id="character-note-form" data-character="${esc(key)}"><label for="review-text">What needs to change?</label>
        <textarea id="review-text" name="text" rows="4" maxlength="20000" required placeholder="Describe the problem and the result you want.">${esc(draft.text)}</textarea>
        <div class="controls"><label>Priority <select name="priority">${['high', 'normal', 'low'].map(priority => `<option${draft.priority === priority ? ' selected' : ''}>${priority}</option>`).join('')}</select></label>
          <label>Animation <select name="animation"><option value="">Whole character</option>${rows.map(row => `<option${draft.animation === row ? ' selected' : ''}>${esc(row)}</option>`).join('')}</select></label>
          <button type="submit"${!writable || !subject ? ' disabled' : ''}>Save note</button></div><p id="review-save-state" role="status"></p></form>
      ${notes.length ? notes.sort((a, b) => (a.status === b.status ? b.created_at.localeCompare(a.created_at) : a.status === 'open' ? -1 : 1)).map(noteHtml).join('') : '<p class="empty">No review notes for this character.</p>'}</section>`;
  }
  function wire(root = document.getElementById('app')) {
    const form = root.querySelector('#character-note-form');
    if (form) {
      function keepDraft() { drafts.set(form.dataset.character, Object.fromEntries(new FormData(form))); }
      form.oninput = keepDraft;
      form.onsubmit = async event => {
        event.preventDefault(); keepDraft();
        const key = form.dataset.character, subject = pageSubjects[key], button = form.querySelector('[type="submit"]');
        button.disabled = true;
        try {
          const data = Object.fromEntries(new FormData(form));
          drafts.delete(key);
          await request('POST', '', {...data, character: key, generation_id: subject.generation_id});
        } catch (error) {
          drafts.set(key, Object.fromEntries(new FormData(form)));
          document.getElementById('review-save-state').textContent = error.message;
          button.disabled = false;
        }
      };
    }
    root.querySelectorAll('[data-review-status]').forEach(button => button.onclick = async () => {
      const note = report.notes.find(note => note.id === button.dataset.note);
      const resolution = root.querySelector(`[data-resolution="${note.id}"]`)?.value || '';
      button.disabled = true;
      try { await request('PATCH', '/' + note.id, {status: button.dataset.reviewStatus, resolution, updated_at: note.updated_at}); }
      catch (error) { button.disabled = false; button.parentElement.append(document.createTextNode(error.message)); }
    });
  }
  function backlog() {
    const query = document.getElementById('q').value.trim().toLowerCase();
    const groups = report.backlog.characters.filter(group => JSON.stringify(group).toLowerCase().includes(query));
    document.getElementById('app').innerHTML = `<h1>Character review backlog</h1><p>${report.backlog.open_count} open notes across ${report.backlog.characters.length} characters.</p>
      <p>Open notes stay in this queue after regeneration. Review the result, then resolve the note with a fix summary.</p>
      <p class="controls"><button id="export-art-backlog" type="button">Export backlog JSON</button>${writable ? '<button id="refresh-art-notes" type="button">Refresh notes</button>' : ''} <code>python3 scripts/asset_inspector.py notes backlog --json</code></p>
      ${!writable ? '<p class="empty">This page shows the notes saved at build time. Use <code>serve --open</code> to save notes and read live changes.</p>' : ''}
      ${groups.length ? groups.map(group => `<section class="box"><h2><a href="#character/${encodeURIComponent(group.character)}">${esc(group.display)}</a></h2><code>${esc(group.character)}</code>${group.notes.map(noteHtml).join('')}</section>`).join('') : '<p class="empty">No open notes match.</p>'}`;
    document.getElementById('export-art-backlog').onclick = () => {
      const url = URL.createObjectURL(new Blob([JSON.stringify(report.backlog, null, 2)], {type: 'application/json'}));
      const link = document.createElement('a'); link.href = url; link.download = 'art-review-backlog.json'; link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
    };
    const refresh = document.getElementById('refresh-art-notes');
    if (refresh) refresh.onclick = async () => {
      try { await request('GET', ''); }
      catch (error) { refresh.parentElement.append(document.createTextNode(error.message)); }
    };
    wire();
  }
  window.ArtReviews = {section, wire, backlog, openCount};
  setTimeout(async () => {
    if (['http:', 'https:'].includes(location.protocol)) {
      try { await request('GET', ''); }
      catch (error) { failure = 'The review service is unavailable.'; changed(); }
    } else changed();
  }, 0);
})();
