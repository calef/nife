//! **calendar: Unix seconds to a civil date, and back.** The part of wall-clock time with no
//! authority in it: seconds to a year, month and day and back, and the few textual forms the system
//! needs. No IO, syscalls or allocation, so it is host-tested and Kani-checked.
//!
//! The calendar is proleptic Gregorian, as every Unix-time library's is, so dates before October
//! 1582 differ from a historian's, and year 0 exists and is a leap year. Negative timestamps are
//! supported, because files predating the epoch are ordinary and a dead RTC reads as anything.
//!
//! The range is **years 0000 through 9999** ([`MIN_TIMESTAMP`] to [`MAX_TIMESTAMP`]), RFC 3339's
//! four-digit year, so no value can be computed that cannot be written; outside it is
//! [`Error::OutOfRange`], never a wrapped date. Every timestamp is [`i64`], so 2038 is a non-event,
//! and the tests prove it.
//!
//! Leap seconds have no Unix timestamp, so [`DateTime::parse_rfc3339`] refuses `:60` with
//! [`Error::LeapSecond`] rather than folding it. Time zones are a fixed [`UtcOffset`], which is
//! what RFC 3339 carries; the IANA database is out of scope permanently, being a frequently
//! updated data distribution with its own capability question, and would be a separate crate. No
//! durations or month arithmetic (one month after January 31 has no single answer) and no
//! `strftime` ([`Format`] says why).
//!
//! Name: ratified 2026-08-23 (calef): the plain noun for what the crate does.

#![no_std]

/// The lowest supported year, 0000. See the module docs for why the range is what it is.
pub const MIN_YEAR: i32 = 0;

/// The highest supported year, 9999, which is RFC 3339's four-digit `date-fullyear`.
pub const MAX_YEAR: i32 = 9999;

/// 0000-01-01T00:00:00Z, the earliest instant this crate will name.
pub const MIN_TIMESTAMP: i64 = days_from_civil(MIN_YEAR, 1, 1) * SECS_PER_DAY;

/// 9999-12-31T23:59:59Z, the latest instant this crate will name.
pub const MAX_TIMESTAMP: i64 = days_from_civil(MAX_YEAR, 12, 31) * SECS_PER_DAY + SECS_PER_DAY - 1;

/// Seconds in a day. Exactly 86,400, always, by the definition of Unix time (see the module docs on
/// leap seconds).
pub const SECS_PER_DAY: i64 = 86_400;

/// What can go wrong. One enum for conversion and parsing both, because the API is small enough that
/// two would only make callers convert between them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// A timestamp outside [`MIN_TIMESTAMP`]..=[`MAX_TIMESTAMP`], or a year outside
    /// [`MIN_YEAR`]..=[`MAX_YEAR`].
    OutOfRange,
    /// Month was not 1..=12.
    BadMonth,
    /// Day was not 1..=(the length of that month in that year). This is the one that catches
    /// February 30, and February 29 in a non-leap year.
    BadDay,
    /// Hour was not 0..=23.
    BadHour,
    /// Minute was not 0..=59.
    BadMinute,
    /// Second was not 0..=59.
    BadSecond,
    /// A UTC offset beyond ±23:59, which RFC 3339's grammar cannot write.
    BadOffset,
    /// A seconds field of 60. Well-formed RFC 3339, and unrepresentable as a Unix timestamp; see the
    /// module docs.
    LeapSecond,
    /// The text is not RFC 3339.
    Syntax,
}

impl Error {
    /// A short message, for a shell that has no allocator and wants to print the refusal.
    pub const fn as_str(self) -> &'static str {
        match self {
            Error::OutOfRange => "date outside the supported range (years 0000-9999)",
            Error::BadMonth => "month out of range",
            Error::BadDay => "day out of range for that month",
            Error::BadHour => "hour out of range",
            Error::BadMinute => "minute out of range",
            Error::BadSecond => "second out of range",
            Error::BadOffset => "UTC offset out of range",
            Error::LeapSecond => "leap second (:60) has no Unix timestamp",
            Error::Syntax => "not an RFC 3339 timestamp",
        }
    }
}

/// Is this a leap year in the proleptic Gregorian calendar?
///
/// The rule people half-remember, in full: every fourth year, except every hundredth, except every
/// four-hundredth. 1900 is **not** a leap year and 2000 **is**, which is the pair that breaks date
/// code written from memory, and the pair the tests pin.
pub const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// How many days that month has in that year, or `None` if `month` is not 1..=12.
pub const fn days_in_month(year: i32, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(if is_leap_year(year) { 29 } else { 28 }),
        _ => None,
    }
}

// Howard Hinnant's civil-date algorithms (the ones in "chrono-Compatible Low-Level Date Algorithms",
// and the same pair inside libc++ and every serious date library). They are not obvious, so what
// each does and why it is shaped that way:
//
// The trick is to shift the year so it *starts in March*. Then the leap day is the last day of the
// year rather than a discontinuity in the middle of it, and every month-length irregularity collapses
// into one closed-form expression, `(153*m + 2)/5`, which walks the 31/30/31/30/31 pattern exactly.
// The era is a 400-year cycle, which is the period of the whole Gregorian rule and contains exactly
// 146,097 days; working within one era makes the century corrections plain division.
//
// Both directions are pure integer arithmetic with no loops and no tables, which is also what makes
// them a good Kani target: the round-trip proof below is a formula, not a search.

/// Days since 1970-01-01 for a civil date. Private: the public entry points validate first, and this
/// has a precondition (`month` in 1..=12, `day` in 1..=31) it does not check.
///
/// `-719468` is the shift from the internal era-relative day count to the Unix epoch: it is how many
/// days lie between 0000-03-01 and 1970-01-01.
const fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let (y, m, d) = (year as i64, month as i64, day as i64);
    // March-based year: January and February belong to the *previous* one.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // year of era, [0, 399]
    let mp = if m > 2 { m - 3 } else { m + 9 }; // March = 0, February = 11
    let doy = (153 * mp + 2) / 5 + d - 1; // day of the March-based year, [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // day of era, [0, 146096]
    era * 146_097 + doe - 719_468
}

