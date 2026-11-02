//! The usable / unavailable partition of a symptom search.

use super::*;
use crate::testkit::medicine;

fn with_symptoms(id: i64, name: &str, symptoms: &[&str]) -> Medicine {
    let mut m = medicine(id, name);
    m.symptoms = symptoms.iter().map(|s| s.to_string()).collect();
    m
}

fn names(list: &[Medicine]) -> Vec<&str> {
    list.iter().map(|m| m.name.as_str()).collect()
}

#[test]
fn a_healthy_medicine_is_usable() {
    assert!(is_usable(&medicine(1, "A")));
}

#[test]
fn each_disqualifier_makes_a_medicine_unavailable() {
    let mut off = medicine(1, "A");
    off.active = false;
    let mut out = medicine(2, "B");
    out.stock_level = StockLevel::Out;
    let mut expired = medicine(3, "C");
    expired.freshness = Freshness::Expired;
    let mut over = medicine(4, "D");
    over.course_over = true;
    for m in [off, out, expired, over] {
        assert!(!is_usable(&m), "{}", m.name);
    }
}

#[test]
fn low_stock_and_expiring_soon_are_still_usable() {
    let mut m = medicine(1, "A");
    m.stock_level = StockLevel::Low;
    m.freshness = Freshness::ExpiringSoon;
    assert!(is_usable(&m));
}

#[test]
fn matching_is_by_substring_and_partitions_by_availability() {
    let mut off = with_symptoms(2, "Switched off", &["headache"]);
    off.active = false;
    let mut out = with_symptoms(3, "Empty", &["head cold"]);
    out.stock_level = StockLevel::Out;
    let all = vec![
        with_symptoms(1, "Good", &["headache", "fever"]),
        off,
        out,
        with_symptoms(4, "Unrelated", &["cough"]),
        with_symptoms(5, "No symptoms", &[]),
    ];
    let (usable, unavailable) = partition_for(all, "head");
    assert_eq!(names(&usable), ["Good"]);
    assert_eq!(names(&unavailable), ["Switched off", "Empty"]);
}

#[test]
fn a_medicine_with_several_matching_symptoms_appears_once() {
    let all = vec![with_symptoms(1, "A", &["headache", "head cold"])];
    let (usable, unavailable) = partition_for(all, "head");
    assert_eq!(usable.len(), 1);
    assert!(unavailable.is_empty());
}

#[test]
fn nothing_matching_gives_two_empty_lists() {
    let (u, n) = partition_for(vec![with_symptoms(1, "A", &["cough"])], "fever");
    assert!(u.is_empty() && n.is_empty());
}

#[test]
fn the_original_order_is_kept_within_each_side() {
    let all = vec![
        with_symptoms(1, "B", &["pain"]),
        with_symptoms(2, "A", &["pain"]),
        with_symptoms(3, "C", &["pain"]),
    ];
    let (usable, _) = partition_for(all, "pain");
    assert_eq!(names(&usable), ["B", "A", "C"]);
}
