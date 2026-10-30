// Symptoms: the one place for them. "What's bothering you?" searches the
// cabinet (what you have / what is not available) with suggestion chips
// grouped by category; below it the symptoms and categories are managed.

import { h } from "../dom.js";
import { api } from "../api.js";
import { chips } from "../controls.js";
import { takeOne } from "../dose.js";
import { onTick } from "../ticker.js";
import { dataCard, linkButton, pageHeader, reportError } from "../ui.js";
import { resultRow } from "./symptoms-rows.js";
import { createManager } from "./symptom-manage.js";

const TYPING_MS = 250;
const NO_CATEGORY = "Other";

/** The suggestion chips as form-grid rows: the category in the label column, its symptoms as chips. */
function suggestionRows(/** @type {any[]} */ symptoms, /** @type {any[]} */ categories, /** @type {string} */ symptom, /** @type {(v: string) => void} */ pick) {
  const groups = [...categories.map((c) => [c.name, c.id]), [NO_CATEGORY, null]];
  const rows = groups.map(([name, id]) => [name, symptoms.filter((s) => s.category_id === id)]).filter(([, list]) => list.length);
  return rows.map(([name, list]) => h("div", { class: "kp-formgrid__row" },
    h("span", { class: "kp-formgrid__label" }, name),
    h("div", { class: "kp-formgrid__control" }, chips({ label: `${name} symptoms`, value: symptom.toLowerCase(), options: list.map((/** @type {any} */ s) => [s.name, s.name, s.medicines]), onChange: pick }))));
}

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "Symptoms · Medicine cabinet";
  let symptom = new URLSearchParams(location.search).get("symptom") ?? "";
  let seq = 0;
  let alive = true;
  let timer = 0;
  let known = { symptoms: /** @type {any[]} */ ([]), categories: /** @type {any[]} */ ([]) };

  const input = /** @type {HTMLInputElement} */ (h("input", { class: "kp-field__input mc-find__input", type: "search", id: "find-q", autocomplete: "off", placeholder: "e.g. headache, fever, hay fever", value: symptom, "aria-labelledby": "find-t" }));
  const suggest = h("div", { class: "kp-formgrid kp-formgrid--columns mc-find__chips" });
  const box = h("section", { class: "kp-card mc-find__box", "aria-labelledby": "find-t" },
    h("div", { class: "mc-find__ask" },
      h("div", { class: "kp-card__header" }, h("h2", { class: "kp-card__title", id: "find-t" }, "What's bothering you?")),
      h("p", { class: "kp-card__body" }, "Type a symptom or pick one. You see which medicine at home can help."),
      input),
    suggest);
  const have = dataCard({ id: "have", title: "You have these", description: "In stock, switched on and in date.", className: "mc-find__have", onRetry: () => run() });
  const lack = dataCard({ id: "lack", title: "Not available", description: "They match, but cannot be used right now.", className: "mc-find__lack", onRetry: () => run() });
  const manager = createManager({ reload: () => load() });
  const head = pageHeader({ title: "Symptoms", description: "Say what is wrong and see which medicine in your cabinet can help. Keep your symptoms and their categories tidy below." });
  root.append(head, box, h("div", { class: "mc-find" }, have.card, lack.card), manager.el);

  function idle() {
    have.state("empty", { title: "Start with a symptom", text: "Type one above, or tap a suggestion." });
    lack.state("empty", { title: "Nothing to show yet", text: "Medicines that match but cannot be used appear here, with the reason." });
  }

  function drawChips() {
    suggest.replaceChildren(...suggestionRows(known.symptoms, known.categories, symptom, pick));
  }

  /** @param {string} v */
  function pick(v) {
    symptom = v;
    input.value = v;
    drawChips();
    run();
  }

  /** @param {ReturnType<typeof dataCard>} card @param {any[]} list @param {boolean} usable @param {object} empty */
  function fill(card, list, usable, empty) {
    if (!list.length) return card.state("empty", empty);
    card.data.replaceChildren(h("ul", { class: "mc-list mc-list--rows" }, ...list.map((m) => resultRow(m, { usable, take: (x) => takeOne(x, () => run(true)) }))));
    card.state("ready");
  }

  /** @param {boolean} [quiet] keep the screen as it is while loading */
  async function run(quiet = false) {
    history.replaceState(null, "", symptom ? `?symptom=${encodeURIComponent(symptom)}` : location.pathname);
    if (!symptom.trim()) return idle();
    const mine = ++seq;
    if (!quiet) for (const c of [have, lack]) c.state("loading");
    try {
      const r = await api.search(symptom.trim());
      if (!alive || mine !== seq) return;
      fill(have, r.usable, true, { title: `Nothing in your cabinet for “${symptom}”`, text: r.unavailable.length ? "Something matches, but it cannot be used right now (see Not available)." : "Add a medicine and list this symptom under “Helps with”.", action: linkButton("Add a medicine", "/medicine/new") });
      fill(lack, r.unavailable, false, { title: "Everything that matches is available", text: "Nothing is out of stock, expired or switched off for this symptom." });
    } catch (e) {
      if (!alive || mine !== seq) return;
      for (const c of [have, lack]) c.state("error", { title: "The search failed", text: e instanceof Error ? e.message : String(e) });
      reportError("The search failed.", e);
    }
  }

  /** The symptoms and categories: they feed the chips and the management area. */
  async function load() {
    try {
      const [symptoms, categories] = await Promise.all([api.symptoms(), api.categories()]);
      if (!alive) return;
      known = { symptoms, categories };
      drawChips();
      manager.draw(known);
    } catch (e) {
      if (!alive) return;
      manager.fail(e instanceof Error ? e.message : String(e));
      reportError("The symptoms could not be loaded.", e);
    }
  }

  input.addEventListener("input", () => {
    clearTimeout(timer);
    timer = window.setTimeout(() => { symptom = input.value; drawChips(); void run(); }, TYPING_MS);
  });
  manager.loading();
  void load();
  void run();
  const stop = onTick(() => (symptom ? run(true) : undefined));
  input.focus();
  return () => { alive = false; clearTimeout(timer); stop(); manager.stop(); };
}
