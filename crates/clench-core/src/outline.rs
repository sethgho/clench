//! The shape of a script as a reader sees it before and while reading:
//! sections (split at headings), the blocks inside them with their word
//! counts, a compact "terrain" of ticks for a document map, and reading
//! time estimates that can be recomputed at any speed.
//!
//! Timing is kept in two WPM-independent parts (see [`Timing`]), so a host
//! can store an outline and still say how long something takes at a speed
//! chosen later.

use alloc::string::String;
use alloc::vec::Vec;

use crate::document::{BlockKind, Document};
use crate::pacing::Pacing;
use crate::script::{Script, TokenKind};

/// Reading time in a form that does not depend on the speed: the sum of the
/// tokens' multiples of the base word duration ([`Pacing::token_permille`])
/// plus the fixed time of marker tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Timing {
    /// Summed word durations in thousandths of the base duration
    /// `60_000 / wpm` ms.
    pub beats: u64,
    /// Time that does not scale with the speed (markers), ms.
    pub fixed_ms: u64,
}

impl Timing {
    /// No time at all.
    pub const ZERO: Timing = Timing {
        beats: 0,
        fixed_ms: 0,
    };

    /// The estimated duration at `wpm` (used as given; clamp it first), ms.
    /// Each token's own duration is truncated to whole milliseconds by the
    /// player, so this is at most one millisecond per word above the sum of
    /// the player's token durations.
    pub fn ms_at(&self, wpm: u32) -> u64 {
        if wpm == 0 {
            return self.fixed_ms;
        }
        60 * self.beats / u64::from(wpm) + self.fixed_ms
    }

    /// The estimated duration at `wpm` clamped to `pacing`'s range, ms.
    pub fn ms_with(&self, pacing: &Pacing, wpm: u32) -> u64 {
        self.ms_at(pacing.clamp_wpm(wpm))
    }

    fn add(self, other: Timing) -> Timing {
        Timing {
            beats: self.beats + other.beats,
            fixed_ms: self.fixed_ms + other.fixed_ms,
        }
    }

    fn sub(self, other: Timing) -> Timing {
        Timing {
            beats: self.beats.saturating_sub(other.beats),
            fixed_ms: self.fixed_ms.saturating_sub(other.fixed_ms),
        }
    }
}

/// One block of the script, as the outline sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OutlineBlock {
    /// Index of the block in [`Document::blocks`].
    pub block: usize,
    /// The block's kind.
    pub kind: BlockKind,
    /// Heading level for headings.
    pub level: Option<u8>,
    /// Index of the section holding this block in [`Outline::sections`].
    pub section: usize,
    /// First token of the block.
    pub token_start: usize,
    /// One past the block's last token.
    pub token_end: usize,
    /// Author words read in this block (`0` for a marker).
    pub words: usize,
    /// Reading time of the block.
    pub timing: Timing,
}

/// A run of blocks that starts at a section heading (or at the start of the
/// document, for text before the first heading).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Section {
    /// The heading text, verbatim; `None` for the text before the first
    /// section heading.
    pub title: Option<String>,
    /// The heading's level; `None` without a heading.
    pub level: Option<u8>,
    /// First block of the section in [`Outline::blocks`].
    pub block_start: usize,
    /// One past the section's last block.
    pub block_end: usize,
    /// First token of the section.
    pub token_start: usize,
    /// One past the section's last token.
    pub token_end: usize,
    /// Author words read in the section, heading included.
    pub words: usize,
    /// Reading time of the section.
    pub timing: Timing,
}

/// One mark on a document map: a section heading, or one or more
/// consecutive non-heading blocks of the same section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tick {
    /// The tick is a section heading.
    pub heading: bool,
    /// Kind of the tick's first block.
    pub kind: BlockKind,
    /// Index of the tick's section in [`Outline::sections`].
    pub section: usize,
    /// First token covered by the tick.
    pub token_start: usize,
    /// One past the last token covered by the tick.
    pub token_end: usize,
    /// Author words covered by the tick.
    pub words: usize,
    /// The tick's size relative to the largest non-heading tick, in
    /// permille (`0..=1000`). Headings report `1000`.
    pub weight: u16,
}

