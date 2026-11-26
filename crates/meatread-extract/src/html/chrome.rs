//! Deterministic rules for page furniture that survives main-content
//! detection. Every rule keeps or drops whole elements or blocks; none edits
//! text.

use std::collections::HashSet;

use dom_smoothie::dom_query::{NodeRef, Selection};
use meatread_core::{Block, BlockKind};

use crate::text::{collapse, word_count};

/// Class-token prefixes of labels that sit above a headline ("kicker").
const LABEL_PREFIXES: &[&str] = &["kicker", "eyebrow", "upper-deck"];

/// Class-token prefix of labels that is also a common text-decoration utility,
/// and so is only trusted on a block-level element outside any paragraph.
const AMBIGUOUS_LABEL_PREFIX: &str = "overline";

/// Class tokens of maintenance markers ("[citation needed]", section edit
/// links). These are MediaWiki's own names, not general styling words.
const NOISE_CLASSES: &[&str] = &["inline-template", "mw-editsection"];

/// A label or module with more words than this is treated as content.
const LABEL_MAX_WORDS: usize = 20;

/// A recirculation module that holds a paragraph at least this long is
/// treated as content.
const MODULE_MAX_PARAGRAPH_WORDS: usize = 60;

/// Class, id or `data-area` values naming recirculation modules and
/// subscription prompts.
const MODULE_NAMES: &[&str] = &[
    "related",
    "related-posts",
    "related-articles",
    "related_articles",
    "related-stories",
    "recirculation",
    "recommendations",
    "read-next",
    "newsletter-signup",
    // Subscription prompts that replace the rest of a gated article.
    "post-access-cta",
    "gh-post-upgrade-cta",
];

/// Short lines publishers put on their own around ads.
const AD_LABELS: &[&str] = &["advertisement", "sponsored", "sponsored content"];

/// Phrases that mark a closing promotional or syndication note.
const TAIL_PHRASES: &[&str] = &[
    "originally appeared",
    "originally published",
    "subscribe to",
    "sign up for",
    "follow us on",
    "all rights reserved",
];

/// A closing paragraph with more words than this is treated as content.
const TAIL_MAX_WORDS: usize = 60;

/// Whether an element is a headline label, maintenance marker, recirculation
/// module or subscription prompt.
///
/// Class names alone are weak evidence (`overline` is also a text-decoration
/// utility). A label must be short, free of block-level children and the sole
/// text of its block, so it can never take words out of running text; the
/// ambiguous `overline` must also be a block-level element. A module must be
/// a block-level element outside any paragraph and is kept if it contains a
/// long paragraph. Maintenance markers carry MediaWiki-specific names and are
/// dropped wherever they appear.
pub(super) fn is_furniture(n: &NodeRef) -> bool {
    let tokens = |attr: &str| -> Vec<String> {
        let value = if attr == "class" {
            super::shield::class_value(n)
        } else {
            n.attr(attr).map(|v| v.to_string())
        };
        value
            .map(|v| v.split_whitespace().map(str::to_ascii_lowercase).collect())
            .unwrap_or_default()
    };
    let classes = tokens("class");
    if classes.iter().any(|c| NOISE_CLASSES.contains(&c.as_str())) {
        return true;
    }
    let has_prefix = |p: &str| classes.iter().any(|c| c.starts_with(p));
    if is_short_leaf(n, LABEL_MAX_WORDS)
        && ((LABEL_PREFIXES.iter().any(|p| has_prefix(p)) && owns_its_block(n))
            || (has_prefix(AMBIGUOUS_LABEL_PREFIX) && is_standalone_block(n)))
    {
        return true;
    }
    if !is_standalone_block(n) {
        return false;
    }
    let named = |v: &String| MODULE_NAMES.contains(&v.as_str());
    (classes.iter().any(named)
        || tokens("id").iter().any(named)
        || tokens("data-area").iter().any(named))
        && !has_long_paragraph(n, MODULE_MAX_PARAGRAPH_WORDS)
}

