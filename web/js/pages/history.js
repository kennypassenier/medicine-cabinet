// History: the intake log by day, newest first, with undo (which restores stock).

import { h, redraw } from "../dom.js";
import { api } from "../api.js";
import { friendlyDate, formatDate } from "../clock.js";
import { chips } from "../controls.js";
import { badge, qty } from "../format.js";
import { onTick } from "../ticker.js";
import { button, dataCard, linkButton, pageHeader, reportError, say } from "../ui.js";

const PAGE = 200;
const FILTERS = { all: "All", taken: "Taken", skipped: "Skipped" };

/** "08:05 planned 08:00" style line for an intake. @param {any} i */
function planned(i) {
  if (!i.planned_for) return "As needed";
  const when = i.planned_for.slice(11, 16);
  return i.planned_for.slice(0, 10) === i.taken_at.slice(0, 10) ? `Planned for ${when}` : `Planned for ${formatDate(i.planned_for)} ${when}`;
}

/** @param {any} i @param {Map<number, any>} meds @param {(i: any) => void} undo */
function row(i, meds, undo) {
  const unit = meds.get(i.medicine_id)?.stock_unit ?? "";
  const time = i.taken_at.slice(11, 16);
  return h("li", null,
    h("div", { class: "kp-stack kp-gap-xs" },
      h("p", { class: "kp-m-0" }, h("strong", null, time), " · ", i.medicine_name),
      h("div", { class: "kp-row" },
        i.status === "taken" ? badge("kp-badge--success", "Taken") : badge("", "Skipped"),
        h("span", { class: "kp-text-muted" }, [i.status === "taken" ? qty(i.dose, unit) : null, planned(i), i.note].filter(Boolean).join(" · ")))),
    h("div", { class: "kp-row-actions" }, button("Undo", { look: "kp-button--ghost", key: `undo-${i.id}`, onClick: () => undo(i), label: `Undo ${i.medicine_name} at ${time}` })));
}

/**
 * Everything the page's functions share.
 * @typedef {{
 *   items: any[], meds: Map<number, any>, limit: number, filter: keyof typeof FILTERS,
 *   loaded: boolean, alive: boolean, ui: ReturnType<typeof buildLayout>
 * }} HistoryPage
 */

/** Build the page skeleton into `root`. @param {HTMLElement} root @param {{ reload: () => void, more: () => void }} on */
function buildLayout(root, on) {
  const filterSlot = h("div", { class: "kp-toolbar__group", role: "group", "aria-label": "Show" });
  const tools = h("div", { class: "kp-toolbar kp-ops__toolbar", role: "group", "aria-label": "Filter the log" }, h("div", { class: "kp-toolbar__filters" }, filterSlot));
  const card = dataCard({ id: "hist", title: "Intake log", description: "Everything you took or skipped. Undo removes an entry and gives the stock back.", onRetry: on.reload });
  card.card.dataset.kpSpan = "12";
  const more = button("Show earlier entries", { look: "kp-self-start", onClick: on.more });
  more.hidden = true;
  root.append(pageHeader({ title: "History", description: "What you took, when, and what you skipped." }), tools, h("div", { class: "kp-ops__body" }, card.card));
  card.state("loading");
  return { filterSlot, card, more };
}

/** One day of the log: its heading and entries. @param {string} day @param {any[]} list @param {Map<number, any>} meds @param {(i: any) => void} undo */
function daySection(day, list, meds, undo) {
  return h("section", { class: "kp-stack kp-gap-sm", "aria-label": friendlyDate(day) },
    h("h3", { class: "kp-m-0" }, friendlyDate(day), " ", h("span", { class: "kp-text-muted" }, `· ${list.length} ${list.length === 1 ? "entry" : "entries"}`)),
    h("ul", { class: "kp-action-list" }, ...list.map((i) => row(i, meds, undo))));
}

/** @param {HistoryPage} p */
function draw(p) {
  if (!p.loaded) return;
  const { filterSlot, card, more } = p.ui;
  const { items, filter } = p;
  const shown = items.filter((i) => filter === "all" || i.status === filter);
  filterSlot.replaceChildren(h("b", { class: "kp-toolbar__label" }, "Show"), chips({ label: "Show", value: filter, options: Object.entries(FILTERS).map(([k, t]) => [k, t, k === "all" ? items.length : items.filter((i) => i.status === k).length]), onChange: (v) => { p.filter = /** @type {keyof typeof FILTERS} */ (v); draw(p); } }));
  more.hidden = items.length < p.limit;
  if (!shown.length) {
    return card.state("empty", items.length
      ? { title: `No ${filter} entries`, text: "Switch the filter to see the rest." }
      : { title: "Nothing logged yet", text: "Doses you take or skip on Today show up here.", action: linkButton("Go to Today", "/") });
  }
  const days = [...new Set(shown.map((i) => i.taken_at.slice(0, 10)))];
  redraw(card.data, () => card.data.replaceChildren(h("div", { class: "kp-stack kp-gap-lg" }, ...days.map((d) => daySection(d, shown.filter((i) => i.taken_at.startsWith(d)), p.meds, (i) => undo(p, i))), more)));
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
    if (!p.loaded) p.ui.card.state("error", { title: "The history could not be loaded", text: e instanceof Error ? e.message : String(e) });
    else reportError("The history could not be refreshed.", e);
  }
}

/** Remove an entry at once, put it back when the server refuses. @param {HistoryPage} p @param {any} i */
async function undo(p, i) {
  const before = p.items;
  p.items = p.items.filter((x) => x.id !== i.id);
  draw(p);
  try {
    await api.undoIntake(i.id);
    say(`Removed ${i.medicine_name} at ${i.taken_at.slice(11, 16)}.${i.status === "taken" ? " Stock restored." : ""}`);
  } catch (e) {
    p.items = before;
    reportError(`Could not undo ${i.medicine_name}.`, e);
  }
  await load(p);
}

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "History · Medicine cabinet";
  /** @type {HistoryPage} */
  const p = {
    items: [], meds: new Map(), limit: PAGE, filter: "all", loaded: false, alive: true,
    ui: buildLayout(root, { reload: () => void load(p), more: () => { p.limit += PAGE; void load(p); } }),
  };
  void load(p);
  const stop = onTick(() => load(p));
  return () => { p.alive = false; stop(); };
}
