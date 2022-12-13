use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct Headline<'a> {
    pub line: usize,
    pub parent: usize,
    pub level: usize,
    pub title: &'a str,
    pub tags_string: Option<&'a str>,
}

pub(crate) static HEADLINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
(\*+)   # parse *
\s*
(.+)    # title + tags
",
    )
    .expect("headline re")
});

pub(crate) static TITLE_TAGS_RE: Lazy<Regex> = Lazy::new(|| {
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
pub(crate) mod headline_tests {
    use super::Headline;

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
