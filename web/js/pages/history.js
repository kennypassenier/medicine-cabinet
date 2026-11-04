// History: the intake log by day, newest first. A row's time and dose are edited
// in place; Remove gives the stock back and offers Undo; "Add an earlier dose"
// logs a dose that was never recorded.

import { attachTimes, attachSelects } from "/static/kp/dist/kp-themes.min.js";
import { h, redraw } from "../dom.js";
import { api } from "../api.js";
import { friendlyDate } from "../clock.js";
import { chips, wire } from "../controls.js";
import { onTick } from "../ticker.js";
import { undoable } from "../undo.js";
import { button, dataCard, linkButton, pageHeader, errorText, reportError } from "../ui.js";
import { historyRow } from "./history-row.js";
import { addEarlierDose } from "./history-add.js";

const PAGE = 200;
const FILTERS = { all: "All", taken: "Taken", skipped: "Skipped" };

/**
 * Everything the page's functions share.
 * @typedef {{
 *   items: any[], meds: Map<string, any>, limit: number, filter: keyof typeof FILTERS, detach: (() => void)[], rows: () => void,
 *   loaded: boolean, alive: boolean, addSlot: HTMLElement, ui: ReturnType<typeof buildLayout>
 * }} HistoryPage
 */

/** Build the page skeleton into `root`. @param {HTMLElement} root @param {{ reload: () => void, more: () => void }} on */
function buildLayout(root, on) {
  const filterSlot = h("div", { class: "kp-toolbar__group", role: "group", "aria-label": "Show" });
  const tools = h("div", { class: "kp-toolbar kp-ops__toolbar", role: "group", "aria-label": "Filter the log" }, h("div", { class: "kp-toolbar__filters" }, filterSlot));
  const card = dataCard({ id: "hist", title: "Intake log", description: "Everything you took or skipped. Change a time or a dose right in the row; Remove gives the stock back, and you can undo it right after.", onRetry: on.reload });
  card.card.dataset.kpSpan = "12";
  const more = button("Show earlier entries", { look: "kp-self-start", onClick: on.more });
  more.hidden = true;
  const addSlot = h("div", { class: "kp-ops__body" });
  root.append(pageHeader({ title: "History", description: "What you took, when, and what you skipped." }), tools, addSlot, h("div", { class: "kp-ops__body" }, card.card));
  card.state("loading");
  return { filterSlot, card, more, addSlot };
}

/** One day of the log: its heading and entries, in the list's shared columns. @param {string} day @param {any[]} list @param {Map<string, any>} meds @param {Parameters<typeof historyRow>[2]} on */
function daySection(day, list, meds, on) {
  return h("section", { class: "kp-stack kp-gap-sm", "aria-label": friendlyDate(day) },
    h("h3", { class: "kp-m-0" }, friendlyDate(day), " ", h("span", { class: "kp-text-muted" }, `· ${list.length} ${list.length === 1 ? "entry" : "entries"}`)),
    h("ul", { class: "kp-list", "aria-label": `Entries of ${friendlyDate(day)}` }, ...list.map((i) => historyRow(i, meds, on))));
}

/** The "Add an earlier dose" card, drawn once the medicines are known. @param {HistoryPage} p */
function drawAdd(p) {
  if (p.addSlot.firstChild || !p.meds.size) return;
  const add = addEarlierDose({ meds: [...p.meds.values()], onAdded: () => void load(p) });
  p.addSlot.append(add.card);
  p.detach.push(add.detach);
}

/** The filter chips with their counts; a pressed chip only redraws the list, so the chip keeps the focus. @param {HistoryPage} p */
function drawChips(p) {
  const { items, filter } = p;
  const count = (/** @type {string} */ k) => (k === "all" ? items.length : items.filter((i) => i.status === k).length);
  p.ui.filterSlot.replaceChildren(h("b", { class: "kp-toolbar__label" }, "Show"), chips({
    label: "Show", value: filter, options: Object.entries(FILTERS).map(([k, t]) => [k, t, count(k)]),
    onChange: (v) => { p.filter = /** @type {keyof typeof FILTERS} */ (v); drawList(p); },
  }));
}

/** @param {HistoryPage} p */
function draw(p) {
  if (!p.loaded) return;
  drawAdd(p);
  redraw(p.ui.filterSlot, () => drawChips(p));
  drawList(p);
}

