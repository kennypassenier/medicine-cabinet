// Cabinet: every medicine, searchable and filterable, with the on/off switch
// (it switches the schedule too) and quick stock adjust. A tap opens the edit form.

import { h, redraw } from "../dom.js";
import { api } from "../api.js";
import { chips, wire } from "../controls.js";
import { onTick } from "../ticker.js";
import { button, dataCard, pageHeader, reportError, linkButton, say } from "../ui.js";
import { medicineRow } from "./cabinet-row.js";
import { openFormsManager } from "./forms-manage.js";

/** The filters, by key. */
const FILTERS = /** @type {const} */ ({
  all: ["All", () => true],
  on: ["Switched on", (/** @type {any} */ m) => m.active],
  off: ["Switched off", (/** @type {any} */ m) => !m.active],
  low: ["Low or out", (/** @type {any} */ m) => m.stock_level !== "ok"],
  expiring: ["Expiring", (/** @type {any} */ m) => m.freshness !== "fresh"],
});

const STOCK_DEBOUNCE_MS = 700;

const haystack = (/** @type {any} */ m) => [m.name, m.purpose, m.form, m.strength, ...m.symptoms].join(" ").toLowerCase();

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "Cabinet · Medicine cabinet";
  const params = new URLSearchParams(location.search);
  let query = params.get("q") ?? "";
  let filter = /** @type {keyof typeof FILTERS} */ (params.get("f") in FILTERS ? params.get("f") : "all");
  let formFilter = params.get("form") ?? "";
  let meds = /** @type {any[]} */ ([]);
  let alive = true;
  let loaded = false;
  /** @type {Map<number, { timer: number, base: number, value: number }>} */
  const pending = new Map();

  const header = pageHeader({ title: "Cabinet", description: "Everything you keep at home. Switch a medicine off to hide its schedule everywhere.", actions: [button("Manage forms", { onClick: () => void manageForms() }), linkButton("Add medicine", "/medicine/new", "kp-button--primary")] });
  const search = /** @type {HTMLInputElement} */ (h("input", { class: "kp-field__input", type: "search", id: "cab-q", placeholder: "Name, purpose or symptom", autocomplete: "off", value: query }));
  const filterSlot = h("div", { class: "kp-field mc-filters" });
  const formSlot = h("div", { class: "kp-field mc-filters" });
  const tools = h("div", { class: "mc-toolbar" },
    h("div", { class: "kp-field mc-toolbar__search" }, h("label", { class: "kp-field__label", for: "cab-q" }, "Search"), search),
    filterSlot, formSlot);
  const list = dataCard({ id: "cab", title: "Medicines", description: "Newest changes show at once; stock edits are saved a moment after the last tap.", onRetry: () => load() });
  list.data.classList.add("mc-rows");
  root.append(header, tools, list.card);
  list.state("loading");

  function syncUrl() {
    const p = new URLSearchParams();
    if (query) p.set("q", query);
    if (filter !== "all") p.set("f", filter);
    if (formFilter) p.set("form", formFilter);
    history.replaceState(null, "", p.size ? `?${p}` : location.pathname);
  }

  function drawFilters() {
    filterSlot.replaceChildren(h("span", { class: "kp-field__label" }, "Show"), chips({
      label: "Show",
      value: filter,
      options: Object.entries(FILTERS).map(([k, [label, fn]]) => [k, label, meds.filter(fn).length]),
      onChange: (v) => {
        filter = /** @type {keyof typeof FILTERS} */ (v);
        syncUrl();
        draw();
      },
    }));
    drawFormFilter();
  }

  /** The form pills: only forms some medicine has, with how many. */
  function drawFormFilter() {
    const names = [...new Map(meds.filter((m) => m.form_id).map((m) => [String(m.form_id), m.form])).entries()].sort((a, b) => a[1].localeCompare(b[1]));
    if (formFilter && !names.some(([id]) => id === formFilter)) formFilter = "";
    formSlot.hidden = names.length === 0;
    formSlot.replaceChildren(h("span", { class: "kp-field__label" }, "Form"), chips({
      label: "Form",
      value: formFilter,
      options: [["", "Any form", meds.length], ...names.map(([id, name]) => /** @type {[string, string, number]} */ ([id, name, meds.filter((m) => String(m.form_id) === id).length]))],
      onChange: (v) => { formFilter = v; syncUrl(); draw(); },
    }));
  }

  async function manageForms() {
    if (await openFormsManager()) await load();
  }

  function draw() {
    if (!loaded) return;
    const q = query.trim().toLowerCase();
    const shown = meds.filter((m) => FILTERS[filter][1](m) && (!formFilter || String(m.form_id) === formFilter) && (!q || haystack(m).includes(q))).sort((a, b) => a.name.localeCompare(b.name));
    header.querySelector(".kp-page-header__description")?.replaceChildren(`${meds.length} ${meds.length === 1 ? "medicine" : "medicines"} at home. Switch one off to hide its schedule everywhere.`);
    if (!meds.length) return list.state("empty", { title: "Your cabinet is empty", text: "Add the first medicine to see it here.", action: linkButton("Add your first medicine", "/medicine/new", "kp-button--primary") });
    if (!shown.length) {
      const clear = h("button", { type: "button", class: "kp-button", onclick: () => reset() }, "Clear search and filters");
      return list.state("empty", { title: "No medicine matches", text: "Try another word or another filter.", action: clear });
    }
    redraw(list.data, () => {
      list.data.replaceChildren(h("ul", { class: "mc-list mc-list--rows" }, ...shown.map((m) => medicineRow(m, { setActive, step }))));
      wire(list.data);
    });
    list.state("ready");
  }

  function reset() {
    query = "";
    filter = "all";
    formFilter = "";
    search.value = "";
    syncUrl();
    drawFilters();
    draw();
  }

  async function load() {
    try {
      meds = await api.medicines();
      loaded = true;
      drawFilters();
      draw();
    } catch (e) {
      if (!alive) return;
      if (!loaded) list.state("error", { title: "The cabinet could not be loaded", text: e instanceof Error ? e.message : String(e) });
      else reportError("The cabinet could not be refreshed.", e);
    }
  }

  /** @param {any} m @param {boolean} on */
  async function setActive(m, on) {
    m.active = on;
    draw();
    try {
      const updated = await api.setActive(m.id, on);
      Object.assign(m, updated);
      say(`${m.name} switched ${on ? "on" : "off"}${m.slots.length ? ` (its schedule ${on ? "is back" : "is hidden"})` : ""}.`);
    } catch (e) {
      reportError(`Could not switch ${m.name} ${on ? "on" : "off"}.`, e);
      m.active = !on;
    }
    drawFilters();
    draw();
  }

  /** Quick stock: show the number at once, save one delta after the last tap. @param {any} m @param {number} v */
  function step(m, v) {
    const p = pending.get(m.id) ?? { timer: 0, base: m.stock, value: v };
    clearTimeout(p.timer);
    p.value = v;
    p.timer = window.setTimeout(() => save(m, p), STOCK_DEBOUNCE_MS);
    pending.set(m.id, p);
  }

  /** @param {any} m @param {{ base: number, value: number }} p */
  async function save(m, p) {
    pending.delete(m.id);
    const delta = Math.round((p.value - p.base) * 100) / 100;
    if (delta === 0) return;
    try {
      Object.assign(m, await api.addStock(m.id, delta));
    } catch (e) {
      reportError(`Could not change the stock of ${m.name}.`, e);
      await load();
      return;
    }
    drawFilters();
    if (!pending.size) draw();
  }

  search.addEventListener("input", () => {
    query = search.value;
    syncUrl();
    draw();
  });
  void load();
  const stop = onTick(() => (pending.size ? undefined : load()));
  return () => {
    alive = false;
    stop();
    for (const p of pending.values()) clearTimeout(p.timer);
  };
}
