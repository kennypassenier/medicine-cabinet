//! Exact amounts. Stock and doses are stored and computed as whole
//! hundredths (an `i64`), because `f64` accumulates error
//! (`1.0 - 0.1 - 0.1 - 0.1` is `0.7000000000000001`). Decimals exist only at
//! the edge: the JSON API and the pages still send and receive plain numbers.

use chassis::Error;

use super::limits;

/// Hundredths in one unit: the precision stock and doses are kept with.
pub const SCALE: i64 = 100;

/// A decimal amount as whole hundredths, rounded half away from zero
/// (0.005 becomes 1). Callers validate finiteness and range first
/// ([`checked`]); a non-finite value maps to 0 rather than to garbage.
pub fn to_hundredths(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    (value * SCALE as f64).round() as i64
}

/// Whole hundredths back to the decimal the API shows (`70` is `0.7`).
pub fn to_decimal(hundredths: i64) -> f64 {
    hundredths as f64 / SCALE as f64
}

/// The largest stock, dose or amount, in hundredths.
pub fn max_hundredths() -> i64 {
    to_hundredths(limits::AMOUNT_MAX)
}

/// Validate a typed amount (see [`limits::amount`]) and convert it. A
/// `positive` amount that would round down to nothing is refused, so a dose
/// of 0.001 never becomes a dose that takes nothing.
pub fn checked(field: &str, value: f64, positive: bool) -> Result<i64, Error> {
    let hundredths = to_hundredths(limits::amount(field, value, positive)?);
    if positive && hundredths == 0 {
        return Err(Error::invalid(
            format!("{field} is smaller than 0.01"),
            "enter an amount of 0.01 or more",
        ));
    }
    Ok(hundredths)
}

/// [`checked`] for a signed change (a stock correction), clamped to the same range.
pub fn checked_signed(field: &str, value: f64) -> Result<i64, Error> {
    Ok(to_hundredths(limits::signed_amount(field, value)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_round_trip_at_two_places() {
        for v in [0.0, 0.01, 0.1, 0.25, 0.7, 1.0, 2.5, 123.45, 1_000_000.0] {
            assert_eq!(to_decimal(to_hundredths(v)), v, "{v}");
        }
    }

    #[test]
    fn point_one_plus_point_two_is_exactly_point_three() {
        assert_ne!(0.1 + 0.2, 0.3, "floats drift");
        assert_eq!(to_hundredths(0.1) + to_hundredths(0.2), to_hundredths(0.3));
        assert_eq!(to_decimal(to_hundredths(0.1) + to_hundredths(0.2)), 0.3);
    }

    #[test]
    fn three_tenths_from_one_leave_exactly_seventy_hundredths() {
        let drifted = 1.0 - 0.1 - 0.1 - 0.1;
        assert_ne!(drifted, 0.7, "floats drift");
        let mut stock = to_hundredths(1.0);
        for _ in 0..3 {
            stock -= to_hundredths(0.1);
        }
        assert_eq!(stock, 70);
        assert_eq!(to_decimal(stock), 0.7);
    }

    #[test]
    fn rounding_is_half_away_from_zero() {
        assert_eq!(to_hundredths(0.005), 1);
        assert_eq!(to_hundredths(0.004), 0);
        assert_eq!(to_hundredths(-0.005), -1);
        assert_eq!(to_hundredths(2.675), 268);
    }

    #[test]
    fn a_non_finite_value_is_zero_not_garbage() {
        assert_eq!(to_hundredths(f64::NAN), 0);
        assert_eq!(to_hundredths(f64::INFINITY), 0);
    }

    #[test]
    fn checked_refuses_bad_and_vanishing_amounts() {
        assert_eq!(checked("dose", 0.25, true).unwrap(), 25);
        assert_eq!(checked("stock", 0.0, false).unwrap(), 0);
        assert!(checked("dose", 0.001, true).is_err(), "rounds to nothing");
        assert!(checked("dose", 0.0, true).is_err());
        assert!(checked("stock", f64::NAN, false).is_err());
        assert!(checked("stock", 1e300, false).is_err());
    }

    #[test]
    fn signed_changes_keep_their_sign_and_range() {
        assert_eq!(checked_signed("change", -1.5).unwrap(), -150);
        assert!(checked_signed("change", 1e300).is_err());
        assert_eq!(max_hundredths(), 100_000_000);
    }
}
