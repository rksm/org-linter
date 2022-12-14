use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
    path::PathBuf,
};

use crate::{Clock, Headline, OrgDocument};

#[derive(Debug, Clone, Copy)]
pub enum ConflictResolution {
    ShrinkEarlier,
    ShrinkLater,
    SplitContaining,
    RemoveInner,
    Auto,
    Skip,
}

impl ConflictResolution {
    pub fn explanation(&self) -> &'static str {
        match self {
            Self::ShrinkEarlier => "Shrink earlier timestamp",
            Self::ShrinkLater => "Shrink later timestamp",
            Self::SplitContaining => "Split the outer timestamp",
            Self::RemoveInner => "Remove the inner timestamp",
            Self::Auto => "Merge timestamps",
            Self::Skip => "Skip",
        }
    }
}

#[derive(Clone, Debug, Eq)]
pub struct ClockConflict<'a> {
    clock1: &'a Clock<'a>,
    clock2: &'a Clock<'a>,
    headline1: &'a Headline<'a>,
    headline2: &'a Headline<'a>,
    file1: &'a PathBuf,
    file2: &'a PathBuf,
}

impl<'a> PartialEq for ClockConflict<'a> {
    fn eq(&self, other: &Self) -> bool {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash1 = hasher.finish();
        let mut hasher = DefaultHasher::new();
        other.hash(&mut hasher);
        let hash2 = hasher.finish();
        hash1 == hash2
    }
}

impl<'a> std::hash::Hash for ClockConflict<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self {
            clock1,
            clock2,
            file1,
            file2,
            ..
        } = self;
        let (file1, file2) = if file1 < file2 {
            (*file1, *file2)
        } else {
            (*file2, *file1)
        };
        let (clock1, clock2) = if clock1.line < clock2.line {
            (*clock1, *clock2)
        } else {
            (*clock2, *clock1)
        };
        file1.hash(state);
        file2.hash(state);
        clock1.hash(state);
        clock2.hash(state);
    }
}

impl<'a> ClockConflict<'a> {
    pub fn find_conflicts(
        org_docs: &'a [OrgDocument<'a>],
    ) -> impl Iterator<Item = ClockConflict<'a>> + 'a {
        let mut clocks = Vec::new();

        for doc in org_docs {
            for clock in &doc.clocks {
                let headline = &doc.headlines[clock.parent];
                clocks.push((&doc.file, headline, clock));
            }
        }

        ClockConflictIterator {
            data: clocks,
            last_i: 0,
            last_j: 0,
            seen: Default::default(),
        }
    }

    pub fn report(&self) -> String {
        let Self {
            clock1,
            clock2,
            headline1,
            headline2,
            file1,
            file2,
        } = self;
        let line1 = clock1.line;
        let line2 = clock2.line;
        let title1 = headline1.title;
        let title2 = headline2.title;
        format!(
            "OVERLAPPING TIME\n  {clock1} {title1:?} {}:{line1}\n  {clock2} {title2:?} {}:{line2}",
            file1.display(),
            file2.display()
        )
    }

    pub fn hashme(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    pub fn resolution_options(&self) -> Vec<ConflictResolution> {
        use ConflictResolution::*;

        let Self {
            clock1,
            clock2,
            headline1,
            headline2,
            file1,
            file2,
        } = self;

        if file1 == file2 && headline1.line == headline2.line {
            return vec![Auto, Skip];
        }

        let (earlier, later) = if clock1.start <= clock2.start {
            (clock1, clock2)
        } else {
            (clock2, clock1)
        };

        // intervals are not contained
        if earlier.end < later.end {
            return vec![ShrinkEarlier, ShrinkLater, Skip];
        }

        vec![RemoveInner, SplitContaining, Skip]
    }

    pub fn resolve(self, resolution: ConflictResolution) -> Vec<FileChange<'a>> {
        if matches!(resolution, ConflictResolution::Skip) {
            return Default::default();
        }

        let Self {
            clock1,
            clock2,
            headline1,
            headline2,
            file1,
            file2,
        } = self;

        let (clock1, clock2) = (clock1.clone(), clock2.clone());

        if file1 == file2 && headline1.line == headline2.line {
            let (mut keep_clock, delete_clock) = if clock1.line <= clock2.line {
                (clock1, clock2)
            } else {
                (clock2, clock1)
            };
            keep_clock.start = keep_clock.start.min(delete_clock.start);
            keep_clock.end = keep_clock.end.max(delete_clock.end);
            return vec![
                FileChange::update(file1, keep_clock),
                FileChange::delete(file2, delete_clock),
            ];
        }

        let (mut earlier, earlier_file, mut later, later_file) = if clock1.start <= clock2.start {
            (clock1, file1, clock2, file2)
        } else {
            (clock2, file2, clock1, file1)
        };

        // intervals are not contained
        if earlier.end < later.end {
            return match resolution {
                ConflictResolution::ShrinkEarlier => {
                    earlier.end = Some(later.start);
                    vec![FileChange::update(earlier_file, earlier)]
                }
                ConflictResolution::ShrinkLater => {
                    later.start = earlier.end.unwrap();
                    vec![FileChange::update(later_file, later)]
                }
                _ => panic!("invalid resolution {resolution:?}"),
            };
        }

        // earlier contains "later" interval
        match resolution {
            ConflictResolution::SplitContaining => {
                let mut third = earlier.clone();
                // third.line += 1;
                third.start = later.end.unwrap();
                earlier.end = Some(later.start);
                vec![
                    FileChange::update(earlier_file, earlier),
                    FileChange::add(earlier_file, third),
                ]
            }
            ConflictResolution::RemoveInner => {
                vec![FileChange::delete(later_file, later)]
            }
            _ => panic!("invalid resolution {resolution:?}"),
        }
    }
}

