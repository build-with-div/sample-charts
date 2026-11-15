//! The local-time intervals and the ticker of d3-time 3.1.1
//! (`src/interval.js`, `millisecond.js`, `second.js`, `minute.js`,
//! `hour.js`, `day.js`, `week.js`, `month.js`, `year.js`, `ticks.js`): what
//! `d3.scaleTime` ticks with.
//!
//! Dates are `js::Date`s, so local time is UTC (see `js::Date`). Times are
//! passed and returned as time values (`+date`; NaN for an Invalid Date).
//! The UTC intervals (`scaleUtc`) are not ported.
//!
//! Where a filtered interval (`interval.every(step)`) meets an Invalid
//! Date while stepping, d3 loops forever, since the date never passes the
//! filter; the port stops there instead.

use js::Date;

use crate::array::tick_step;

const DURATION_SECOND: f64 = 1000.0;
const DURATION_MINUTE: f64 = DURATION_SECOND * 60.0;
const DURATION_HOUR: f64 = DURATION_MINUTE * 60.0;
const DURATION_DAY: f64 = DURATION_HOUR * 24.0;
const DURATION_WEEK: f64 = DURATION_DAY * 7.0;
const DURATION_MONTH: f64 = DURATION_DAY * 30.0;
const DURATION_YEAR: f64 = DURATION_DAY * 365.0;

/// The local intervals: `timeMillisecond`, `timeSecond`, …, `timeYear`
/// (`timeWeek` is `timeSunday`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

impl Unit {
    /// The interval's `floori`.
    fn floor(self, date: &mut Date) {
        match self {
            Unit::Millisecond => {}
            Unit::Second => date.set_time(date.time() - date.get_milliseconds()),
            Unit::Minute => date.set_time(
                date.time() - date.get_milliseconds() - date.get_seconds() * DURATION_SECOND,
            ),
            Unit::Hour => date.set_time(
                date.time()
                    - date.get_milliseconds()
                    - date.get_seconds() * DURATION_SECOND
                    - date.get_minutes() * DURATION_MINUTE,
            ),
            Unit::Day => date.set_hours(0.0, Some(0.0), Some(0.0), Some(0.0)),
            Unit::Week => {
                // timeWeekday(0): `(date.getDay() + 7 - i) % 7`.
                date.set_date(date.get_date() - (date.get_day() + 7.0) % 7.0);
                date.set_hours(0.0, Some(0.0), Some(0.0), Some(0.0));
            }
            Unit::Month => {
                date.set_date(1.0);
                date.set_hours(0.0, Some(0.0), Some(0.0), Some(0.0));
            }
            Unit::Year => {
                date.set_month(0.0, Some(1.0));
                date.set_hours(0.0, Some(0.0), Some(0.0), Some(0.0));
            }
        }
    }

    /// The interval's `offseti`.
    fn offset(self, date: &mut Date, step: f64) {
        match self {
            Unit::Millisecond => date.set_time(date.time() + step),
            Unit::Second => date.set_time(date.time() + step * DURATION_SECOND),
            Unit::Minute => date.set_time(date.time() + step * DURATION_MINUTE),
            Unit::Hour => date.set_time(date.time() + step * DURATION_HOUR),
            Unit::Day => date.set_date(date.get_date() + step),
            Unit::Week => date.set_date(date.get_date() + step * 7.0),
            Unit::Month => date.set_month(date.get_month() + step, None),
            Unit::Year => date.set_full_year(date.get_full_year() + step, None, None),
        }
    }

    /// The interval's `count(start, end)`, given floored dates.
    fn count(self, start: &Date, end: &Date) -> f64 {
        let (s, e) = (start.time(), end.time());
        let zone = (end.get_timezone_offset() - start.get_timezone_offset()) * DURATION_MINUTE;
        match self {
            Unit::Millisecond => e - s,
            Unit::Second => (e - s) / DURATION_SECOND,
            Unit::Minute => (e - s) / DURATION_MINUTE,
            Unit::Hour => (e - s) / DURATION_HOUR,
            Unit::Day => (e - s - zone) / DURATION_DAY,
            Unit::Week => (e - s - zone) / DURATION_WEEK,
            Unit::Month => {
                end.get_month() - start.get_month()
                    + (end.get_full_year() - start.get_full_year()) * 12.0
            }
            Unit::Year => end.get_full_year() - start.get_full_year(),
        }
    }

    /// The interval's `field`, which `every` filters on; the week has none.
    fn field(self, date: &Date) -> Option<f64> {
        match self {
            Unit::Millisecond => None,
            Unit::Second => Some(date.get_utc_seconds()),
            Unit::Minute => Some(date.get_minutes()),
            Unit::Hour => Some(date.get_hours()),
            Unit::Day => Some(date.get_date() - 1.0),
            Unit::Week => None,
            Unit::Month => Some(date.get_month()),
            Unit::Year => Some(date.get_full_year()),
        }
    }

