// Quick stock adjust in the cabinet: the stepper shows its number at once,
// and one delta is saved a moment after the last tap.

import { api } from "../api.js";
import { reportError } from "../ui.js";

const STOCK_DEBOUNCE_MS = 700;

/**
 * @param {{ reload: () => Promise<void>, onSaved: (idle: boolean) => void }} on
 *   `reload` re-reads the cabinet after a failed save; `onSaved` runs after a
 *   saved change, `idle` true when no other medicine is still waiting to save
 */
export function createStockEditor({ reload, onSaved }) {
  /** @type {Map<number, { timer: number, base: number, value: number }>} */
  const pending = new Map();

  /** @param {any} m @param {{ base: number, value: number }} p */
  async function save(m, p) {
    pending.delete(m.id);
    const delta = Math.round((p.value - p.base) * 100) / 100;
    if (delta === 0) return;
    try {
      Object.assign(m, await api.addStock(m.id, delta));
    } catch (e) {
      reportError(`Could not change the stock of ${m.name}.`, e);
      await reload();
      return;
    }
    onSaved(!pending.size);
  }

  return {
    /** Show the number at once, save one delta after the last tap. @param {any} m @param {number} v */
    step(m, v) {
      const p = pending.get(m.id) ?? { timer: 0, base: m.stock, value: v };
      clearTimeout(p.timer);
      p.value = v;
      p.timer = window.setTimeout(() => save(m, p), STOCK_DEBOUNCE_MS);
      pending.set(m.id, p);
    },
    /** True while a stock edit waits to be saved (the page must not refresh over it). */
    busy: () => pending.size > 0,
    /** Drop the waiting saves. */
    stop() {
      for (const p of pending.values()) clearTimeout(p.timer);
    },
  };
}
