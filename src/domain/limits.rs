//! Bounds on what a person may type, so one bad request cannot fill the
//! database, break date arithmetic or put a NaN in a stock figure.

use chassis::Error;

/// Longest names and free texts, in characters.
pub const NAME_MAX: usize = 120;
pub const UNIT_MAX: usize = 30;
pub const TEXT_MAX: usize = 2000;
pub const NOTE_MAX: usize = 500;
/// Largest stock, dose or amount; far beyond any real cabinet.
pub const AMOUNT_MAX: f64 = 1_000_000.0;
/// Most symptoms on one medicine, slots on one medicine, ids in one relink.
pub const LIST_MAX: usize = 100;

/// Trim `value` and enforce its length. `single_line` refuses line breaks
/// and other control characters (names), free texts may keep newlines.
pub fn text(field: &str, value: &str, max: usize, single_line: bool) -> Result<String, Error> {
    let value = value.trim();
    let bad_char = |c: char| c.is_control() && !(c == '\n' && !single_line);
    if value.chars().any(bad_char) {
        return Err(Error::invalid(
            format!("{field} contains a control character or line break"),
            "type plain text only",
        ));
    }
    if value.chars().count() > max {
        return Err(Error::invalid(
            format!("{field} is longer than {max} characters"),
            format!("shorten {field} to {max} characters or fewer"),
        ));
    }
    Ok(value.to_string())
}

/// A finite amount in `0..=AMOUNT_MAX` (or `>0` when `positive`).
pub fn amount(field: &str, value: f64, positive: bool) -> Result<f64, Error> {
    let ok =
        value.is_finite() && value <= AMOUNT_MAX && (value > 0.0 || (!positive && value == 0.0));
    if ok {
        return Ok(value);
    }
    let what = if positive { "more than 0" } else { "0 or more" };
    Err(Error::invalid(
        format!("{field} must be a number {what} and at most {AMOUNT_MAX}"),
        format!("enter a number from {what} up to {AMOUNT_MAX}"),
    ))
}

/// A finite change in `-AMOUNT_MAX..=AMOUNT_MAX`.
pub fn signed_amount(field: &str, value: f64) -> Result<f64, Error> {
    if value.is_finite() && value.abs() <= AMOUNT_MAX {
        return Ok(value);
    }
    Err(Error::invalid(
        format!("{field} must be a number between -{AMOUNT_MAX} and {AMOUNT_MAX}"),
        "enter a normal amount",
    ))
}

/// A whole number within `min..=max`.
pub fn whole(field: &str, value: u32, min: u32, max: u32) -> Result<u32, Error> {
    if (min..=max).contains(&value) {
        return Ok(value);
    }
    Err(Error::invalid(
        format!("{field} must be between {min} and {max}"),
        format!("enter a whole number from {min} to {max}"),
    ))
}

/// Refuse a list longer than [`LIST_MAX`].
pub fn list_len(field: &str, len: usize) -> Result<(), Error> {
    if len > LIST_MAX {
        return Err(Error::invalid(
            format!("{field} has {len} entries; at most {LIST_MAX} are allowed"),
            "remove entries until the list is shorter",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_trimmed_bounded_and_single_line() {
        assert_eq!(text("name", "  Café  ", 10, true).unwrap(), "Café");
        assert!(text("name", &"é".repeat(11), 10, true).is_err());
        assert!(text("name", "a\nb", 10, true).is_err());
        assert!(text("notes", "a\nb", 10, false).is_ok());
        assert!(text("name", "a\u{0}b", 10, false).is_err());
    }

    #[test]
    fn amounts_refuse_nan_infinity_negative_and_huge() {
        for bad in [f64::NAN, f64::INFINITY, -1.0, 1e300] {
            assert!(amount("stock", bad, false).is_err());
        }
        assert!(amount("stock", 0.0, false).is_ok());
        assert!(amount("dose", 0.0, true).is_err());
        assert!(amount("dose", 0.25, true).is_ok());
    }

    #[test]
    fn whole_numbers_stay_in_range() {
        assert!(whole("gap", 0, 1, 10).is_err());
        assert!(whole("gap", 11, 1, 10).is_err());
        assert_eq!(whole("gap", 5, 1, 10).unwrap(), 5);
    }
}
