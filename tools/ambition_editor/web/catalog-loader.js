(function () {
  "use strict";
  const C = window.ART_CATALOG;
  if (!C) {
    document.getElementById("app").innerHTML = '<p class="empty">Build the catalog with <code>python3 scripts/asset_inspector.py build</code>.</p>';
    return;
  }
  const pending = new Map();
  const data = new Map();
  function load(name) {
    if (!Object.hasOwn(C.chunks, name)) return Promise.resolve({});
    if (!pending.has(name)) {
      const script = document.createElement("script");
      const promise = new Promise((resolve, reject) => {
        script.onload = () => data.has(name) ? resolve(data.get(name)) : reject(new Error(name));
        script.onerror = () => reject(new Error(name));
        script.src = "catalog-data/" + C.chunks[name] + ".js";
        document.head.append(script);
      }).catch(error => {
        pending.delete(name);
        throw error;
      }).finally(() => script.remove());
      pending.set(name, promise);
    }
    return pending.get(name);
  }
  function record(type, key) {
    let bucket = 0;
    for (const char of key) bucket = (bucket * 31 + char.codePointAt(0)) % 128;
    return load(type + "-" + bucket);
  }
  async function assetRecords(ids) {
    const files = await Promise.all(new Set(ids.map(id => record("asset", id))));
    for (const file of files) {
      Object.assign(C.assetDetails, file.assets);
      Object.assign(C.products, file.products);
      Object.assign(C.assetFindings, file.findings);
      Object.assign(C.assetGenerators, file.generators);
      Object.assign(C.characterLabels, file.character_labels);
    }
  }
  const preview = document.getElementById("card-previews");
  try { preview.checked = localStorage.getItem("asset-inspector-list-previews") !== "off"; } catch (error) {}
  const setPreviewMode = () => { document.documentElement.dataset.listPreviews = preview.checked ? "on" : "off"; };
  setPreviewMode();
  preview.onchange = () => {
    setPreviewMode();
    try { localStorage.setItem("asset-inspector-list-previews", preview.checked ? "on" : "off"); } catch (error) {}
    window.dispatchEvent(new Event("asset-previews-changed"));
  };
  Object.assign(C, {assets: {}, products: {}, assetDetails: {}, assetFindings: {}, assetGenerators: {},
    characterLabels: {}, characterDetails: {}, items: {...C.attention}});
  Object.assign(C.inventory, {identities: {}, units: {}});
  window.ArtCatalog = {load, record, assetRecords, receive: (name, value) => data.set(name, value), previews: () => preview.checked};
})();
