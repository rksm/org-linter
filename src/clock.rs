use chrono::{prelude::*, Duration};
use chrono_tz::Tz;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TimestampType {
    Active,
    Inactive,
}

impl TimestampType {
    fn open(&self) -> char {
        match self {
            TimestampType::Active => '<',
            TimestampType::Inactive => '[',
        }
    }

    fn close(&self) -> char {
        match self {
            TimestampType::Active => '>',
            TimestampType::Inactive => ']',
        }
    }
}

impl From<char> for TimestampType {
    fn from(c: char) -> Self {
        if c == '<' {
            Self::Active
        } else {
            Self::Inactive
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Clock<'a> {
    pub line: usize,
    pub parent: usize,
    pub duration_string: Option<&'a str>,
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
    pub timestamp_type: TimestampType,
}

impl<'a> std::fmt::Display for Clock<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_open = self.timestamp_type.open();
        let type_close = self.timestamp_type.close();
        write!(
            f,
            "{type_open}{}{type_close}",
            self.start.format("%Y-%m-%d %a %H:%M")
        )?;
        if let Some(end) = self.end {
            write!(
                f,
                "--{type_open}{}{type_close} => {:>5}",
                end.format("%Y-%m-%d %a %H:%M"),
                self.duration_formatted()
            )?;
        }
        Ok(())
    }
}

impl<'a> Clock<'a> {
    pub fn is_running(&self) -> bool {
        self.end.is_none()
    }

    pub fn duration(&self) -> Duration {
        let Some(end) = self.end else {return Duration::zero()};
        end - self.start
    }

    pub fn duration_formatted(&self) -> String {
        let d = self.duration();
        let negative = d < Duration::zero();
        let hours = self.duration().num_hours().abs();
        let minutes = self.duration().num_minutes().abs() - hours * 60;
        format!("{}{hours}:{minutes:0>2}", if negative { "-" } else { "" })
    }

    /// Does the specified duration matche start->end?
    pub fn matches_duration(&self) -> bool {
        if self.is_running() {
            return true;
        }
        let Some(duration_string) = self.duration_string else { return false };
        let Some((h, m)) = duration_string.split_once(':') else { return false };
        let negative = h.starts_with('-');
        let parsed = Duration::hours(i64::abs(h.parse().unwrap_or(0)))
            + Duration::minutes(m.parse().unwrap_or(0));
        let parsed = if negative { -parsed } else { parsed };
        let (start, end) = start_end(self.start, self.end);
        let actual = end - start;
        parsed == actual
    }

    pub fn overlaps<'o>(&self, other: &Clock<'o>) -> bool {
        let (start, end) = start_end(self.start, self.end);
        let (other_start, other_end) = start_end(other.start, other.end);
        if end <= other_start || start >= other_end {
            return false;
        }
        true
    }
}

#[inline]
pub(crate) fn tz_for_date(d: NaiveDate) -> Tz {
    static TZ_CUTOFF_DATE: Lazy<NaiveDate> =
        Lazy::new(|| NaiveDate::parse_from_str("2019-05-01", "%Y-%m-%d").unwrap());
    if d < *TZ_CUTOFF_DATE {
        chrono_tz::US::Pacific
    } else {
        chrono_tz::Europe::Berlin
    }
}

#[inline]
pub(crate) fn start_end(
    start: NaiveDateTime,
    end: Option<NaiveDateTime>,
) -> (DateTime<Tz>, DateTime<Tz>) {
    let tz = tz_for_date(start.date());
    let start = start.and_local_timezone(tz).unwrap();
    let end = end
        .unwrap_or_else(|| Local::now().naive_local())
        .and_local_timezone(tz)
        .unwrap();
    (start, end)
}

pub(crate) static CLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
\s*clock:\s*                                      # CLOCK:
([\[<])                                           # < or [ timestamp type
([0-9]{4})-([0-9]{2})-([0-9]{2})                  # yyyy-mm-dd
\s+[a-z]+\s+                                      # day of week (can be localized)
([0-9]{2}):([0-9]{2})                             # HH:MM
[\]>]                                             # > or ]
(?:\s*--\s*                                       # parse end timestamp
[\[<]
([0-9]{4})-([0-9]{2})-([0-9]{2})                  # yyyy-mm-dd
\s+[a-z]+\s+                                      # day of week (can be localized)
([0-9]{2}):([0-9]{2})                             # HH:MM
[\]>]
)?
(?:\s*=>\s*                                       # parse duration
(-?[0-9]{1,2}:[0-9]{2})
)?
",
    )
    .expect("clock re")
});

