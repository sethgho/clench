//! Deterministic rules for page furniture that survives main-content
//! detection. Every rule keeps or drops whole elements or blocks; none edits
//! text.

use dom_smoothie::dom_query::NodeRef;
use meatread_core::{Block, BlockKind};

use crate::text::word_count;

/// Class-token prefixes of labels that sit above a headline ("kicker").
const LABEL_PREFIXES: &[&str] = &["kicker", "eyebrow", "overline", "upper-deck"];

/// Class tokens of maintenance markers and print-hidden furniture
/// ("[citation needed]", section edit links).
const NOISE_CLASSES: &[&str] = &["noprint", "inline-template", "mw-editsection"];

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

/// Phrases that mark a closing promotional or syndication paragraph.
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
pub(super) fn is_furniture(n: &NodeRef) -> bool {
    let tokens = |attr: &str| -> Vec<String> {
        n.attr(attr)
            .map(|v| v.split_whitespace().map(str::to_ascii_lowercase).collect())
            .unwrap_or_default()
    };
    let classes = tokens("class");
    if classes.iter().any(|c| {
        NOISE_CLASSES.contains(&c.as_str()) || LABEL_PREFIXES.iter().any(|p| c.starts_with(p))
    }) {
        return true;
    }
    let named = |v: &String| MODULE_NAMES.contains(&v.as_str());
    classes.iter().any(named)
        || tokens("id").iter().any(named)
        || tokens("data-area").iter().any(named)
}

/// A `<time>` that stands alone among its siblings is a dateline, which is
/// page metadata rather than prose.
pub(super) fn is_standalone_time(n: &NodeRef) -> bool {
    n.has_name("time")
        && n.parent().is_some_and(|p| {
            p.children()
                .iter()
                .all(|c| !c.is_text() || c.text().trim().is_empty())
        })
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

/// Drops short promotional paragraphs from the very end of the body.
pub(super) fn trim_tail(blocks: &mut Vec<Block>) {
    let is_tail = |b: &Block| {
        b.kind == BlockKind::Paragraph && word_count(&b.text) <= TAIL_MAX_WORDS && {
            let t = b.text.to_lowercase();
            TAIL_PHRASES.iter().any(|p| t.contains(p))
        }
    };
    while blocks.last().is_some_and(is_tail) {
        blocks.pop();
    }
}