// -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-

#[derive(Debug)]
pub enum FileChange<'a> {
    DeletedClock { file: PathBuf, clock: Clock<'a> },
    AddedClock { file: PathBuf, clock: Clock<'a> },
    UpdateClock { file: PathBuf, clock: Clock<'a> },
}

impl<'a> FileChange<'a> {
    fn add(file: impl Into<PathBuf>, clock: Clock<'a>) -> Self {
        Self::AddedClock {
            file: file.into(),
            clock,
        }
    }

    fn delete(file: impl Into<PathBuf>, clock: Clock<'a>) -> Self {
        Self::DeletedClock {
            file: file.into(),
            clock,
        }
    }

    fn update(file: impl Into<PathBuf>, clock: Clock<'a>) -> Self {
        Self::UpdateClock {
            file: file.into(),
            clock,
        }
    }

    #[inline]
    fn clock(&self) -> &Clock<'a> {
        match self {
            FileChange::DeletedClock { clock, .. } => clock,
            FileChange::AddedClock { clock, .. } => clock,
            FileChange::UpdateClock { clock, .. } => clock,
        }
    }

    #[inline]
    fn line(&self) -> usize {
        self.clock().line
    }

    #[inline]
    fn file(&self) -> &PathBuf {
        match self {
            FileChange::DeletedClock { file, .. } => file,
            FileChange::AddedClock { file, .. } => file,
            FileChange::UpdateClock { file, .. } => file,
        }
    }

    pub fn fixup_headline<'b>(&self, headline: &mut Headline<'b>) {
        if headline.line < self.line() {
            return;
        }
        if headline.line == self.line() {
            panic!("file change modifies line number of headline. This is not supported.");
        }
        match self {
            FileChange::DeletedClock { .. } => headline.line -= 1,
            FileChange::AddedClock { .. } => headline.line += 1,
            _ => {}
        }
    }

    pub fn fixup_clock<'b>(&self, mut clock: Clock<'b>) -> Option<Clock<'b>> {
        if clock.line < self.line() {
            return Some(clock);
        }
        match self {
            FileChange::DeletedClock { .. } => {
                if clock.line == self.line() {
                    None
                } else {
                    clock.line -= 1;
                    Some(clock)
                }
            }
            FileChange::AddedClock { .. } => {
                clock.line += 1;
                Some(clock)
            }
            _ => Some(clock),
        }
    }

    fn modify_file_content(&self, content: Cow<str>) -> String {
        let target_line = self.line() - 1;
        let mut result = String::new();
        for (line_no, line) in content.lines().enumerate() {
            if line_no == target_line {
                match self {
                    FileChange::DeletedClock { .. } => {
                        continue;
                    }
                    FileChange::UpdateClock { clock, .. } => {
                        result.push_str("CLOCK: ");
                        result.push_str(&format!("{clock}\n"));
                        continue;
                    }
                    FileChange::AddedClock { clock, .. } => {
                        result.push_str("CLOCK: ");
                        result.push_str(&format!("{clock}\n"));
                    }
                }
            }
            result.push_str(line);
            result.push('\n');
        }
        result
    }

    pub fn apply_to_string(mut changes: Vec<Self>, file_content: &str) -> anyhow::Result<Cow<str>> {
        if changes.is_empty() {
            return Ok(Cow::Borrowed(file_content));
        }

        // order from largest line no to smallest so we can apply in order without fixups
        changes.sort_by(|a, b| {
            use Ordering::*;
            let line1 = a.line();
            let line2 = b.line();
            if line1 < line2 {
                return Less;
            }
            if line1 > line2 {
                return Greater;
            }
            match (a, b) {
                (FileChange::AddedClock { .. }, FileChange::AddedClock { .. }) => Equal,
                (FileChange::AddedClock { .. }, _) => Less,
                (_, FileChange::AddedClock { .. }) => Greater,
                _ => Equal,
            }
        });
        changes.reverse();

        let file = changes[0].file().clone();
        for c in changes.iter().skip(1) {
            if &file != c.file() {
                return Err(anyhow::anyhow!("changes don't point to the same file"));
            }
        }

        let mut result = file_content.to_string();
        for c in changes {
            result = c.modify_file_content(result.into());
        }

        Ok(Cow::Owned(result))
    }

    pub fn apply(changes: Vec<Self>) -> anyhow::Result<()> {
        if changes.is_empty() {
            return Ok(());
        }
        let file = changes[0].file().clone();

        let content = std::fs::read_to_string(&file)?;
        let result = Self::apply_to_string(changes, &content)?;
        std::fs::write(&file, &*result)?;

        Ok(())
    }
}

