// "Add an earlier dose": for a dose that was taken (or skipped) and never
// logged. Medicine, day, time, dose and what happened, on one grid; saved with
// the past time, and the toast offers Undo.

import { attachDatePickers, attachTimes, attachSelects, attachSegmented } from "/static/kp/dist/kp-themes.min.js";
import { h } from "../dom.js";
import { api } from "../api.js";
import { dayParam, nowParam } from "../clock.js";
import { chips, stepper, stepperValue, wire } from "../controls.js";
import { qty } from "../format.js";
import { undoable } from "../undo.js";
import { dataCard, errorText } from "../ui.js";
import { datePicker, dateValue } from "./form-fields.js";

/** A labelled field of the grid. @param {string} label @param {string} id @param {HTMLElement} control */
const field = (label, id, control) => h("div", { class: "kp-field" }, h("label", { class: "kp-field__label", for: id }, label), control);

/**
 * @param {{ meds: any[], onAdded: () => void }} o `onAdded` runs after the dose was saved or its Undo (reload the log)
 * @returns {{ card: HTMLElement, detach: () => void }}
 */
export function addEarlierDose({ meds, onAdded }) {
  const holder = dataCard({ id: "earlier", title: "Add an earlier dose", description: "Forgot to log a dose? Add it here with the day and time you took it." });
  holder.card.dataset.kpSpan = "12";
  const err = h("p", { class: "kp-field__error", role: "alert", hidden: true });
  const pick = /** @type {HTMLSelectElement} */ (h("select", { class: "kp-field__input", id: "earlier-med" }, ...[...meds].sort((a, b) => a.name.localeCompare(b.name)).map((m) => h("option", { value: String(m.id) }, m.name))));
  const day = datePicker({ name: "earlier_day", label: "Day", id: "earlier-day", value: dayParam(), max: dayParam() });
  const clock = h("div", { class: "kp-time", "data-kp-time": "", "data-kp-name": "earlier-time", "data-kp-value": nowParam().slice(11), "data-kp-label": "Time", "data-kp-time-id": "earlier-time", "data-kp-time-step": "5" });
  const dose = stepper({ id: "earlier-dose", label: "Dose", value: 1, min: 0.25, step: 0.25, unit: meds[0]?.stock_unit ?? "" });
  let status = "taken";
  const what = chips({ label: "What happened", options: [["taken", "Taken"], ["skipped", "Skipped"]], value: status, onChange: (v) => { status = v; } });
  const go = h("button", { type: "button", class: "kp-button kp-button--primary kp-w-full" }, "Add dose");
  const form = h("div", { class: "kp-stack" },
    h("div", { class: "kp-autogrid" }, field("Medicine", "earlier-med", pick), field("Day", "earlier-day", day), field("Time", "earlier-time", clock), field("Dose", "earlier-dose", dose), field("What happened", "earlier-what", what), h("div", { class: "kp-field kp-self-end" }, go)),
    err);
  holder.data.append(form);
  holder.state("ready");

  async function submit() {
    const med = meds.find((m) => String(m.id) === pick.value);
    const iso = dateValue(form, "earlier_day");
    const time = clock.querySelector("input")?.value ?? "";
    if (!med || !iso || !/^\d{1,2}:\d{2}$/.test(time)) { err.textContent = "Pick the medicine, a valid day and a time like 08:30, then press Add dose."; err.hidden = false; return; }
    const takenAt = `${iso}T${time.padStart(5, "0")}`;
    try {
      const { id } = await api.logIntake({ medicineId: med.id, status: /** @type {"taken" | "skipped"} */ (status), takenAt, plannedFor: null, dose: stepperValue(dose) });
      err.hidden = true;
      onAdded();
      undoable(`${med.name} ${status === "taken" ? `${qty(stepperValue(dose), med.stock_unit)} taken` : "skipped"} at ${time.padStart(5, "0")} added.`, { undo: () => api.undoIntake(id), failed: `Could not remove the ${med.name} entry again.`, after: onAdded });
    } catch (e) {
      err.textContent = errorText(e);
      err.hidden = false;
    }
  }
  go.addEventListener("click", () => void submit());
  wire(holder.data);
  const detach = [attachDatePickers(holder.data), attachTimes(holder.data), attachSelects(holder.data), attachSegmented(holder.data)];
  return { card: holder.card, detach: () => detach.forEach((d) => d?.()) };
}
