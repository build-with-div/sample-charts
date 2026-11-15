//! `Date` objects (ECMA-262 21.4): a time value, milliseconds since the
//! epoch, with the local-time getters and setters d3-time calls and
//! `Date.prototype.toString`.
//!
//! The local time zone is UTC: `LocalTZA` is 0 for every time value, as in
//! a browser on a machine set to UTC (and in the fixture generators, which
//! set `TZ=UTC`). Div has no time zone database yet, so a page in another
//! zone gets other local times from Chromium (`docs/framework-gaps.md`).
//!
//! `MakeDay` follows V8 (`src/date/date.cc`) where the specification leaves
//! the range open: a year outside ±1,000,000 or a month outside
//! ±10,000,000 gives NaN. Every other operation is the specification's
//! arithmetic on doubles.

const MS_PER_SECOND: f64 = 1000.0;
const MS_PER_MINUTE: f64 = 60_000.0;
const MS_PER_HOUR: f64 = 3_600_000.0;
const MS_PER_DAY: f64 = 86_400_000.0;
const MS_PER_DAY_I: i64 = 86_400_000;

/// TimeClip (21.4.1.31): NaN beyond ±8.64e15 ms, else the integer part
/// (with -0 as +0).
pub fn time_clip(time: f64) -> f64 {
    if !time.is_finite() || time.abs() > 8.64e15 {
        return f64::NAN;
    }
    to_integer_or_infinity(time)
}

/// ToIntegerOrInfinity (7.1.5) of a number: NaN and -0 give +0.
fn to_integer_or_infinity(x: f64) -> f64 {
    if x.is_nan() {
        return 0.0;
    }
    let t = x.trunc();
    if t == 0.0 {
        0.0
    } else {
        t
    }
}

/// Days from 1970-01-01 to the first of `month` (0-based) of `year`
/// (proleptic Gregorian; the civil-from-days inverse of `civil`).
fn days_from_civil(year: i64, month: i64) -> i64 {
    let (y, m) = if month < 2 {
        (year - 1, month + 13)
    } else {
        (year, month + 1)
    };
    // March-based year: months 3..=14.
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = m - 3;
    let doy = (153 * mp + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The year, month (0-based) and date (1-based) of a day number.
fn civil(day: i64) -> (i64, i64, i64) {
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let date = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 2 } else { mp - 10 };
    let year = yoe + era * 400 + i64::from(month < 2);
    (year, month, date)
}

/// MakeTime (21.4.1.27).
fn make_time(hour: f64, min: f64, sec: f64, ms: f64) -> f64 {
    if !(hour.is_finite() && min.is_finite() && sec.is_finite() && ms.is_finite()) {
        return f64::NAN;
    }
    let (h, m, s, milli) = (
        to_integer_or_infinity(hour),
        to_integer_or_infinity(min),
        to_integer_or_infinity(sec),
        to_integer_or_infinity(ms),
    );
    h * MS_PER_HOUR + m * MS_PER_MINUTE + s * MS_PER_SECOND + milli
}

/// MakeDay (21.4.1.28), with V8's range limits.
fn make_day(year: f64, month: f64, date: f64) -> f64 {
    if !(year.is_finite() && month.is_finite() && date.is_finite()) {
        return f64::NAN;
    }
    let (y, m, dt) = (
        to_integer_or_infinity(year),
        to_integer_or_infinity(month),
        to_integer_or_infinity(date),
    );
    if !(-1_000_000.0..=1_000_000.0).contains(&y) || !(-10_000_000.0..=10_000_000.0).contains(&m) {
        return f64::NAN;
    }
    // Both are integers well inside i64.
    let (y, m) = (y as i64, m as i64);
    let ym = y + m.div_euclid(12);
    let mn = m.rem_euclid(12);
    days_from_civil(ym, mn) as f64 + dt - 1.0
}

/// MakeDate (21.4.1.29).
fn make_date(day: f64, time: f64) -> f64 {
    if !(day.is_finite() && time.is_finite()) {
        return f64::NAN;
    }
    let tv = day * MS_PER_DAY + time;
    if tv.is_finite() {
        tv
    } else {
        f64::NAN
    }
}

/// The fields of a valid time value.
struct Fields {
    /// Day (21.4.1.3).
    day: i64,
    year: i64,
    month: i64,
    date: i64,
    week_day: i64,
    hours: i64,
    minutes: i64,
    seconds: i64,
    ms: i64,
    /// TimeWithinDay.
    time_within_day: f64,
}

/// Splits a time value (an integer within ±8.64e15, so exact as i64).
fn fields(t: f64) -> Fields {
    let t = t as i64;
    let day = t.div_euclid(MS_PER_DAY_I);
    let within = t.rem_euclid(MS_PER_DAY_I);
    let (year, month, date) = civil(day);
    Fields {
        day,
        year,
        month,
        date,
        week_day: (day + 4).rem_euclid(7),
        hours: within / 3_600_000,
        minutes: within / 60_000 % 60,
        seconds: within / 1000 % 60,
        ms: within % 1000,
        time_within_day: within as f64,
    }
}

// div-gap: TZ1 system time zone (docs/framework-gaps.md). Scaffolding: the
// offset is 0 until Div reports the zone's rules.

/// LocalTime (21.4.1.25) with a zero offset.
fn local_time(t: f64) -> f64 {
    t
}

/// UTC (21.4.1.26) with a zero offset.
fn utc(t: f64) -> f64 {
    t
}

/// A `Date` object: its [[DateValue]].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Date {
    time: f64,
}

