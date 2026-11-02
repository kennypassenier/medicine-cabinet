//! Unit wording: plural rules and number formatting.

use super::*;

#[test]
fn fractions_print_without_trailing_zeros_and_stay_plural() {
    assert_eq!(amount(0.5, "tablets"), "0.5 tablets");
    assert_eq!(amount(1.5, "tablets"), "1.5 tablets");
    assert_eq!(amount(2.25, "ml"), "2.25 ml");
}

#[test]
fn zero_is_plural() {
    assert_eq!(amount(0.0, "tablets"), "0 tablets");
}

#[test]
fn whole_numbers_have_no_decimal_point() {
    assert_eq!(amount(10.0, "drops"), "10 drops");
}

#[test]
fn singular_forms_of_the_units_the_app_ships() {
    for (plural, one) in [
        ("tablets", "tablet"),
        ("capsules", "capsule"),
        ("drops", "drop"),
        ("puffs", "puff"),
        ("patches", "patch"),
        ("doses", "dose"),
    ] {
        assert_eq!(amount(1.0, plural), format!("1 {one}"));
    }
}

#[test]
fn unit_that_cannot_be_singularised_is_left_alone() {
    assert_eq!(amount(1.0, "g"), "1 g");
    assert_eq!(amount(1.0, "ml"), "1 ml");
    assert_eq!(amount(1.0, "ss"), "1 ss", "too short to strip an s");
}

#[test]
fn whitespace_only_or_padded_units_are_trimmed() {
    assert_eq!(amount(3.0, "   "), "3");
    assert_eq!(amount(1.0, "  capsules "), "1 capsule");
    assert_eq!(amount(2.0, " ml "), "2 ml");
}

#[test]
fn shes_plurals_lose_es() {
    assert_eq!(amount(1.0, "washes"), "1 wash");
}

#[test]
fn negative_one_is_not_singular() {
    assert_eq!(amount(-1.0, "tablets"), "-1 tablets");
}
