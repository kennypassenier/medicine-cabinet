# Changelog

All notable changes to medicine-cabinet. Semantic versioning: a breaking change
in the API, the configuration knobs or the on-disk state is a major, and
carries a **Migration** section saying what an operator must do.

## [Unreleased]

## [1.0.0] - 2026-10-31

### Added
- Medicines with purpose, form, strength, notes, expiry and opened-on dates,
  a course end date, and stock with a low-stock threshold.
- Schedules (time, weekdays, dose) and as-needed medicines with a minimum gap
  and a daily maximum; switching a medicine off switches its schedule off.
- Today screen: next dose, take, skip and undo, attention band, as-needed
  quick row with a dosing guard.
- Symptoms with categories, search by symptom ("what do I have for a
  headache"), and CRUD for symptoms, categories and medicine forms.
- Intake history with undo that restores stock.
- Reminders through the kit notifier (`dose.due`, `dose.overdue`,
  `cabinet.attention`) with a sent-log so a restart repeats nothing;
  `MEDICINE_CABINET_TIMEZONE` (default UTC) and `MEDICINE_CABINET_DIGEST_HOUR`
  (default 9).
- Calendar sync with Almanac (optional: `MEDICINE_CABINET_ALMANAC_URL`,
  `MEDICINE_CABINET_ALMANAC_TOKEN`, `MEDICINE_CABINET_ALMANAC_SOURCE`): all-day
  events for an expiry, the last day of a course and a reorder date a week
  before the predicted run-out; only changes are sent, stale events deleted.
- Installable app icons (192, 512, maskable) for the PWA support of chassis.
- Built on chassis-rs 4.0.0 and kp-themes 12.0.0 (the single bundle, the kit's
  runtime and PWA support). Reminders reach kyu through the kit's shared
  sender: set `MEDICINE_CABINET_NOTIFY_URL` and `MEDICINE_CABINET_NOTIFY_TOKEN`.
- Project created with `chassis new`.