impl Date {
    /// `new Date(value)` for a number: TimeClip(value).
    pub fn new(time: f64) -> Self {
        Self {
            time: time_clip(time),
        }
    }

    /// `+date`, `date.getTime()`: the time value (NaN for an Invalid Date).
    pub fn time(&self) -> f64 {
        self.time
    }

    /// `date.setTime(time)`.
    pub fn set_time(&mut self, time: f64) {
        self.time = time_clip(time);
    }

    /// The local fields, `None` for an Invalid Date.
    fn local(&self) -> Option<Fields> {
        (!self.time.is_nan()).then(|| fields(local_time(self.time)))
    }

    fn get(&self, field: impl Fn(&Fields) -> i64) -> f64 {
        self.local().map_or(f64::NAN, |f| field(&f) as f64)
    }

    pub fn get_full_year(&self) -> f64 {
        self.get(|f| f.year)
    }

    pub fn get_month(&self) -> f64 {
        self.get(|f| f.month)
    }

    pub fn get_date(&self) -> f64 {
        self.get(|f| f.date)
    }

    pub fn get_day(&self) -> f64 {
        self.get(|f| f.week_day)
    }

    pub fn get_hours(&self) -> f64 {
        self.get(|f| f.hours)
    }

    pub fn get_minutes(&self) -> f64 {
        self.get(|f| f.minutes)
    }

    pub fn get_seconds(&self) -> f64 {
        self.get(|f| f.seconds)
    }

    pub fn get_milliseconds(&self) -> f64 {
        self.get(|f| f.ms)
    }

    /// `getUTCSeconds()`.
    pub fn get_utc_seconds(&self) -> f64 {
        if self.time.is_nan() {
            f64::NAN
        } else {
            fields(self.time).seconds as f64
        }
    }

    /// `getTimezoneOffset()`: (t - LocalTime(t)) in minutes.
    pub fn get_timezone_offset(&self) -> f64 {
        if self.time.is_nan() {
            f64::NAN
        } else {
            (self.time - local_time(self.time)) / MS_PER_MINUTE
        }
    }

    /// `setHours(hour, min, sec, ms)`; `None` arguments are absent.
    pub fn set_hours(&mut self, hour: f64, min: Option<f64>, sec: Option<f64>, ms: Option<f64>) {
        let Some(f) = self.local() else { return };
        let m = min.unwrap_or(f.minutes as f64);
        let s = sec.unwrap_or(f.seconds as f64);
        let milli = ms.unwrap_or(f.ms as f64);
        let date = make_date(f.day as f64, make_time(hour, m, s, milli));
        self.time = time_clip(utc(date));
    }

    /// `setDate(date)`.
    pub fn set_date(&mut self, date: f64) {
        let Some(f) = self.local() else { return };
        let new_date = make_date(
            make_day(f.year as f64, f.month as f64, date),
            f.time_within_day,
        );
        self.time = time_clip(utc(new_date));
    }

