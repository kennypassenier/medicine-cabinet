//! The `If-Match` rule for medicines. A medicine has two revisions: `rev`
//! (every change, stock included, so devices re-pull) and `desc_rev` (the
//! `rev` at the last change of anything but the stock quantity). A client
//! that read the medicine at a revision in `desc_rev..=rev` holds the same
//! descriptive content, so its edit is not stale just because a dose was
//! logged since.

use chassis::core::revision::{self, Precondition, Stale};

/// Whether a write naming `want` may go ahead on a medicine at `rev` /
/// `desc_rev`. Anything outside the range (older than the last descriptive
/// change, or from the future) is stale and reports the current `rev`.
pub fn check_descriptive(want: Option<Precondition>, rev: u64, desc_rev: u64) -> Result<(), Stale> {
    match want {
        Some(Precondition::Rev(n)) if (desc_rev..=rev).contains(&n) => Ok(()),
        other => revision::check(other, rev),
    }
}

/// Whether the client's copy still has the medicine's current stock. A
/// request that names an older revision has a stale stock figure, which an
/// edit must not write back over the doses logged since.
pub fn stock_is_current(want: Option<Precondition>, rev: u64) -> bool {
    !matches!(want, Some(Precondition::Rev(n)) if n < rev)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rev(n: u64) -> Option<Precondition> {
        Some(Precondition::Rev(n))
    }

    #[test]
    fn the_exact_revision_passes() {
        assert!(check_descriptive(rev(7), 7, 7).is_ok());
    }

    #[test]
    fn a_revision_raised_by_stock_alone_passes() {
        // Descriptive content last changed at 4; doses took it to 9.
        for n in 4..=9 {
            assert!(check_descriptive(rev(n), 9, 4).is_ok(), "rev {n}");
        }
    }

    #[test]
    fn older_than_the_last_descriptive_change_is_stale() {
        let stale = check_descriptive(rev(3), 9, 4).unwrap_err();
        assert_eq!(stale.current, 9);
    }

    #[test]
    fn a_revision_from_the_future_is_stale() {
        assert_eq!(check_descriptive(rev(10), 9, 4).unwrap_err().current, 9);
    }

    #[test]
    fn no_header_and_star_always_pass() {
        assert!(check_descriptive(None, 9, 9).is_ok());
        assert!(check_descriptive(Some(Precondition::Any), 9, 9).is_ok());
    }

    #[test]
    fn stock_is_current_only_at_the_latest_revision() {
        assert!(stock_is_current(rev(9), 9));
        assert!(!stock_is_current(rev(8), 9));
        assert!(stock_is_current(None, 9));
        assert!(stock_is_current(Some(Precondition::Any), 9));
    }
}