/// The inverse. Private for the same reason: the caller has already bounded `days`.
const fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    // The three corrections undo the leap-day insertions: /1460 removes the 4-year days, /36524 puts
    // back the century non-leaps, /146096 removes the 400-year exception to that exception.
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // back to January = 1
    let y = if m <= 2 { y + 1 } else { y }; // undo the March-based shift
    (y as i32, m as u8, d as u8)
}

/// A day of the week. `Sunday` is first because that is the order the day-number arithmetic falls
/// out in (`(days + 4) mod 7`, since 1970-01-01 was a Thursday); nothing here depends on the
/// convention, and [`Weekday::iso_number`] gives the ISO 8601 one, where Monday is 1.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weekday {
    /// ISO 8601 day 7.
    Sunday,
    /// ISO 8601 day 1, the week's first day per [`Weekday::iso_number`].
    Monday,
    /// ISO 8601 day 2.
    Tuesday,
    /// ISO 8601 day 3.
    Wednesday,
    /// ISO 8601 day 4.
    Thursday,
    /// ISO 8601 day 5.
    Friday,
    /// ISO 8601 day 6.
    Saturday,
}

impl Weekday {
    /// The three-letter abbreviation, which is the only natural language in this crate. See
    /// [`Format::Human`] for why there is no month-name table to go with it.
    pub const fn abbrev(self) -> &'static str {
        match self {
            Weekday::Sunday => "Sun",
            Weekday::Monday => "Mon",
            Weekday::Tuesday => "Tue",
            Weekday::Wednesday => "Wed",
            Weekday::Thursday => "Thu",
            Weekday::Friday => "Fri",
            Weekday::Saturday => "Sat",
        }
    }

    /// The full English name.
    pub const fn name(self) -> &'static str {
        match self {
            Weekday::Sunday => "Sunday",
            Weekday::Monday => "Monday",
            Weekday::Tuesday => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday => "Thursday",
            Weekday::Friday => "Friday",
            Weekday::Saturday => "Saturday",
        }
    }

    /// ISO 8601's numbering: Monday is 1, Sunday is 7.
    pub const fn iso_number(self) -> u8 {
        match self {
            Weekday::Monday => 1,
            Weekday::Tuesday => 2,
            Weekday::Wednesday => 3,
            Weekday::Thursday => 4,
            Weekday::Friday => 5,
            Weekday::Saturday => 6,
            Weekday::Sunday => 7,
        }
    }

    const fn from_days(days: i64) -> Weekday {
        // 1970-01-01 was a Thursday, so day 0 is index 4 with Sunday at 0.
        match (days + 4).rem_euclid(7) {
            0 => Weekday::Sunday,
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            _ => Weekday::Saturday,
        }
    }
}

