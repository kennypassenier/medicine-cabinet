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

/** The buttons of one dose row; every row has the same two-slot grid. @param {any} d @param {DoseHandlers} on */
function actions(d, on) {
  const k = doseKey(d);
  if (d.state === "taken" || d.state === "skipped") {
    return h("div", { class: "mc-dose__actions mc-dose__actions--one" }, button("Undo", { key: `undo-${k}`, onClick: () => on.undo(d), label: `Undo ${d.medicine_name} at ${d.time}` }));
  }
  const early = d.state === "upcoming";
  return h("div", { class: "mc-dose__actions" },
    button(early ? "Take early" : "Take", { look: early ? "" : "kp-button--primary", key: `take-${k}`, onClick: () => on.take(d), label: `Take ${d.medicine_name} at ${d.time}` }),
    button("Skip", { look: "kp-button--ghost", key: `skip-${k}`, onClick: () => on.skip(d), label: `Skip ${d.medicine_name} at ${d.time}` }));
}

/** @param {any} d @param {Map<number, any>} meds @param {DoseHandlers} on */
function row(d, meds, on) {
  const med = meds.get(d.medicine_id);
  return h("li", { class: "mc-dose", "data-state": d.state },
    h("div", { class: "mc-dose__main" },
      h("a", { class: "mc-dose__name", href: `/medicine/${d.medicine_id}` }, d.medicine_name),
      h("span", { class: "mc-dose__meta" }, qty(d.dose, unitOf(meds, d)), med?.purpose ? ` · ${med.purpose}` : "")),
    h("div", { class: "mc-dose__state" }, stateBadge(d)),
    actions(d, on));
}

/** The doses grouped by time, earliest first. @param {HTMLElement} into @param {any[]} doses @param {Map<number, any>} meds @param {DoseHandlers} on */
export function renderSchedule(into, doses, meds, on) {
  const times = [...new Set(doses.map((d) => d.time))].sort();
  into.replaceChildren(...times.map((t) => {
    const group = doses.filter((d) => d.time === t);
    return h("section", { class: "mc-slot", "aria-label": `Doses at ${t}` },
      h("h3", { class: "mc-slot__time" }, t),
      h("ul", { class: "mc-slot__list" }, ...group.map((d) => row(d, meds, on))));
  }));
}

/** The hero card body: what to take next, with the same two actions. @param {HTMLElement} into @param {any[]} doses @param {Map<number, any>} meds @param {DoseHandlers} on */
export function renderNext(into, doses, meds, on) {
  const d = nextDose(doses);
  if (!d) {
    const left = doses.length;
    const text = left ? "Everything scheduled for today is done." : "Nothing is scheduled for today.";
    return into.replaceChildren(h("div", { class: "mc-next mc-next--done" }, h("p", { class: "mc-next__title" }, left ? "All done for today" : "A free day"), h("p", { class: "mc-next__meta" }, text)));
  }
  const same = doses.filter((x) => x.time === d.time && OPEN.has(x.state) && x !== d).length;
  const when = d.state === "upcoming" ? `At ${d.time}, in ${duration(d.minutes_until)}` : d.state === "due" ? `Due now (${d.time})` : `Was due at ${d.time}, ${duration(-d.minutes_until)} ago`;
  const early = d.state === "upcoming";
  into.replaceChildren(h("div", { class: `mc-next mc-next--${d.state}` },
    h("p", { class: "mc-next__state" }, stateBadge(d)),
    h("p", { class: "mc-next__title" }, d.medicine_name),
    h("p", { class: "mc-next__dose" }, qty(d.dose, unitOf(meds, d))),
    h("p", { class: "mc-next__meta" }, when, same ? ` · ${same} more at the same time` : ""),
    h("div", { class: "mc-next__actions" },
      button(early ? "Take early" : "Take now", { look: "kp-button--primary kp-button--lg", key: "next-take", onClick: () => on.take(d) }),
      button("Skip", { look: "kp-button--lg", key: "next-skip", onClick: () => on.skip(d) }))));
}