    /// `setMonth(month, date)`; a `None` date is absent.
    pub fn set_month(&mut self, month: f64, date: Option<f64>) {
        let Some(f) = self.local() else { return };
        let dt = date.unwrap_or(f.date as f64);
        let new_date = make_date(make_day(f.year as f64, month, dt), f.time_within_day);
        self.time = time_clip(utc(new_date));
    }

    /// `setFullYear(year, month, date)`; `None` arguments are absent. An
    /// Invalid Date counts as +0 here.
    pub fn set_full_year(&mut self, year: f64, month: Option<f64>, date: Option<f64>) {
        let t = if self.time.is_nan() {
            0.0
        } else {
            local_time(self.time)
        };
        let f = fields(t);
        let m = month.unwrap_or(f.month as f64);
        let dt = date.unwrap_or(f.date as f64);
        let new_date = make_date(make_day(year, m, dt), f.time_within_day);
        self.time = time_clip(utc(new_date));
    }

    /// `Date.prototype.toString()` (21.4.4.41): "Invalid Date", or the
    /// date, time and zone, as V8 writes UTC: "Sat Jan 01 2000 00:00:00
    /// GMT+0000 (Coordinated Universal Time)".
    pub fn to_js_string(&self) -> String {
        const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let Some(f) = self.local() else {
            return "Invalid Date".into();
        };
        let sign = if f.year < 0 { "-" } else { "" };
        format!(
            "{} {} {:02} {sign}{:04} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
            DAYS[f.week_day as usize],
            MONTHS[f.month as usize],
            f.date,
            f.year.abs(),
            f.hours,
            f.minutes,
            f.seconds,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_round_trips() {
        for day in (-100_000_001..=100_000_001).step_by(9_973) {
            let (y, m, d) = civil(day);
            assert_eq!(days_from_civil(y, m) + d - 1, day);
        }
        assert_eq!(civil(0), (1970, 0, 1));
        assert_eq!(civil(-1), (1969, 11, 31));
        assert_eq!(days_from_civil(2000, 0), 10_957);
    }

    #[test]
    fn strings_as_v8_writes_them_in_utc() {
        // Node 24.22.3 with TZ=UTC.
        for (t, s) in [
            (
                0.0,
                "Thu Jan 01 1970 00:00:00 GMT+0000 (Coordinated Universal Time)",
            ),
            (
                -1.0,
                "Wed Dec 31 1969 23:59:59 GMT+0000 (Coordinated Universal Time)",
            ),
            (
                -62198755200000.0,
                "Fri Jan 01 -0001 00:00:00 GMT+0000 (Coordinated Universal Time)",
            ),
            (
                -8.64e15,
                "Tue Apr 20 -271821 00:00:00 GMT+0000 (Coordinated Universal Time)",
            ),
            (
                8.64e15,
                "Sat Sep 13 275760 00:00:00 GMT+0000 (Coordinated Universal Time)",
            ),
            (
                1e12,
                "Sun Sep 09 2001 01:46:40 GMT+0000 (Coordinated Universal Time)",
            ),
        ] {
            assert_eq!(Date::new(t).to_js_string(), s);
        }
        assert_eq!(Date::new(f64::NAN).to_js_string(), "Invalid Date");
        assert!(Date::new(8.64e15 + 1.0).time().is_nan());
    }

    #[test]
    fn setters_carry_over_like_javascript() {
        // new Date(Date.UTC(2000, 0, 31)).setMonth(1) is 2 March 2000.
        let mut d = Date::new(949_276_800_000.0);
        d.set_month(1.0, None);
        assert_eq!((d.get_month(), d.get_date()), (2.0, 2.0));
        d.set_date(0.0);
        assert_eq!((d.get_month(), d.get_date()), (1.0, 29.0));
        d.set_hours(-1.0, Some(0.0), Some(0.0), Some(0.0));
        assert_eq!((d.get_date(), d.get_hours()), (28.0, 23.0));
        let mut invalid = Date::new(f64::NAN);
        invalid.set_full_year(2001.0, None, None);
        assert_eq!(invalid.time(), 978_307_200_000.0);
    }
}
