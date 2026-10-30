// The transport layer: one place that talks to /api/mc. Every call that
// depends on the time carries the browser's local clock (`now=`); the server
// is the only source of truth, nothing is kept in browser storage.

import { nowParam, dayParam } from "./clock.js";

const BASE = "/api/mc";

/** An API failure with the server's own sentence and its remedy. */
export class ApiError extends Error {
  /** @param {string} message @param {string} remedy @param {number} status */
  constructor(message, remedy, status) {
    super(message);
    this.name = "ApiError";
    this.remedy = remedy;
    this.status = status;
  }
}

/**
 * @param {string} method
 * @param {string} path
 * @param {{ query?: Record<string, string | number | undefined>, body?: unknown, timed?: boolean }} [opts]
 */
async function call(method, path, { query = {}, body, timed = false } = {}) {
  const params = new URLSearchParams();
  if (timed) params.set("now", nowParam());
  for (const [k, v] of Object.entries(query)) if (v !== undefined && v !== "") params.set(k, String(v));
  const qs = params.toString();
  let res;
  try {
    res = await fetch(`${BASE}${path}${qs ? `?${qs}` : ""}`, {
      method,
      credentials: "same-origin",
      headers: body === undefined ? {} : { "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new ApiError("The server could not be reached.", "Check the connection and try again.", 0);
  }
  if (res.redirected && res.url.includes("/login")) {
    location.assign("/login");
    throw new ApiError("Your session has ended.", "Log in again.", 401);
  }
  if (!res.ok) throw await failure(res);
  return res.json();
}

/** @param {Response} res */
async function failure(res) {
  const text = await res.text();
  try {
    const j = JSON.parse(text);
    if (j && typeof j.error === "string") return new ApiError(j.error, j.remedy ?? "", res.status);
  } catch {
    // Not JSON: the framework's plain-text refusal, shown as it is.
  }
  return new ApiError(text.trim() || `The server answered ${res.status}.`, "Check the entries and try again.", res.status);
}

export const api = {
  medicines: () => call("GET", "/medicines", { timed: true }),
  medicine: (/** @type {number} */ id) => call("GET", `/medicines/${id}`, { timed: true }),
  create: (/** @type {object} */ body) => call("POST", "/medicines", { timed: true, body }),
  update: (/** @type {number} */ id, /** @type {object} */ body) => call("PUT", `/medicines/${id}`, { timed: true, body }),
  remove: (/** @type {number} */ id) => call("DELETE", `/medicines/${id}`),
  setActive: (/** @type {number} */ id, /** @type {boolean} */ active) => call("POST", `/medicines/${id}/active`, { timed: true, body: { active } }),
  addStock: (/** @type {number} */ id, /** @type {number} */ delta) => call("POST", `/medicines/${id}/stock`, { timed: true, body: { delta } }),
  doseCheck: (/** @type {number} */ id) => call("GET", `/medicines/${id}/dose-check`, { timed: true }),
  today: () => call("GET", "/today", { timed: true }),
  search: (/** @type {string} */ symptom) => call("GET", "/search", { timed: true, query: { symptom } }),
  symptoms: () => call("GET", "/symptoms"),
  addSymptom: (/** @type {{ name: string, category_id: number | null }} */ b) => call("POST", "/symptoms", { body: b }),
  editSymptom: (/** @type {number} */ id, /** @type {{ name: string, category_id: number | null }} */ b) => call("PUT", `/symptoms/${id}`, { body: b }),
  removeSymptom: (/** @type {number} */ id) => call("DELETE", `/symptoms/${id}`),
  categories: () => call("GET", "/categories"),
  addCategory: (/** @type {string} */ name) => call("POST", "/categories", { body: { name } }),
  editCategory: (/** @type {number} */ id, /** @type {string} */ name) => call("PUT", `/categories/${id}`, { body: { name } }),
  removeCategory: (/** @type {number} */ id) => call("DELETE", `/categories/${id}`),
  forms: () => call("GET", "/forms"),
  addForm: (/** @type {{ name: string, unit: string }} */ b) => call("POST", "/forms", { body: b }),
  editForm: (/** @type {number} */ id, /** @type {{ name: string, unit: string }} */ b) => call("PUT", `/forms/${id}`, { body: b }),
  removeForm: (/** @type {number} */ id) => call("DELETE", `/forms/${id}`),
  intakes: (/** @type {string | undefined} */ day, limit = 200) => call("GET", "/intakes", { query: { day, limit } }),
  /**
   * Record an intake now (or at `takenAt`). `plannedFor` ties it to a schedule slot.
   * @param {{ medicineId: number, status: "taken" | "skipped", plannedFor?: string | null, dose?: number, note?: string, takenAt?: string }} i
   */
  logIntake: (i) =>
    call("POST", "/intakes", {
      body: { medicine_id: i.medicineId, taken_at: i.takenAt ?? nowParam(), planned_for: i.plannedFor ?? null, status: i.status, dose: i.dose, note: i.note },
    }),
  undoIntake: (/** @type {number} */ id) => call("DELETE", `/intakes/${id}`),
};

/** The planned stamp of a schedule slot today: "YYYY-MM-DDTHH:MM". @param {string} time "HH:MM" */
export const plannedToday = (time) => `${dayParam()}T${time}`;
