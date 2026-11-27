(function () {
  "use strict";
  const served = ["http:", "https:"].includes(location.protocol);
  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"})[c]);
  const dialog = document.createElement("dialog");
  dialog.className = "box content-editor";
  dialog.setAttribute("aria-labelledby", "content-editor-title");
  dialog.innerHTML = `<form>
    <h2 id="content-editor-title"></h2>
    <div data-fields></div>
    <p class="controls"><button type="submit" disabled>Save</button><button type="button" data-close>Close</button></p>
    <p role="status" aria-live="polite"></p>
  </form>`;
  document.body.append(dialog);
  const form = dialog.querySelector("form"), fields = form.querySelector("[data-fields]");
  const submit = form.querySelector("[type=submit]"), status = form.querySelector("[role=status]");
  form.querySelector("[data-close]").onclick = () => dialog.close();
  let session = 0;

  async function request(path, keys, update) {
    const response = await fetch(update ? path : path + "?" + new URLSearchParams(keys), update ? {
      method: "POST", headers: {"Content-Type": "application/json"},
      body: JSON.stringify({...keys, ...update}),
    } : {cache: "no-store"});
    const data = await response.json().catch(() => ({}));
    if (response.status === 404) throw new Error("Restart the inspector server to load the editor.");
    if (!response.ok) throw new Error(data.error || "The editor could not read or save this source.");
    return data;
  }

  async function open(title, path, keys, draw, read, saved) {
    const active = ++session;
    dialog.querySelector("h2").textContent = title;
    fields.replaceChildren();
    fields.inert = false;
    status.textContent = "Reading the source…";
    submit.disabled = true;
    form.onsubmit = event => event.preventDefault();
    dialog.showModal();
    const current = () => dialog.open && active === session;
    try {
      let record = await request(path, keys);
      if (!current()) return;
      const render = () => { submit.disabled = !draw(fields, record); };
      status.textContent = "";
      render();
      form.onsubmit = async event => {
        event.preventDefault();
        const update = read(fields, record);
        if (!update) { status.textContent = "No changes to save."; return; }
        submit.disabled = true;
        fields.inert = true;
        status.textContent = "Saving…";
        try {
          const result = await request(path, keys, {...update, revision: record.revision});
          saved(result);
          if (!current()) return;
          record = {...record, ...result};
          render();
          status.textContent = "Saved.";
        } catch (error) {
          if (!current()) return;
          submit.disabled = false;
          status.textContent = error.message;
        } finally {
          if (current()) fields.inert = false;
        }
      };
    } catch (error) {
      if (current()) status.textContent = error.message;
    }
  }

  const MUSIC = {music_track: "Room music", fight_music_track: "Fight music"};
  function music(world, room, saved) {
    return open("Music for " + room, "/api/room-music", {world, room}, (root, record) => {
      const tracks = Object.entries(record.tracks).sort(([id, a], [other, b]) => a.localeCompare(b) || id.localeCompare(other));
      root.innerHTML = (record.area === room ? "" : `<p>Shared room <code>${esc(record.area)}</code></p>`) +
        Object.entries(MUSIC).map(([field, label]) => {
          const {value, reason} = record.fields[field];
          if (reason) return `<p><b>${label}</b> ${esc(record.tracks[value] || value || "Unset")}<br><small>${esc(reason)}</small></p>`;
          const option = (id, title) => `<option value="${esc(id)}"${id === value ? " selected" : ""}>${esc(title)}</option>`;
          return `<label>${label}<select name="${field}" aria-label="${label}">
            ${option("", "No room override")}
            ${value && !Object.hasOwn(record.tracks, value) ? option(value, value) : ""}
            ${tracks.map(([id, title]) => option(id, title + " (" + id + ")")).join("")}
          </select></label>`;
        }).join("");
      return Object.values(record.fields).some(field => !field.reason);
    }, (root, record) => {
      const changes = {};
      for (const select of root.querySelectorAll("select")) {
        if (select.value !== record.fields[select.name].value) changes[select.name] = select.value;
      }
      return Object.keys(changes).length ? {tracks: changes} : null;
    }, saved);
  }

  function cast(id, name, levels, saved) {
    return open("Cast for " + name, "/api/character-cast", {id}, (root, record) => {
      root.innerHTML = `<label>Importance<select name="importance" aria-label="Importance">
        ${levels.map(level => `<option${level === record.cast.importance ? " selected" : ""}>${esc(level)}</option>`).join("")}
        </select></label>
        <div data-roles></div><button type="button" data-add-role>Add role</button>`;
      const rows = root.querySelector("[data-roles]");
      function add(group = "", role = "") {
        const row = document.createElement("div");
        row.className = "controls cast-role-edit";
        row.innerHTML = `<label>Group<input name="group" value="${esc(group)}" placeholder="story"></label>
          <label>Role<input name="role" value="${esc(role)}" placeholder="ally"></label>
          <button type="button" data-remove-role aria-label="Remove role">Remove</button>`;
        row.querySelector("[data-remove-role]").onclick = () => row.remove();
        rows.append(row);
        return row;
      }
      for (const [group, labels] of Object.entries(record.cast.roles)) for (const role of labels) add(group, role);
      root.querySelector("[data-add-role]").onclick = () => add().querySelector("input").focus();
      return true;
    }, root => {
      const roles = Object.create(null);
      for (const row of root.querySelectorAll(".cast-role-edit")) {
        const group = row.querySelector("[name=group]").value.trim(), role = row.querySelector("[name=role]").value.trim();
        if (group || role) (roles[group] ||= []).push(role);
      }
      return {cast: {importance: root.querySelector("[name=importance]").value, roles}};
    }, saved);
  }

  window.AssetEdits = {served, music, cast};
})();
