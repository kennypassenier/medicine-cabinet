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

/**
 * Everything the page's functions share.
 * @typedef {{
 *   model: { today: any, meds: Map<number, any> } | null, loadedOnce: boolean, alive: boolean,
 *   ui: ReturnType<typeof buildLayout>
 * }} TodayPage
 */

/** Build the page skeleton into `root`. @param {HTMLElement} root @param {{ reload: () => void, band: () => void }} on */
function buildLayout(root, on) {
  const heading = pageHeader({ title: "Today", description: longDay() });
  const next = dataCard({ id: "next", title: "Next dose", description: "What to take now, or next.", className: "mc-today__next", onRetry: on.reload });
  const sched = dataCard({ id: "sched", title: "Schedule", description: "Every dose planned for today. Tap Take when you have taken it.", className: "mc-today__schedule", onRetry: on.reload });
  const need = dataCard({ id: "need", title: "When needed", description: "Medicines you take as symptoms call for them. The cabinet checks the safe gap first.", className: "mc-today__asneeded", onRetry: on.reload });
  const band = attentionBand(on.band);
  const grid = h("div", { class: "mc-today" }, next.card, band, sched.card, need.card);
  root.append(heading, grid);
  for (const c of [next, sched, need]) c.state("loading");
  return { heading, next, sched, need, band, grid };
}

/** Draw from the model (also used for the optimistic step before the server answers). @param {TodayPage} t */
function draw(t) {
  if (!t.model) return;
  const { today, meds } = t.model;
  const { heading, grid, next, sched, need, band } = t.ui;
  heading.querySelector(".kp-page-header__description")?.replaceChildren(`${longDay()} · updated ${nowParam().slice(11)}`);
  if (meds.size === 0) return firstRun(grid);
  const on = { take: (/** @type {any} */ d) => log(t, d, "taken"), skip: (/** @type {any} */ d) => log(t, d, "skipped"), undo: (/** @type {any} */ d) => undo(t, d) };
  redraw(grid, () => {
    renderAttention(band, today.attention, meds, () => load(t));
    renderNext(next.data, today.doses, meds, on);
    renderSchedule(sched.data, today.doses, meds, on);
    need.card.hidden = today.as_needed.length === 0;
    renderAsNeeded(need.data, today.as_needed.map((/** @type {any} */ m) => meds.get(m.id) ?? m), (m) => takeOne(m, () => load(t)));
  });
  next.state("ready");
  if (today.doses.length) sched.state("ready");
  else sched.state("empty", { title: "No doses planned today", text: "Medicines with a schedule show up here at their times. Switch one on in the cabinet, or add a schedule.", action: linkButton("Open the cabinet", "/cabinet") });
  need.state("ready");
}

/** The welcome of an empty cabinet. @param {HTMLElement} grid */
function firstRun(grid) {
  grid.replaceChildren(h("section", { class: "kp-card mc-today__first" },
    h("div", { class: "kp-empty" },
      h("h2", { class: "kp-empty__title" }, "Your cabinet is empty"),
      h("p", { class: "kp-empty__body" }, "Add the first medicine, with its schedule and how much you have. Today then tells you what to take and when, and warns before anything runs out or expires."),
      linkButton("Add your first medicine", "/medicine/new", "kp-button--primary kp-button--lg"))));
}

/** @param {TodayPage} t @param {boolean} [manual] */
async function load(t, manual = false) {
  const { next, sched, need } = t.ui;
  try {
    const [today, list] = await Promise.all([api.today(), api.medicines()]);
    if (!t.alive) return;
    t.model = { today, meds: new Map(list.map((/** @type {any} */ m) => [m.id, m])) };
    t.loadedOnce = true;
    draw(t);
  } catch (e) {
    if (!t.alive) return;
    if (!t.loadedOnce) for (const c of [next, sched, need]) c.state("error", { title: "Today could not be loaded", text: e instanceof Error ? e.message : String(e) });
    else if (manual || !document.hidden) reportError("Today could not be refreshed.", e);
  }
}

/** @param {TodayPage} t @param {any} d @param {"taken" | "skipped"} status */
async function log(t, d, status) {
  if (t.model) {
    d.state = status;
    draw(t);
  }
  try {
    await api.logIntake({ medicineId: d.medicine_id, status, plannedFor: plannedToday(d.time), dose: d.dose });
  } catch (e) {
    reportError(`Could not record ${d.medicine_name}.`, e);
  }
  await load(t);
}

/** @param {TodayPage} t @param {any} d */
async function undo(t, d) {
  if (!d.intake_id) return load(t);
  d.state = "due";
  draw(t);
  try {
    await api.undoIntake(d.intake_id);
  } catch (e) {
    reportError(`Could not undo ${d.medicine_name}.`, e);
  }
  await load(t);
}

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "Today · Medicine cabinet";
  /** @type {TodayPage} */
  const t = {
    model: null, loadedOnce: false, alive: true,
    ui: buildLayout(root, { reload: () => void load(t, true), band: () => draw(t) }),
  };
  void load(t);
  const stop = onTick(() => load(t));
  return () => {
    t.alive = false;
    stop();
  };
}
