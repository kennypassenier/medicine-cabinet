// The cards of the medicine form, one builder each: about, symptoms, when,
// stock, dates and delete. The unit field and the "when" slot keep a little
// state of their own, so they come as small controllers.

import { attachWeekdays } from "/static/kp/js/weekdays.js";
import { attachTimes } from "/static/kp/js/timefield.js";
import { h } from "../dom.js";
import { stepper, wire } from "../controls.js";
import { button } from "../ui.js";
import { date, row, section, text } from "./form-fields.js";
import { scheduleSection } from "./form-schedule.js";
import { COMMON_UNITS } from "./forms-manage.js";

/**
 * The stock unit: a text field with a datalist of the known units. It follows
 * the chosen form until the person types their own.
 * @param {any | null} med @param {{ unit: string }} model mutated as the unit changes @param {any[]} forms
 */
export function unitField(med, model, forms) {
  const list = h("datalist", { id: "unit-list" });
  const setUnits = (/** @type {any[]} */ fs) => list.replaceChildren(...[...new Set([...fs.map((f) => f.unit), ...COMMON_UNITS])].map((u) => h("option", { value: u })));
  setUnits(forms);
  const unitRow = text({ name: "stock_unit", label: "Unit", value: model.unit, list: "unit-list", hint: "Counted in: tablets, ml, drops…", required: true });
  const input = /** @type {HTMLInputElement} */ (unitRow.querySelector("input"));
  let auto = !med || med.stock_unit === forms.find((f) => f.id === med.form_id)?.unit;
  input.addEventListener("input", () => { auto = false; model.unit = input.value.trim(); });
  return {
    row: unitRow,
    list,
    setUnits,
    /** A form pill was picked: fill the unit while it is empty or was filled this way. @param {any | null} f */
    followForm(f) {
      if (!f || !(auto || !input.value.trim())) return;
      input.value = f.unit;
      model.unit = f.unit;
      auto = true;
      input.dispatchEvent(new Event("change", { bubbles: true }));
    },
  };
}

/** @param {any | null} med @param {HTMLElement} formRow the form pills row */
export function aboutSection(med, formRow) {
  return section({ id: "about", title: "About", description: "What it is and how it comes.", className: "mc-form__about", rows: [
    text({ name: "name", label: "Name", value: med?.name, required: true, maxlength: 80, placeholder: "e.g. Paracetamol" }),
    text({ name: "purpose", label: "Purpose", value: med?.purpose, hint: "In your own words, e.g. “Pain and fever”." }),
    formRow,
    text({ name: "strength", label: "Strength", value: med?.strength, hint: "e.g. 500 mg" }),
    text({ name: "notes", label: "Notes", value: med?.notes, multiline: true, hint: "With food, avoid alcohol, where the spare box is." }),
  ] });
}

/** @param {HTMLElement} symptomRow */
export const helpsSection = (symptomRow) =>
  section({ id: "helps", title: "Symptoms", description: "What you take it for. These power the Symptoms page.", className: "mc-form__helps", rows: [symptomRow] });

/**
 * The "when to take it" card, redrawn after every structural change of the
 * schedule; its weekday and time pickers are attached anew each time.
 * @param {Parameters<typeof scheduleSection>[0]} model
 */
export function createWhenSlot(model) {
  const el = h("div", { class: "mc-form__when" });
  /** @type {(() => void)[]} */
  let detach = [];
  const stop = () => detach.forEach((d) => d());
  const draw = () => {
    stop();
    el.replaceChildren(scheduleSection(model, draw));
    wire(el);
    detach = [attachWeekdays(el), attachTimes(el)];
  };
  return { el, draw, stop };
}

/** @param {any | null} med @param {{ unit: string }} model @param {HTMLElement} unitRow */
export function stockSection(med, model, unitRow) {
  return section({ id: "stock", title: "Stock", description: "What is left, and when to be warned.", className: "mc-form__stock", rows: [
    row({ label: "In stock", control: stepper({ label: "Stock", value: med?.stock ?? 0, min: 0, step: 1, unit: model.unit }), hint: "How many you have now." }),
    unitRow,
    row({ label: "Low-stock warning", control: stepper({ label: "Warn when stock is at or below", value: med?.low_stock_at ?? 0, min: 0, step: 1, unit: model.unit }), hint: "Warn at or below this number. 0 means no warning." }),
  ] });
}

/** @param {any | null} med */
export function datesSection(med) {
  return section({ id: "dates", title: "Dates", description: "Expiry, opening and the end of a course. All optional.", className: "mc-form__dates", rows: [
    date({ name: "expires_on", label: "Expires on", value: med?.expires_on, hint: "Printed on the box." }),
    date({ name: "opened_on", label: "Opened on", value: med?.opened_on, hint: "For drops, syrups and creams." }),
    row({ label: "Good after opening", control: stepper({ label: "Good for days after opening", value: med?.shelf_life_days ?? 0, min: 0, step: 1, unit: "days" }), hint: "0 means it keeps until the printed date." }),
    date({ name: "course_ends_on", label: "Course ends on", value: med?.course_ends_on, hint: "An antibiotic course, for instance." }),
  ] });
}

/** The delete card of an existing medicine. @param {any} med @param {() => void} onDelete */
export function dangerSection(med, onDelete) {
  return section({ id: "danger", title: "Delete", description: "Removes this medicine and its schedule. Switch it off in the cabinet instead to keep its history.", className: "mc-form__danger", rows: [
    row({ label: "Permanently", control: button(`Delete ${med.name}`, { look: "kp-button--destructive", onClick: onDelete }) })] });
}
