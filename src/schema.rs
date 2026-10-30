//! The app database schema, as numbered migrations (applied once, in order).

use chassis::storage::Migration;

const V1: &str = "
CREATE TABLE medicines (
    id               INTEGER PRIMARY KEY,
    name             TEXT NOT NULL,
    purpose          TEXT NOT NULL DEFAULT '',
    form             TEXT NOT NULL DEFAULT '',
    strength         TEXT NOT NULL DEFAULT '',
    notes            TEXT NOT NULL DEFAULT '',
    active           INTEGER NOT NULL DEFAULT 1,
    as_needed        INTEGER NOT NULL DEFAULT 0,
    min_gap_minutes  INTEGER,
    max_per_day      INTEGER,
    stock            REAL NOT NULL DEFAULT 0,
    stock_unit       TEXT NOT NULL DEFAULT '',
    low_stock_at     REAL,
    expires_on       TEXT,
    opened_on        TEXT,
    shelf_life_days  INTEGER,
    course_ends_on   TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE symptoms (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE
);
CREATE TABLE medicine_symptoms (
    medicine_id INTEGER NOT NULL REFERENCES medicines(id) ON DELETE CASCADE,
    symptom_id  INTEGER NOT NULL REFERENCES symptoms(id) ON DELETE CASCADE,
    PRIMARY KEY (medicine_id, symptom_id)
);
CREATE TABLE schedule_slots (
    id          INTEGER PRIMARY KEY,
    medicine_id INTEGER NOT NULL REFERENCES medicines(id) ON DELETE CASCADE,
    time_of_day TEXT NOT NULL,
    days        INTEGER NOT NULL DEFAULT 127,
    dose        REAL NOT NULL DEFAULT 1
);
CREATE TABLE intakes (
    id          INTEGER PRIMARY KEY,
    medicine_id INTEGER NOT NULL REFERENCES medicines(id) ON DELETE CASCADE,
    -- Local wall-clock times as the browser sent them: 'YYYY-MM-DDTHH:MM'.
    taken_at    TEXT NOT NULL,
    planned_for TEXT,
    status      TEXT NOT NULL CHECK (status IN ('taken','skipped')),
    dose        REAL NOT NULL DEFAULT 1,
    note        TEXT NOT NULL DEFAULT ''
);
CREATE INDEX intakes_by_medicine ON intakes (medicine_id, taken_at);
CREATE INDEX slots_by_medicine ON schedule_slots (medicine_id);
";

/// Symptoms become first-class: they get optional categories and are no
/// longer deleted when the last medicine stops using them.
const V2: &str = "
CREATE TABLE symptom_categories (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE COLLATE NOCASE
);
ALTER TABLE symptoms ADD COLUMN category_id INTEGER
    REFERENCES symptom_categories(id) ON DELETE SET NULL;
";

/// Medicine forms (tablet, syrup, ...) become a managed list with the unit
/// stock is counted in. The old free-text `medicines.form` column is kept
/// (SQLite cannot drop it cheaply) but no longer read; its values are
/// carried over into the new list.
const V3: &str = "
CREATE TABLE medicine_forms (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE COLLATE NOCASE,
    unit  TEXT NOT NULL DEFAULT ''
);
INSERT INTO medicine_forms (name, unit) VALUES
    ('Tablet', 'tablets'), ('Capsule', 'capsules'), ('Syrup', 'ml'),
    ('Drops', 'drops'), ('Cream', 'g'), ('Spray', 'doses'),
    ('Inhaler', 'puffs'), ('Patch', 'patches'), ('Injection', 'doses');
INSERT OR IGNORE INTO medicine_forms (name) SELECT DISTINCT form FROM medicines WHERE form <> '';
ALTER TABLE medicines ADD COLUMN form_id INTEGER
    REFERENCES medicine_forms(id) ON DELETE SET NULL;
UPDATE medicines SET form_id = (SELECT id FROM medicine_forms WHERE name = medicines.form)
    WHERE form <> '';
";

pub fn migrations() -> Vec<Migration> {
    vec![Migration::sql(V1), Migration::sql(V2), Migration::sql(V3)]
}
