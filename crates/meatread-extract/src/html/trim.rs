//! Cleanup at the edges of the extracted body.

use dom_smoothie::dom_query::Document as Dom;
use meatread_core::{Block, BlockKind, Document};

use crate::text::{block, collapse, word_count};

/// Paragraphs shorter than this many words can be page chrome.
const BODY_PARAGRAPH_WORDS: usize = 12;

/// Drops short metadata residue ("3 hours ago", "103 Comments", "8 min read",
/// a repeat of the byline) from the very start and end of the body.
pub(super) fn trim_edge_chrome(doc: &mut Document) {
    let byline = doc.byline.as_deref().map(|b| collapse(b).to_lowercase());
    let is_chrome = |b: &Block| {
        if b.kind != BlockKind::Paragraph {
            return false;
        }
        let t = b.text.to_lowercase();
        if byline.as_deref() == Some(t.as_str()) {
            return true;
        }
        let words: Vec<&str> = t.split_whitespace().collect();
        let numeric = words
            .first()
            .is_some_and(|f| f.chars().all(|c| c.is_ascii_digit() || c == ','));
        let tail = words.last().copied().unwrap_or("");
        words.len() <= 4
            && numeric
            && matches!(tail, "comments" | "comment" | "ago" | "read" | "replies")
    };
    let first_body = doc
        .blocks
        .iter()
        .position(|b| b.kind == BlockKind::Paragraph && word_count(&b.text) >= BODY_PARAGRAPH_WORDS)
        .unwrap_or(0);
    let mut index = 0;
    doc.blocks.retain(|b| {
        let keep = index >= first_body || !is_chrome(b);
        index += 1;
        keep
    });
    while doc.blocks.last().is_some_and(&is_chrome) {
        doc.blocks.pop();
    }
}

/// Many templates render the standfirst outside the scored container. If the
/// page's description is the verbatim text of an element in the page and the
/// extracted body lacks it, that text is put back at the start as a paragraph.
pub(super) fn rescue_lede(page: &Dom, doc: &mut Document) {
    let Some(excerpt) = doc.excerpt.as_deref() else {
        return;
    };
    let excerpt = collapse(excerpt);
    if word_count(&excerpt) < 8 || doc.blocks.iter().any(|b| b.text.contains(&excerpt)) {
        return;
    }
    let matches = |n: &dom_smoothie::dom_query::NodeRef| collapse(&n.text()) == excerpt;
    let found = page.select("p").nodes().iter().any(matches)
        || page
            .select("h2, h3, div, span")
            .nodes()
            .iter()
            .any(|n| n.children().iter().all(|c| !c.is_element()) && matches(n));
    if found {
        doc.blocks.insert(0, block(BlockKind::Paragraph, excerpt));
    }
}
