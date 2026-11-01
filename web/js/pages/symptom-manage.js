// The management area of the Symptoms page: categories (filter pills with a
// menu each, an add row) on the small span, symptoms (name and category edited in
// place, an add row, a menu with Delete) on the large one. Every delete is done
// at once and offers Undo; an edit saves when you leave the field.

import { h } from "../dom.js";
import { api } from "../api.js";
import { drawEntityTable } from "../entity-table.js";
import { activate, addRow, inlineField, relabelMenu, rowMenu } from "../inline.js";
import { refreshNow } from "../ticker.js";
import { undoable } from "../undo.js";
import { dataCard, reportError } from "../ui.js";
import { drawCategoryPills, UNCATEGORISED } from "./symptom-categories.js";

/**
 * The manager's shared state.
 * @typedef {{
 *   data: { symptoms: any[], categories: any[] }, filter: string, detach: (() => void)[], reload: () => Promise<void>, edited: () => void,
 *   cats: ReturnType<typeof dataCard>, syms: ReturnType<typeof dataCard>, pillHost: HTMLElement, pillError: HTMLElement, symHost: HTMLElement, addSymHost: HTMLElement
 * }} Manager
 */

const plural = (/** @type {number} */ n, /** @type {string} */ one, /** @type {string} */ many) => `${n} ${n === 1 ? one : many}`;

/** @param {Manager} m */
const inFilter = (m) => (/** @type {any} */ s) => m.filter === "all" || (m.filter === "none" ? s.category_id === null : String(s.category_id) === m.filter);

/** Show (or clear, with "") the refusal under the pills. @param {Manager} m @param {string} text */
function pillFail(m, text) {
  m.pillError.textContent = text;
  m.pillError.hidden = !text;
}

/** Delete a category at once, keeping its symptoms (uncategorised); Undo files them back. @param {Manager} m @param {any} c */
async function removeCategory(m, c) {
  let gone;
  try {
    gone = await api.removeCategory(c.id);
  } catch (e) {
    return reportError(`Could not delete ${c.name}.`, e);
  }
  if (m.filter === String(c.id)) m.filter = "all";
  await m.reload();
  const n = gone.was.symptom_ids.length;
  undoable(`${c.name} deleted.${n ? ` ${plural(n, "symptom is", "symptoms are")} now uncategorised.` : ""}`, { undo: () => api.addCategory(gone.was.name, gone.was.symptom_ids), failed: `Could not bring ${c.name} back.`, after: refreshNow });
}

/** Delete a symptom at once; Undo creates it again with its category and its medicines. @param {Manager} m @param {any} s */
async function removeSymptom(m, s) {
  let gone;
  try {
    gone = await api.removeSymptom(s.id);
  } catch (e) {
    return reportError(`Could not delete ${s.name}.`, e);
  }
  await m.reload();
  const n = gone.was.medicine_ids.length;
  undoable(`${s.name} deleted.${n ? ` ${plural(n, "medicine no longer helps", "medicines no longer help")} with it.` : ""}`, { undo: () => api.addSymptom(gone.was), failed: `Could not bring ${s.name} back.`, after: refreshNow });
}

/** The pills and the category add row. @param {Manager} m */
function drawCategories(m) {
  const { symptoms, categories } = m.data;
  if (m.filter !== "all" && m.filter !== "none" && !categories.some((c) => String(c.id) === m.filter)) m.filter = "all";
  const stop = drawCategoryPills(m.pillHost, {
    categories, symptoms, filter: m.filter,
    onFilter: (v) => { m.filter = v; drawSymptoms(m); },
    rename: async (c, name) => { await api.editCategory(c.id, name); pillFail(m, ""); await m.reload(); },
    remove: (c) => void removeCategory(m, c),
    fail: (text) => pillFail(m, text),
  });
  m.cats.state("ready");
  return stop;
}

/** A symptom's table row: name and category edited in place, its count, its menu. @param {Manager} m @param {any} s */
function symptomRow(m, s) {
  const { categories } = m.data;
  const options = /** @type {[string, string][]} */ ([["", UNCATEGORISED], ...categories.map((c) => [String(c.id), c.name])]);
  const save = async (/** @type {Partial<{ name: string, category_id: number | null }>} */ patch) => {
    await api.editSymptom(s.id, { name: s.name, category_id: s.category_id, ...patch });
    Object.assign(s, patch);
    s.category = categories.find((c) => c.id === s.category_id)?.name ?? null;
    m.edited();
    m.detach.push(drawCategories(m));
  };
  const name = inlineField({ kind: "text", label: `Name of the symptom ${s.name}`, value: s.name, required: true, key: `sym-name-${s.id}`, onCommit: async (v) => { await save({ name: v }); name.input.closest("td")?.setAttribute("data-kp-sort-value", v); name.input.setAttribute("aria-label", `Name of the symptom ${v}`); cat.input.setAttribute("aria-label", `Category of ${v}`); relabelMenu(menu, `Actions for the symptom ${v}`); } });
  const cat = inlineField({ kind: "select", label: `Category of ${s.name}`, value: s.category_id === null ? "" : String(s.category_id), options, key: `sym-cat-${s.id}`, onCommit: async (v) => { await save({ category_id: v ? Number(v) : null }); cat.input.closest("td")?.setAttribute("data-kp-sort-value", s.category ?? ""); } });
  const menu = rowMenu(`Actions for the symptom ${s.name}`, [{ label: "Delete", value: "delete", danger: true, run: () => void removeSymptom(m, s) }]);
  return { key: `s${s.id}`, cells: [name.el, cat.el, s.medicines, menu], sortValues: [s.name, s.category ?? "", s.medicines, undefined] };
}

