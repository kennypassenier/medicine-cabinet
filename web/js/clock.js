// The browser's local clock, in the shapes the API speaks, and the friendly
// forms the screens show (dd/mm/yyyy, 24 h, "Today" / "Yesterday").

const pad = (/** @type {number} */ n) => String(n).padStart(2, "0");

/** @param {Date} d "YYYY-MM-DD" in local time */
export const dayParam = (d = new Date()) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/** @param {Date} d "YYYY-MM-DDTHH:MM" in local time: the one `now=` every time-dependent call takes */
export const nowParam = (d = new Date()) => `${dayParam(d)}T${pad(d.getHours())}:${pad(d.getMinutes())}`;

/** @param {string} iso "YYYY-MM-DD" @returns {Date} local midnight */
export function parseDay(iso) {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, m - 1, d);
}

/** Whole days from today to `iso` (negative: in the past). @param {string} iso */
export function daysFromToday(iso) {
  return Math.round((parseDay(iso).getTime() - parseDay(dayParam()).getTime()) / 86_400_000);
}

/** "30/10/2026" @param {string} iso */
export function formatDate(iso) {
  const [y, m, d] = iso.slice(0, 10).split("-");
  return `${d}/${m}/${y}`;
}

/** "Today", "Tomorrow", "Yesterday", else dd/mm/yyyy. @param {string} iso */
export function friendlyDate(iso) {
  const n = daysFromToday(iso.slice(0, 10));
  if (n === 0) return "Today";
  if (n === 1) return "Tomorrow";
  if (n === -1) return "Yesterday";
  return formatDate(iso);
}

/** "Friday 30/10/2026" @param {Date} d */
export const longDay = (d = new Date()) => `${d.toLocaleDateString("en-GB", { weekday: "long" })} ${formatDate(dayParam(d))}`;

/** "Today 16:00" / "Yesterday 08:05" / "28/10/2026 08:05" @param {string} stamp "YYYY-MM-DDTHH:MM" */
export const friendlyStamp = (stamp) => `${friendlyDate(stamp)} ${stamp.slice(11, 16)}`;

/**
 * A duration in minutes as words: "25 min", "2 h 10 min", "1 day 3 h".
 * Minutes up to 60, hours past that (Kenny's duration rule).
 * @param {number} minutes
 */
export function duration(minutes) {
  const m = Math.max(0, Math.round(minutes));
  if (m <= 60) return `${m} min`;
  if (m < 1440) return m % 60 ? `${Math.floor(m / 60)} h ${m % 60} min` : `${m / 60} h`;
  const days = Math.floor(m / 1440);
  const rest = Math.floor((m % 1440) / 60);
  return `${days} ${days === 1 ? "day" : "days"}${rest ? ` ${rest} h` : ""}`;
}

/** "in 25 min" / "25 min ago". @param {number} minutesUntil */
export const relative = (minutesUntil) => (minutesUntil >= 0 ? `in ${duration(minutesUntil)}` : `${duration(-minutesUntil)} ago`);

/** Weekday letters, Monday first: bit 0 = Monday ... bit 6 = Sunday. */
export const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/** "Every day" / "Weekdays" / "Mon, Thu" for a slot's day bitmask. @param {number} mask */
export function daysLabel(mask) {
  if (mask === 127) return "Every day";
  if (mask === 31) return "Weekdays";
  if (mask === 96) return "Weekend";
  return WEEKDAYS.filter((_, i) => mask & (1 << i)).join(", ");
}
