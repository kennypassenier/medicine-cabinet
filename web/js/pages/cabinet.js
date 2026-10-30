// Cabinet: every medicine, searchable and filterable, with the on/off switch
// (it switches the schedule too) and quick stock adjust. A tap opens the edit form.

import { h, redraw } from "../dom.js";
import { api } from "../api.js";
import { wire } from "../controls.js";
import { onTick } from "../ticker.js";
import { button, dataCard, pageHeader, reportError, linkButton, say } from "../ui.js";
import { medicineRow } from "./cabinet-row.js";
import { drawFilterChips, readView, syncUrl, visibleMedicines } from "./cabinet-filters.js";
import { createStockEditor } from "./cabinet-stock.js";
import { openFormsManager } from "./forms-manage.js";

/**
 * Everything the page's functions share.
 * @typedef {{
 *   view: import("./cabinet-filters.js").FilterView, meds: any[], loaded: boolean, alive: boolean,
 *   ui: ReturnType<typeof buildLayout>, stock: ReturnType<typeof createStockEditor>
 * }} Cabinet
 */

/** Build the page skeleton into `root`. @param {HTMLElement} root @param {string} query @param {{ manageForms: () => void, load: () => void }} on */
function buildLayout(root, query, on) {
  const header = pageHeader({ title: "Cabinet", description: "Everything you keep at home. Switch a medicine off to hide its schedule everywhere.", actions: [button("Manage forms", { onClick: on.manageForms }), linkButton("Add medicine", "/medicine/new", "kp-button--primary")] });
  const search = /** @type {HTMLInputElement} */ (h("input", { class: "kp-field__input", type: "search", id: "cab-q", placeholder: "Name, purpose or symptom", autocomplete: "off", value: query }));
  const filterSlot = h("div", { class: "kp-field mc-filters" });
  const formSlot = h("div", { class: "kp-field mc-filters" });
  const tools = h("div", { class: "mc-toolbar" },
    h("div", { class: "kp-field mc-toolbar__search" }, h("label", { class: "kp-field__label", for: "cab-q" }, "Search"), search),
    filterSlot, formSlot);
  const list = dataCard({ id: "cab", title: "Medicines", description: "Newest changes show at once; stock edits are saved a moment after the last tap.", onRetry: on.load });
  list.data.classList.add("mc-rows");
  root.append(header, tools, list.card);
  list.state("loading");
  return { header, search, filterSlot, formSlot, list };
}

function drawFilters(/** @type {Cabinet} */ c) {
  drawFilterChips(c.ui, c.meds, c.view, () => draw(c));
}

/** The list, or the reason there is none. @param {Cabinet} c */
function draw(c) {
  if (!c.loaded) return;
  const { list, header } = c.ui;
  const shown = visibleMedicines(c.meds, c.view);
  header.querySelector(".kp-page-header__description")?.replaceChildren(`${c.meds.length} ${c.meds.length === 1 ? "medicine" : "medicines"} at home. Switch one off to hide its schedule everywhere.`);
  if (!c.meds.length) return list.state("empty", { title: "Your cabinet is empty", text: "Add the first medicine to see it here.", action: linkButton("Add your first medicine", "/medicine/new", "kp-button--primary") });
  if (!shown.length) {
    const clear = h("button", { type: "button", class: "kp-button", onclick: () => reset(c) }, "Clear search and filters");
    return list.state("empty", { title: "No medicine matches", text: "Try another word or another filter.", action: clear });
  }
  const on = { setActive: (/** @type {any} */ m, /** @type {boolean} */ v) => setActive(c, m, v), step: c.stock.step };
  redraw(list.data, () => {
    list.data.replaceChildren(h("ul", { class: "mc-list mc-list--rows" }, ...shown.map((m) => medicineRow(m, on))));
    wire(list.data);
  });
  list.state("ready");
}

function reset(/** @type {Cabinet} */ c) {
  Object.assign(c.view, { query: "", filter: "all", formFilter: "" });
  c.ui.search.value = "";
  syncUrl(c.view);
  drawFilters(c);
  draw(c);
}

async function load(/** @type {Cabinet} */ c) {
  try {
    c.meds = await api.medicines();
    c.loaded = true;
    drawFilters(c);
    draw(c);
  } catch (e) {
    if (!c.alive) return;
    if (!c.loaded) c.ui.list.state("error", { title: "The cabinet could not be loaded", text: e instanceof Error ? e.message : String(e) });
    else reportError("The cabinet could not be refreshed.", e);
  }
}

/** Switch a medicine on or off: show it at once, roll back when the server refuses. @param {Cabinet} c @param {any} m @param {boolean} on */
async function setActive(c, m, on) {
  m.active = on;
  draw(c);
  try {
    const updated = await api.setActive(m.id, on);
    Object.assign(m, updated);
    say(`${m.name} switched ${on ? "on" : "off"}${m.slots.length ? ` (its schedule ${on ? "is back" : "is hidden"})` : ""}.`);
  } catch (e) {
    reportError(`Could not switch ${m.name} ${on ? "on" : "off"}.`, e);
    m.active = !on;
  }
  drawFilters(c);
  draw(c);
}

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "Cabinet · Medicine cabinet";
  const view = readView();
  /** @type {Cabinet} */
  const c = {
    view, meds: [], loaded: false, alive: true,
    ui: buildLayout(root, view.query, { manageForms: () => void manageForms(c), load: () => void load(c) }),
    stock: createStockEditor({ reload: () => load(c), onSaved: (idle) => { drawFilters(c); if (idle) draw(c); } }),
  };
  c.ui.search.addEventListener("input", () => {
    view.query = c.ui.search.value;
    syncUrl(view);
    draw(c);
  });
  void load(c);
  const stop = onTick(() => (c.stock.busy() ? undefined : load(c)));
  return () => {
    c.alive = false;
    stop();
    c.stock.stop();
  };
}

async function manageForms(/** @type {Cabinet} */ c) {
  if (await openFormsManager()) await load(c);
}