impl<'a> TryFrom<&'a str> for Clock<'a> {
    type Error = anyhow::Error;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        if let Some(captures) = CLOCK_RE.captures(s) {
            fn datetime(
                year: &str,
                month: &str,
                day: &str,
                hour: &str,
                min: &str,
            ) -> anyhow::Result<NaiveDateTime> {
                let year = year.parse()?;
                let month = month.parse()?;
                let day = day.parse()?;
                let local = Local
                    .with_ymd_and_hms(year, month, day, 0, 0, 0)
                    .single()
                    .unwrap();
                let tz = tz_for_date(local.date_naive());
                let local = tz.with_ymd_and_hms(year, month, day, hour.parse()?, min.parse()?, 0);
                let Some(d) = local.earliest().or_else(|| local.latest()) else {
                    return Err(anyhow::anyhow!("unable create date"))
                };
                Ok(d.naive_local())
            }

            let full = captures.get(0).unwrap().as_str();

            let timestamp_type = captures
                .get(1)
                .unwrap()
                .as_str()
                .chars()
                .next()
                .unwrap()
                .into();

            let start = datetime(
                captures.get(2).unwrap().as_str(),
                captures.get(3).unwrap().as_str(),
                captures.get(4).unwrap().as_str(),
                captures.get(5).unwrap().as_str(),
                captures.get(6).unwrap().as_str(),
            )
            .map_err(|err| {
                error!("error parsing start: {full:?}");
                anyhow::anyhow!("error parsing start: {err}")
            })?;

            let end = if let (
                Some(end_year),
                Some(end_month),
                Some(end_day),
                Some(end_hour),
                Some(end_min),
            ) = (
                captures.get(7).map(|c| c.as_str()),
                captures.get(8).map(|c| c.as_str()),
                captures.get(9).map(|c| c.as_str()),
                captures.get(10).map(|c| c.as_str()),
                captures.get(11).map(|c| c.as_str()),
            ) {
                Some(
                    datetime(end_year, end_month, end_day, end_hour, end_min).map_err(|err| {
                        error!("error parsing end: {full:?}");
                        anyhow::anyhow!("error parsing end: {err}")
                    })?,
                )
            } else {
                None
            };

            let duration_string = captures.get(12).map(|c| c.as_str());

            Ok(Clock {
                parent: 0,
                line: 0,
                start,
                end,
                duration_string,
                timestamp_type,
            })
        } else {
            Err(anyhow::anyhow!("unable to parse as clock: {s:?}"))
        }
    }
}

#[cfg(test)]
pub(crate) mod clock_tests {
    use chrono::NaiveDateTime;

    use super::Clock;

    #[test]
    fn test_parse_clock() {
        let inputs = [
            "CLOCK: [2021-04-18 Sun 00:57]--[2021-04-18 Sun 02:30] =>  1:33",
            "CLOCK: [2021-04-18 Sun 00:57]",
            "clock:[2021-04-18 Sun 00:57]--[2021-04-18 Sun 02:30] => 0:00",
        ];
        let result = inputs
            .into_iter()
            .map(|input| Clock::try_from(input).expect("parse clock"))
            .collect::<Vec<_>>();

        let expected_start =
            NaiveDateTime::parse_from_str("2021-04-18 00:57", "%Y-%m-%d %H:%M").unwrap();
        assert_eq!(result[0].start, expected_start);
        assert_eq!(
            result[0].end,
            Some(NaiveDateTime::parse_from_str("2021-04-18 02:30", "%Y-%m-%d %H:%M").unwrap())
        );
        assert!(result[0].matches_duration());

        assert_eq!(result[1].start, expected_start);
        assert_eq!(result[1].end, None);
        assert_eq!(result[2].start, expected_start);
    }

    #[test]
    fn test_parse_negative() {
        let clock =
            Clock::try_from("CLOCK: [2021-04-18 Sun 01:57]--[2021-04-18 Sun 00:47] =>  -1:10")
                .expect("parse clock");
        assert!(clock.matches_duration());
    }

    #[test]
    fn test_overlaps() {
        let clock1 =
            Clock::try_from("CLOCK: [2021-04-18 Sun 00:57]--[2021-04-18 Sun 01:47]").unwrap();
        let clock2 =
            Clock::try_from("CLOCK: [2021-04-18 Sun 01:20]--[2021-04-18 Sun 01:30]").unwrap();
        let clock3 =
            Clock::try_from("CLOCK: [2021-04-18 Sun 01:46]--[2021-04-18 Sun 01:48]").unwrap();
        let clock4 =
            Clock::try_from("CLOCK: [2021-04-18 Sun 01:47]--[2021-04-18 Sun 01:48]").unwrap();
        assert!(clock1.overlaps(&clock1));
        assert!(clock1.overlaps(&clock2));
        assert!(clock2.overlaps(&clock1));
        assert!(clock3.overlaps(&clock1));
        assert!(!clock4.overlaps(&clock1));
    }
}
