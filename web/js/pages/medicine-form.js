// Add / edit a medicine: four cards (about, when, stock, dates) in one form,
// symptom chips with suggestions, and a clearly confirmed delete.

import { attachForms } from "/static/kp/js/forms.js";
import { attachComboboxes } from "/static/kp/js/combobox.js";
import { attachDatePickers } from "/static/kp/js/datepicker.js";
import { attachWeekdays } from "/static/kp/js/weekdays.js";
import { attachTimes } from "/static/kp/js/timefield.js";
import { h } from "../dom.js";
import { api } from "../api.js";
import { stepper, wire } from "../controls.js";
import { ask } from "../dialogs.js";
import { button, dataCard, linkButton, pageHeader, reportError, say } from "../ui.js";
import { date, row, section, text } from "./form-fields.js";
import { scheduleSection } from "./form-schedule.js";
import { blankModel, modelOf, readBody } from "./form-body.js";
import { symptomField } from "./form-symptoms.js";
import { formPills } from "./form-pills.js";
import { COMMON_UNITS } from "./forms-manage.js";

/** @param {HTMLElement} root @param {{ params: URLSearchParams, navigate: (h: string) => void }} ctx */
export function mount(root, { params, navigate }) {
  const id = params.get("id") ? Number(params.get("id")) : null;
  document.title = `${id ? "Edit" : "Add"} medicine · Medicine cabinet`;
  const holder = dataCard({ id: "load", title: id ? "Loading the medicine" : "New medicine", onRetry: () => start() });
  root.append(holder.card);
  holder.state("loading");
  let alive = true;
  /** @type {ReturnType<typeof formPills>} */
  let pills;
  /** @type {(() => void)[]} */
  let detach = [];

  async function start() {
    try {
      const [med, known, forms] = await Promise.all([id ? api.medicine(id) : null, api.symptoms(), api.forms()]);
      if (alive) build(med, known, forms);
    } catch (e) {
      if (alive) holder.state("error", { title: "The medicine could not be loaded", text: e instanceof Error ? e.message : String(e) });
    }
  }

  /** @param {any} med @param {any[]} known @param {any[]} forms */
  function build(med, known, forms) {
    const model = med ? modelOf(med) : blankModel();
    const sym = symptomField(med?.symptoms ?? [], known);
    const unitList = h("datalist", { id: "unit-list" });
    const setUnits = (/** @type {any[]} */ fs) => unitList.replaceChildren(...[...new Set([...fs.map((f) => f.unit), ...COMMON_UNITS])].map((u) => h("option", { value: u })));
    setUnits(forms);
    const unitRow = text({ name: "stock_unit", label: "Unit", value: model.unit, list: "unit-list", hint: "Counted in: tablets, ml, drops…", required: true });
    const unitInput = /** @type {HTMLInputElement} */ (unitRow.querySelector("input"));
    // The unit follows the chosen form until the person types their own.
    let unitAuto = !med || med.stock_unit === forms.find((f) => f.id === med.form_id)?.unit;
    unitInput.addEventListener("input", () => { unitAuto = false; model.unit = unitInput.value.trim(); });
    pills = formPills({
      forms, formId: med?.form_id ?? null, reload: () => api.forms(), onReloaded: setUnits,
      onPick: (f) => {
        if (!f || !(unitAuto || !unitInput.value.trim())) return;
        unitInput.value = f.unit;
        model.unit = f.unit;
        unitAuto = true;
        unitInput.dispatchEvent(new Event("change", { bubbles: true }));
      },
    });
    const about = section({ id: "about", title: "About", description: "What it is and how it comes.", className: "mc-form__about", rows: [
      text({ name: "name", label: "Name", value: med?.name, required: true, maxlength: 80, placeholder: "e.g. Paracetamol" }),
      text({ name: "purpose", label: "Purpose", value: med?.purpose, hint: "In your own words, e.g. “Pain and fever”." }),
      pills.row,
      text({ name: "strength", label: "Strength", value: med?.strength, hint: "e.g. 500 mg" }),
      text({ name: "notes", label: "Notes", value: med?.notes, multiline: true, hint: "With food, avoid alcohol, where the spare box is." }),
    ] });
    const helps = section({ id: "helps", title: "Symptoms", description: "What you take it for. These power the Symptoms page.", className: "mc-form__helps", rows: [sym.field] });
    const whenSlot = h("div", { class: "mc-form__when" });
    const drawWhen = () => {
      detach.forEach((d) => d());
      whenSlot.replaceChildren(scheduleSection(model, drawWhen));
      wire(whenSlot);
      detach = [attachWeekdays(whenSlot), attachTimes(whenSlot)];
    };
    drawWhen();
    const stock = section({ id: "stock", title: "Stock", description: "What is left, and when to be warned.", className: "mc-form__stock", rows: [
      row({ label: "In stock", control: stepper({ label: "Stock", value: med?.stock ?? 0, min: 0, step: 1, unit: model.unit }), hint: "How many you have now." }),
      unitRow,
      row({ label: "Low-stock warning", control: stepper({ label: "Warn when stock is at or below", value: med?.low_stock_at ?? 0, min: 0, step: 1, unit: model.unit }), hint: "Warn at or below this number. 0 means no warning." }),
    ] });
    const dates = section({ id: "dates", title: "Dates", description: "Expiry, opening and the end of a course. All optional.", className: "mc-form__dates", rows: [
      date({ name: "expires_on", label: "Expires on", value: med?.expires_on, hint: "Printed on the box." }),
      date({ name: "opened_on", label: "Opened on", value: med?.opened_on, hint: "For drops, syrups and creams." }),
      row({ label: "Good after opening", control: stepper({ label: "Good for days after opening", value: med?.shelf_life_days ?? 0, min: 0, step: 1, unit: "days" }), hint: "0 means it keeps until the printed date." }),
      date({ name: "course_ends_on", label: "Course ends on", value: med?.course_ends_on, hint: "An antibiotic course, for instance." }),
    ] });
    const form = /** @type {HTMLFormElement} */ (h("form", { class: "kp-form mc-form", "data-kp-form": "", novalidate: "", "aria-labelledby": "page-title" },
      h("div", { class: "kp-form__summary", "data-kp-form-summary": "", tabindex: "-1", hidden: true }), about, whenSlot, helps, stock, dates, unitList,
      h("div", { class: "mc-form__bar" }, linkButton("Cancel", "/cabinet"), h("button", { type: "submit", class: "kp-button kp-button--primary", "data-kp-submit": "" }, med ? "Save changes" : "Add medicine"))));
    const danger = med ? section({ id: "danger", title: "Delete", description: "Removes this medicine and its schedule. Switch it off in the cabinet instead to keep its history.", className: "mc-form__danger", rows: [
      row({ label: "Permanently", control: button(`Delete ${med.name}`, { look: "kp-button--destructive", onClick: () => remove(med) }) })] }) : null;
    root.replaceChildren(
      pageHeader({ title: med ? med.name : "Add a medicine", description: med ? "Change anything, then save. Stock here replaces the current count." : "Fill in what you know. Only the name is required; you can edit everything later." }),
      form, ...(danger ? [danger] : []));
    wire(form);
    detach.push(attachForms(root, { wrapper: ".kp-formgrid__row" }), attachComboboxes(form), attachDatePickers(form));
    sym.wire();
    form.addEventListener("kp-form-valid", () => save(form, model, sym.box, med));
  }

  /** @param {HTMLFormElement} form @param {any} model @param {HTMLElement} tags @param {any} med */
  async function save(form, model, tags, med) {
    const submit = /** @type {HTMLButtonElement} */ (form.querySelector("[data-kp-submit]"));
    submit.disabled = true;
    try {
      const body = readBody(form, model, tags, pills.value());
      const saved = med ? await api.update(med.id, body) : await api.create(body);
      say(`${saved.name} ${med ? "saved" : "added to the cabinet"}.`);
      navigate("/cabinet");
    } catch (e) {
      submit.disabled = false;
      reportError("Could not save the medicine.", e);
    }
  }

  /** @param {any} med */
  async function remove(med) {
    const yes = await ask({ title: `Delete ${med.name}?`, text: "This removes the medicine and its schedule for good. If you only want it out of sight, switch it off in the cabinet instead.", accept: `Delete ${med.name}`, destructive: true });
    if (!yes) return;
    try {
      await api.remove(med.id);
      say(`${med.name} deleted.`);
      navigate("/cabinet");
    } catch (e) {
      reportError(`Could not delete ${med.name}.`, e);
    }
  }

  void start();
  return () => {
    alive = false;
    detach.forEach((d) => d());
  };
}
