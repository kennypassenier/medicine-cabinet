// The "when to take it" card: as-needed switch with its safety limits, or the
// package's row editor holding the schedule slots (time, weekdays, dose).

import { timeValue, weekdaysOf, getQuantity, editorRowsOf } from "/static/kp/dist/kp-themes.min.js";
import { h } from "../dom.js";
import { stepper, toggle } from "../controls.js";
import { row, section } from "./form-fields.js";

/** Day numbers 0=Sunday..6 (the picker's) <-> the API's mask (bit0 = Monday). */
const toBit = (/** @type {number} */ d) => 1 << ((d + 6) % 7);
export const maskOf = (/** @type {number[]} */ days) => days.reduce((m, d) => m | toBit(d), 0);
export const daysOf = (/** @type {number} */ mask) => [0, 1, 2, 3, 4, 5, 6].filter((d) => mask & toBit(d));

/** @typedef {{ time: string, days: number, dose: number }} Slot */
export const newSlot = () => /** @type {Slot} */ ({ time: "08:00", days: 127, dose: 1 });

/** One slot as the editor's row: the package's time field, weekday picker and quantity, labelled once by the head row. @param {Slot} s @param {string} unit @param {string} n the row's `{n}` */
function slotRow(s, unit, n) {
  const unitOne = unit.endsWith("s") ? unit.slice(0, -1) : unit;
  return h("li", { class: "kp-rows__row" },
    h("div", { class: "kp-rows__cell", "data-label": "Time" },
      h("div", { class: "kp-time", "data-kp-time": "", "data-kp-name": `at-${n}`, "data-kp-value": s.time, "data-kp-label": "Time", "data-kp-time-step": "5" })),
    h("div", { class: "kp-rows__cell", "data-label": "Days" },
      h("div", { class: "kp-weekdays", "data-kp-weekdays": "", "data-kp-value": daysOf(s.days).join(","), "data-kp-presets": "false", "data-kp-summary": "false", "aria-label": "Days" })),
    h("div", { class: "kp-rows__cell", "data-label": "Dose" },
      h("div", { class: "kp-quantity", "data-kp-quantity": "", "data-kp-quantity-fill": "", "data-kp-quantity-min": "0.25", "data-kp-quantity-step": "0.25", "data-kp-quantity-unit": unit, "data-kp-quantity-unit-one": unitOne, "data-kp-quantity-label": "Dose" },
        h("input", { class: "kp-quantity__input", value: String(s.dose) }))));
}

/** The row editor: the slots, a template for new ones, and nothing else (the package writes the head, remove and add buttons). @param {Slot[]} slots @param {string} unit */
function slotEditor(slots, unit) {
  const template = h("template", { "data-kp-rows-template": "" });
  template.content.append(slotRow(newSlot(), unit, "{n}"));
  return h("div", { class: "kp-rows", "data-kp-rows": "", "data-kp-rows-min": "1", "data-kp-rows-max": "6", "data-kp-rows-unit": "time,times", "aria-label": "When to take it" },
    h("ol", { class: "kp-rows__list", "data-kp-rows-list": "" }, ...slots.map((s, i) => slotRow(s, unit, `s${i}`))),
    template);
}

/** The slots the editor's rows hold now, read from each row's own controls. @param {HTMLElement} editor @returns {Slot[]} */
export function readSlots(editor) {
  return editorRowsOf(editor).map((r) => {
    const time = /** @type {HTMLElement} */ (r.querySelector(".kp-time"));
    const days = /** @type {HTMLElement} */ (r.querySelector(".kp-weekdays"));
    const dose = /** @type {HTMLElement} */ (r.querySelector(".kp-quantity"));
    return { time: timeValue(time) || "08:00", days: maskOf(weekdaysOf(days).map(Number)) || 127, dose: getQuantity(dose) || 1 };
  });
}

/**
 * @param {{ asNeeded: boolean, minGap: number, maxPerDay: number, slots: Slot[], unit: string }} m the form's model (mutated)
 * @param {() => void} redraw redraw this card after a structural change
 * @param {{ editor: HTMLElement | null }} live holds the editor on the page, so the model can read it before a redraw
 */
export function scheduleSection(m, redraw, live) {
  const asNeeded = toggle({ label: "Only when needed", checked: m.asNeeded, onChange: (on) => { if (live.editor) m.slots = readSlots(live.editor); m.asNeeded = on; redraw(); } });
  const rows = [row({ label: "Kind", control: asNeeded, hint: "Needed medicines have no fixed times; the cabinet guards the safe gap instead." })];
  if (m.asNeeded) {
    live.editor = null;
    rows.push(
      row({ label: "Minimum gap", control: stepper({ label: "Minimum gap in minutes", value: m.minGap, min: 0, step: 30, unit: "min", onChange: (v) => { m.minGap = v; } }), hint: "Between two doses. 0 means no limit." }),
      row({ label: "Most per day", control: stepper({ label: "Most doses per day", value: m.maxPerDay, min: 0, step: 1, unit: "doses", onChange: (v) => { m.maxPerDay = v; } }), hint: "In 24 hours. 0 means no limit." }));
  } else {
    live.editor = slotEditor(m.slots.length ? m.slots : [newSlot()], m.unit);
    rows.push(row({ label: "Times", control: live.editor }));
  }
  return section({ id: "when", title: "When to take it", description: "A fixed schedule, or a guard for medicines you take as needed.", rows });
}
