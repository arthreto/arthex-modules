/// A local date and time, in minutes. Reminders follow the clock on the wall, so a change of
/// season or of time zone moves nothing the user wrote.
pub type Moment = i64;

const MINUTES_PER_DAY: i64 = 1440;

/// Days since 1970-01-01 of a date of the proleptic Gregorian calendar.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

const fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

const fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

pub fn moment_of(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> Moment {
    days_from_civil(year, month, day) * MINUTES_PER_DAY + hour * 60 + minute
}

/// `YYYY-MM-DDTHH:MM`, the only form the interface writes. Anything else is refused.
pub fn parse(text: &str) -> Option<Moment> {
    let bytes = text.as_bytes();
    let well_formed = bytes.len() == 16
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15]
            .iter()
            .all(|&index| bytes[index].is_ascii_digit());
    if !well_formed {
        return None;
    }
    let number = |range: std::ops::Range<usize>| text[range].parse::<i64>().ok();
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute) = (number(11..13)?, number(14..16)?);
    let valid = (1..=12).contains(&month)
        && (1..=days_in_month(year, month)).contains(&day)
        && (0..24).contains(&hour)
        && (0..60).contains(&minute);
    valid.then(|| moment_of(year, month, day, hour, minute))
}

pub const fn day_of(moment: Moment) -> i64 {
    moment.div_euclid(MINUTES_PER_DAY)
}

pub const fn hour_and_minute(moment: Moment) -> (i64, i64) {
    let minutes = moment.rem_euclid(MINUTES_PER_DAY);
    (minutes / 60, minutes % 60)
}

/// The month and day of a moment.
pub fn month_and_day(moment: Moment) -> (i64, i64) {
    // The inverse of `days_from_civil`.
    let shifted = day_of(moment) + 719_468;
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_the_interface_writes() {
        assert_eq!(parse("1970-01-01T00:00"), Some(0));
        assert_eq!(parse("1970-01-02T01:30"), Some(1440 + 90));
    }

    #[test]
    fn refuses_anything_else() {
        for text in [
            "",
            "2026-13-01T10:00",
            "2026-02-30T10:00",
            "2026-11-06T24:00",
            "2026-11-06T10:60",
            "2026-11-06 10:00",
            "2026-11-0xT10:00",
            "2026-11-06T10:00:00",
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
    }

    #[test]
    fn knows_leap_years() {
        assert!(parse("2028-02-29T10:00").is_some());
        assert!(parse("2026-02-29T10:00").is_none());
        assert!(parse("2000-02-29T10:00").is_some());
        assert!(parse("1900-02-29T10:00").is_none());
    }

    #[test]
    fn counts_across_months_and_years() {
        let (Some(end_of_year), Some(new_year)) =
            (parse("2026-12-31T23:59"), parse("2027-01-01T00:00"))
        else {
            panic!("valid dates");
        };
        assert_eq!(new_year - end_of_year, 1);
    }

    #[test]
    fn gives_back_the_parts_of_a_moment() {
        let moment = parse("2028-02-29T18:05")
            .map(|moment| (month_and_day(moment), hour_and_minute(moment)));
        assert_eq!(moment, Some(((2, 29), (18, 5))));
    }
}