/// The sections, blocks and timing of a [`Script`].
///
/// Sections start at headings of the *section level*: the smallest heading
/// level that occurs at least twice counting all shallower headings, so a
/// lone title heading above several chapter headings does not swallow the
/// whole document. Deeper headings stay inside their section as ordinary
/// blocks. Text before the first section heading forms an untitled section.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Outline {
    /// Every block of the script, in order.
    pub blocks: Vec<OutlineBlock>,
    /// Sections, in order. Every block belongs to exactly one.
    pub sections: Vec<Section>,
    /// Author words read (markers and later pieces of split words excluded).
    pub words: usize,
    /// Number of tokens.
    pub tokens: usize,
    /// Number of sentences.
    pub sentences: usize,
    /// Reading time of the whole script.
    pub timing: Timing,
    #[cfg_attr(feature = "serde", serde(skip))]
    prefix: Vec<Timing>,
}

impl Outline {
    /// The outline of `script`, which was built from `doc`, timed with
    /// `pacing`'s word rules (its speed does not matter).
    pub fn new(doc: &Document, script: &Script, pacing: &Pacing) -> Outline {
        let tokens = script.tokens();
        let mut prefix = Vec::with_capacity(tokens.len() + 1);
        let mut total = Timing::ZERO;
        prefix.push(total);
        for token in tokens {
            let t = match pacing.token_permille(token) {
                Some(beats) => Timing { beats, fixed_ms: 0 },
                None => Timing {
                    beats: 0,
                    fixed_ms: u64::from(pacing.marker_ms),
                },
            };
            total = total.add(t);
            prefix.push(total);
        }

        let level_of = |b: usize| -> Option<u8> {
            doc.blocks
                .get(b)
                .map(|block| block.level.unwrap_or(1).clamp(1, 6))
        };
        let section_level = section_level(
            script
                .blocks()
                .iter()
                .filter(|b| b.kind == BlockKind::Heading)
                .filter_map(|b| level_of(b.index)),
        );

        let mut outline = Outline {
            tokens: tokens.len(),
            sentences: script.sentence_count(),
            timing: total,
            ..Outline::default()
        };
        for sb in script.blocks() {
            let level = (sb.kind == BlockKind::Heading)
                .then(|| level_of(sb.index))
                .flatten();
            let starts_section = level.is_some_and(|l| section_level.is_some_and(|s| l <= s));
            if starts_section || outline.sections.is_empty() {
                let at = outline.blocks.len();
                outline.sections.push(Section {
                    title: starts_section.then(|| String::from(sb.text.trim())),
                    level: if starts_section { level } else { None },
                    block_start: at,
                    block_end: at,
                    token_start: sb.token_start,
                    token_end: sb.token_start,
                    words: 0,
                    timing: Timing::ZERO,
                });
            }
            let words = tokens[sb.token_start..sb.token_end]
                .iter()
                .filter(|t| t.kind == TokenKind::Word && !t.joined)
                .count();
            let timing = prefix[sb.token_end].sub(prefix[sb.token_start]);
            let section = outline.sections.len() - 1;
            let s = &mut outline.sections[section];
            s.block_end += 1;
            s.token_end = sb.token_end;
            s.words += words;
            s.timing = s.timing.add(timing);
            outline.words += words;
            outline.blocks.push(OutlineBlock {
                block: sb.index,
                kind: sb.kind,
                level,
                section,
                token_start: sb.token_start,
                token_end: sb.token_end,
                words,
                timing,
            });
        }
        outline.prefix = prefix;
        outline
    }

    /// Index in [`Outline::blocks`] of the block holding `token`, or `None`
    /// if out of range.
    pub fn block_of(&self, token: usize) -> Option<usize> {
        if token >= self.tokens {
            return None;
        }
        Some(
            self.blocks
                .partition_point(|b| b.token_start <= token)
                .saturating_sub(1),
        )
    }

    /// Index in [`Outline::sections`] of the section holding `token`.
    pub fn section_of(&self, token: usize) -> Option<usize> {
        self.block_of(token).map(|b| self.blocks[b].section)
    }

