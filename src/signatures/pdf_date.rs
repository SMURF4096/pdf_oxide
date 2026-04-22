//! PDF date-string parsing (ISO 32000-1 §7.9.4 "Dates").
//!
//! PDF dates look like `D:YYYYMMDDHHmmSSOHH'mm'` where `O` is the UTC
//! offset sign (`+`, `-`, or `Z` for UTC). Trailing components after
//! the year are optional; missing ones default to zero/UTC.
//!
//! This lives alongside the signature code because the /M entry on a
//! signature dictionary is a PDF date and every binding's `Signature`
//! surface wants a numeric timestamp rather than the raw string.

/// Parse a PDF date string into a Unix timestamp (seconds since epoch).
///
/// Returns `None` if the string doesn't match the PDF date grammar.
/// Leading `D:` prefix is optional (as seen in some producers). Time
/// components are optional and default to zero. Timezone defaults to
/// UTC when absent.
pub fn parse_pdf_date_to_epoch(s: &str) -> Option<i64> {
    let raw = s.strip_prefix("D:").unwrap_or(s);
    let bytes = raw.as_bytes();
    if bytes.len() < 4 {
        return None;
    }

    let year = parse_digits(bytes, 0, 4)?;
    let month = parse_digits(bytes, 4, 2).unwrap_or(1);
    let day = parse_digits(bytes, 6, 2).unwrap_or(1);
    let hour = parse_digits(bytes, 8, 2).unwrap_or(0);
    let minute = parse_digits(bytes, 10, 2).unwrap_or(0);
    let second = parse_digits(bytes, 12, 2).unwrap_or(0);

    // Offset handling starts after the seconds field.
    let tz_offset = parse_offset(bytes, 14)?;

    let days = days_from_civil(year, month, day)?;
    let seconds_utc =
        days * 86_400 + i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second);

    // PDF offsets are local - UTC; subtract to convert the local wall
    // time into UTC epoch seconds.
    Some(seconds_utc - tz_offset)
}

fn parse_digits(bytes: &[u8], start: usize, len: usize) -> Option<u32> {
    if start + len > bytes.len() {
        return None;
    }
    let mut value: u32 = 0;
    for b in &bytes[start..start + len] {
        if !b.is_ascii_digit() {
            return None;
        }
        value = value * 10 + u32::from(*b - b'0');
    }
    Some(value)
}

fn parse_offset(bytes: &[u8], start: usize) -> Option<i64> {
    if start >= bytes.len() {
        return Some(0);
    }
    match bytes[start] {
        b'Z' => Some(0),
        sign @ (b'+' | b'-') => {
            let hours = parse_digits(bytes, start + 1, 2)?;
            // Minutes can be `mm'` or `mm'mm'` variants; we skip the
            // apostrophe and read two more digits if present.
            let minutes_start = start + 3;
            let minutes_start = if bytes.get(minutes_start) == Some(&b'\'') {
                minutes_start + 1
            } else {
                minutes_start
            };
            let minutes = parse_digits(bytes, minutes_start, 2).unwrap_or(0);
            let total = i64::from(hours) * 3600 + i64::from(minutes) * 60;
            Some(if sign == b'-' { -total } else { total })
        },
        _ => Some(0),
    }
}

/// Days from 1970-01-01 to the given civil date, following the
/// "date algorithms" by Howard Hinnant (public domain).
fn days_from_civil(year: u32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if month <= 2 {
        i64::from(year) - 1
    } else {
        i64::from(year)
    };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u64;
    let m = i64::from(month);
    let doy = ((153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1) as u64;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe as i64 - 719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-04-21 12:00:00 UTC → 1_776_772_800 s since epoch.
    const APR21_NOON_UTC: i64 = 1_776_772_800;
    // 2026-04-21 00:00:00 UTC → 1_776_729_600 s since epoch.
    const APR21_MIDNIGHT_UTC: i64 = 1_776_729_600;

    #[test]
    fn utc_z_suffix() {
        assert_eq!(parse_pdf_date_to_epoch("D:20260421120000Z"), Some(APR21_NOON_UTC));
    }

    #[test]
    fn no_d_prefix_still_parses() {
        assert_eq!(parse_pdf_date_to_epoch("20260421120000Z"), Some(APR21_NOON_UTC));
    }

    #[test]
    fn positive_offset_converts_to_utc() {
        // 14:00 +02 = 12:00 UTC.
        assert_eq!(
            parse_pdf_date_to_epoch("D:20260421140000+02'00'"),
            Some(APR21_NOON_UTC)
        );
    }

    #[test]
    fn negative_offset_converts_to_utc() {
        // 08:00 -04 = 12:00 UTC.
        assert_eq!(
            parse_pdf_date_to_epoch("D:20260421080000-04'00'"),
            Some(APR21_NOON_UTC)
        );
    }

    #[test]
    fn partial_date_defaults_zero_midnight_utc() {
        assert_eq!(parse_pdf_date_to_epoch("D:20260421"), Some(APR21_MIDNIGHT_UTC));
    }

    #[test]
    fn epoch_itself() {
        assert_eq!(parse_pdf_date_to_epoch("D:19700101000000Z"), Some(0));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_pdf_date_to_epoch(""), None);
        assert_eq!(parse_pdf_date_to_epoch("D:not-a-date"), None);
        assert_eq!(parse_pdf_date_to_epoch("D:"), None);
    }

    #[test]
    fn rejects_invalid_month_day() {
        assert_eq!(parse_pdf_date_to_epoch("D:20260099"), None);
        assert_eq!(parse_pdf_date_to_epoch("D:20260400"), None);
    }
}
