// The management area of the Symptoms page: categories on the small span,
// symptoms (filtered by category pill) on the large one. Both are data tables
// whose Edit / Delete buttons stand in one column; every change reloads.

import { h } from "../dom.js";
import { chips } from "../controls.js";
import { drawEntityTable, rowButton } from "../entity-table.js";
import { button, dataCard } from "../ui.js";
import { addCategory, addSymptom, deleteCategory, deleteSymptom, editCategory, editSymptom } from "./symptom-actions.js";

/** Both buttons of a managed row, in one fixed cell. @param {string} noun @param {string} name @param {() => void} edit @param {() => void} del @param {string} editLabel */
const actions = (noun, name, edit, del, editLabel) =>
  h("span", { class: "mc-actions__pair" },
    rowButton({ label: editLabel, aria: `${editLabel} ${noun} ${name}`, onClick: edit }),
    rowButton({ label: "Delete", aria: `Delete ${noun} ${name}`, look: "kp-button--destructive", onClick: del }));

/**
 * The manager's shared state: the lists, the chosen category pill, the cards.
 * @typedef {{
 *   data: { symptoms: any[], categories: any[] }, filter: string, detach: (() => void)[], reload: () => Promise<void>,
 *   cats: ReturnType<typeof dataCard>, syms: ReturnType<typeof dataCard>,
 *   symHost: HTMLElement, pillHost: HTMLElement, pickedLine: HTMLElement, catButtons: HTMLElement
 * }} Manager
 */

/** Run a dialog flow; reload the lists when it saved something. @param {Manager} m @param {Promise<boolean>} done */
const then = async (m, done) => { if (await done) await m.reload(); };

/** The category the pill chose, or null for "All" / "Uncategorised". @param {Manager} m */
const picked = (m) => m.data.categories.find((c) => String(c.id) === m.filter) ?? null;

/** @param {Manager} m */
const inFilter = (m) => (/** @type {any} */ s) => m.filter === "all" || (m.filter === "none" ? s.category_id === null : String(s.category_id) === m.filter);

/** The two cards with their hosts; nothing is drawn yet. @param {() => Promise<void>} reload @returns {Manager} */
function buildManager(reload) {
  /** @type {Manager} */
  const m = {
    data: { symptoms: [], categories: [] }, filter: "all", detach: [], reload,
    cats: dataCard({ id: "cats", title: "Categories", description: "Pick one to filter the symptoms. Rename or delete the one you picked.", className: "mc-manage__cats", onRetry: () => reload(), actions: [button("Add category", { onClick: () => then(m, addCategory()) })] }),
    syms: dataCard({ id: "syms", title: "Symptoms", description: "Everything you can search for or list on a medicine.", className: "mc-manage__syms", onRetry: () => reload(), actions: [button("Add symptom", { look: "kp-button--primary", onClick: () => then(m, addSymptom(m.data.categories, /^\d+$/.test(m.filter) ? Number(m.filter) : null)) })] }),
    symHost: h("div", { class: "mc-manage__table" }),
    pillHost: h("div", { class: "mc-manage__filter" }),
    pickedLine: h("p", { class: "mc-manage__picked" }),
    catButtons: h("div", { class: "mc-manage__catbuttons" }),
  };
  m.syms.data.append(m.symHost);
  m.cats.data.append(m.pillHost, m.pickedLine, m.catButtons);
  return m;
}

/** Rename / Delete for the picked category; both stay disabled until one is picked. @param {Manager} m */
function drawCategoryButtons(m) {
  const c = picked(m);
  const why = c ? "" : "Pick a category above first.";
  m.pickedLine.textContent = c ? `${c.name}: ${c.symptoms} ${c.symptoms === 1 ? "symptom" : "symptoms"}.` : m.filter === "none" ? "These symptoms have no category yet." : "Showing every symptom.";
  m.catButtons.replaceChildren(
    button("Rename", { onClick: () => c && then(m, editCategory(c)), title: why, label: c ? `Rename category ${c.name}` : "Rename category" }),
    button("Delete", { look: "kp-button--destructive", onClick: () => c && then(m, deleteCategory(c)), title: why, label: c ? `Delete category ${c.name}` : "Delete category" }));
  for (const b of m.catButtons.children) { b.toggleAttribute("disabled", !c); }
}

/** @param {Manager} m */
function drawCategories(m) {
  const { symptoms, categories } = m.data;
  const unc = symptoms.filter((s) => s.category_id === null).length;
  const options = /** @type {[string, string, number][]} */ ([["all", "All", symptoms.length], ...categories.map((c) => [String(c.id), c.name, c.symptoms]), ...(unc || m.filter === "none" ? [["none", "Uncategorised", unc]] : [])]);
  if (!options.some(([v]) => v === m.filter)) m.filter = "all";
  m.pillHost.replaceChildren(chips({ label: "Show category", options, value: m.filter, onChange: (v) => { m.filter = v; drawCategories(m); drawSymptoms(m); } }));
  drawCategoryButtons(m);
  m.cats.state("ready");
}

/** @param {Manager} m */
function drawSymptoms(m) {
  const { symptoms, categories } = m.data;
  if (!symptoms.length) return m.syms.state("empty", { title: "No symptoms yet", text: "Add the first one, or list it while adding a medicine.", action: button("Add symptom", { look: "kp-button--primary", onClick: () => then(m, addSymptom(categories)) }) });
  m.detach.forEach((d) => d());
  m.detach = [drawEntityTable(m.symHost, {
    label: "Symptoms", none: "No symptom matches.", search: "Search symptoms", pageSize: 12,
    columns: [{ title: "Symptom" }, { title: "Category" }, { title: "Medicines", sort: "number", numeric: true }, { title: "Actions", actions: true }],
    rows: symptoms.filter(inFilter(m)).map((s) => ({ key: `s${s.id}`, cells: [s.name, s.category ?? "None", s.medicines, actions("symptom", s.name, () => then(m, editSymptom(s, categories)), () => then(m, deleteSymptom(s)), "Edit")] })),
  })];
  m.syms.state("ready");
}

/** @param {{ reload: () => Promise<void> }} ctx */
export function createManager({ reload }) {
  const m = buildManager(reload);
  return {
    el: h("div", { class: "mc-manage" }, m.cats.card, m.syms.card),
    /** @param {{ symptoms: any[], categories: any[] }} next */
    draw(next) {
      m.data = next;
      drawCategories(m);
      drawSymptoms(m);
    },
    loading() { m.cats.state("loading"); m.syms.state("loading"); },
    /** @param {string} text */
    fail(text) { for (const c of [m.cats, m.syms]) c.state("error", { title: "The lists could not be loaded", text }); },
    stop() { m.detach.forEach((d) => d()); },
  };
}
