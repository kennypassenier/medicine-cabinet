// Add / edit a medicine: cards (about, when, stock, symptoms, dates) in one form
// with the package's unsaved-changes bar in the header (Save and Discard, docked
// at the foot once the header scrolls away, and a warning when leaving). Delete is
// a soft delete with an Undo toast.

import { attachForms, attachComboboxes, attachDatePickers, attachUnsaved, markUnsavedClean, setUnsavedSaving, unsaved } from "/static/kp/dist/kp-themes.min.js";
import { UNSAVED_SAVE_EVENT, UNSAVED_LEAVE_EVENT } from "/static/kp/dist/kp-themes.min.js";
import { h } from "../dom.js";
import { api } from "../api.js";
import { wire } from "../controls.js";
import { setChangeCount } from "../leave.js";
import { undoable } from "../undo.js";
import { refreshNow } from "../ticker.js";
import { dataCard, linkButton, pageHeader, reportError, say } from "../ui.js";
import { blankModel, modelOf, readBody } from "./form-body.js";
import { symptomField } from "./form-symptoms.js";
import { formPills } from "./form-pills.js";
import { createWhen, slotsSummary } from "./form-when.js";
import { aboutSection, dangerSection, datesSection, helpsSection, stockSection, unitField } from "./form-sections.js";

/**
 * What the page's functions share.
 * @typedef {{
 *   root: HTMLElement, id: number | null, navigate: (href: string) => void, alive: boolean,
 *   holder: ReturnType<typeof dataCard>, pills: ReturnType<typeof formPills> | null,
 *   when: ReturnType<typeof createWhen> | null, bar: HTMLElement | null, detach: (() => void)[]
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

/** The form element: the cards in the page's 12-column body. Saving is the bar's job. @param {HTMLElement[]} cards */
function formElement(cards) {
  return /** @type {HTMLFormElement} */ (h("form", { class: "kp-form kp-ops__body", id: "med-form", "data-kp-form": "", novalidate: "", "aria-labelledby": "page-title" },
    h("div", { class: "kp-form__summary", "data-kp-form-summary": "", "data-kp-span": "12", tabindex: "-1", hidden: true }),
    ...cards));
}

/** The unsaved-changes bar of the header. */
const barElement = () => h("div", { class: "kp-unsaved", "data-kp-unsaved": "", "data-kp-unsaved-for": "#med-form", "data-kp-name": "medicine", "data-kp-keep": "off", "data-kp-warn-leave": "both" });

/** How the bar words a value: the form by its name, the schedule by its times. @param {any[]} forms */
const wordsFor = (forms) => (/** @type {HTMLElement} */ c, /** @type {string} */ v) => {
  if (c.dataset.kpMirror === "form") return forms.find((f) => String(f.id) === v)?.name ?? "(none)";
  return c.dataset.kpMirror === "slots" ? slotsSummary(v) : undefined;
};

/** Wire the bar: Save submits the form, links ask through the app's router, the router asks how many changes are open. @param {MedicinePage} p @param {HTMLFormElement} form @param {HTMLElement} bar @param {any[]} forms */
function wireBar(p, form, bar, forms) {
  p.bar = bar;
  p.detach.push(attachUnsaved(p.root, { show: wordsFor(forms) }));
  markUnsavedClean(bar);
  setChangeCount(() => unsaved(bar)?.changes().length ?? 0);
  bar.addEventListener(UNSAVED_SAVE_EVENT, () => form.requestSubmit());
  bar.addEventListener(UNSAVED_LEAVE_EVENT, (e) => {
    e.preventDefault();
    p.navigate(/** @type {CustomEvent} */ (e).detail.href);
  });
  form.addEventListener("kp-form-invalid", () => setUnsavedSaving(bar, false));
}

/** Replace the loading card by the form, filled from `med` when editing. @param {MedicinePage} p @param {any | null} med @param {any[]} known @param {any[]} forms */
function build(p, med, known, forms) {
  const model = med ? modelOf(med) : blankModel();
  const sym = symptomField(med?.symptoms ?? [], known);
  const unit = unitField(med, model, forms);
  const pills = formPills({ forms, formId: med?.form_id ?? null, onPick: unit.followForm });
  const when = createWhen(model);
  p.pills = pills;
  p.when = when;
  const form = formElement([aboutSection(med, pills.row), when.el, stockSection(med, model, unit.row), helpsSection(sym.field), datesSection(med), unit.list]);
  const danger = med ? dangerSection(med, () => void remove(p, med)) : null;
  const bar = barElement();
  const cancel = linkButton("Cancel", "/cabinet");
  cancel.setAttribute("data-kp-ops-wide", ""); // a phone has Discard, the tab bar and Back; the bar needs the room
  p.root.replaceChildren(
    pageHeader({ title: med ? med.name : "Add a medicine", description: med ? "Change anything, then save. Stock here replaces the current count." : "Fill in what you know. Only the name is required; you can edit everything later.", actions: [cancel, bar] }),
    form, ...(danger ? [h("div", { class: "kp-ops__body" }, danger)] : []));
  wire(form);
  p.detach.push(attachForms(p.root, { wrapper: ".kp-formgrid__row" }), attachComboboxes(form), attachDatePickers(form));
  p.detach.push(sym.wire());
  wireBar(p, form, bar, forms);
  form.addEventListener("kp-form-valid", () => void save(p, form, model, sym.box, med));
}

/** @param {MedicinePage} p @param {HTMLFormElement} form @param {any} model @param {HTMLElement} tags @param {any} med */
async function save(p, form, model, tags, med) {
  try {
    if (!model.asNeeded && p.when) model.slots = p.when.read();
    const body = readBody(form, model, tags, p.pills?.value() ?? null);
    const saved = med ? await api.update(med.id, body) : await api.create(body);
    if (p.bar) markUnsavedClean(p.bar);
    say(`${saved.name} ${med ? "saved" : "added to the cabinet"}.`);
    p.navigate("/cabinet");
  } catch (e) {
    if (p.bar) setUnsavedSaving(p.bar, false);
    reportError("Could not save the medicine.", e);
  }
}

/** Delete at once (a soft delete the server keeps for 14 days) and offer Undo. @param {MedicinePage} p @param {any} med */
async function remove(p, med) {
  try {
    await api.remove(med.id);
  } catch (e) {
    return reportError(`Could not delete ${med.name}.`, e);
  }
  if (p.bar) markUnsavedClean(p.bar);
  p.navigate("/cabinet");
  undoable(`${med.name} deleted.`, { undo: () => api.restore(med.id), failed: `Could not restore ${med.name}.`, after: refreshNow });
}

/** @param {HTMLElement} root @param {{ params: URLSearchParams, navigate: (h: string) => void }} ctx */
export function mount(root, { params, navigate }) {
  const id = params.get("id") ? Number(params.get("id")) : null;
  document.title = `${id ? "Edit" : "Add"} medicine · Medicine cabinet`;
  /** @type {MedicinePage} */
  const p = {
    root, id, navigate, alive: true, pills: null, when: null, bar: null, detach: [],
    holder: dataCard({ id: "load", title: id ? "Loading the medicine" : "New medicine", onRetry: () => start(p) }),
  };
  p.holder.card.dataset.kpSpan = "12";
  root.append(h("div", { class: "kp-ops__body" }, p.holder.card));
  p.holder.state("loading");
  void start(p);
  return () => {
    p.alive = false;
    setChangeCount(null);
    p.detach.forEach((d) => d());
    p.when?.stop();
  };
}
