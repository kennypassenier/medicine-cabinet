// Find: "What's bothering you?" A big input and suggestion chips; results
// split into what you have and what is not available, with the reason.

import { h } from "../dom.js";
import { api } from "../api.js";
import { chips } from "../controls.js";
import { takeOne } from "../dose.js";
import { onTick } from "../ticker.js";
import { dataCard, linkButton, pageHeader, reportError } from "../ui.js";
import { resultRow } from "./find-rows.js";

const TYPING_MS = 250;

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "Find · Medicine cabinet";
  let symptom = new URLSearchParams(location.search).get("symptom") ?? "";
  let seq = 0;
  let alive = true;
  let timer = 0;

  const input = /** @type {HTMLInputElement} */ (h("input", { class: "kp-field__input mc-find__input", type: "search", id: "find-q", autocomplete: "off", placeholder: "e.g. headache, fever, hay fever", value: symptom }));
  const suggest = h("div", { class: "mc-find__chips" });
  const box = h("div", { class: "kp-field mc-find__box" }, h("label", { class: "kp-field__label", for: "find-q" }, "What's bothering you?"), input, suggest);
  const have = dataCard({ id: "have", title: "You have these", description: "In stock, switched on and in date.", className: "mc-find__have", onRetry: () => run() });
  const lack = dataCard({ id: "lack", title: "Not available", description: "They match, but cannot be used right now.", className: "mc-find__lack", onRetry: () => run() });
  const results = h("div", { class: "mc-find" }, have.card, lack.card);
  root.append(pageHeader({ title: "Find", description: "Say what is wrong and see which medicine in your cabinet can help." }), box, results);

  function idle() {
    have.state("empty", { title: "Start with a symptom", text: "Type one above, or tap a suggestion." });
    lack.state("empty", { title: "Nothing to show yet", text: "Medicines that match but cannot be used appear here, with the reason." });
  }

  /** @param {any[]} names */
  function drawChips(names) {
    suggest.replaceChildren(...(names.length ? [chips({ label: "Known symptoms", value: symptom.toLowerCase(), options: names.map((s) => [s.name, s.name, s.medicines]), onChange: (v) => pick(v) })] : []));
  }

  /** @param {string} v */
  function pick(v) {
    symptom = v;
    input.value = v;
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
    const url = symptom ? `?symptom=${encodeURIComponent(symptom)}` : location.pathname;
    history.replaceState(null, "", url);
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

  input.addEventListener("input", () => {
    clearTimeout(timer);
    timer = window.setTimeout(() => { symptom = input.value; void run(); }, TYPING_MS);
  });
  api.symptoms().then((names) => alive && drawChips(names)).catch((e) => reportError("Suggestions could not be loaded.", e));
  void run();
  const stop = onTick(() => (symptom ? run(true) : undefined));
  input.focus();
  return () => { alive = false; clearTimeout(timer); stop(); };
}
