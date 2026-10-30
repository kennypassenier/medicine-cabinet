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
  return h("li", { class: "mc-hist" },
    h("span", { class: "mc-hist__time" }, i.taken_at.slice(11, 16)),
    h("div", { class: "mc-hist__main" },
      h("span", { class: "mc-dose__name" }, i.medicine_name),
      h("span", { class: "mc-dose__meta" }, [i.status === "taken" ? qty(i.dose, unit) : null, planned(i), i.note].filter(Boolean).join(" · "))),
    h("span", { class: "mc-hist__state" }, i.status === "taken" ? badge("kp-badge--success", "Taken") : badge("", "Skipped")),
    button("Undo", { look: "kp-button--ghost", key: `undo-${i.id}`, onClick: () => undo(i), label: `Undo ${i.medicine_name} at ${i.taken_at.slice(11, 16)}` }));
}

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "History · Medicine cabinet";
  let items = /** @type {any[]} */ ([]);
  let meds = new Map();
  let limit = PAGE;
  let filter = /** @type {keyof typeof FILTERS} */ ("all");
  let loaded = false;
  let alive = true;

  const filterSlot = h("div", { class: "mc-filters" });
  const card = dataCard({ id: "hist", title: "Intake log", description: "Everything you took or skipped. Undo removes an entry and gives the stock back.", onRetry: () => load() });
  const more = button("Show earlier entries", { onClick: () => { limit += PAGE; void load(); } });
  more.hidden = true;
  root.append(pageHeader({ title: "History", description: "What you took, when, and what you skipped." }), filterSlot, card.card, more);
  card.state("loading");

  function draw() {
    if (!loaded) return;
    const shown = items.filter((i) => filter === "all" || i.status === filter);
    filterSlot.replaceChildren(chips({ label: "Show", value: filter, options: Object.entries(FILTERS).map(([k, t]) => [k, t, k === "all" ? items.length : items.filter((i) => i.status === k).length]), onChange: (v) => { filter = /** @type {keyof typeof FILTERS} */ (v); draw(); } }));
    more.hidden = items.length < limit;
    if (!shown.length) {
      return card.state("empty", items.length
        ? { title: `No ${filter} entries`, text: "Switch the filter to see the rest." }
        : { title: "Nothing logged yet", text: "Doses you take or skip on Today show up here.", action: linkButton("Go to Today", "/") });
    }
    const days = [...new Set(shown.map((i) => i.taken_at.slice(0, 10)))];
    redraw(card.data, () => card.data.replaceChildren(...days.map((d) => {
      const list = shown.filter((i) => i.taken_at.startsWith(d));
      return h("section", { class: "mc-slot", "aria-label": friendlyDate(d) },
        h("h3", { class: "mc-slot__time mc-slot__time--day" }, friendlyDate(d), h("span", { class: "mc-slot__count" }, `${list.length} ${list.length === 1 ? "entry" : "entries"}`)),
        h("ul", { class: "mc-slot__list" }, ...list.map((i) => row(i, meds, undo))));
    })));
    card.state("ready");
  }

  async function load() {
    try {
      const [log, list] = await Promise.all([api.intakes(undefined, limit), api.medicines()]);
      if (!alive) return;
      items = log;
      meds = new Map(list.map((/** @type {any} */ m) => [m.id, m]));
      loaded = true;
      draw();
    } catch (e) {
      if (!alive) return;
      if (!loaded) card.state("error", { title: "The history could not be loaded", text: e instanceof Error ? e.message : String(e) });
      else reportError("The history could not be refreshed.", e);
    }
  }

  /** @param {any} i */
  async function undo(i) {
    const before = items;
    items = items.filter((x) => x.id !== i.id);
    draw();
    try {
      await api.undoIntake(i.id);
      say(`Removed ${i.medicine_name} at ${i.taken_at.slice(11, 16)}.${i.status === "taken" ? " Stock restored." : ""}`);
    } catch (e) {
      items = before;
      reportError(`Could not undo ${i.medicine_name}.`, e);
    }
    await load();
  }

  void load();
  const stop = onTick(() => load());
  return () => { alive = false; stop(); };
}