/** The symptom add row (name and category) above the table. @param {Manager} m */
function drawAddSymptom(m) {
  const options = /** @type {[string, string][]} */ ([["", UNCATEGORISED], ...m.data.categories.map((c) => [String(c.id), c.name])]);
  const { el } = addRow({
    label: "Add a symptom", hint: "Name what you take medicine for and where it is listed.", accept: "Add symptom",
    fields: [{ name: "name", label: "Name of the new symptom", placeholder: "e.g. headache", required: true }, { name: "category", label: "Category of the new symptom", kind: "select", options }],
    onAdd: async (v) => { await api.addSymptom({ name: v.name, category_id: v.category ? Number(v.category) : null }); await m.reload(); },
  });
  m.addSymHost.replaceChildren(el);
  m.detach.push(activate(m.addSymHost));
}

/** @param {Manager} m */
function drawSymptoms(m) {
  const { symptoms } = m.data;
  m.detach.forEach((d) => d());
  m.detach = [];
  m.detach.push(drawCategories(m));
  drawAddSymptom(m);
  if (!symptoms.length) {
    m.symHost.replaceChildren();
    return m.syms.state("empty", { title: "No symptoms yet", text: "Add the first one above, or list it while adding a medicine." });
  }
  m.detach.push(drawEntityTable(m.symHost, {
    label: "Symptoms", none: "No symptom matches.", search: "Search symptoms", pageSize: 12,
    columns: [{ title: "Symptom" }, { title: "Category" }, { title: "Medicines", sort: "number", numeric: true }, { title: "Actions", actions: true }],
    rows: symptoms.filter(inFilter(m)).map((s) => symptomRow(m, s)),
  }));
  m.detach.push(activate(m.symHost));
  m.syms.state("ready");
}

/** The two cards with their hosts; nothing is drawn yet. @param {() => Promise<void>} reload @param {() => void} edited @returns {Manager} */
function buildManager(reload, edited) {
  /** @type {Manager} */
  const m = {
    data: { symptoms: [], categories: [] }, filter: "all", detach: [], reload, edited,
    cats: dataCard({ id: "cats", title: "Categories", description: "A pill only filters the symptoms. Use the ⋯ menu beside a category to rename or delete it.", onRetry: () => reload() }),
    syms: dataCard({ id: "syms", title: "Symptoms", description: "Everything you can search for or list on a medicine. Edit a name or category in place; it is saved when you leave the field.", onRetry: () => reload() }),
    pillHost: h("div"), pillError: h("p", { class: "kp-field__error", role: "alert", hidden: true }), symHost: h("div"), addSymHost: h("div"),
  };
  m.cats.card.dataset.kpSpan = "4";
  m.syms.card.dataset.kpSpan = "8";
  m.syms.data.append(m.symHost);
  m.syms.card.querySelector(".kp-card__stage")?.before(m.addSymHost);
  const addCat = addRow({
    label: "Add a category", hint: "A group such as Head or Stomach.", accept: "Add category",
    fields: [{ name: "name", label: "Name of the new category", placeholder: "e.g. Head", required: true }],
    onAdd: async (v) => { await api.addCategory(v.name); await reload(); },
  });
  m.cats.data.append(h("div", { class: "kp-stack" }, m.pillHost, m.pillError, addCat.el));
  return m;
}

/** @param {{ reload: () => Promise<void>, edited: () => void }} ctx */
export function createManager({ reload, edited }) {
  const m = buildManager(reload, edited);
  return {
    cards: [m.cats.card, m.syms.card],
    /** @param {{ symptoms: any[], categories: any[] }} next */
    draw(next) {
      m.data = next;
      drawSymptoms(m);
      m.cats.state("ready");
    },
    loading() { m.cats.state("loading"); m.syms.state("loading"); },
    /** @param {string} text */
    fail(text) { for (const c of [m.cats, m.syms]) c.state("error", { title: "The lists could not be loaded", text }); },
    stop() { m.detach.forEach((d) => d()); },
  };
}
