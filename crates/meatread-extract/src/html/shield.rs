//! Protects running text from Readability's attribute heuristics.
//!
//! Readability removes any element whose `class` or `id` looks like page
//! furniture ("related", "social", "comment", "author", "share", ...), and
//! the first element that looks like a byline. That is right for blocks but
//! wrong for an inline element inside a sentence: `<span class="comment">`
//! around two words of a paragraph would delete them. Before Readability
//! runs, such inline elements get their hook attributes renamed to
//! `data-mr-*` copies, which Readability ignores and our own rules read back
//! through [`class_value`]. Likewise a `<button>` that carries words inside a
//! sentence, with text right beside it (a link-styled control), becomes a `<span>`, since Readability
//! removes buttons outright.

use std::borrow::Cow;

use dom_smoothie::dom_query::{Document as Dom, NodeRef};

use super::chrome::{block_container, has_adjacent_text, is_inline_element};
use crate::text::word_count;

/// Name of the attribute that carries a shielded `class`.
const SHIELDED_CLASS: &str = "data-mr-class";

/// Attribute substrings after which Readability removes an element.
const HOOKS: &[&str] = &[
    "-ad-",
    "ai2html",
    "banner",
    "breadcrumbs",
    "combx",
    "comment",
    "community",
    "cover-wrap",
    "disqus",
    "extra",
    "footer",
    "gdpr",
    "header",
    "legends",
    "menu",
    "related",
    "remark",
    "replies",
    "rss",
    "shoutbox",
    "sidebar",
    "skyscraper",
    "social",
    "sponsor",
    "supplemental",
    "ad-break",
    "agegate",
    "pagination",
    "pager",
    "popup",
    "yom-remote",
    "share",
    "byline",
    "author",
    "dateline",
    "writtenby",
];

/// A block must hold at least this many words to count as running text.
const RUNNING_TEXT_WORDS: usize = 12;

/// The `class` attribute of `n`, whether or not it was shielded.
pub(super) fn class_value(n: &NodeRef) -> Option<String> {
    n.attr("class")
        .or_else(|| n.attr(SHIELDED_CLASS))
        .map(|c| c.to_string())
}

fn hooked(n: &NodeRef) -> bool {
    let hit = |attr: &str| {
        n.attr(attr).is_some_and(|v| {
            let v = v.to_ascii_lowercase();
            HOOKS.iter().any(|h| v.contains(h))
        })
    };
    hit("class")
        || hit("id")
        || n.attr("rel")
            .is_some_and(|r| r.eq_ignore_ascii_case("author"))
        || hit("itemprop")
}

/// `html` with hook attributes of inline elements inside running text
/// renamed. Returns the input untouched when nothing needed shielding.
pub(super) fn run(html: &str) -> Cow<'_, str> {
    let dom = Dom::from(html);
    let mut changed = false;
    for n in dom.select("[class], [id], [rel], [itemprop]").nodes() {
        if !is_inline_element(n) || !hooked(n) {
            continue;
        }
        let in_block = word_count(&block_container(n).text());
        if in_block < RUNNING_TEXT_WORDS || word_count(&n.text()) >= in_block {
            continue;
        }
        for (from, to) in [
            ("class", SHIELDED_CLASS),
            ("id", "data-mr-id"),
            ("rel", "data-mr-rel"),
            ("itemprop", "data-mr-itemprop"),
        ] {
            if let Some(v) = n.attr(from) {
                n.remove_attr(from);
                n.set_attr(to, &v);
                changed = true;
            }
        }
    }
    for n in dom.select("button").nodes() {
        if has_adjacent_text(n) {
            n.rename("span");
            changed = true;
        }
    }
    if changed {
        Cow::Owned(dom.html().to_string())
    } else {
        Cow::Borrowed(html)
    }
}