/// A calendar date and a wall-clock time, with **no zone attached**. "The 30th of July 2026 at
/// 12:34:56" is a `Civil`; which instant that is depends on an offset, which is [`DateTime`]'s job.
///
/// The fields are private and the constructor validates, so **a `Civil` that exists is a real
/// date**: there is no February 30 in this type, and [`Civil::to_unix`] is therefore infallible.
/// That is the same discipline the capability types use (a `Cap` you hold is one that was minted),
/// and it is what makes the conversion arithmetic total rather than defensive.
///
/// The derived `Ord` is chronological, because the fields are declared most-significant first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Civil {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl Civil {
    /// Build a date, checking every field. Returns the *specific* [`Error`] for the field that was
    /// wrong, so a `date -s` verb can say what it did not like.
    ///
    /// # Examples
    ///
    /// ```
    /// use calendar::{Civil, Error};
    ///
    /// assert_eq!(Civil::new(2026, 13, 1, 0, 0, 0), Err(Error::BadMonth));
    /// // 2026 is not a leap year, so there is no 29 February.
    /// assert_eq!(Civil::new(2026, 2, 29, 0, 0, 0), Err(Error::BadDay));
    /// assert!(Civil::new(2024, 2, 29, 0, 0, 0).is_ok());
    /// ```
    pub const fn new(
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<Civil, Error> {
        if year < MIN_YEAR || year > MAX_YEAR {
            return Err(Error::OutOfRange);
        }
        let Some(len) = days_in_month(year, month) else {
            return Err(Error::BadMonth);
        };
        if day == 0 || day > len {
            return Err(Error::BadDay);
        }
        if hour > 23 {
            return Err(Error::BadHour);
        }
        if minute > 59 {
            return Err(Error::BadMinute);
        }
        if second > 59 {
            return Err(Error::BadSecond);
        }
        Ok(Civil {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }

    /// The UTC civil date of a Unix timestamp.
    ///
    /// # Examples
    ///
    /// The two conversions are inverses:
    ///
    /// ```
    /// use calendar::{Civil, Weekday};
    ///
    /// // The Unix epoch, as a civil date. 1970-01-01 was a Thursday.
    /// let epoch = Civil::from_unix(0).unwrap();
    /// assert_eq!((epoch.year(), epoch.month(), epoch.day()), (1970, 1, 1));
    /// assert_eq!(epoch.weekday(), Weekday::Thursday);
    /// assert_eq!(epoch.to_unix(), 0);
    ///
    /// // And round-tripping an arbitrary instant.
    /// let d = Civil::new(2026, 8, 2, 12, 30, 15).unwrap();
    /// assert_eq!(Civil::from_unix(d.to_unix()).unwrap(), d);
    /// ```
    ///
    /// One second before the epoch is the last second of 1969, which truncating division gets wrong:
    ///
    /// ```
    /// use calendar::Civil;
    ///
    /// // One second before the epoch is the last second of 1969, not the first of 1970.
    /// let d = Civil::from_unix(-1).unwrap();
    /// assert_eq!((d.year(), d.month(), d.day()), (1969, 12, 31));
    /// assert_eq!((d.hour(), d.minute(), d.second()), (23, 59, 59));
    /// ```
    pub const fn from_unix(secs: i64) -> Result<Civil, Error> {
        if secs < MIN_TIMESTAMP || secs > MAX_TIMESTAMP {
            return Err(Error::OutOfRange);
        }
        // Euclidean division, not truncating: for a negative timestamp, `-1 / 86400` is 0 in Rust and
        // the answer we need is -1 (the day before the epoch). Truncating division here is the classic
        // way a date library gets 1969-12-31 wrong.
        let days = secs.div_euclid(SECS_PER_DAY);
        let sod = secs.rem_euclid(SECS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        Ok(Civil {
            year,
            month,
            day,
            hour: (sod / 3600) as u8,
            minute: ((sod / 60) % 60) as u8,
            second: (sod % 60) as u8,
        })
    }

    /// The Unix timestamp of this date, read as UTC. Infallible: [`Civil::new`] already established
    /// that the date is real and in range.
    pub const fn to_unix(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day) * SECS_PER_DAY
            + self.hour as i64 * 3600
            + self.minute as i64 * 60
            + self.second as i64
    }

    /// Days since 1970-01-01, discarding the time of day.
    pub const fn days_since_epoch(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    /// Which day of the week this date falls on.
    pub const fn weekday(&self) -> Weekday {
        Weekday::from_days(self.days_since_epoch())
    }

    /// Day of the year, 1..=366. January 1 is 1; 366 happens only in a leap year, which is one of
    /// the Kani harnesses rather than a comment.
    pub const fn day_of_year(&self) -> u16 {
        let jan1 = days_from_civil(self.year, 1, 1);
        (self.days_since_epoch() - jan1 + 1) as u16
    }

    /// The year, 0..=9999.
    pub const fn year(&self) -> i32 {
        self.year
    }
    /// The month, 1..=12.
    pub const fn month(&self) -> u8 {
        self.month
    }
    /// The day of the month, 1..=31.
    pub const fn day(&self) -> u8 {
        self.day
    }
    /// The hour, 0..=23.
    pub const fn hour(&self) -> u8 {
        self.hour
    }
    /// The minute, 0..=59.
    pub const fn minute(&self) -> u8 {
        self.minute
    }
    /// The second, 0..=59.
    pub const fn second(&self) -> u8 {
        self.second
    }
}

/// A fixed offset from UTC, in minutes. **Not a time zone**: see the module docs for why the IANA
/// database is out of scope rather than pending.
///
/// The range is ±23:59, which is RFC 3339's grammar rather than geography (real offsets run
/// -12:00..+14:00). Parsing accepts what the grammar allows; nothing here needs to be stricter than
/// the format it reads.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct UtcOffset {
    minutes: i16,
}

impl UtcOffset {
    /// UTC itself, the offset almost everything in this system should be using.
    pub const UTC: UtcOffset = UtcOffset { minutes: 0 };

    /// An offset of `minutes` from UTC, positive east. Refuses anything beyond ±1439.
    pub const fn from_minutes(minutes: i32) -> Result<UtcOffset, Error> {
        if minutes <= -1440 || minutes >= 1440 {
            return Err(Error::BadOffset);
        }
        Ok(UtcOffset {
            minutes: minutes as i16,
        })
    }

    /// An offset written the way people write it: `from_hm(-8, 0)` is `-08:00`, `from_hm(5, 30)` is
    /// `+05:30`. Both arguments carry the sign, and mixing them (`from_hm(-8, 30)` meaning
    /// -08:30) is why `minutes` is signed here rather than a magnitude plus a flag.
    pub const fn from_hm(hours: i32, minutes: i32) -> Result<UtcOffset, Error> {
        if (hours < 0) != (minutes < 0) && hours != 0 && minutes != 0 {
            return Err(Error::BadOffset);
        }
        if minutes <= -60 || minutes >= 60 {
            return Err(Error::BadOffset);
        }
        UtcOffset::from_minutes(hours * 60 + minutes)
    }

    /// The offset in minutes, positive east of Greenwich.
    pub const fn minutes(self) -> i32 {
        self.minutes as i32
    }

    /// The offset in seconds, which is the form the conversions want.
    pub const fn seconds(self) -> i64 {
        self.minutes as i64 * 60
    }
}

/// An instant, seen from a particular offset. This is what a timestamp on the wire is: a civil
/// reading plus the offset it was read at, which together name one instant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DateTime {
    civil: Civil,
    offset: UtcOffset,
}

impl DateTime {
    /// Pair a civil reading with the offset it was read at. Infallible: both halves are already
    /// valid, and no arithmetic happens until [`DateTime::to_unix`].
    pub const fn new(civil: Civil, offset: UtcOffset) -> DateTime {
        DateTime { civil, offset }
    }

    /// What the clock reads at `offset` when the Unix time is `secs`.
    ///
    /// `OutOfRange` here means the *local* reading falls outside years 0000-9999, which for any
    /// offset differs from the UTC bound by less than a day. That edge is the only place a caller
    /// can be surprised, and it is why this returns a `Result` where [`DateTime::new`] does not.
    pub const fn from_unix(secs: i64, offset: UtcOffset) -> Result<DateTime, Error> {
        let Some(local) = secs.checked_add(offset.seconds()) else {
            return Err(Error::OutOfRange);
        };
        match Civil::from_unix(local) {
            Ok(civil) => Ok(DateTime { civil, offset }),
            Err(e) => Err(e),
        }
    }

    /// The instant this names, as a Unix timestamp.
    ///
    /// Note this is *not* bounded by [`MIN_TIMESTAMP`]/[`MAX_TIMESTAMP`]: a valid local reading at
    /// the extreme end of the range with a large offset lands up to a day outside it. The value is
    /// still exact; it just may not survive a trip back through [`Civil::from_unix`].
    pub const fn to_unix(&self) -> i64 {
        self.civil.to_unix() - self.offset.seconds()
    }

    /// The local civil reading.
    pub const fn civil(&self) -> Civil {
        self.civil
    }

    /// The offset it was read at.
    pub const fn offset(&self) -> UtcOffset {
        self.offset
    }
}

/// The formats this crate can print. Five, chosen rather than derived, and the argument for each is
/// that something in this system needs exactly it.
///
/// **Why not `strftime`.** A format-string interpreter would be a second parser (one whose errors
/// appear at runtime, in a `no_std` program with no allocator, for a string nobody type-checked) in
/// exchange for combinations nothing here asks for. `%c` alone drags in locales. The five below
/// cover every consumer milestone 51 actually has, and adding a sixth is a two-line match arm plus a
/// test, which is a cheaper way to be wrong than a grammar is.
///
/// **Why no month names.** [`Format::Human`] keeps the ISO date and prepends the weekday, instead of
/// copying `date`'s `Wed Jul 30 12:34:56 UTC 2026`. Two reasons: that layout does not sort, and it
/// needs a month-name table which is the first step towards a locale question this crate has no
/// business answering. The weekday abbreviation is the entire natural-language surface here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    /// `2026-07-30T12:34:56Z`, or `2026-07-30T12:34:56+05:30` away from UTC. RFC 3339, the
    /// interchange form: what [`DateTime::parse_rfc3339`] reads back, what a log line or an NTP
    /// exchange should carry. Zero offset prints as `Z`.
    Rfc3339,
    /// `2026-07-30`. The date alone, for a directory listing or a filename.
    Date,
    /// `12:34:56`. The time alone.
    Time,
    /// `Thu 2026-07-30 12:34:56 UTC`, with the offset in place of `UTC` when there is one. The
    /// default a `date` command with no arguments should print: sortable, unambiguous, and it tells
    /// you the day of the week, which is the one thing a human wants that ISO does not give.
    Human,
    /// `1785414896`. The raw seconds, for arithmetic and for a caller that wants to hand the number
    /// straight back to the clock service.
    Unix,
}

