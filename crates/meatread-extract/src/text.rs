//! Whitespace handling shared by every extractor.

use meatread_core::{Block, BlockKind};

/// HTML's definition of white space: space, tab, line feed, form feed and
/// carriage return. Notably this excludes U+00A0 and every other Unicode
/// space, which are content.
pub(crate) fn is_html_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{c}' | '\r')
}

/// Collapses runs of HTML white space to a single space and trims the ends.
/// Nothing else about the text changes.
pub(crate) fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for word in s.split(is_html_space).filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// Whether `c` separates words. Mirrors the tokenizer in `meatread-core`:
/// Unicode white space separates words except the no-break spaces.
fn is_word_separator(c: char) -> bool {
    c.is_whitespace() && !matches!(c, '\u{a0}' | '\u{2007}' | '\u{202f}')
}

/// Number of whitespace-delimited words in `s`.
pub(crate) fn word_count(s: &str) -> usize {
    s.split(is_word_separator).filter(|w| !w.is_empty()).count()
}

/// Words in the blocks a reader plays by default (those not flagged `skip`).
pub(crate) fn readable_words(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .filter(|b| !b.skip)
        .map(|b| word_count(&b.text))
        .sum()
}

/// Whether blocks of `kind` are skipped by default.
pub(crate) fn skipped_by_default(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Code
            | BlockKind::Table
            | BlockKind::Figure
            | BlockKind::Caption
            | BlockKind::Rule
    )
}

/// A block of `kind` with the default `skip` hint for that kind.
pub(crate) fn block(kind: BlockKind, text: String) -> Block {
    Block {
        kind,
        text,
        skip: skipped_by_default(kind),
        ..Block::default()
    }
}

/// Paragraphs split on blank lines, each with its internal whitespace
/// collapsed. Used for sources that carry text without markup.
pub(crate) fn paragraphs(s: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut current = String::new();
    for line in s.lines() {
        if line.trim_matches(is_html_space).is_empty() {
            if !current.is_empty() {
                out.push(block(BlockKind::Paragraph, collapse(&current)));
                current.clear();
            }
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }
    if !current.is_empty() {
        out.push(block(BlockKind::Paragraph, collapse(&current)));
    }
    out
}
