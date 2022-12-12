use std::{path::PathBuf, str::Lines};

use anyhow::Result;
use chrono::{prelude::*, Duration};
use once_cell::sync::Lazy;
use regex::Regex;

// -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-

pub struct OrgFile {
    file_name: PathBuf,
    content: String,
}

#[derive(Debug)]
pub struct OrgDocument<'a> {
    headlines: Vec<Headline<'a>>,
    clocks: Vec<Clock<'a>>,
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
                            eprintln!(
                                "WARNING: found clock on line {i}. Previous clock was on line {}",
                                last_clock.line
                            );
                        }
                    }
                    clocks.push(clock);
                } else {
                    eprintln!("WARNING: found clock on line {i} but have no headline");
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
    line: usize,
    parent: usize,
    level: usize,
    title: &'a str,
    tags_string: Option<&'a str>,
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
    line: usize,
    parent: usize,
    start_string: &'a str,
    end_string: Option<&'a str>,
    duration_string: Option<&'a str>,
    start: NaiveDateTime,
    end: Option<NaiveDateTime>,
}

impl<'a> Clock<'a> {
    /// Does the specified duration matche start->end?
    pub fn matches_duration(&self) -> bool {
        let (Some(duration_string), Some(end)) = (self.duration_string, self.end) else {return false};
        let Some((h, m)) = duration_string.split_once(':') else {return false};
        (Duration::hours(h.parse().unwrap_or(0)) + Duration::minutes(m.parse().unwrap_or(0)))
            == (end - self.start)
    }
}

// -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-
// parsing

static CLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
\s*clock:\s*                                      # CLOCK:
[\[<]                                             # < or [
([0-9]{4}-[0-9]{2}-[0-9]{2}\s+[a-z]+\s+[0-9:]+)   # timestamp like [2022-12-12 Mon 19:49]
[\]>]                                             # > or ]
(?:--                                             # parse end timestamp
[\[<]
([0-9]{4}-[0-9]{2}-[0-9]{2}\s+[a-z]+\s+[0-9:]+)
[\]>]
)?
(?:\s*=>\s*                                       # parse duration
([0-9]{1,2}:[0-9]{2})
)?
",
    )
    .expect("clock re")
});

const TIME_FORMAT: &str = "%Y-%m-%d %a %H:%M";

impl<'a> TryFrom<&'a str> for Clock<'a> {
    type Error = anyhow::Error;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        if let Some(captures) = CLOCK_RE.captures(s) {
            let start_string = &captures.get(1).unwrap().as_str();
            let end_string = captures.get(2).map(|m| m.as_str());
            let duration_string = captures.get(3).map(|m| m.as_str());
            let start = NaiveDateTime::parse_from_str(start_string, TIME_FORMAT)
                .map_err(|err| anyhow::anyhow!("error parsing start: {err}"))?;
            let end = if let Some(end_string) = end_string {
                Some(
                    NaiveDateTime::parse_from_str(end_string, TIME_FORMAT)
                        .map_err(|err| anyhow::anyhow!("error parsing end: {err}"))?,
                )
            } else {
                None
            };

            Ok(Clock {
                parent: 0,
                line: 0,
                start,
                end,
                start_string,
                end_string,
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
}