/// The longest output any [`Format`] produces is `Human` at an extreme offset,
/// `Thu 9999-12-31 23:59:59 +23:59`, which is 30 bytes.
const FMT_CAP: usize = 32;

/// A formatted timestamp, in a fixed buffer. No allocator, and no lifetime tied to the value it came
/// from, so a caller can format and then print at leisure.
#[derive(Clone, Copy)]
pub struct Formatted {
    buf: [u8; FMT_CAP],
    len: u8,
}

impl Formatted {
    /// The bytes. Always ASCII (proved: `every_format_is_ascii`).
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }

    /// The text.
    ///
    /// Total, with no panic path: the bytes are ASCII by construction and the Kani harness
    /// `every_format_is_ascii` proves the `unwrap_or` branch is unreachable for every timestamp and
    /// every format. A `.expect()` here would be a panic in a userspace program to defend against a
    /// case that cannot arise.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).unwrap_or("")
    }
}

impl core::fmt::Display for Formatted {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::fmt::Debug for Formatted {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl AsRef<str> for Formatted {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// A cursor that writes into the fixed buffer. Every `push` is bounds-checked and silently drops on
/// overflow, which cannot happen ([`FMT_CAP`] is two bytes larger than the longest output) and is
/// still better than a panic in a program that only wanted to print the time.
struct Writer {
    buf: [u8; FMT_CAP],
    len: usize,
}

impl Writer {
    const fn new() -> Writer {
        Writer {
            buf: [0; FMT_CAP],
            len: 0,
        }
    }

    fn byte(&mut self, b: u8) {
        if self.len < FMT_CAP {
            self.buf[self.len] = b;
            self.len += 1;
        }
    }

    fn str(&mut self, s: &str) {
        for &b in s.as_bytes() {
            self.byte(b);
        }
    }

    /// A fixed-width zero-padded number, which is every numeric field in every format here except
    /// `Unix`.
    fn pad(&mut self, value: u32, width: usize) {
        let mut digits = [0u8; 10];
        let mut n = 0;
        let mut v = value;
        loop {
            digits[n] = b'0' + (v % 10) as u8;
            v /= 10;
            n += 1;
            if v == 0 {
                break;
            }
        }
        for _ in n..width {
            self.byte(b'0');
        }
        while n > 0 {
            n -= 1;
            self.byte(digits[n]);
        }
    }

    fn signed(&mut self, value: i64) {
        if value < 0 {
            self.byte(b'-');
        }
        // Via u64 so i64::MIN has no special case.
        let mut v = value.unsigned_abs();
        let mut digits = [0u8; 20];
        let mut n = 0;
        loop {
            digits[n] = b'0' + (v % 10) as u8;
            v /= 10;
            n += 1;
            if v == 0 {
                break;
            }
        }
        while n > 0 {
            n -= 1;
            self.byte(digits[n]);
        }
    }

    /// `+HH:MM` / `-HH:MM`.
    fn offset(&mut self, offset: UtcOffset) {
        let m = offset.minutes();
        self.byte(if m < 0 { b'-' } else { b'+' });
        let m = m.unsigned_abs();
        self.pad(m / 60, 2);
        self.byte(b':');
        self.pad(m % 60, 2);
    }

