use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;

pub struct Block<'a> {
    pub start_line: usize,
    pub end_line: usize,
    pub kind: &'a str,
}

impl<'a> Block<'a> {
    pub(crate) fn parse_end(&mut self, line: &str, line_no: usize) -> bool {
        if let Some(captures) = BLOCK_END_RE.captures(line) {
            let kind = captures.get(1).unwrap().as_str();
            if kind == self.kind {
                self.end_line = line_no;
                return true;
            }
        }
        false
    }
}

pub(crate) static BLOCK_START_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\s*#\+begin_([^\s]+)").expect("block start re"));

pub(crate) static BLOCK_END_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\s*#\+end_([^\s]+)").expect("block end re"));

impl<'a> TryFrom<&'a str> for Block<'a> {
    type Error = anyhow::Error;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        if let Some(captures) = BLOCK_START_RE.captures(s) {
            let kind = captures.get(1).unwrap().as_str();
            Ok(Self {
                start_line: 0,
                end_line: 0,
                kind,
            })
        } else {
            Err(anyhow::anyhow!("not a block"))
        }
    }
}