    /// Reading time of tokens `start..end` (clamped to the script).
    pub fn timing_between(&self, start: usize, end: usize) -> Timing {
        let end = end.min(self.tokens);
        let start = start.min(end);
        match (self.prefix.get(end), self.prefix.get(start)) {
            (Some(e), Some(s)) => e.sub(*s),
            _ => Timing::ZERO,
        }
    }

    /// Reading time from `token` (inclusive) to the end of the script.
    pub fn remaining_from(&self, token: usize) -> Timing {
        self.timing_between(token, self.tokens)
    }

    /// Reading time from `token` (inclusive) to the end of its section;
    /// zero if out of range.
    pub fn section_remaining_from(&self, token: usize) -> Timing {
        match self.section_of(token) {
            Some(s) => self.timing_between(token, self.sections[s].token_end),
            None => Timing::ZERO,
        }
    }

    /// A document map: one tick per section heading, and the other blocks
    /// of each section grouped into consecutive runs. When there is room
    /// every block gets its own tick; otherwise each section gets a share
    /// of the room left after the headings, in proportion to its number of
    /// blocks and at least one, and its blocks are split into that many runs
    /// of roughly equal word count. The result has at most `max_ticks`
    /// ticks, or two per section if that is more. `max_ticks == 0` means no
    /// limit.
    pub fn terrain(&self, max_ticks: usize) -> Vec<Tick> {
        let headings = self.sections.iter().filter(|s| s.title.is_some()).count();
        let body_blocks = self.blocks.len() - headings;
        let room = if max_ticks == 0 || self.blocks.len() <= max_ticks {
            usize::MAX
        } else {
            max_ticks.saturating_sub(headings).max(1)
        };

        let mut ticks = Vec::new();
        for (si, section) in self.sections.iter().enumerate() {
            let mut first = section.block_start;
            if section.title.is_some() {
                let b = &self.blocks[first];
                ticks.push(Tick {
                    heading: true,
                    kind: b.kind,
                    section: si,
                    token_start: b.token_start,
                    token_end: b.token_end,
                    words: b.words,
                    weight: 1000,
                });
                first += 1;
            }
            let body = &self.blocks[first..section.block_end];
            if body.is_empty() {
                continue;
            }
            let share = if room == usize::MAX {
                body.len()
            } else {
                (body.len() * room / body_blocks.max(1)).clamp(1, body.len())
            };
            group(body, share, si, &mut ticks);
        }

        let largest = ticks
            .iter()
            .filter(|t| !t.heading)
            .map(|t| t.words)
            .max()
            .unwrap_or(0)
            .max(1);
        for t in ticks.iter_mut().filter(|t| !t.heading) {
            t.weight = u16::try_from(t.words * 1000 / largest).unwrap_or(1000);
        }
        ticks
    }
}

/// Index of the tick in `ticks` that covers `token`, or `None` if none does.
pub fn tick_of(ticks: &[Tick], token: usize) -> Option<usize> {
    let i = ticks
        .partition_point(|t| t.token_start <= token)
        .checked_sub(1)?;
    (token < ticks[i].token_end).then_some(i)
}

/// The section level for headings of `levels` (see [`Outline`]).
fn section_level(levels: impl Iterator<Item = u8>) -> Option<u8> {
    let mut counts = [0usize; 7];
    let mut any = false;
    for l in levels {
        counts[usize::from(l)] += 1;
        any = true;
    }
    if !any {
        return None;
    }
    let mut seen = 0;
    let mut min = None;
    for (level, &count) in counts.iter().enumerate().skip(1) {
        if count == 0 {
            continue;
        }
        min.get_or_insert(level as u8);
        seen += count;
        if seen >= 2 {
            return Some(level as u8);
        }
    }
    min
}