    fn finish(self) -> Formatted {
        Formatted {
            buf: self.buf,
            len: self.len as u8,
        }
    }
}

impl DateTime {
    /// Render in one of the five [`Format`]s. No allocation; the result owns its bytes.
    pub fn format(&self, format: Format) -> Formatted {
        let mut w = Writer::new();
        let c = self.civil;
        match format {
            Format::Rfc3339 => {
                write_date(&mut w, &c);
                w.byte(b'T');
                write_time(&mut w, &c);
                if self.offset.minutes == 0 {
                    w.byte(b'Z');
                } else {
                    w.offset(self.offset);
                }
            }
            Format::Date => write_date(&mut w, &c),
            Format::Time => write_time(&mut w, &c),
            Format::Human => {
                w.str(c.weekday().abbrev());
                w.byte(b' ');
                write_date(&mut w, &c);
                w.byte(b' ');
                write_time(&mut w, &c);
                w.byte(b' ');
                if self.offset.minutes == 0 {
                    w.str("UTC");
                } else {
                    w.offset(self.offset);
                }
            }
            Format::Unix => w.signed(self.to_unix()),
        }
        w.finish()
    }
}

impl Civil {
    /// Render this date as UTC. Shorthand for `DateTime::new(c, UtcOffset::UTC).format(f)`, which is
    /// what most callers want.
    pub fn format(&self, format: Format) -> Formatted {
        DateTime::new(*self, UtcOffset::UTC).format(format)
    }
}

fn write_date(w: &mut Writer, c: &Civil) {
    w.pad(c.year as u32, 4);
    w.byte(b'-');
    w.pad(c.month as u32, 2);
    w.byte(b'-');
    w.pad(c.day as u32, 2);
}

fn write_time(w: &mut Writer, c: &Civil) {
    w.pad(c.hour as u32, 2);
    w.byte(b':');
    w.pad(c.minute as u32, 2);
    w.byte(b':');
    w.pad(c.second as u32, 2);
}

impl DateTime {
    /// Parse RFC 3339, which is the interoperable subset of ISO 8601 and the only textual timestamp
    /// this crate reads.
    ///
    /// ```text
    /// YYYY-MM-DDTHH:MM:SS[.fraction](Z | ±HH:MM)
    /// ```
    ///
    /// Three deliberate relaxations, each of which RFC 3339 itself sanctions:
    ///
    /// - **Lowercase `t` and `z`** are accepted (§5.6 says they may be).
    /// - **A space in place of `T`** is accepted (§5.6's NOTE allows it by agreement, and it is what
    ///   a person types at a shell prompt).
    /// - **Fractional seconds are parsed and discarded.** This clock has one-second resolution, so
    ///   keeping them would mean either lying about precision or growing the type. Refusing them
    ///   would reject perfectly ordinary timestamps from other systems.
    ///
    /// Everything else is strict: exactly four year digits and two of everything else, no missing
    /// offset (an RFC 3339 timestamp always carries one, which is the whole difference from bare
    /// ISO 8601), no trailing bytes. `:60` is [`Error::LeapSecond`], not a syntax error, because it
    /// is well-formed text this calendar cannot represent (see the module docs).
    pub fn parse_rfc3339(s: &str) -> Result<DateTime, Error> {
        DateTime::parse_rfc3339_bytes(s.as_bytes())
    }

    /// [`DateTime::parse_rfc3339`] over bytes, which is what the grammar is actually defined on: RFC
    /// 3339 is ASCII, and every byte outside it is a syntax error whether or not it forms valid
    /// UTF-8.
    ///
    /// This is the real implementation and the `&str` version is the wrapper, not the other way
    /// round, for two reasons. A caller holding a network buffer or a shell argument has bytes and
    /// would otherwise have to validate UTF-8 first, only for this function to reject every non-ASCII
    /// byte anyway. And it lets the totality proof quantify over **arbitrary bytes**, including
    /// sequences that are not UTF-8 at all, rather than over the subset `from_utf8` would have let
    /// through.
    pub fn parse_rfc3339_bytes(b: &[u8]) -> Result<DateTime, Error> {
        // "0000-00-00T00:00:00Z" is 20 bytes, the shortest legal input.
        if b.len() < 20 {
            return Err(Error::Syntax);
        }
        let year = number(b, 0, 4)? as i32;
        expect(b, 4, b'-')?;
        let month = number(b, 5, 2)? as u8;
        expect(b, 7, b'-')?;
        let day = number(b, 8, 2)? as u8;
        match b[10] {
            b'T' | b't' | b' ' => {}
            _ => return Err(Error::Syntax),
        }
        let hour = number(b, 11, 2)? as u8;
        expect(b, 13, b':')?;
        let minute = number(b, 14, 2)? as u8;
        expect(b, 16, b':')?;
        let second = number(b, 17, 2)? as u8;
        if second == 60 {
            return Err(Error::LeapSecond);
        }

        let mut i = 19;
        if b[i] == b'.' {
            i += 1;
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i == start {
                return Err(Error::Syntax); // a '.' with no digits after it
            }
        }

        if i >= b.len() {
            return Err(Error::Syntax);
        }
        let offset = match b[i] {
            b'Z' | b'z' => {
                i += 1;
                UtcOffset::UTC
            }
            sign @ (b'+' | b'-') => {
                // ±HH:MM, six bytes.
                if i + 6 > b.len() {
                    return Err(Error::Syntax);
                }
                let oh = number(b, i + 1, 2)? as i32;
                expect(b, i + 3, b':')?;
                let om = number(b, i + 4, 2)? as i32;
                if om > 59 {
                    return Err(Error::BadOffset);
                }
                i += 6;
                let total = oh * 60 + om;
                UtcOffset::from_minutes(if sign == b'-' { -total } else { total })?
            }
            _ => return Err(Error::Syntax),
        };
        if i != b.len() {
            return Err(Error::Syntax);
        }

        let civil = Civil::new(year, month, day, hour, minute, second)?;
        Ok(DateTime::new(civil, offset))
    }
}

/// `width` ASCII digits at `at`, as a number. Refuses anything else, including a leading `+` or a
/// space, which is what keeps the grammar fixed-width.
fn number(b: &[u8], at: usize, width: usize) -> Result<u32, Error> {
    if at + width > b.len() {
        return Err(Error::Syntax);
    }
    let mut v = 0u32;
    for &c in &b[at..at + width] {
        if !c.is_ascii_digit() {
            return Err(Error::Syntax);
        }
        v = v * 10 + (c - b'0') as u32;
    }
    Ok(v)
}

fn expect(b: &[u8], at: usize, c: u8) -> Result<(), Error> {
    if b.get(at) == Some(&c) {
        Ok(())
    } else {
        Err(Error::Syntax)
    }
}

#[cfg(test)]
mod tests;

/// **The machine-checked half** (`script/verify`, notes/verification.md).
///
/// A calendar looks like an ideal bounded-model-checking target: closed-form integer arithmetic, no
/// loops, no tables, no allocation. It half is, and where it is not turned out to be the interesting
/// finding of this lane, so it is recorded here rather than left as an unexplained set of bounds.
///
/// **The calendar arithmetic is cheap to prove and the seconds-per-day division is not.** The
/// Hinnant algorithms round-trip over the *entire* supported range, all 3,652,425 days of it, in
/// under a minute. Add one `div_euclid(86_400)` over a symbolic 64-bit timestamp and the same
/// property does not finish in twenty. That is not a fact about calendars; it is that a bit-blasted
/// 64-bit divider with an unconstrained dividend is one of the hardest things you can hand a CDCL
/// SAT solver, and the divisions inside `civil_from_days` are cheap only because the era arithmetic
/// keeps their operands small. Measured, not assumed: one day of timestamps proves in 3s, four years
/// in 64s, the full range in neither twenty minutes nor with kissat instead of CaDiCaL.
///
/// So the harnesses are **factored along that seam** rather than bounded uniformly:
///
/// - the calendar half (`days <-> (y, m, d)`, and everything derived from a day number) is proved
///   over the **full range**, unbounded below the type;
/// - the seconds half (the `86_400` split, which is the only place the expensive division appears)
///   is proved over a **four-year window that straddles the epoch**, chosen because the risk it
///   carries is "did someone use truncating division instead of Euclidean", which is a property of
///   the *code* and shows up on any negative timestamp. The window contains negative seconds, zero,
///   positive seconds, a leap day and two year boundaries.
///
/// The two compose: `from_unix` is `civil_from_days(secs.div_euclid(86_400))` plus a time of day,
/// and `to_unix` is `days_from_civil(...) * 86_400` plus the same time of day. Saying that plainly is
/// better than a uniform bound that quietly excluded the year 3000, and the unit test
/// `format_and_parse_round_trip_across_the_range` walks the whole ten thousand years at a stride to
/// cover what a bounded proof cannot.
#[cfg(kani)]
mod verification {
    use super::*;

