//! Symptoms, categories and forms: uniqueness, case rules, CRUD errors.

use super::*;

#[test]
fn symptom_names_are_trimmed_lowercased_and_unique() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    symptoms::create(&tx, "  Nausea ", None, &[]).unwrap();
    assert_eq!(symptoms::list(&tx).unwrap()[0].name, "nausea");
    for dup in ["nausea", "NAUSEA", " Nausea "] {
        let e = symptoms::create(&tx, dup, None, &[]).unwrap_err();
        assert!(e.message.contains("already exists"), "{dup}: {}", e.message);
    }
    assert!(symptoms::create(&tx, "   ", None, &[]).is_err());
}

#[test]
fn renaming_a_symptom_onto_another_is_refused_but_onto_itself_is_fine() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let a = symptoms::create(&tx, "a", None, &[]).unwrap();
    symptoms::create(&tx, "b", None, &[]).unwrap();
    assert!(symptoms::update(&tx, a, "B", None).is_err());
    symptoms::update(&tx, a, "A", None).unwrap();
    assert_eq!(symptoms::list(&tx).unwrap()[0].name, "a");
    assert!(symptoms::update(&tx, 999, "z", None).is_err());
    assert!(symptoms::delete(&tx, 999).is_err());
}

#[test]
fn symptom_list_counts_linked_medicines_and_sorts_by_name() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let mut m = medicine("A", 1.0);
    m.symptoms = vec!["zeta".into(), "alpha".into()];
    medicines::insert(&tx, &m).unwrap();
    m.symptoms = vec!["alpha".into()];
    medicines::insert(&tx, &m).unwrap();
    let rows = symptoms::list(&tx).unwrap();
    let got: Vec<_> = rows
        .iter()
        .map(|s| (s.name.as_str(), s.medicines))
        .collect();
    assert_eq!(got, [("alpha", 2), ("zeta", 1)]);
}

#[test]
fn recreating_a_symptom_ignores_medicine_ids_that_no_longer_exist() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = symptoms::create(&tx, "x", None, &[12345]).unwrap();
    assert_eq!(symptoms::list(&tx).unwrap()[0].id, id);
    assert_eq!(symptoms::list(&tx).unwrap()[0].medicines, 0);
}

#[test]
fn category_names_are_unique_without_regard_to_case_and_trimmed() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    categories::create(&tx, "  Pain ", &[]).unwrap();
    assert_eq!(categories::list(&tx).unwrap()[0].name, "Pain");
    assert!(categories::create(&tx, "PAIN", &[]).is_err());
    assert!(categories::create(&tx, " ", &[]).is_err());
    let other = categories::create(&tx, "Sleep", &[]).unwrap();
    assert!(categories::rename(&tx, other, "pain").is_err());
    categories::rename(&tx, other, "SLEEP").unwrap();
    assert!(categories::rename(&tx, 999, "x").is_err());
    assert!(categories::delete(&tx, 999).is_err());
}

#[test]
fn category_list_counts_symptoms_and_sorts_without_case() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let s = symptoms::create(&tx, "s", None, &[]).unwrap();
    categories::create(&tx, "banana", &[]).unwrap();
    categories::create(&tx, "Apple", &[s]).unwrap();
    let got: Vec<_> = categories::list(&tx)
        .unwrap()
        .into_iter()
        .map(|c| (c.name, c.symptoms))
        .collect();
    assert_eq!(got, [("Apple".to_string(), 1), ("banana".to_string(), 0)]);
}

#[test]
fn the_seeded_forms_exist_and_new_ones_are_unique_without_case() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let before = forms::list(&tx).unwrap();
    assert!(
        before
            .iter()
            .any(|f| f.name == "Tablet" && f.unit == "tablets")
    );
    assert!(forms::create(&tx, "tablet", "x", &[]).is_err());
    assert!(forms::create(&tx, "  ", "x", &[]).is_err());
    forms::create(&tx, " Gel ", " g ", &[]).unwrap();
    let gel = forms::list(&tx)
        .unwrap()
        .into_iter()
        .find(|f| f.name == "Gel")
        .unwrap();
    assert_eq!(gel.unit, "g", "the unit is trimmed too");
    assert_eq!(forms::list(&tx).unwrap().len(), before.len() + 1);
}

#[test]
fn forms_update_conflicts_and_unknown_ids() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let all = forms::list(&tx).unwrap();
    let (a, b) = (&all[0], &all[1]);
    assert!(forms::update(&tx, a.id, &b.name.to_uppercase(), "u").is_err());
    forms::update(&tx, a.id, &a.name, "new unit").unwrap();
    assert!(forms::update(&tx, 9999, "x", "y").is_err());
    assert!(forms::delete(&tx, 9999).is_err());
}

#[test]
fn form_list_counts_only_live_medicines_by_form() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let tablet = forms::list(&tx)
        .unwrap()
        .into_iter()
        .find(|f| f.name == "Tablet")
        .unwrap();
    let mut input = medicine("A", 1.0);
    input.form_id = Some(tablet.id);
    medicines::insert(&tx, &input).unwrap();
    medicines::insert(&tx, &input).unwrap();
    let after = forms::list(&tx)
        .unwrap()
        .into_iter()
        .find(|f| f.name == "Tablet")
        .unwrap();
    assert_eq!(after.medicines, 2);
}
