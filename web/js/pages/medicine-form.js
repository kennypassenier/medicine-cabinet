// Add / edit a medicine: four cards (about, when, stock, dates) in one form,
// symptom chips with suggestions, and a clearly confirmed delete.

import { attachForms } from "/static/kp/js/forms.js";
import { attachComboboxes } from "/static/kp/js/combobox.js";
import { attachDatePickers } from "/static/kp/js/datepicker.js";
import { h } from "../dom.js";
import { api } from "../api.js";
import { wire } from "../controls.js";
import { ask } from "../dialogs.js";
import { dataCard, linkButton, pageHeader, reportError, say } from "../ui.js";
import { blankModel, modelOf, readBody } from "./form-body.js";
import { symptomField } from "./form-symptoms.js";
import { formPills } from "./form-pills.js";
import { aboutSection, createWhenSlot, dangerSection, datesSection, helpsSection, stockSection, unitField } from "./form-sections.js";

/**
 * What the page's functions share.
 * @typedef {{
 *   root: HTMLElement, id: number | null, navigate: (href: string) => void, alive: boolean,
 *   holder: ReturnType<typeof dataCard>, pills: ReturnType<typeof formPills> | null,
 *   when: ReturnType<typeof createWhenSlot> | null, detach: (() => void)[]
 * }} MedicinePage
 */

/** @param {MedicinePage} p */
async function start(p) {
  try {
    const [med, known, forms] = await Promise.all([p.id ? api.medicine(p.id) : null, api.symptoms(), api.forms()]);
    if (p.alive) build(p, med, known, forms);
  } catch (e) {
    if (p.alive) p.holder.state("error", { title: "The medicine could not be loaded", text: e instanceof Error ? e.message : String(e) });
  }
}

/** The form element: the cards in order, then the Cancel / Save bar. @param {any | null} med @param {HTMLElement[]} cards */
function formElement(med, cards) {
  return /** @type {HTMLFormElement} */ (h("form", { class: "kp-form mc-form", "data-kp-form": "", novalidate: "", "aria-labelledby": "page-title" },
    h("div", { class: "kp-form__summary", "data-kp-form-summary": "", tabindex: "-1", hidden: true }), ...cards,
    h("div", { class: "mc-form__bar" }, linkButton("Cancel", "/cabinet"), h("button", { type: "submit", class: "kp-button kp-button--primary", "data-kp-submit": "" }, med ? "Save changes" : "Add medicine"))));
}

/** Replace the loading card by the form, filled from `med` when editing. @param {MedicinePage} p @param {any | null} med @param {any[]} known @param {any[]} forms */
function build(p, med, known, forms) {
  const model = med ? modelOf(med) : blankModel();
  const sym = symptomField(med?.symptoms ?? [], known);
  const unit = unitField(med, model, forms);
  const pills = formPills({ forms, formId: med?.form_id ?? null, reload: () => api.forms(), onReloaded: unit.setUnits, onPick: unit.followForm });
  const when = createWhenSlot(model);
  p.pills = pills;
  p.when = when;
  when.draw();
  const form = formElement(med, [aboutSection(med, pills.row), when.el, helpsSection(sym.field), stockSection(med, model, unit.row), datesSection(med), unit.list]);
  const danger = med ? dangerSection(med, () => void remove(p, med)) : null;
  p.root.replaceChildren(
    pageHeader({ title: med ? med.name : "Add a medicine", description: med ? "Change anything, then save. Stock here replaces the current count." : "Fill in what you know. Only the name is required; you can edit everything later." }),
    form, ...(danger ? [danger] : []));
  wire(form);
  p.detach.push(attachForms(p.root, { wrapper: ".kp-formgrid__row" }), attachComboboxes(form), attachDatePickers(form));
  sym.wire();
  form.addEventListener("kp-form-valid", () => void save(p, form, model, sym.box, med));
}

/** @param {MedicinePage} p @param {HTMLFormElement} form @param {any} model @param {HTMLElement} tags @param {any} med */
async function save(p, form, model, tags, med) {
  const submit = /** @type {HTMLButtonElement} */ (form.querySelector("[data-kp-submit]"));
  submit.disabled = true;
  try {
    const body = readBody(form, model, tags, p.pills?.value() ?? null);
    const saved = med ? await api.update(med.id, body) : await api.create(body);
    say(`${saved.name} ${med ? "saved" : "added to the cabinet"}.`);
    p.navigate("/cabinet");
  } catch (e) {
    submit.disabled = false;
    reportError("Could not save the medicine.", e);
  }
}

/** @param {MedicinePage} p @param {any} med */
async function remove(p, med) {
  const yes = await ask({ title: `Delete ${med.name}?`, text: "This removes the medicine and its schedule for good. If you only want it out of sight, switch it off in the cabinet instead.", accept: `Delete ${med.name}`, destructive: true });
  if (!yes) return;
  try {
    await api.remove(med.id);
    say(`${med.name} deleted.`);
    p.navigate("/cabinet");
  } catch (e) {
    reportError(`Could not delete ${med.name}.`, e);
  }
}

/** @param {HTMLElement} root @param {{ params: URLSearchParams, navigate: (h: string) => void }} ctx */
export function mount(root, { params, navigate }) {
  const id = params.get("id") ? Number(params.get("id")) : null;
  document.title = `${id ? "Edit" : "Add"} medicine · Medicine cabinet`;
  /** @type {MedicinePage} */
  const p = {
    root, id, navigate, alive: true, pills: null, when: null, detach: [],
    holder: dataCard({ id: "load", title: id ? "Loading the medicine" : "New medicine", onRetry: () => start(p) }),
  };
  root.append(p.holder.card);
  p.holder.state("loading");
  void start(p);
  return () => {
    p.alive = false;
    p.detach.forEach((d) => d());
    p.when?.stop();
  };
}