    /// The first and last day numbers in the supported range: 0000-01-01 and 9999-12-31.
    const FIRST_DAY: i64 = days_from_civil(MIN_YEAR, 1, 1);
    const LAST_DAY: i64 = days_from_civil(MAX_YEAR, 12, 31);

    /// **Any representable `DateTime`**, built from six symbolic fields and a symbolic offset, or
    /// `None` for the field combinations that are not a real date.
    ///
    /// The text harnesses take their input this way rather than from a symbolic timestamp, and that
    /// is the difference between a proof over one day and a proof over the whole range: constructing
    /// the value from fields skips `from_unix`, and `from_unix` is where the expensive
    /// `div_euclid(86_400)` lives (see the module comment). Coverage is identical, since every
    /// `DateTime` this crate can hold is exactly a valid `Civil` and a valid offset.
    fn any_datetime() -> Option<DateTime> {
        let year: i32 = kani::any();
        let month: u8 = kani::any();
        let day: u8 = kani::any();
        let hour: u8 = kani::any();
        let minute: u8 = kani::any();
        let second: u8 = kani::any();
        let om: i32 = kani::any();
        kani::assume((-1439..=1439).contains(&om));
        let civil = Civil::new(year, month, day, hour, minute, second).ok()?;
        Some(DateTime::new(civil, UtcOffset::from_minutes(om).unwrap()))
    }

    /// **The central theorem: the two calendar algorithms are mutual inverses.** For every one of the
    /// 3,652,425 days in the supported range, `civil_from_days` then `days_from_civil` returns the
    /// day it started with. No bound below the type's own.
    ///
    /// If this holds, everything else about the calendar follows: the leap-year rules, the month
    /// lengths, the century exceptions and their exception are all *inside* these two functions, and
    /// a bijection cannot have them wrong in a way that cancels (a wrong month length would have to
    /// map two days to one date, which is not injective).
    /// Falsification: replayable `crates/calendar/falsifications/verification.the_calendar_algorithms_are_mutual_inverses.patch`
    #[kani::proof]
    fn the_calendar_algorithms_are_mutual_inverses() {
        let days: i64 = kani::any();
        kani::assume((FIRST_DAY..=LAST_DAY).contains(&days));
        let (y, m, d) = civil_from_days(days);
        assert_eq!(days_from_civil(y, m, d), days);
    }

    /// **A day number always decodes to a date that exists.** For every day in the range: the year is
    /// in range, the month is 1..=12, and the day is within *that month's length in that year*, which
    /// is the leap-year rule applied. The strongest phrasing is the last line: the validating
    /// constructor accepts what the decoder produced, so `from_unix` cannot manufacture a `Civil` that
    /// `Civil::new` would have refused.
    ///
    /// This is the harness that stops the bijection above from being vacuous. A decoder that
    /// consistently invented February 30 would still be a bijection.
    /// Falsification: replayable `crates/calendar/falsifications/verification.a_day_number_always_decodes_to_a_real_date.patch`
    #[kani::proof]
    fn a_day_number_always_decodes_to_a_real_date() {
        let days: i64 = kani::any();
        kani::assume((FIRST_DAY..=LAST_DAY).contains(&days));
        let (y, m, d) = civil_from_days(days);
        assert!((MIN_YEAR..=MAX_YEAR).contains(&y));
        assert!((1..=12).contains(&m));
        assert!(1 <= d && d <= days_in_month(y, m).unwrap());
        assert!(Civil::new(y, m, d, 0, 0, 0).is_ok());
    }