/// Splits `body` into `parts` consecutive runs of roughly equal word count
/// (every run has at least one block) and appends them as ticks.
fn group(body: &[OutlineBlock], parts: usize, section: usize, ticks: &mut Vec<Tick>) {
    let total: usize = body.iter().map(|b| b.words.max(1)).sum();
    let mut start = 0;
    let mut acc = 0;
    let mut made = 0;
    for (i, b) in body.iter().enumerate() {
        acc += b.words.max(1);
        let left_blocks = body.len() - i - 1;
        let left_parts = parts - made - 1;
        let target = total * (made + 1) / parts;
        let last = i + 1 == body.len();
        if last || (left_parts > 0 && (acc >= target || left_blocks == left_parts)) {
            let run = &body[start..=i];
            ticks.push(Tick {
                heading: false,
                kind: run[0].kind,
                section,
                token_start: run[0].token_start,
                token_end: run[run.len() - 1].token_end,
                words: run.iter().map(|b| b.words).sum(),
                weight: 0,
            });
            made += 1;
            start = i + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, Player};
    use alloc::format;
    use alloc::vec;

    fn essay() -> Document {
        Document::from_blocks(vec![
            Block::heading(1, "Nature"),
            Block::paragraph("A short preface. It has two sentences."),
            Block::heading(2, "Introduction"),
            Block::paragraph("Our age is retrospective."),
            Block::paragraph("It builds the sepulchres of the fathers."),
            Block::heading(3, "An aside"),
            Block::paragraph("Deeper headings stay inside."),
            Block::heading(2, "Chapter I"),
            Block::new(BlockKind::Code, "fn main() {}"),
            Block::paragraph("To go into solitude, a man needs to retire."),
        ])
    }

    fn outline(doc: &Document) -> (Outline, Player) {
        let script = Script::new(doc);
        let player = Player::new(script, Pacing::default());
        let outline = Outline::new(doc, player.script(), player.pacing());
        (outline, player)
    }

    #[test]
    fn sections_split_at_the_section_level() {
        let (o, _) = outline(&essay());
        let titles: Vec<_> = o.sections.iter().map(|s| s.title.as_deref()).collect();
        assert_eq!(
            titles,
            [Some("Nature"), Some("Introduction"), Some("Chapter I")]
        );
        assert_eq!(o.sections[0].level, Some(1));
        assert_eq!(o.sections[1].level, Some(2));
        assert_eq!(o.sections[0].block_end - o.sections[0].block_start, 2);
        assert_eq!(o.sections[1].block_end - o.sections[1].block_start, 5);
        assert_eq!(o.sections[2].block_end - o.sections[2].block_start, 3);
    }

    #[test]
    fn words_and_ranges_add_up() {
        let doc = essay();
        let (o, player) = outline(&doc);
        assert_eq!(o.words, crate::session::word_count(player.script()));
        assert_eq!(o.words, doc.word_count);
        assert_eq!(o.sections.iter().map(|s| s.words).sum::<usize>(), o.words);
        assert_eq!(o.tokens, player.script().len());
        let mut next = 0;
        for s in &o.sections {
            assert_eq!(s.token_start, next);
            next = s.token_end;
        }
        assert_eq!(next, o.tokens);
        assert_eq!(o.blocks[8].kind, BlockKind::Code);
        assert_eq!(o.blocks[8].words, 0);
        assert_eq!(o.blocks[8].timing.fixed_ms, 1500);
    }

    #[test]
    fn timing_matches_the_player_within_a_millisecond_per_token() {
        let (o, mut player) = outline(&essay());
        for wpm in [100, 300, 451, 1500] {
            player.set_wpm(wpm);
            let total = player.total_ms();
            let estimate = o.timing.ms_at(wpm);
            assert!(estimate >= total, "{wpm}: {estimate} < {total}");
            assert!(estimate - total <= o.tokens as u64, "{wpm}");
            assert_eq!(
                o.timing.ms_with(player.pacing(), 5000),
                o.timing.ms_at(1500)
            );
        }
    }

    #[test]
    fn remaining_time_counts_from_the_token() {
        let (o, mut player) = outline(&essay());
        assert_eq!(o.remaining_from(0), o.timing);
        assert_eq!(o.remaining_from(o.tokens), Timing::ZERO);
        player.seek(5);
        let est = o.remaining_from(5).ms_at(player.wpm());
        let exact = player.remaining_ms();
        assert!(est >= exact && est - exact <= o.tokens as u64);
        let s = o.section_of(5).unwrap();
        assert_eq!(
            o.section_remaining_from(o.sections[s].token_start),
            o.sections[s].timing
        );
        assert_eq!(o.section_of(o.tokens), None);
    }

    #[test]
    fn block_and_section_lookup() {
        let (o, _) = outline(&essay());
        for (bi, b) in o.blocks.iter().enumerate() {
            for t in b.token_start..b.token_end {
                assert_eq!(o.block_of(t), Some(bi));
                assert_eq!(o.section_of(t), Some(b.section));
            }
        }
    }

    #[test]
    fn text_before_the_first_heading_is_an_untitled_section() {
        let doc = Document::from_blocks(vec![
            Block::paragraph("Preface words."),
            Block::heading(2, "One"),
            Block::paragraph("Body."),
        ]);
        let (o, _) = outline(&doc);
        let titles: Vec<_> = o.sections.iter().map(|s| s.title.as_deref()).collect();
        assert_eq!(titles, [None, Some("One")]);
        assert_eq!(o.sections[0].words, 2);
    }

    #[test]
    fn a_document_without_headings_is_one_untitled_section() {
        let doc = Document::from_plain_text("One.\n\nTwo words.\n\nThree more words.");
        let (o, _) = outline(&doc);
        assert_eq!(o.sections.len(), 1);
        assert_eq!(o.sections[0].title, None);
        let ticks = o.terrain(0);
        assert_eq!(ticks.len(), 3);
        assert!(ticks.iter().all(|t| !t.heading));
        assert_eq!(ticks[2].weight, 1000);
        assert_eq!(ticks[0].weight, 333);
    }

    #[test]
    fn an_empty_document_has_an_empty_outline() {
        let (o, _) = outline(&Document::default());
        assert!(o.sections.is_empty() && o.blocks.is_empty());
        assert!(o.terrain(10).is_empty());
        assert_eq!(o.block_of(0), None);
        assert_eq!(o.remaining_from(0), Timing::ZERO);
    }

    #[test]
    fn terrain_has_one_tick_per_block_when_there_is_room() {
        let (o, _) = outline(&essay());
        let ticks = o.terrain(64);
        assert_eq!(ticks.len(), o.blocks.len());
        assert_eq!(ticks.iter().filter(|t| t.heading).count(), 3);
        assert!(ticks[2].heading && ticks[2].section == 1);
        assert_eq!(ticks[5].kind, BlockKind::Heading);
        assert!(!ticks[5].heading, "a deeper heading is not a section tick");
    }

    #[test]
    fn terrain_groups_blocks_to_fit() {
        let mut blocks = Vec::new();
        for s in 0..9 {
            blocks.push(Block::heading(2, format!("Section {s}")));
            for p in 0..40 {
                let words = 3 + (p * 7 + s) % 30;
                let text: Vec<&str> = core::iter::repeat_n("word", words).collect();
                blocks.push(Block::paragraph(text.join(" ")));
            }
        }
        let doc = Document::from_blocks(blocks);
        let (o, _) = outline(&doc);
        for max in [12, 60, 100, 200] {
            let ticks = o.terrain(max);
            assert!(ticks.len() <= max.max(9 * 2), "{max}: {}", ticks.len());
            assert_eq!(ticks.iter().filter(|t| t.heading).count(), 9);
            let mut next = 0;
            for t in &ticks {
                assert_eq!(t.token_start, next, "{max}: contiguous");
                next = t.token_end;
            }
            assert_eq!(next, o.tokens);
            assert_eq!(ticks.iter().map(|t| t.words).sum::<usize>(), o.words);
            assert!(ticks.iter().any(|t| t.weight == 1000 && !t.heading));
        }
        assert_eq!(o.terrain(0).len(), o.blocks.len());
    }

    #[test]
    fn tick_lookup() {
        let (o, _) = outline(&essay());
        let ticks = o.terrain(4);
        for t in 0..o.tokens {
            let i = tick_of(&ticks, t).unwrap();
            assert!(ticks[i].token_start <= t && t < ticks[i].token_end);
        }
        assert_eq!(tick_of(&ticks, o.tokens), None);
        assert_eq!(tick_of(&[], 0), None);
    }

    #[test]
    fn section_level_rule() {
        assert_eq!(section_level([].into_iter()), None);
        assert_eq!(section_level([1].into_iter()), Some(1));
        assert_eq!(section_level([1, 2, 2].into_iter()), Some(2));
        assert_eq!(section_level([1, 1, 2].into_iter()), Some(1));
        assert_eq!(section_level([3, 2].into_iter()), Some(3));
    }
}
