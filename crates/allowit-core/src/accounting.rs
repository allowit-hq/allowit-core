//! A host's account of Core's standard figures between operations, in Core's UTC days.
//!
//! The portable rules count an executed operation (`post_flight`). A host that keeps a wallet's
//! figures also counts what happens after the operation: the network fee that a receipt shows,
//! and the release of the part of a fee bound that the operation did not pay. These functions are
//! that one definition, so no host decides a day boundary or a fee transition of its own.
use crate::{AllowitError, AllowitStandardState, DAY, Result};

/// The UTC day of `now` (Unix seconds), as the standard figures and the daily rules count it.
pub fn day(now: u64) -> u64 {
    now / DAY
}

/// The last second of the UTC day of `now`. An operation that a policy evaluates at `now` counts
/// in that day, so its validity ends at this second at the latest. In the last day that `u64`
/// holds, the answer is `u64::MAX`.
pub fn day_end(now: u64) -> u64 {
    day(now).saturating_mul(DAY).saturating_add(DAY - 1)
}

/// `state` after a fee change of `delta` units of the installed asset, for an operation of the
/// UTC day `day`. A positive `delta` is a network fee that a receipt shows. A negative `delta`
/// releases the unused part of a fee bound that the figures counted already.
///
/// The lifetime spending takes `delta`. A positive `delta` of a later day than the figures' day
/// starts that day, as `post_flight` does: the day's spending, volume and unpriced mark start at
/// zero. The figures of the operation's day then take `delta` in the day's spending and in the
/// day's volume, which counts gas. A fee of a day that is over counts in the lifetime spending
/// only, and a release never starts a day. A release takes no figure below zero. The calls and
/// the unpriced marks do not change otherwise. A sum that overflows is `Overflow`, and figures
/// that leave no room for one more unit, which `post_flight` would refuse at the next operation,
/// are `InvalidState`.
pub fn charge_fee(
    state: &AllowitStandardState,
    day: u64,
    delta: i128,
) -> Result<AllowitStandardState> {
    let mut s = state.clone();
    if delta == 0 {
        return Ok(s);
    }
    let add = |value: i128| -> Result<i128> {
        Ok(value
            .checked_add(delta)
            .ok_or(AllowitError::Overflow)?
            .max(0))
    };
    s.spent = add(s.spent)?;
    if s.day < day && delta > 0 {
        s.day = day;
        s.day_spent = 0;
        s.day_total = 0;
        s.day_unpriced = false;
    }
    if s.day == day {
        s.day_spent = add(s.day_spent)?;
        s.day_total = add(s.day_total)?;
    }
    if [s.spent, s.day_spent, s.day_total].contains(&i128::MAX) {
        return Err(AllowitError::InvalidState);
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn figures(day: u64, spent: i128, day_spent: i128, day_total: i128) -> AllowitStandardState {
        AllowitStandardState {
            calls: 4,
            spent,
            unpriced: true,
            day,
            day_spent,
            day_total,
            day_unpriced: true,
        }
    }

    /// A day runs from its first second to its last; the next second is the next day.
    #[test]
    fn a_day_ends_at_its_last_second() {
        assert_eq!((day(0), day_end(0)), (0, DAY - 1));
        assert_eq!((day(3 * DAY + 5), day_end(3 * DAY + 5)), (3, 4 * DAY - 1));
        assert_eq!(day(day_end(7 * DAY)), 7);
        assert_eq!(day(day_end(7 * DAY) + 1), 8);
        assert_eq!(day_end(u64::MAX), u64::MAX);
    }

    /// A fee of the figures' own day takes the day's spending and volume and the lifetime.
    #[test]
    fn a_fee_of_the_day_counts_in_the_day_and_the_lifetime() {
        let after = charge_fee(&figures(9, 100, 40, 70), 9, 15).unwrap();
        assert_eq!(after, figures(9, 115, 55, 85));
    }

    /// A fee of a later day starts that day: the day's spending, volume and unpriced mark start
    /// at zero before the fee counts. The calls and the lifetime mark stay.
    #[test]
    fn a_fee_of_a_later_day_starts_that_day() {
        let after = charge_fee(&figures(9, 100, 40, 70), 10, 15).unwrap();
        assert_eq!(
            after,
            AllowitStandardState {
                calls: 4,
                spent: 115,
                unpriced: true,
                day: 10,
                day_spent: 15,
                day_total: 15,
                day_unpriced: false,
            }
        );
    }

    /// A fee of a day that is over counts in the lifetime spending only.
    #[test]
    fn a_fee_of_an_earlier_day_counts_in_the_lifetime_only() {
        let after = charge_fee(&figures(9, 100, 40, 70), 8, 15).unwrap();
        assert_eq!(after, figures(9, 115, 40, 70));
    }

    /// A release takes no figure below zero and never starts a day.
    #[test]
    fn a_release_takes_no_figure_below_zero_and_starts_no_day() {
        let same = charge_fee(&figures(9, 100, 40, 70), 9, -50).unwrap();
        assert_eq!(same, figures(9, 50, 0, 20));
        let later = charge_fee(&figures(9, 30, 40, 70), 10, -50).unwrap();
        assert_eq!(later, figures(9, 0, 40, 70));
    }

    /// No change leaves the figures as they are, also on another day.
    #[test]
    fn no_change_leaves_the_figures() {
        let s = figures(9, 100, 40, 70);
        assert_eq!(charge_fee(&s, 12, 0).unwrap(), s);
    }

    /// A sum past the largest figure is refused, and so are figures that leave no room for the
    /// next operation, which `post_flight` would refuse.
    #[test]
    fn an_overflow_and_a_full_figure_are_refused() {
        let s = figures(9, i128::MAX, 0, 0);
        assert_eq!(charge_fee(&s, 9, 1), Err(AllowitError::Overflow));
        let s = figures(9, i128::MAX - 1, 0, 0);
        assert_eq!(charge_fee(&s, 9, 1), Err(AllowitError::InvalidState));
        assert_eq!(charge_fee(&s, 8, 1), Err(AllowitError::InvalidState));
        let s = figures(9, 0, i128::MAX - 1, 0);
        assert_eq!(charge_fee(&s, 9, 1), Err(AllowitError::InvalidState));
        assert!(charge_fee(&figures(9, i128::MAX - 2, 0, 0), 9, 1).is_ok());
    }
}