    /// **The other direction, over every field value there is.** Year, month, day, hour, minute and
    /// second are all fully symbolic, so February 30, hour 99 and year 10,000 are among the inputs.
    /// For every combination `Civil::new` accepts, the date survives a trip through its day number,
    /// and the timestamp it computes lies inside the supported range.
    ///
    /// `to_unix` appears here in full (it multiplies by 86,400 rather than dividing, which is the
    /// cheap direction), so the range assertion is proved for every representable date.
    /// Falsification: replayable `crates/calendar/falsifications/verification.every_real_date_survives_its_own_day_number.patch`
    #[kani::proof]
    fn every_real_date_survives_its_own_day_number() {
        let year: i32 = kani::any();
        let month: u8 = kani::any();
        let day: u8 = kani::any();
        let hour: u8 = kani::any();
        let minute: u8 = kani::any();
        let second: u8 = kani::any();

        let Ok(civil) = Civil::new(year, month, day, hour, minute, second) else {
            return; // refused, and `a_day_number_always_decodes_to_a_real_date` covers the accepted set
        };
        assert_eq!(
            civil_from_days(civil.days_since_epoch()),
            (year, month, day)
        );
        let secs = civil.to_unix();
        assert!((MIN_TIMESTAMP..=MAX_TIMESTAMP).contains(&secs));
        // Not vacuous, and specifically not on the case that matters: the constructor does accept a
        // 29th of February, so this is quantifying over leap days rather than over an input set the
        // validation quietly emptied.
        kani::cover!(month == 2 && day == 29);
    }

    /// **Time only moves forward.** Every day in the range decodes to a date that sorts strictly
    /// after the day before it, under `Civil`'s derived `Ord`. That is what makes `Format::Date`
    /// sortable text and what a log reader assumes, and it rules out the class of bug where a
    /// boundary (a month end, a leap day, a century) steps the calendar backwards for one day.
    ///
    /// Stated on **adjacent** days rather than on an arbitrary pair, which is the same theorem: the
    /// order is transitive, so `d(n) < d(n+1)` for every `n` gives `d(a) < d(b)` for every `a < b`.
    /// It is also several times cheaper to check, because a second symbolic day number costs a second
    /// copy of the whole decode: 228s measured for the pair, 38-50s for the adjacent form. Worth
    /// stating rather than quietly enjoying, since it reads like a weaker claim and is not: this is a
    /// proof of the general property, phrased as its induction step.
    /// Falsification: replayable `crates/calendar/falsifications/verification.each_day_sorts_after_the_one_before_it.patch`
    #[kani::proof]
    fn each_day_sorts_after_the_one_before_it() {
        let days: i64 = kani::any();
        kani::assume((FIRST_DAY..LAST_DAY).contains(&days));
        let (ay, am, ad) = civil_from_days(days);
        let (by, bm, bd) = civil_from_days(days + 1);
        assert!(
            Civil::new(ay, am, ad, 0, 0, 0).unwrap() < Civil::new(by, bm, bd, 0, 0, 0).unwrap()
        );
    }

    /// **Day of year is 1..=366, and 366 happens only on 31 December of a leap year.** Full range,
    /// every day. The `if and only if` is the part worth having: a day-of-year that saturates at 365,
    /// or that reaches 366 in a common year, both fail this.
    /// Falsification: replayable `crates/calendar/falsifications/verification.day_of_year_is_bounded_and_366_means_leap.patch`
    #[kani::proof]
    fn day_of_year_is_bounded_and_366_means_leap() {
        let days: i64 = kani::any();
        kani::assume((FIRST_DAY..=LAST_DAY).contains(&days));
        let (y, m, d) = civil_from_days(days);
        let c = Civil::new(y, m, d, 0, 0, 0).unwrap();
        let doy = c.day_of_year();
        assert!((1..=366).contains(&doy));
        assert_eq!(doy == 366, is_leap_year(y) && m == 12 && d == 31);
        assert!(doy <= 365 || is_leap_year(y));
        // The interesting value is reachable: without this, "366 implies leap year" would hold
        // vacuously on an implementation that never returns 366.
        kani::cover!(doy == 366);
    }

    /// **The week advances one day at a time and closes after seven.** Three facts, over every day in
    /// the range: consecutive days differ, the ISO number steps 1..=7 in order (not merely changes),
    /// and seven days on is the same weekday. The last is what makes it a cycle rather than a
    /// sequence, and together they pin the `rem_euclid` that a truncating `%` would break for
    /// pre-epoch dates.
    /// Falsification: replayable `crates/calendar/falsifications/verification.weekdays_advance_one_day_at_a_time.patch`
    #[kani::proof]
    fn weekdays_advance_one_day_at_a_time() {
        let days: i64 = kani::any();
        kani::assume((FIRST_DAY..LAST_DAY).contains(&days));
        let today = Weekday::from_days(days);
        let tomorrow = Weekday::from_days(days + 1);
        assert!(today != tomorrow);
        assert_eq!(tomorrow.iso_number(), today.iso_number() % 7 + 1);
        assert_eq!(Weekday::from_days(days + 7), today);
    }

    /// **The seconds round trip**, which is the calendar theorem above plus the one thing it does not
    /// cover: the `86_400` split, and whether the code divides the way a calendar needs.
    ///
    /// Bounded to 1968-01-01T00:00:00Z through 1971-12-31T23:59:59Z, and the window is chosen rather
    /// than convenient. It straddles the epoch, so it contains negative timestamps, zero, and positive
    /// ones; in Rust `-1 / 86400` is `0` and the answer a calendar needs is `-1`, so truncating
    /// division here reports 1970-01-01 for every instant in the last day of 1969, and *only* a
    /// negative timestamp catches it. It is four years, so it contains a leap day (1968-02-29), all
    /// twelve month lengths, and three year boundaries.
    ///
    /// What the bound excludes is the *calendar* behaviour at other years, and that is exactly what
    /// `the_calendar_algorithms_are_mutual_inverses` proves over the full range. The composition is
    /// stated in the module comment above.
    /// Falsification: replayable `crates/calendar/falsifications/verification.unix_to_civil_and_back_is_the_identity.patch`
    #[kani::proof]
    fn unix_to_civil_and_back_is_the_identity() {
        const START: i64 = days_from_civil(1968, 1, 1) * SECS_PER_DAY;
        const END: i64 = days_from_civil(1971, 12, 31) * SECS_PER_DAY + SECS_PER_DAY - 1;
        let secs: i64 = kani::any();
        kani::assume((START..=END).contains(&secs));

        let c = Civil::from_unix(secs).expect("in range");
        assert!(c.hour() <= 23 && c.minute() <= 59 && c.second() <= 59);
        assert_eq!(c.to_unix(), secs);
        // The two things the window was chosen for are reachable inside it, checked rather than
        // asserted in prose: a pre-epoch timestamp (where truncating division breaks) and a leap day.
        kani::cover!(secs < 0);
        kani::cover!(c.month() == 2 && c.day() == 29);
    }

