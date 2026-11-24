/* Sort a table by a click on its column header. The page sorts its rows (all of
   them, before it limits how many it shows) and draws the table again; this
   keeps what is sorted and draws the headers.

     const state = {by: "name", dir: 1};               // kept by the page
     const cols = {name: {label: "Name", value: row => row.name}, ...};
     rows = TableSort.apply(rows, cols, state);
     `<thead><tr>${TableSort.headers(cols, state)}</tr></thead>`
     TableSort.wire(table, state, redraw);              // after it is drawn

   A column `value` gives a number or a string; an empty value (null,
   undefined, "") sorts last in either direction. `first: -1` makes the first
   click sort high to low (scores, dates). A column with no `value` does not
   sort. `state.by` empty: the rows keep their own order. */
(function () {
  "use strict";
  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"})[c]);
  const empty = value => value === null || value === undefined || value === "" || Number.isNaN(value);

  function apply(rows, cols, state) {
    const col = cols[state.by];
    if (!col?.value) return rows;
    const keyed = rows.map((row, index) => [col.value(row), index, row]);
    keyed.sort(([a, ai], [b, bi]) => {
      if (empty(a) || empty(b)) return empty(a) === empty(b) ? ai - bi : empty(a) ? 1 : -1;
      const order = typeof a === "number" && typeof b === "number" ? a - b : String(a).localeCompare(String(b), undefined, {numeric: true});
      return order * state.dir || ai - bi;
    });
    return keyed.map(([, , row]) => row);
  }

  function headers(cols, state) {
    return Object.entries(cols).map(([key, col]) => {
      const title = col.title ? ` title="${esc(col.title)}"` : "";
      if (!col.value) return `<th${title}>${esc(col.label)}</th>`;
      const on = state.by === key;
      const mark = on ? (state.dir > 0 ? " ▲" : " ▼") : "";
      return `<th class="sortable${on ? " sorted" : ""}" data-sort-key="${esc(key)}" data-sort-first="${col.first || 1}"${title}
        aria-sort="${on ? (state.dir > 0 ? "ascending" : "descending") : "none"}" tabindex="0">${esc(col.label)}${mark}</th>`;
    }).join("");
  }

  function wire(table, state, redraw) {
    if (!table) return;
    table.querySelectorAll("th[data-sort-key]").forEach(th => {
      const pick = () => {
        const key = th.dataset.sortKey;
        state.dir = state.by === key ? -state.dir : Number(th.dataset.sortFirst) || 1;
        state.by = key;
        redraw();
      };
      th.onclick = pick;
      th.onkeydown = event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); pick(); } };
    });
  }

  window.TableSort = {apply, headers, wire};
})();
