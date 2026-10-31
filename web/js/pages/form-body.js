// Reads the form's widgets back into the API's MedicineBody, and a loaded
// medicine into the form's model. Empty optional fields stay out of the body.

import { stepperValue } from "../controls.js";
import { combobox } from "/static/kp/dist/kp-themes.min.js";
import { dateValue } from "./form-fields.js";
import { newSlot } from "./form-schedule.js";

/** The model a new medicine starts from. */
export const blankModel = () => ({ active: true, asNeeded: false, minGap: 240, maxPerDay: 0, slots: [newSlot()], unit: "tablets" });

/** @param {any} m a Medicine from the API */
export function modelOf(m) {
  return { active: m.active, asNeeded: m.as_needed, minGap: m.min_gap_minutes ?? 0, maxPerDay: m.max_per_day ?? 0, slots: m.slots.map((/** @type {any} */ s) => ({ ...s })), unit: m.stock_unit };
}

/** @param {HTMLFormElement} form @param {string} name */
const val = (form, name) => /** @type {HTMLInputElement | null} */ (form.elements.namedItem(name))?.value.trim() ?? "";

/** A stepper's number, or null when it is 0 ("no limit"). @param {HTMLElement | null} el */
const orNull = (el) => {
  const v = el ? stepperValue(el) : 0;
  return v > 0 ? v : null;
};

/**
 * @param {HTMLFormElement} form
 * @param {ReturnType<typeof blankModel>} model
 * @param {HTMLElement} tags the symptom combobox
 * @param {number | null} formId the chosen form pill
 */
export function readBody(form, model, tags, formId) {
  const q = (/** @type {string} */ label) => /** @type {HTMLElement | null} */ (form.querySelector(`[data-kp-quantity-label="${label}"]`));
  return {
    name: val(form, "name"),
    purpose: val(form, "purpose"),
    form_id: formId,
    strength: val(form, "strength"),
    notes: val(form, "notes"),
    active: model.active,
    as_needed: model.asNeeded,
    min_gap_minutes: model.asNeeded && model.minGap > 0 ? model.minGap : null,
    max_per_day: model.asNeeded && model.maxPerDay > 0 ? model.maxPerDay : null,
    stock: q("Stock") ? stepperValue(/** @type {HTMLElement} */ (q("Stock"))) : 0,
    stock_unit: val(form, "stock_unit"),
    low_stock_at: orNull(q("Warn when stock is at or below")),
    expires_on: dateValue(form, "expires_on"),
    opened_on: dateValue(form, "opened_on"),
    shelf_life_days: orNull(q("Good for days after opening")),
    course_ends_on: dateValue(form, "course_ends_on"),
    symptoms: combobox(tags)?.values() ?? [],
    slots: model.asNeeded ? [] : model.slots.map((s) => ({ time: s.time, days: s.days, dose: s.dose })),
  };
}
