// The "when to take it" card: as-needed switch with its safety limits, or the
// list of schedule slots (time, weekdays, dose).

import { WEEKDAYS_EVENT } from "/static/kp/js/weekdays.js";
import { TIME_EVENT } from "/static/kp/js/timefield.js";
import { h } from "../dom.js";
import { stepper, toggle } from "../controls.js";
import { button } from "../ui.js";
import { row, section } from "./form-fields.js";

/** Day numbers 0=Sunday..6 (the picker's) <-> the API's mask (bit0 = Monday). */
const toBit = (/** @type {number} */ d) => 1 << ((d + 6) % 7);
export const maskOf = (/** @type {number[]} */ days) => days.reduce((m, d) => m | toBit(d), 0);
export const daysOf = (/** @type {number} */ mask) => [0, 1, 2, 3, 4, 5, 6].filter((d) => mask & toBit(d));

/** @typedef {{ time: string, days: number, dose: number }} Slot */
export const newSlot = () => /** @type {Slot} */ ({ time: "08:00", days: 127, dose: 1 });

/** One slot's editor row; edits write straight into the slot object. @param {Slot} s @param {number} i @param {{ remove: (i: number) => void, unit: string }} on */
function slotRow(s, i, on) {
  const time = h("div", { class: "kp-time", "data-kp-time": "", "data-kp-value": s.time, "data-kp-label": `Time ${i + 1}`, "data-kp-time-step": "5" });
  const days = h("div", { class: "kp-weekdays", "data-kp-weekdays": "", "data-kp-value": daysOf(s.days).join(","), "aria-label": `Days for time ${i + 1}`, "data-kp-summary": "false", "data-kp-presets": "weekdays,weekend,every" });
  time.addEventListener(TIME_EVENT, (e) => { s.time = /** @type {CustomEvent} */ (e).detail.value || s.time; });
  days.addEventListener(WEEKDAYS_EVENT, (e) => { const v = /** @type {CustomEvent} */ (e).detail.values; if (v.length) s.days = maskOf(v.map(Number)); });
  const dose = stepper({ label: `Dose ${i + 1}`, value: s.dose, min: 0.25, step: 0.25, unit: on.unit, onChange: (v) => { s.dose = v; } });
  return h("li", { class: "mc-slotedit" },
    h("div", { class: "mc-slotedit__time" }, h("span", { class: "kp-field__label" }, "At"), time),
    h("div", { class: "mc-slotedit__dose" }, h("span", { class: "kp-field__label" }, "Dose each time"), dose),
    button("Remove", { look: "kp-button--ghost", onClick: () => on.remove(i), label: `Remove time ${i + 1}` }),
    h("div", { class: "mc-slotedit__days" }, days));
}

/**
 * @param {{ asNeeded: boolean, minGap: number, maxPerDay: number, slots: Slot[], unit: string }} m the form's model (mutated)
 * @param {() => void} redraw redraw this card after a structural change
 */
export function scheduleSection(m, redraw) {
  const asNeeded = toggle({ label: "Only when needed", checked: m.asNeeded, onChange: (on) => { m.asNeeded = on; redraw(); } });
  const rows = [row({ label: "Kind", control: asNeeded, hint: "Needed medicines have no fixed times; the cabinet guards the safe gap instead." })];
  if (m.asNeeded) {
    rows.push(
      row({ label: "Minimum gap", control: stepper({ label: "Minimum gap in minutes", value: m.minGap, min: 0, step: 30, unit: "min", onChange: (v) => { m.minGap = v; } }), hint: "Between two doses. 0 means no limit." }),
      row({ label: "Most per day", control: stepper({ label: "Most doses per day", value: m.maxPerDay, min: 0, step: 1, unit: "doses", onChange: (v) => { m.maxPerDay = v; } }), hint: "In 24 hours. 0 means no limit." }));
  } else {
    const list = h("ul", { class: "mc-slotedit-list" }, ...m.slots.map((s, i) => slotRow(s, i, { unit: m.unit, remove: (n) => { m.slots.splice(n, 1); redraw(); } })));
    const add = button("Add a time", { onClick: () => { m.slots.push({ ...newSlot(), time: m.slots.at(-1)?.time ?? "08:00" }); redraw(); } });
    rows.push(row({ label: "Times", control: h("div", { class: "mc-slotedit-wrap" }, m.slots.length ? list : h("p", { class: "kp-formgrid__value" }, "No times yet. Without a time, this medicine will not appear in Today."), add) }));
  }
  return section({ id: "when", title: "When to take it", description: "A fixed schedule, or a guard for medicines you take as needed.", rows });
}
