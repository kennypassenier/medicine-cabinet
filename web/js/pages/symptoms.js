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

/**
 * Everything the page's functions share.
 * @typedef {{
 *   symptom: string, seq: number, alive: boolean, timer: number,
 *   known: { symptoms: any[], categories: any[] },
 *   ui: ReturnType<typeof buildLayout>, manager: ReturnType<typeof createManager>
 * }} Finder
 */

/** Build the page skeleton into `root`. @param {HTMLElement} root @param {string} symptom @param {{ run: () => void, reload: () => Promise<void> }} on */
function buildLayout(root, symptom, on) {
  const input = /** @type {HTMLInputElement} */ (h("input", { class: "kp-field__input mc-find__input", type: "search", id: "find-q", autocomplete: "off", placeholder: "e.g. headache, fever, hay fever", value: symptom, "aria-labelledby": "find-t" }));
  const suggest = h("div", { class: "kp-formgrid kp-formgrid--columns mc-find__chips" });
  const box = h("section", { class: "kp-card mc-find__box", "aria-labelledby": "find-t" },
    h("div", { class: "mc-find__ask" },
      h("div", { class: "kp-card__header" }, h("h2", { class: "kp-card__title", id: "find-t" }, "What's bothering you?")),
      h("p", { class: "kp-card__body" }, "Type a symptom or pick one. You see which medicine at home can help."),
      input),
    suggest);
  const have = dataCard({ id: "have", title: "You have these", description: "In stock, switched on and in date.", className: "mc-find__have", onRetry: on.run });
  const lack = dataCard({ id: "lack", title: "Not available", description: "They match, but cannot be used right now.", className: "mc-find__lack", onRetry: on.run });
  const manager = createManager({ reload: on.reload });
  const head = pageHeader({ title: "Symptoms", description: "Say what is wrong and see which medicine in your cabinet can help. Keep your symptoms and their categories tidy below." });
  root.append(head, box, h("div", { class: "mc-find" }, have.card, lack.card), manager.el);
  return { input, suggest, have, lack, manager };
}

function idle(/** @type {Finder} */ s) {
  s.ui.have.state("empty", { title: "Start with a symptom", text: "Type one above, or tap a suggestion." });
  s.ui.lack.state("empty", { title: "Nothing to show yet", text: "Medicines that match but cannot be used appear here, with the reason." });
}

function drawChips(/** @type {Finder} */ s) {
  s.ui.suggest.replaceChildren(...suggestionRows(s.known.symptoms, s.known.categories, s.symptom, (v) => pick(s, v)));
}

/** @param {Finder} s @param {string} v */
function pick(s, v) {
  s.symptom = v;
  s.ui.input.value = v;
  drawChips(s);
  void run(s);
}

/** @param {ReturnType<typeof dataCard>} card @param {any[]} list @param {boolean} usable @param {object} empty @param {() => void} again */
function fill(card, list, usable, empty, again) {
  if (!list.length) return card.state("empty", empty);
  card.data.replaceChildren(h("ul", { class: "mc-list mc-list--rows" }, ...list.map((m) => resultRow(m, { usable, take: (x) => takeOne(x, again) }))));
  card.state("ready");
}

/** Show a search answer in both cards. @param {Finder} s @param {{ usable: any[], unavailable: any[] }} r */
function showResults(s, r) {
  const again = () => void run(s, true);
  fill(s.ui.have, r.usable, true, { title: `Nothing in your cabinet for “${s.symptom}”`, text: r.unavailable.length ? "Something matches, but it cannot be used right now (see Not available)." : "Add a medicine and list this symptom under “Helps with”.", action: linkButton("Add a medicine", "/medicine/new") }, again);
  fill(s.ui.lack, r.unavailable, false, { title: "Everything that matches is available", text: "Nothing is out of stock, expired or switched off for this symptom." }, again);
}

/** @param {Finder} s @param {boolean} [quiet] keep the screen as it is while loading */
async function run(s, quiet = false) {
  history.replaceState(null, "", s.symptom ? `?symptom=${encodeURIComponent(s.symptom)}` : location.pathname);
  if (!s.symptom.trim()) return idle(s);
  const mine = ++s.seq;
  if (!quiet) for (const c of [s.ui.have, s.ui.lack]) c.state("loading");
  try {
    const r = await api.search(s.symptom.trim());
    if (!s.alive || mine !== s.seq) return;
    showResults(s, r);
  } catch (e) {
    if (!s.alive || mine !== s.seq) return;
    for (const c of [s.ui.have, s.ui.lack]) c.state("error", { title: "The search failed", text: e instanceof Error ? e.message : String(e) });
    reportError("The search failed.", e);
  }
}

/** The symptoms and categories: they feed the chips and the management area. @param {Finder} s */
async function load(s) {
  try {
    const [symptoms, categories] = await Promise.all([api.symptoms(), api.categories()]);
    if (!s.alive) return;
    s.known = { symptoms, categories };
    drawChips(s);
    s.ui.manager.draw(s.known);
  } catch (e) {
    if (!s.alive) return;
    s.ui.manager.fail(e instanceof Error ? e.message : String(e));
    reportError("The symptoms could not be loaded.", e);
  }
}

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "Symptoms · Medicine cabinet";
  const symptom = new URLSearchParams(location.search).get("symptom") ?? "";
  /** @type {Finder} */
  const s = {
    symptom, seq: 0, alive: true, timer: 0, known: { symptoms: [], categories: [] },
    ui: buildLayout(root, symptom, { run: () => void run(s), reload: () => load(s) }),
  };
  const { input, manager } = s.ui;
  input.addEventListener("input", () => {
    clearTimeout(s.timer);
    s.timer = window.setTimeout(() => { s.symptom = input.value; drawChips(s); void run(s); }, TYPING_MS);
  });
  manager.loading();
  void load(s);
  void run(s);
  const stop = onTick(() => (s.symptom ? run(s, true) : undefined));
  input.focus();
  return () => { s.alive = false; clearTimeout(s.timer); stop(); manager.stop(); };
}