struct ClockConflictIterator<'a> {
    data: Vec<(&'a PathBuf, &'a Headline<'a>, &'a Clock<'a>)>,
    last_i: usize,
    last_j: usize,
    seen: HashSet<u64>,
}

impl<'a> Iterator for ClockConflictIterator<'a> {
    type Item = ClockConflict<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for (i, (file1, headline1, clock1)) in self.data.iter().enumerate() {
            if i < self.last_i {
                continue;
            }
            for (j, (file2, headline2, clock2)) in self.data.iter().enumerate() {
                if i == self.last_i && j < self.last_j {
                    continue;
                }
                if i != j && clock1.overlaps(clock2) {
                    let conflict = ClockConflict {
                        clock1,
                        clock2,
                        headline1,
                        headline2,
                        file1,
                        file2,
                    };

                    // Don't report duplicates when finding reversed pair
                    let hash = conflict.hashme();
                    if self.seen.contains(&hash) {
                        continue;
                    }

                    self.last_i = i;
                    self.last_j = j;
                    self.seen.insert(hash);
                    return Some(conflict);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{clock_conflict::ConflictResolution, ClockConflict, FileChange, OrgDocument};

    #[test]
    fn resolve_conflict_by_joining_times() {
        let org_string = "
* fooo
CLOCK: [2022-12-12 Mon 10:45]--[2022-12-12 Mon 10:55] =>  0:10
CLOCK: [2022-12-12 Mon 10:40]--[2022-12-12 Mon 10:50] =>  0:10
";

        let docs = &[OrgDocument::parse(PathBuf::from("test.org"), org_string)];
        let conflicts = ClockConflict::find_conflicts(docs);
        assert_eq!(conflicts.count(), 1);

        let result = FileChange::apply_to_string(
            ClockConflict::find_conflicts(docs)
                .next()
                .unwrap()
                .resolve(ConflictResolution::Auto),
            org_string,
        )
        .expect("apply changes");
        let expected = "
* fooo
CLOCK: [2022-12-12 Mon 10:40]--[2022-12-12 Mon 10:55] =>  0:15
";
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_conflict_by_adjusting_time() {
        let org_string = "
* fooo
CLOCK: [2022-12-12 Mon 10:45]--[2022-12-12 Mon 10:55] =>  0:10
* bar
CLOCK: [2022-12-12 Mon 10:40]--[2022-12-12 Mon 10:50] =>  0:10
";

        let docs = &[OrgDocument::parse(PathBuf::from("test.org"), org_string)];
        let conflict = ClockConflict::find_conflicts(docs)
            .into_iter()
            .next()
            .unwrap();

        let changes = conflict.clone().resolve(ConflictResolution::ShrinkEarlier);
        let result = FileChange::apply_to_string(changes, org_string).expect("apply changes");
        let expected = "
* fooo
CLOCK: [2022-12-12 Mon 10:45]--[2022-12-12 Mon 10:55] =>  0:10
* bar
CLOCK: [2022-12-12 Mon 10:40]--[2022-12-12 Mon 10:45] =>  0:05
";
        assert_eq!(result, expected);

        let changes = conflict.resolve(ConflictResolution::ShrinkLater);
        let result = FileChange::apply_to_string(changes, org_string).expect("apply changes");
        let expected = "
* fooo
CLOCK: [2022-12-12 Mon 10:50]--[2022-12-12 Mon 10:55] =>  0:05
* bar
CLOCK: [2022-12-12 Mon 10:40]--[2022-12-12 Mon 10:50] =>  0:10
";
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_conflict_by_splitting() {
        let org_string = "
* fooo
CLOCK: [2022-12-12 Mon 10:45]--[2022-12-12 Mon 10:55] =>  0:10
* bar
CLOCK: [2022-12-12 Mon 10:40]--[2022-12-12 Mon 10:59] =>  0:10
";

        let docs = &[OrgDocument::parse(PathBuf::from("test.org"), org_string)];
        let conflict = ClockConflict::find_conflicts(docs)
            .into_iter()
            .next()
            .unwrap();

        let changes = conflict
            .clone()
            .resolve(ConflictResolution::SplitContaining);
        let result = FileChange::apply_to_string(changes, org_string).expect("apply changes");
        let expected = "
* fooo
CLOCK: [2022-12-12 Mon 10:45]--[2022-12-12 Mon 10:55] =>  0:10
* bar
CLOCK: [2022-12-12 Mon 10:55]--[2022-12-12 Mon 10:59] =>  0:04
CLOCK: [2022-12-12 Mon 10:40]--[2022-12-12 Mon 10:45] =>  0:05
";
        assert_eq!(result, expected);
    }
}
