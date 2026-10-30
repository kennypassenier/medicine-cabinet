// Today: what to take next, the day's schedule with one-tap Take / Skip /
// Undo, what needs attention, and the when-needed medicines.

import { h, redraw } from "../dom.js";
import { api, plannedToday } from "../api.js";
import { longDay, nowParam } from "../clock.js";
import { onTick } from "../ticker.js";
import { dataCard, pageHeader, reportError, linkButton } from "../ui.js";
import { takeOne } from "../dose.js";
import { renderNext, renderSchedule } from "./today-doses.js";
import { renderAttention, attentionBand } from "./today-attention.js";
import { renderAsNeeded } from "./today-asneeded.js";

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "Today · Medicine cabinet";
  const heading = pageHeader({ title: "Today", description: longDay() });
  const next = dataCard({ id: "next", title: "Next dose", description: "What to take now, or next.", className: "mc-today__next", onRetry: () => load(true) });
  const sched = dataCard({ id: "sched", title: "Schedule", description: "Every dose planned for today. Tap Take when you have taken it.", className: "mc-today__schedule", onRetry: () => load(true) });
  const need = dataCard({ id: "need", title: "When needed", description: "Medicines you take as symptoms call for them. The cabinet checks the safe gap first.", className: "mc-today__asneeded", onRetry: () => load(true) });
  const band = attentionBand(() => draw());
  const grid = h("div", { class: "mc-today" }, next.card, band, sched.card, need.card);
  root.append(heading, grid);
  for (const c of [next, sched, need]) c.state("loading");

  let model = /** @type {{ today: any, meds: Map<number, any> } | null} */ (null);
  let loadedOnce = false;
  let alive = true;

  /** Draw from the model (also used for the optimistic step before the server answers). */
  function draw() {
    if (!model) return;
    const { today, meds } = model;
    heading.querySelector(".kp-page-header__description")?.replaceChildren(`${longDay()} · updated ${nowParam().slice(11)}`);
    if (meds.size === 0) return firstRun();
    const on = { take: (/** @type {any} */ d) => log(d, "taken"), skip: (/** @type {any} */ d) => log(d, "skipped"), undo };
    redraw(grid, () => {
      renderAttention(band, today.attention, meds, () => load());
      renderNext(next.data, today.doses, meds, on);
      renderSchedule(sched.data, today.doses, meds, on);
      need.card.hidden = today.as_needed.length === 0;
      renderAsNeeded(need.data, today.as_needed.map((/** @type {any} */ m) => meds.get(m.id) ?? m), (m) => takeOne(m, () => load()));
    });
    next.state("ready");
    if (today.doses.length) sched.state("ready");
    else sched.state("empty", { title: "No doses planned today", text: "Medicines with a schedule show up here at their times. Switch one on in the cabinet, or add a schedule." , action: linkButton("Open the cabinet", "/cabinet") });
    need.state("ready");
  }

  function firstRun() {
    grid.replaceChildren(h("section", { class: "kp-card mc-today__first" },
      h("div", { class: "kp-empty" },
        h("h2", { class: "kp-empty__title" }, "Your cabinet is empty"),
        h("p", { class: "kp-empty__body" }, "Add the first medicine, with its schedule and how much you have. Today then tells you what to take and when, and warns before anything runs out or expires."),
        linkButton("Add your first medicine", "/medicine/new", "kp-button--primary kp-button--lg"))));
  }

  async function load(manual = false) {
    try {
      const [today, list] = await Promise.all([api.today(), api.medicines()]);
      if (!alive) return;
      model = { today, meds: new Map(list.map((/** @type {any} */ m) => [m.id, m])) };
      loadedOnce = true;
      draw();
    } catch (e) {
      if (!alive) return;
      if (!loadedOnce) for (const c of [next, sched, need]) c.state("error", { title: "Today could not be loaded", text: e instanceof Error ? e.message : String(e) });
      else if (manual || !document.hidden) reportError("Today could not be refreshed.", e);
    }
  }

  /** @param {any} d @param {"taken" | "skipped"} status */
  async function log(d, status) {
    if (model) {
      d.state = status;
      draw();
    }
    try {
      await api.logIntake({ medicineId: d.medicine_id, status, plannedFor: plannedToday(d.time), dose: d.dose });
    } catch (e) {
      reportError(`Could not record ${d.medicine_name}.`, e);
    }
    await load();
  }

  /** @param {any} d */
  async function undo(d) {
    if (!d.intake_id) return load();
    d.state = "due";
    draw();
    try {
      await api.undoIntake(d.intake_id);
    } catch (e) {
      reportError(`Could not undo ${d.medicine_name}.`, e);
    }
    await load();
  }

  void load();
  const stop = onTick(() => load());
  return () => {
    alive = false;
    stop();
  };
}