/// A block-level element that is not inside a paragraph.
fn is_standalone_block(n: &NodeRef) -> bool {
    !is_inline_element(n) && !n.ancestors_it(None).any(|a| a.has_name("p"))
}

const BLOCK_SELECTOR: &str = "p, div, ul, ol, li, table, pre, blockquote, section, article, \
    h1, h2, h3, h4, h5, h6, figure, header, footer, aside, main, nav";

/// Whether `n` has no block-level descendants and at most `max_words` words.
pub(super) fn is_short_leaf(n: &NodeRef, max_words: usize) -> bool {
    !Selection::from(*n).select(BLOCK_SELECTOR).exists() && word_count(&n.text()) <= max_words
}

fn has_long_paragraph(n: &NodeRef, words: usize) -> bool {
    Selection::from(*n)
        .select("p")
        .nodes()
        .iter()
        .any(|p| word_count(&p.text()) >= words)
}

/// Elements that flow within a line of text. Anything else starts a new block.
const INLINE_ELEMENTS: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "cite", "code", "data", "del", "dfn", "em", "font", "i", "ins",
    "kbd", "label", "mark", "q", "s", "samp", "small", "span", "strike", "strong", "sub", "sup",
    "time", "tt", "u", "var",
];

pub(super) fn is_inline_element(n: &NodeRef) -> bool {
    n.node_name()
        .is_some_and(|name| INLINE_ELEMENTS.contains(&name.to_ascii_lowercase().as_str()))
}

/// The nearest block-level element containing `n` (`n` itself if it is one).
pub(super) fn block_container<'a>(n: &NodeRef<'a>) -> NodeRef<'a> {
    let mut cur = *n;
    while cur.is_element() && is_inline_element(&cur) {
        match cur.parent() {
            Some(p) if p.is_element() => cur = p,
            _ => break,
        }
    }
    cur
}

/// Whether `n` holds all the text of its block: a bare label rather than
/// part of a sentence.
pub(super) fn owns_its_block(n: &NodeRef) -> bool {
    collapse(&block_container(n).text()) == collapse(&n.text())
}

/// A `<time>` that is the only text of its block is a dateline, which is page
/// metadata rather than prose. A `<time>` inside a sentence is part of it.
pub(super) fn is_standalone_time(n: &NodeRef) -> bool {
    n.has_name("time") && owns_its_block(n)
}

/// Characters that, alone in a block, are decoration rather than text.
const SEPARATOR_CHARS: &[char] = &['•', '·', '|', '-', '–', '—', '*', '/', '→', '»', '›'];

/// Drops blocks made only of separator characters (stray bullets between
/// meta items) and standalone advertisement labels.
pub(super) fn drop_noise(blocks: &mut Vec<Block>) {
    blocks.retain(|b| {
        if b.kind == BlockKind::Rule || b.kind == BlockKind::Code {
            return true;
        }
        let lower = b.text.trim().to_lowercase();
        let separators_only = b
            .text
            .chars()
            .all(|c| c.is_whitespace() || SEPARATOR_CHARS.contains(&c));
        !separators_only && !AD_LABELS.contains(&lower.as_str())
    });
}

/// Drops closing promotional or syndication notes from the very end of the
/// body.
///
/// A paragraph is dropped only when it is short, mentions one of the
/// promotional phrases, and is a `closing_notes` paragraph: set entirely in
/// italics and containing a link, the way publishers typeset an author bio or
/// a syndication line. A paragraph that merely uses the words ("I decided to
/// sign up for a marathon") fails the typographic test and is kept.
pub(super) fn trim_tail(blocks: &mut Vec<Block>, closing_notes: &HashSet<String>) {
    let is_tail = |b: &Block| {
        b.kind == BlockKind::Paragraph
            && word_count(&b.text) <= TAIL_MAX_WORDS
            && closing_notes.contains(&b.text)
            && {
                let t = b.text.to_lowercase();
                TAIL_PHRASES.iter().any(|p| t.contains(p))
            }
    };
    while blocks.last().is_some_and(is_tail) {
        blocks.pop();
    }
}
