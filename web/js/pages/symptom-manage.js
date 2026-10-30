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

/** @param {{ reload: () => Promise<void> }} ctx */
export function createManager({ reload }) {
  /** @type {{ symptoms: any[], categories: any[] }} */
  let data = { symptoms: [], categories: [] };
  let filter = "all";
  /** @type {(() => void)[]} */
  let detach = [];
  /** @param {Promise<boolean>} done */
  const then = async (done) => { if (await done) await reload(); };

  const cats = dataCard({ id: "cats", title: "Categories", description: "Pick one to filter the symptoms. Rename or delete the one you picked.", className: "mc-manage__cats", onRetry: () => reload(), actions: [button("Add category", { onClick: () => then(addCategory()) })] });
  const syms = dataCard({ id: "syms", title: "Symptoms", description: "Everything you can search for or list on a medicine.", className: "mc-manage__syms", onRetry: () => reload(), actions: [button("Add symptom", { look: "kp-button--primary", onClick: () => then(addSymptom(data.categories, /^\d+$/.test(filter) ? Number(filter) : null)) })] });
  const el = h("div", { class: "mc-manage" }, cats.card, syms.card);
  const symHost = h("div", { class: "mc-manage__table" });
  syms.data.append(symHost);
  const pillHost = h("div", { class: "mc-manage__filter" });
  const pickedLine = h("p", { class: "mc-manage__picked" });
  const catButtons = h("div", { class: "mc-manage__catbuttons" });
  cats.data.append(pillHost, pickedLine, catButtons);

  const picked = () => data.categories.find((c) => String(c.id) === filter) ?? null;

  function drawCategories() {
    const unc = data.symptoms.filter((s) => s.category_id === null).length;
    const options = /** @type {[string, string, number][]} */ ([["all", "All", data.symptoms.length], ...data.categories.map((c) => [String(c.id), c.name, c.symptoms]), ...(unc || filter === "none" ? [["none", "Uncategorised", unc]] : [])]);
    if (!options.some(([v]) => v === filter)) filter = "all";
    pillHost.replaceChildren(chips({ label: "Show category", options, value: filter, onChange: (v) => { filter = v; drawCategories(); drawSymptoms(); } }));
    const c = picked();
    const why = c ? "" : "Pick a category above first.";
    pickedLine.textContent = c ? `${c.name}: ${c.symptoms} ${c.symptoms === 1 ? "symptom" : "symptoms"}.` : filter === "none" ? "These symptoms have no category yet." : "Showing every symptom.";
    catButtons.replaceChildren(
      button("Rename", { onClick: () => c && then(editCategory(c)), title: why, label: c ? `Rename category ${c.name}` : "Rename category" }),
      button("Delete", { look: "kp-button--destructive", onClick: () => c && then(deleteCategory(c)), title: why, label: c ? `Delete category ${c.name}` : "Delete category" }));
    for (const b of catButtons.children) { b.toggleAttribute("disabled", !c); }
    cats.state("ready");
  }

  const inFilter = (/** @type {any} */ s) => filter === "all" || (filter === "none" ? s.category_id === null : String(s.category_id) === filter);

  function drawSymptoms() {
    if (!data.symptoms.length) return syms.state("empty", { title: "No symptoms yet", text: "Add the first one, or list it while adding a medicine.", action: button("Add symptom", { look: "kp-button--primary", onClick: () => then(addSymptom(data.categories)) }) });
    detach.forEach((d) => d());
    detach = [drawEntityTable(symHost, {
      label: "Symptoms", none: "No symptom matches.", search: "Search symptoms", pageSize: 12,
      columns: [{ title: "Symptom" }, { title: "Category" }, { title: "Medicines", sort: "number", numeric: true }, { title: "Actions", actions: true }],
      rows: data.symptoms.filter(inFilter).map((s) => ({ key: `s${s.id}`, cells: [s.name, s.category ?? "None", s.medicines, actions("symptom", s.name, () => then(editSymptom(s, data.categories)), () => then(deleteSymptom(s)), "Edit")] })),
    })];
    syms.state("ready");
  }

  return {
    el,
    /** @param {{ symptoms: any[], categories: any[] }} next */
    draw(next) {
      data = next;
      drawCategories();
      drawSymptoms();
    },
    loading() { cats.state("loading"); syms.state("loading"); },
    /** @param {string} text */
    fail(text) { for (const c of [cats, syms]) c.state("error", { title: "The lists could not be loaded", text }); },
    stop() { detach.forEach((d) => d()); },
  };
}