/** @param {HistoryPage} p */
function drawList(p) {
  const { card, more } = p.ui;
  const { items, filter } = p;
  const shown = items.filter((i) => filter === "all" || i.status === filter);
  more.hidden = items.length < p.limit;
  if (!shown.length) {
    return card.state("empty", items.length
      ? { title: `No ${filter} entries`, text: "Switch the filter to see the rest." }
      : { title: "Nothing logged yet", text: "Doses you take or skip on Today show up here.", action: linkButton("Go to Today", "/") });
  }
  const on = { remove: (/** @type {any} */ i) => void remove(p, i), changed: () => void load(p) };
  const days = [...new Set(shown.map((i) => i.taken_at.slice(0, 10)))];
  redraw(card.data, () => {
    p.rows();
    card.data.replaceChildren(h("div", { class: "kp-stack kp-gap-lg" }, ...days.map((d) => daySection(d, shown.filter((i) => i.taken_at.startsWith(d)), p.meds, on)), more));
    wire(card.data);
    const times = attachTimes(card.data);
    const selects = attachSelects(card.data);
    p.rows = () => { times(); selects(); };
  });
  card.state("ready");
}

/** @param {HistoryPage} p */
async function load(p) {
  try {
    const [log, list] = await Promise.all([api.intakes(undefined, p.limit), api.medicines()]);
    if (!p.alive) return;
    p.items = log;
    p.meds = new Map(list.map((/** @type {any} */ m) => [m.id, m]));
    p.loaded = true;
    draw(p);
  } catch (e) {
    if (!p.alive) return;
    if (!p.loaded) p.ui.card.state("error", { title: "The history could not be loaded", text: errorText(e) });
    else reportError("The history could not be refreshed.", e);
  }
}

/** Log an entry again, exactly as it was (the undo of a removal). @param {any} i */
const relog = (i) =>
  api.logIntake({ medicineId: i.medicine_id, status: i.status, dose: i.dose, takenAt: i.taken_at, plannedFor: i.planned_for, note: i.note });

/** Remove an entry at once (the stock comes back), put it back when the server refuses, and offer Undo. @param {HistoryPage} p @param {any} i */
async function remove(p, i) {
  const before = p.items;
  const buttons = () => [...p.ui.card.data.querySelectorAll("[data-key^='remove-']")];
  const at = buttons().findIndex((b) => /** @type {HTMLElement} */ (b).dataset.key === `remove-${i.id}`);
  p.items = p.items.filter((x) => x.id !== i.id);
  draw(p);
  // The row (and the button that was pressed) is gone: the focus goes to the entry that took its place, else to the title.
  const next = /** @type {HTMLElement | undefined} */ (buttons()[at] ?? buttons()[at - 1]);
  (next ?? document.getElementById("page-title"))?.focus({ preventScroll: true });
  try {
    await api.undoIntake(i.id);
  } catch (e) {
    p.items = before;
    reportError(`Could not remove ${i.medicine_name}.`, e);
    return load(p);
  }
  // A dose whose medicine was purged cannot be logged again (it has no medicine to log against).
  if (i.medicine_id != null) undoable(`Removed ${i.medicine_name} at ${i.taken_at.slice(11, 16)}.${i.status === "taken" ? " Stock restored." : ""}`, { undo: () => relog(i), failed: `Could not log ${i.medicine_name} again.`, after: () => void load(p) });
  await load(p);
}

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "History · Medicine cabinet";
  /** @type {HistoryPage} */
  const p = {
    items: [], meds: new Map(), limit: PAGE, filter: "all", loaded: false, alive: true, detach: [], rows: () => {},
    ui: buildLayout(root, { reload: () => void load(p), more: () => { p.limit += PAGE; void load(p); } }),
    addSlot: /** @type {any} */ (null),
  };
  p.addSlot = p.ui.addSlot;
  void load(p);
  // A refresh must not redraw rows under a field that is being edited.
  const stop = onTick(() => (p.ui.card.data.contains(document.activeElement) && document.activeElement?.matches("input, select") ? undefined : load(p)));
  return () => { p.alive = false; p.rows(); p.detach.forEach((d) => d()); stop(); };
}