    /// `interval.count(start, end)`.
    pub fn count_between(self, start: f64, end: f64) -> f64 {
        let (mut t0, mut t1) = (Date::new(start), Date::new(end));
        self.floor(&mut t0);
        self.floor(&mut t1);
        self.count(&t0, &t1).floor()
    }

    /// `interval.every(step)`: `None` (null) unless the floored step is a
    /// positive finite number.
    pub fn every(self, step: f64) -> Option<Interval> {
        let step = step.floor();
        if !step.is_finite() || !(step > 0.0) {
            return None;
        }
        // millisecond.every and timeYear.every are their own intervals;
        // timeYear.every(1) too.
        if self == Unit::Year {
            return Some(Interval::YearEvery(step));
        }
        if !(step > 1.0) {
            return Some(Interval::Unit(self));
        }
        Some(match self {
            Unit::Millisecond => Interval::MillisecondEvery(step),
            _ => Interval::Filtered(self, step),
        })
    }
}

/// A d3-time interval: a unit, or what `every` returns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interval {
    Unit(Unit),
    /// `millisecond.every(k)` for k > 1.
    MillisecondEvery(f64),
    /// `timeYear.every(k)`.
    YearEvery(f64),
    /// `interval.filter(test)` from `every(step)`: dates whose field (or
    /// count from the epoch) is a multiple of `step`.
    Filtered(Unit, f64),
}

impl Interval {
    /// The test of a filtered interval.
    fn test(unit: Unit, step: f64, date: &Date) -> bool {
        let n = unit
            .field(date)
            .unwrap_or_else(|| unit.count_between(0.0, date.time()));
        n % step == 0.0
    }

    fn floori(self, date: &mut Date) {
        match self {
            Interval::Unit(unit) => unit.floor(date),
            Interval::MillisecondEvery(k) => date.set_time((date.time() / k).floor() * k),
            Interval::YearEvery(k) => {
                date.set_full_year((date.get_full_year() / k).floor() * k, None, None);
                date.set_month(0.0, Some(1.0));
                date.set_hours(0.0, Some(0.0), Some(0.0), Some(0.0));
            }
            Interval::Filtered(unit, step) => {
                // `if (date >= date) while (floori(date), !test(date))
                // date.setTime(date - 1);`
                if date.time().is_nan() {
                    return;
                }
                loop {
                    unit.floor(date);
                    if Self::test(unit, step, date) || date.time().is_nan() {
                        break;
                    }
                    date.set_time(date.time() - 1.0);
                }
            }
        }
    }

