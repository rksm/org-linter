use crate::block::Block;
use crate::clock::Clock;
use crate::headline::Headline;

#[derive(Debug)]
pub struct OrgDocument<'a> {
    pub headlines: Vec<Headline<'a>>,
    pub clocks: Vec<Clock<'a>>,
}

impl<'a> OrgDocument<'a> {
    pub fn parse(content: &'a str) -> Self {
        let mut headlines = Vec::new();
        let mut clocks: Vec<Clock> = Vec::new();
        let mut blocks: Vec<Block> = Vec::new();
        let mut parents: Vec<(usize, usize)> = Vec::new();
        let mut current_block = Option::<Block>::None;

        for (i, line) in content.lines().enumerate() {
            let line_no = i + 1;
            if let Some(mut block) = current_block.take() {
                if block.parse_end(line, line_no) {
                    blocks.push(block);
                } else {
                    current_block = Some(block);
                };
                continue;
            };

            if let Ok(mut block) = Block::try_from(line) {
                block.start_line = line_no;
                current_block = Some(block);
                continue;
            }

            if let Ok(mut headline) = Headline::try_from(line) {
                headline.line = line_no;
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
                clock.line = line_no;
                if let Some(&(index, _)) = parents.last() {
                    clock.parent = index;
                    if let Some(last_clock) = clocks.last() {
                        if last_clock.parent == index && last_clock.line != line_no - 1 {
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
