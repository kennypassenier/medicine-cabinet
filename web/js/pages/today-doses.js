// Today's schedule: the "next dose" hero and the doses grouped by time.
// Pure view code: it draws from data and calls the handlers it is given.

import { h } from "../dom.js";
import { duration } from "../clock.js";
import { badge, qty } from "../format.js";
import { button } from "../ui.js";

/** @typedef {{ take: (d: any) => void, skip: (d: any) => void, undo: (d: any) => void }} DoseHandlers */

const OPEN = new Set(["overdue", "due", "upcoming"]);

/** The dose that matters now: the earliest overdue/due, else the earliest upcoming. @param {any[]} doses */
export function nextDose(doses) {
  const open = doses.filter((d) => OPEN.has(d.state));
  return open.find((d) => d.state !== "upcoming") ?? open[0] ?? null;
}

/** The state badge of a dose. @param {any} d */
export function stateBadge(d) {
  switch (d.state) {
    case "overdue": return badge("kp-badge--destructive", `Overdue ${duration(-d.minutes_until)}`);
    case "due": return badge("kp-badge--info", "Due now");
    case "taken": return badge("kp-badge--success", "Taken");
    case "skipped": return badge("", "Skipped");
    default: return badge("", `In ${duration(d.minutes_until)}`);
  }
}

const doseKey = (/** @type {any} */ d) => `${d.medicine_id}-${d.time}`;
const unitOf = (/** @type {Map<number, any>} */ meds, /** @type {any} */ d) => meds.get(d.medicine_id)?.stock_unit ?? "";

/** The buttons of one dose row, in the list's shared columns (Skip, then the primary last). @param {any} d @param {DoseHandlers} on */
function actions(d, on) {
  const k = doseKey(d);
  if (d.state === "taken" || d.state === "skipped") {
    return h("div", { class: "kp-row-actions" }, button("Undo", { key: `undo-${k}`, onClick: () => on.undo(d), label: `Undo ${d.medicine_name} at ${d.time}` }));
  }
  const early = d.state === "upcoming";
  return h("div", { class: "kp-row-actions" },
    button("Skip", { look: "kp-button--ghost", role: "skip", key: `skip-${k}`, onClick: () => on.skip(d), label: `Skip ${d.medicine_name} at ${d.time}` }),
    button(early ? "Take early" : "Take", { look: early ? "" : "kp-button--primary", role: "take", key: `take-${k}`, onClick: () => on.take(d), label: `Take ${d.medicine_name} at ${d.time}` }));
}

/** @param {any} d @param {Map<number, any>} meds @param {DoseHandlers} on */
function row(d, meds, on) {
  const med = meds.get(d.medicine_id);
  return h("li", { "data-state": d.state },
    h("div", { class: "kp-stack kp-gap-xs" },
      h("p", { class: "kp-m-0" }, h("strong", null, d.time), " · ", h("a", { href: `/medicine/${d.medicine_id}` }, d.medicine_name)),
      h("div", { class: "kp-row" }, stateBadge(d), h("span", { class: "kp-text-muted" }, qty(d.dose, unitOf(meds, d)), med?.purpose ? ` · ${med.purpose}` : ""))),
    actions(d, on));
}

/** The day's doses, earliest first, in one list whose buttons share columns. @param {HTMLElement} into @param {any[]} doses @param {Map<number, any>} meds @param {DoseHandlers} on */
export function renderSchedule(into, doses, meds, on) {
  const sorted = [...doses].sort((a, b) => a.time.localeCompare(b.time));
  into.replaceChildren(h("ul", { class: "kp-action-list", "aria-label": "Doses planned today" }, ...sorted.map((d) => row(d, meds, on))));
}

/** The hero card body: what to take next, with the same two actions. @param {HTMLElement} into @param {any[]} doses @param {Map<number, any>} meds @param {DoseHandlers} on */
export function renderNext(into, doses, meds, on) {
  const d = nextDose(doses);
  if (!d) {
    const left = doses.length;
    const text = left ? "Everything scheduled for today is done." : "Nothing is scheduled for today.";
    return into.replaceChildren(h("div", { class: "kp-stack kp-gap-sm" }, h("strong", { class: "kp-kpi__value" }, left ? "All done for today" : "A free day"), h("p", { class: "kp-lede" }, text)));
  }
  const same = doses.filter((x) => x.time === d.time && OPEN.has(x.state) && x !== d).length;
  const when = d.state === "upcoming" ? `At ${d.time}, in ${duration(d.minutes_until)}` : d.state === "due" ? `Due now (${d.time})` : `Was due at ${d.time}, ${duration(-d.minutes_until)} ago`;
  const early = d.state === "upcoming";
  into.replaceChildren(h("div", { class: "kp-stack kp-gap-sm", "data-state": d.state },
    h("p", { class: "kp-m-0" }, stateBadge(d)),
    h("strong", { class: "kp-kpi__value" }, d.medicine_name),
    h("p", { class: "kp-lede" }, qty(d.dose, unitOf(meds, d))),
    h("p", { class: "kp-lede" }, when, same ? ` · ${same} more at the same time` : ""),
    h("div", { class: "kp-row kp-mt-sm", "data-mc-hero-actions": "" },
      button("Skip", { look: "kp-button--ghost kp-button--lg kp-flex-1", role: "skip", key: "next-skip", onClick: () => on.skip(d) }),
      button(early ? "Take early" : "Take now", { look: "kp-button--primary kp-button--lg kp-flex-1", role: "take", key: "next-take", onClick: () => on.take(d) }))));
}