    fn offseti(self, date: &mut Date, step: f64) {
        match self {
            Interval::Unit(unit) => unit.offset(date, step),
            Interval::MillisecondEvery(k) => date.set_time(date.time() + step * k),
            Interval::YearEvery(k) => {
                date.set_full_year(date.get_full_year() + step * k, None, None)
            }
            Interval::Filtered(unit, every) => {
                if date.time().is_nan() {
                    return;
                }
                let (direction, mut step) = if step < 0.0 {
                    (-1.0, step)
                } else {
                    (1.0, step)
                };
                // `while (++step <= 0)` or `while (--step >= 0)`, each
                // time stepping until the test passes.
                loop {
                    if direction < 0.0 {
                        step += 1.0;
                        if !(step <= 0.0) {
                            break;
                        }
                    } else {
                        step -= 1.0;
                        if !(step >= 0.0) {
                            break;
                        }
                    }
                    loop {
                        unit.offset(date, direction);
                        if Self::test(unit, every, date) || date.time().is_nan() {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// `interval.floor(date)`.
    pub fn floor(self, time: f64) -> f64 {
        let mut date = Date::new(time);
        self.floori(&mut date);
        date.time()
    }

    /// `interval.ceil(date)`.
    pub fn ceil(self, time: f64) -> f64 {
        let mut date = Date::new(time - 1.0);
        self.floori(&mut date);
        self.offseti(&mut date, 1.0);
        self.floori(&mut date);
        date.time()
    }

    /// `interval.offset(date, step)`.
    pub fn offset(self, time: f64, step: f64) -> f64 {
        let mut date = Date::new(time);
        self.offseti(&mut date, step.floor());
        date.time()
    }

    /// `interval.range(start, stop)` (step 1): the interval's dates in
    /// [ceil(start), stop).
    pub fn range(self, start: f64, stop: f64) -> Vec<f64> {
        let mut range = Vec::new();
        let mut start = Date::new(self.ceil(start));
        if !(start.time() < stop) {
            return range;
        }
        loop {
            let previous = start.time();
            range.push(Date::new(previous).time());
            self.offseti(&mut start, 1.0);
            self.floori(&mut start);
            if !(previous < start.time() && start.time() < stop) {
                return range;
            }
        }
    }
}

/// The ticker's intervals, with their approximate durations.
const TICK_INTERVALS: [(Unit, f64, f64); 18] = [
    (Unit::Second, 1.0, DURATION_SECOND),
    (Unit::Second, 5.0, 5.0 * DURATION_SECOND),
    (Unit::Second, 15.0, 15.0 * DURATION_SECOND),
    (Unit::Second, 30.0, 30.0 * DURATION_SECOND),
    (Unit::Minute, 1.0, DURATION_MINUTE),
    (Unit::Minute, 5.0, 5.0 * DURATION_MINUTE),
    (Unit::Minute, 15.0, 15.0 * DURATION_MINUTE),
    (Unit::Minute, 30.0, 30.0 * DURATION_MINUTE),
    (Unit::Hour, 1.0, DURATION_HOUR),
    (Unit::Hour, 3.0, 3.0 * DURATION_HOUR),
    (Unit::Hour, 6.0, 6.0 * DURATION_HOUR),
    (Unit::Hour, 12.0, 12.0 * DURATION_HOUR),
    (Unit::Day, 1.0, DURATION_DAY),
    (Unit::Day, 2.0, 2.0 * DURATION_DAY),
    (Unit::Week, 1.0, DURATION_WEEK),
    (Unit::Month, 1.0, DURATION_MONTH),
    (Unit::Month, 3.0, 3.0 * DURATION_MONTH),
    (Unit::Year, 1.0, DURATION_YEAR),
];

/// `timeTickInterval(start, stop, count)`.
pub fn time_tick_interval(start: f64, stop: f64, count: f64) -> Option<Interval> {
    let target = (stop - start).abs() / count;
    // bisector(([,, step]) => step).right(tickIntervals, target): the
    // length when target is NaN (not self-comparable).
    let i = if target.is_nan() {
        TICK_INTERVALS.len()
    } else {
        TICK_INTERVALS.partition_point(|&(_, _, step)| step <= target)
    };
    if i == TICK_INTERVALS.len() {
        return Unit::Year.every(tick_step(
            start / DURATION_YEAR,
            stop / DURATION_YEAR,
            count,
        ));
    }
    if i == 0 {
        return Unit::Millisecond.every(js::math::max(tick_step(start, stop, count), 1.0));
    }
    let (unit, step, _) = if target / TICK_INTERVALS[i - 1].2 < TICK_INTERVALS[i].2 / target {
        TICK_INTERVALS[i - 1]
    } else {
        TICK_INTERVALS[i]
    };
    unit.every(step)
}

/// `timeTicks(start, stop, count)` for a number count: the dates of the
/// chosen interval from start to stop inclusive, reversed when stop comes
/// first.
pub fn time_ticks(start: f64, stop: f64, count: f64) -> Vec<f64> {
    let reverse = stop < start;
    let (start, stop) = if reverse {
        (stop, start)
    } else {
        (start, stop)
    };
    let mut ticks = time_tick_interval(start, stop, count)
        .map_or_else(Vec::new, |interval| interval.range(start, stop + 1.0));
    if reverse {
        ticks.reverse();
    }
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

    const Y2000: f64 = 946_684_800_000.0;

    #[test]
    fn floors_and_ceilings() {
        // Sat 1 Jan 2000 + 1.5 days 7 h 8 min 9.01 s.
        let t = Y2000 + 1.5 * DURATION_DAY + 7.0 * DURATION_HOUR + 8.0 * DURATION_MINUTE + 9010.0;
        assert_eq!(Interval::Unit(Unit::Day).floor(t), Y2000 + DURATION_DAY);
        assert_eq!(Interval::Unit(Unit::Week).floor(t), Y2000 + DURATION_DAY);
        assert_eq!(Interval::Unit(Unit::Month).ceil(t), 949_363_200_000.0);
        assert_eq!(Interval::Unit(Unit::Year).floor(t), Y2000);
        assert_eq!(
            Unit::Minute.every(15.0).unwrap().floor(t),
            t - 9010.0 - 8.0 * DURATION_MINUTE
        );
    }

    #[test]
    fn ticks_of_a_day() {
        let ticks = time_ticks(Y2000, Y2000 + DURATION_DAY, 10.0);
        let want: Vec<f64> = (0..=8)
            .map(|i| Y2000 + f64::from(i) * 3.0 * DURATION_HOUR)
            .collect();
        assert_eq!(ticks, want);
        assert!(time_ticks(f64::NAN, Y2000, 5.0).is_empty());
        assert_eq!(time_ticks(5.0, 0.0, 1.0), [5.0, 0.0]);
    }
}
