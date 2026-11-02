// The schedule slots of the "when to take it" card: one row per slot (time,
// weekdays, dose) in the package's row editor, and the day-mask conversions.

import { timeValue, weekdaysOf, getQuantity, editorRowsOf } from "/static/kp/dist/kp-themes.min.js";
import { h } from "../dom.js";

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
      h("div", { class: "kp-time", "data-kp-time": "", "data-kp-untracked": "", "data-kp-name": `at-${n}`, "data-kp-value": s.time, "data-kp-label": "Time", "data-kp-time-step": "5" })),
    h("div", { class: "kp-rows__cell", "data-label": "Days" },
      h("div", { class: "kp-weekdays", "data-kp-weekdays": "", "data-kp-untracked": "", "data-kp-value": daysOf(s.days).join(","), "data-kp-presets": "false", "data-kp-summary": "false", "aria-label": "Days" })),
    h("div", { class: "kp-rows__cell", "data-label": "Dose" },
      h("div", { class: "kp-quantity", "data-kp-quantity": "", "data-kp-untracked": "", "data-kp-quantity-fill": "", "data-kp-quantity-min": "0.25", "data-kp-quantity-step": "0.25", "data-kp-quantity-unit": unit, "data-kp-quantity-unit-one": unitOne, "data-kp-quantity-label": "Dose" },
        h("input", { class: "kp-quantity__input", autocomplete: "off", value: String(s.dose) }))));
}

/** The row editor: the slots, a template for new ones, and nothing else (the package writes the head, remove and add buttons). @param {Slot[]} slots @param {string} unit */
export function slotEditor(slots, unit) {
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