    /// **Formatting is total, never truncates, and is always printable ASCII**, for every format, at
    /// every legal UTC offset, over one day of timestamps.
    ///
    /// It proves the fixed buffer is big enough (the writer's silent-drop branch is never taken for
    /// any input it can be handed), and it is what makes `Formatted::as_str`'s `unwrap_or("")`
    /// unreachable rather than an untested fallback. That last step is one line of composition rather
    /// than a proof through `core::str::from_utf8`: **ASCII is a subset of UTF-8**, so a buffer whose
    /// every byte is ASCII decodes, always. Proving it *through* the standard library's validator
    /// instead costs minutes of symbolic execution and adds nothing.
    /// Falsification: replayable `crates/calendar/falsifications/verification.every_format_is_ascii.patch`
    #[kani::proof]
    #[kani::unwind(34)]
    fn every_format_is_ascii() {
        let Some(dt) = any_datetime() else { return };
        printable(&dt.format(Format::Rfc3339));
        printable(&dt.format(Format::Date));
        printable(&dt.format(Format::Time));
        printable(&dt.format(Format::Human));
        printable(&dt.format(Format::Unix));
    }

    /// The check `every_format_is_ascii` applies, written as an **index** loop over the fixed buffer
    /// rather than a `for` over the variable-length slice.
    ///
    /// That is not a style preference, it is the difference between a proof that finishes and one that
    /// does not. Iterating a slice whose *length* is symbolic makes CBMC branch on the length at every
    /// step, and symbolic execution alone ran past ten minutes; the same property over the fixed array
    /// with an `i < len` guard verifies in seconds. Worth knowing before writing the next harness over
    /// a `&[u8]`.
    fn printable(f: &Formatted) {
        assert!(f.len as usize <= FMT_CAP);
        assert!(f.len > 0, "no format is empty");
        let mut i = 0;
        while i < FMT_CAP {
            if i < f.len as usize {
                let b = f.buf[i];
                assert!(b.is_ascii() && b >= 0x20, "output must be printable ASCII");
            }
            i += 1;
        }
    }

    /// **The `Unix` format fits any `i64`**, `i64::MIN` included, where the `-` and twenty digits are
    /// the longest a signed 64-bit number can be. Separate from the harness above because this arm's
    /// length depends on the magnitude of the number, which is precisely what a fixed buffer is at
    /// risk from, and unbounded because the digit loop is where that risk lives.
    /// Falsification: replayable `crates/calendar/falsifications/verification.the_unix_format_fits_any_i64.patch`
    #[kani::proof]
    #[kani::unwind(21)]
    fn the_unix_format_fits_any_i64() {
        let secs: i64 = kani::any();
        let mut w = Writer::new();
        w.signed(secs);
        let f = w.finish();
        assert!(f.len as usize <= 20); // "-9223372036854775808"
        assert!(f.len as usize <= FMT_CAP);
    }

    /// **The parser never panics on arbitrary bytes.** Every input byte is symbolic, at every length
    /// up to 26 (enough for a full `YYYY-MM-DDTHH:MM:SS.f+HH:MM` and more). It may return any
    /// `Error`; what it may not do is index out of bounds, overflow, or divide by zero.
    ///
    /// A `date -s` argument and, later, anything an NTP exchange hands over are bytes this program did
    /// not write, so totality here is a security property rather than tidiness. Nothing is assumed
    /// about the bytes at all: they need not be UTF-8, let alone ASCII, which is why the harness goes
    /// through [`DateTime::parse_rfc3339_bytes`].
    /// Falsification: replayable `crates/calendar/falsifications/verification.parse_is_total_on_hostile_bytes.patch`
    #[kani::proof]
    #[kani::unwind(28)]
    fn parse_is_total_on_hostile_bytes() {
        const N: usize = 26;
        let bytes: [u8; N] = kani::any();
        let len: usize = kani::any();
        kani::assume(len <= N);
        // Totality is the claim, and a panic is what turns it red. The assertion states the one thing
        // a success implies about the input: the shortest legal text is 20 bytes. A `cover!` alone
        // can never fail a Kani harness (Kani 0.67.0), so the success path is also asserted on.
        if DateTime::parse_rfc3339_bytes(&bytes[..len]).is_ok() {
            assert!(len >= 20);
        }
        // Not vacuous: some input in this set actually parses, so the harness is proving the
        // success path total and not merely that every early return returns.
        kani::cover!(DateTime::parse_rfc3339_bytes(&bytes[..len]).is_ok());
    }

    /// **Everything this crate prints, it can read back**, for every representable `DateTime`: every
    /// date in the ten-thousand-year range, at every minute of offset from -23:59 to +23:59.
    ///
    /// This is what makes RFC 3339 usable as the system's interchange form: a timestamp `date` wrote
    /// and `date -s` read is the same instant *and* the same offset (the equality is on the whole
    /// `DateTime`, so an offset silently normalised away would fail it).
    /// Falsification: replayable `crates/calendar/falsifications/verification.rfc3339_output_parses_back_to_itself.patch`
    #[kani::proof]
    #[kani::unwind(28)]
    fn rfc3339_output_parses_back_to_itself() {
        let Some(dt) = any_datetime() else { return };
        let text = dt.format(Format::Rfc3339);
        assert_eq!(DateTime::parse_rfc3339_bytes(text.as_bytes()), Ok(dt));
        // Both output shapes are reachable: the 20-byte `Z` form and the 25-byte `±HH:MM` form. They
        // are different code on both sides, so a harness that only ever saw one would prove half.
        kani::cover!(dt.offset().minutes() == 0);
        kani::cover!(dt.offset().minutes() != 0);
    }
}
