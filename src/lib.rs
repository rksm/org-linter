#[macro_use]
extern crate log;

use anyhow::Result;
use chrono::{prelude::*, Duration};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};

// -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-

#[derive(Debug)]
pub struct OrgFile {
    file: PathBuf,
    content: String,
}

impl OrgFile {
    pub fn from_file(file: impl AsRef<Path>) -> Result<Self> {
        let file = file.as_ref().to_path_buf();
        let content = std::fs::read_to_string(&file)?;
        Ok(Self { file, content })
    }

    pub fn document(&self) -> OrgDocument {
        info!("parsing file {:?}", self.file);
        OrgDocument::parse(&self.content)
    }
}

#[derive(Debug)]
pub struct OrgDocument<'a> {
    pub headlines: Vec<Headline<'a>>,
    pub clocks: Vec<Clock<'a>>,
}

impl<'a> OrgDocument<'a> {
    pub fn parse(content: &'a str) -> Self {
        let mut headlines = Vec::new();
        let mut clocks: Vec<Clock> = Vec::new();
        let mut parents: Vec<(usize, usize)> = Vec::new();

        for (i, line) in content.lines().enumerate() {
            if let Ok(mut headline) = Headline::try_from(line) {
                headline.line = i;
                while !parents.is_empty() {
                    let (_, level) = parents.last().unwrap();
                    if *level >= headline.level {
                        parents.pop();
                    } else {
                        break;
                    }
                }
                if let Some((index, _)) = parents.last() {
                    headline.parent = *index;
                }
                parents.push((headlines.len(), headline.level));
                headlines.push(headline);
                continue;
            }

            if let Ok(mut clock) = Clock::try_from(line) {
                clock.line = i;
                if let Some(&(index, _)) = parents.last() {
                    clock.parent = index;
                    if let Some(last_clock) = clocks.last() {
                        if last_clock.parent == index && last_clock.line != i - 1 {
                            warn!(
                                "WARNING: found clock on line {i}. Previous clock was on line {}",
                                last_clock.line
                            );
                        }
                    }
                    clocks.push(clock);
                } else {
                    warn!("WARNING: found clock on line {i} but have no headline");
                }
                continue;
            }
        }
        Self { headlines, clocks }
    }
}

// -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-
// headlines

#[derive(Debug)]
pub struct Headline<'a> {
    pub line: usize,
    pub parent: usize,
    pub level: usize,
    pub title: &'a str,
    pub tags_string: Option<&'a str>,
}

static HEADLINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
(\*+)   # parse *
\s*
(.+)    # title + tags
",
    )
    .expect("headline re")
});
static TITLE_TAGS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
(.+?)
\s+
(:[^\s]+:)\s*$
",
    )
    .expect("clock re")
});

impl<'a> TryFrom<&'a str> for Headline<'a> {
    type Error = anyhow::Error;
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        if let Some(captures) = HEADLINE_RE.captures(s) {
            let level = captures.get(1).unwrap().as_str().len();
            let title = captures.get(2).unwrap().as_str();

            let (title, tags_string) = if let Some(captures) = TITLE_TAGS_RE.captures(title) {
                (
                    captures.get(1).unwrap().as_str(),
                    Some(captures.get(2).unwrap().as_str()),
                )
            } else {
                (title, None)
            };

            Ok(Self {
                line: 0,
                parent: 0,
                level,
                title,
                tags_string,
            })
        } else {
            Err(anyhow::anyhow!("Not a headline"))
        }
    }
}

#[cfg(test)]
mod headline_tests {
    use crate::Headline;

    #[test]
    fn test_parse_headline() {
        let h = Headline::try_from("* foo").unwrap();
        assert_eq!(h.title, "foo");
        assert_eq!(h.level, 1);

        let h = Headline::try_from("*** foo: xxxx :bar:baz:").unwrap();
        assert_eq!(h.title, "foo: xxxx");
        assert_eq!(h.level, 3);
        assert_eq!(h.tags_string, Some(":bar:baz:"));
    }
}

// -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-
// clocks

#[derive(Debug, Clone)]
pub struct Clock<'a> {
    pub line: usize,
    pub parent: usize,
    pub duration_string: Option<&'a str>,
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
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
        let (Some(duration_string), Some(end)) = (self.duration_string, self.end) else {return false};
        let Some((h, m)) = duration_string.split_once(':') else {return false};
        let negative = h.starts_with('-');
        let parsed = Duration::hours(i64::abs(h.parse().unwrap_or(0)))
            + Duration::minutes(m.parse().unwrap_or(0));
        let parsed = if negative { -parsed } else { parsed };
        let actual = end - self.start;
        parsed == actual
    }
}

// -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-
// parsing

static CLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
\s*clock:\s*                                      # CLOCK:
[\[<]                                             # < or [
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
            ) -> Result<NaiveDateTime> {
                let Some(d) = Local.with_ymd_and_hms(year.parse()?, month.parse()?, day.parse()?, hour.parse()?, min.parse()?, 0).single() else {
                    return Err(anyhow::anyhow!("unable create date"))
                };
                Ok(d.naive_local())
            }

            let full = captures.get(0).unwrap().as_str();

            let start = datetime(
                captures.get(1).unwrap().as_str(),
                captures.get(2).unwrap().as_str(),
                captures.get(3).unwrap().as_str(),
                captures.get(4).unwrap().as_str(),
                captures.get(5).unwrap().as_str(),
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
                captures.get(6).map(|c| c.as_str()),
                captures.get(7).map(|c| c.as_str()),
                captures.get(8).map(|c| c.as_str()),
                captures.get(9).map(|c| c.as_str()),
                captures.get(10).map(|c| c.as_str()),
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

            let duration_string = captures.get(11).map(|c| c.as_str());

            Ok(Clock {
                parent: 0,
                line: 0,
                start,
                end,
                duration_string,
            })
        } else {
            Err(anyhow::anyhow!("unable to parse as clock: {s:?}"))
        }
    }
}

#[cfg(test)]
mod clock_tests {
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
}
