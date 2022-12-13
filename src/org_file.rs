use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::org_document::OrgDocument;

#[derive(Debug)]
pub struct OrgFile {
    pub(crate) file: PathBuf,
    pub(crate) content: String,
}

impl OrgFile {
    pub fn from_file(file: impl AsRef<Path>) -> Result<Self> {
        let file = file.as_ref().to_path_buf();
        let content = std::fs::read_to_string(&file)?;
        Ok(Self { file, content })
    }

    pub fn document(&self) -> OrgDocument {
        trace!("parsing file {:?}", self.file);
        OrgDocument::parse(&self.content)
    }
}
